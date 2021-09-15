use std::{any, mem};

pub mod mesh;
#[cfg(feature = "winit")]
pub mod window;

/// Can be specified as 0xAARRGGBB
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd)]
pub struct Color(pub u32);

impl Color {
    pub const BLACK_TRANSPARENT: Self = Self(0x0);
    pub const BLACK_OPAQUE: Self = Self(0xFF000000);
    pub const RED: Self = Self(0xFF0000FF);
    pub const GREEN: Self = Self(0xFF00FF00);
    pub const BLUE: Self = Self(0xFFFF0000);

    fn import(value: f32) -> u32 {
        (value.clamp(0.0, 1.0) * 255.0) as u32
    }

    pub fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self(
            (Self::import(alpha) << 24)
                | (Self::import(red) << 16)
                | (Self::import(green) << 8)
                | Self::import(blue),
        )
    }

    fn export(self, index: u32) -> f32 {
        ((self.0 >> (index << 3)) & 0xFF) as f32 / 255.0
    }
    pub fn red(self) -> f32 {
        self.export(2)
    }
    pub fn green(self) -> f32 {
        self.export(1)
    }
    pub fn blue(self) -> f32 {
        self.export(0)
    }
    pub fn alpha(self) -> f32 {
        self.export(3)
    }
}

impl From<Color> for wgpu::Color {
    fn from(c: Color) -> Self {
        Self {
            r: c.red() as f64,
            g: c.green() as f64,
            b: c.blue() as f64,
            a: c.alpha() as f64,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK_OPAQUE
    }
}

#[cfg_attr(not(feature = "winit"), allow(dead_code))]
struct SurfaceContext {
    raw: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
}

struct TargetContext {
    view: wgpu::TextureView,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TargetRef(u8);

pub struct Context {
    _instance: wgpu::Instance,
    surface: Option<SurfaceContext>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    targets: Vec<TargetContext>,
}

#[derive(Default)]
pub struct ContextBuilder<'a> {
    #[cfg(feature = "winit")]
    window: Option<&'a window::Window>,
    #[cfg(not(feature = "winit"))]
    _window: Option<&'a ()>,
    power_preference: wgpu::PowerPreference,
}

impl<'a> ContextBuilder<'a> {
    #[cfg(feature = "winit")]
    pub fn screen(self, win: &'a window::Window) -> Self {
        Self {
            window: Some(win),
            ..self
        }
    }

    pub fn power_hungry(self, hungry: bool) -> Self {
        Self {
            power_preference: if hungry {
                wgpu::PowerPreference::HighPerformance
            } else {
                wgpu::PowerPreference::LowPower
            },
            ..self
        }
    }

    pub async fn build(self) -> Context {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        #[cfg_attr(not(feature = "winit"), allow(unused_mut))]
        let (mut surface, mut targets) = (None, Vec::new());

        #[cfg(feature = "winit")]
        if let Some(win) = self.window {
            let size = win.raw.inner_size();
            let raw = unsafe { instance.create_surface(&win.raw) };
            surface = Some(SurfaceContext {
                raw,
                config: wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    // using an erroneous format, it is changed before used
                    format: wgpu::TextureFormat::Depth24Plus,
                    width: size.width,
                    height: size.height,
                    present_mode: wgpu::PresentMode::Mailbox,
                },
            });
        }

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: self.power_preference,
                #[cfg(feature = "winit")]
                compatible_surface: surface.as_ref().map(|sc| &sc.raw),
                #[cfg(not(feature = "winit"))]
                compatible_surface: None,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        #[cfg(feature = "winit")]
        if let Some(ref mut suf) = surface {
            suf.config.format = suf.raw.get_preferred_format(&adapter).unwrap();
            suf.raw.configure(&device, &suf.config);
            // create a dummy target view to occupy the first slot
            let view = device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("dummy screen"),
                    size: wgpu::Extent3d::default(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: suf.config.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                })
                .create_view(&wgpu::TextureViewDescriptor::default());
            targets.push(TargetContext { view });
        }

        Context {
            _instance: instance,
            surface,
            device,
            queue,
            targets,
        }
    }
}

impl Context {
    pub fn new<'a>() -> ContextBuilder<'a> {
        ContextBuilder::default()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let surface = match self.surface {
            Some(ref mut suf) => suf,
            None => return,
        };
        if (surface.config.width, surface.config.height) == (width, height) {
            return;
        }
        surface.config.width = width;
        surface.config.height = height;
        surface.raw.configure(&self.device, &surface.config);
    }

    pub fn present<P: Pass>(&mut self, pass: &mut P, scene: &Scene) {
        let surface = self.surface.as_mut().expect("No screen is configured!");
        let frame = surface.raw.get_current_frame().unwrap();
        let view = frame
            .output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let dummy = mem::replace(&mut self.targets[0].view, view);

        pass.draw(&[TargetRef::default()], scene, self);

        self.targets[0].view = dummy;
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Do we need explicit cleanup?
    }
}

/// Trait that exposes `Context` details that depend on `wgpu`
pub trait ContextDetail {
    fn get_target(&self, tr: TargetRef) -> &wgpu::TextureView;
    fn device(&self) -> &wgpu::Device;
    fn queue(&self) -> &wgpu::Queue;
}

impl ContextDetail for Context {
    fn get_target(&self, tr: TargetRef) -> &wgpu::TextureView {
        &self.targets[tr.0 as usize].view
    }
    fn device(&self) -> &wgpu::Device {
        &self.device
    }
    fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

pub trait Pass {
    fn draw(&mut self, targets: &[TargetRef], scene: &Scene, context: &Context);
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeRef(u32);

#[derive(Debug, PartialEq)]
struct Space {
    position: mint::Vector3<f32>,
    scale: f32,
    orientation: mint::Quaternion<f32>,
}

impl Default for Space {
    fn default() -> Self {
        Self {
            position: mint::Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            scale: 1.0,
            orientation: mint::Quaternion {
                s: 1.0,
                v: mint::Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            },
        }
    }
}

#[derive(Default, Debug, PartialEq)]
struct Node {
    parent: NodeRef,
    local: Space,
}

pub type EntityRef = hecs::Entity;

#[derive(Default)]
pub struct Scene {
    world: hecs::World,
    nodes: Vec<Node>,
    pub background: Color,
}

impl Scene {
    fn add_node(&mut self, node: Node) -> NodeRef {
        if node.local == Space::default() {
            node.parent
        } else {
            let index = self.nodes.len();
            self.nodes.push(node);
            NodeRef(index as u32)
        }
    }

    pub fn entity(&mut self) -> ObjectBuilder<hecs::EntityBuilder> {
        ObjectBuilder {
            scene: self,
            node: Node::default(),
            kind: hecs::EntityBuilder::new(),
        }
    }
}

pub struct ObjectBuilder<'a, T> {
    scene: &'a mut Scene,
    node: Node,
    kind: T,
}

impl<T> ObjectBuilder<'_, T> {
    pub fn position(mut self, position: mint::Vector3<f32>) -> Self {
        self.node.local.position = position;
        self
    }
}

impl ObjectBuilder<'_, ()> {
    pub fn build(self) -> NodeRef {
        self.scene.add_node(self.node)
    }
}

impl ObjectBuilder<'_, hecs::EntityBuilder> {
    /// Register a new material component with this entity.
    ///
    /// The following components are recognized by the library:
    ///   - [`Color`]
    pub fn component<T: hecs::Component>(mut self, component: T) -> Self {
        self.kind.add(component);
        self
    }

    pub fn build(mut self) -> EntityRef {
        let node = self.scene.add_node(self.node);
        let built = self.kind.add(node).build();
        self.scene.world.spawn(built)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeshRef(u32);

struct IndexStream {
    offset: wgpu::BufferAddress,
    format: wgpu::IndexFormat,
    count: usize,
}

struct VertexStream {
    type_id: any::TypeId,
    offset: wgpu::BufferAddress,
    stride: wgpu::BufferAddress,
}

struct Mesh {
    buffer: wgpu::Buffer,
    index_stream: IndexStream,
    vertex_streams: Box<[VertexStream]>,
    vertex_count: usize,
}
