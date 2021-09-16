use raw_window_handle::HasRawWindowHandle;
use std::{any::TypeId, marker::PhantomData, mem, ops};
use wgpu::util::DeviceExt as _;

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
    pub fn into_vec4(self) -> [f32; 4] {
        [self.red(), self.green(), self.blue(), self.alpha()]
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

pub trait HasWindow: HasRawWindowHandle {
    fn size(&self) -> mint::Vector2<u32>;
}

struct SurfaceContext {
    raw: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
}

pub struct Target {
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub size: wgpu::Extent3d,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetRef(u8);

pub struct Context {
    #[allow(unused)]
    instance: wgpu::Instance,
    surface: Option<SurfaceContext>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    targets: Vec<Target>,
    meshes: Vec<Mesh>,
}

#[derive(Default, Debug)]
pub struct ContextBuilder {
    power_preference: wgpu::PowerPreference,
}

impl ContextBuilder {
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

    pub async fn build_offscreen(self) -> Context {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: self.power_preference,
                compatible_surface: None,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        Context {
            instance,
            surface: None,
            device,
            queue,
            targets: Vec::new(),
            meshes: Vec::new(),
        }
    }

    pub async fn build<W: HasWindow>(self, window: &W) -> Context {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

        let size = window.size();
        let mut surface = SurfaceContext {
            raw: unsafe { instance.create_surface(window) },
            config: wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                // using an erroneous format, it is changed before used
                format: wgpu::TextureFormat::Depth24Plus,
                width: size.x,
                height: size.y,
                present_mode: wgpu::PresentMode::Mailbox,
            },
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: self.power_preference,
                compatible_surface: Some(&surface.raw),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        let format = surface.raw.get_preferred_format(&adapter).unwrap();
        surface.config.format = format;
        surface.raw.configure(&device, &surface.config);

        Context {
            instance,
            surface: Some(surface),
            device,
            queue,
            targets: Vec::new(),
            meshes: Vec::new(),
        }
    }
}

impl Context {
    pub fn init() -> ContextBuilder {
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
        self.targets[0].size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
    }

    pub fn present<P: Pass>(&mut self, pass: &mut P, scene: &Scene) {
        let surface = self.surface.as_mut().expect("No screen is configured!");
        let frame = surface.raw.get_current_frame().unwrap();
        let view = frame
            .output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let tr = TargetRef(self.targets.len() as _);
        self.targets.push(Target {
            view,
            format: surface.config.format,
            size: wgpu::Extent3d {
                width: surface.config.width,
                height: surface.config.height,
                depth_or_array_layers: 1,
            },
        });

        pass.draw(&[tr], scene, self);

        self.targets.pop();
    }

    pub fn add_mesh(&mut self) -> MeshBuilder {
        MeshBuilder {
            context: self,
            name: String::new(),
            data: Vec::new(),
            vertex_count: 0,
            index_stream: None,
            vertex_streams: Vec::new(),
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Do we need explicit cleanup?
    }
}

/// Trait that exposes `Context` details that depend on `wgpu`
pub trait ContextDetail {
    fn get_target(&self, tr: TargetRef) -> &Target;
    fn get_mesh(&self, mr: MeshRef) -> &Mesh;
    fn device(&self) -> &wgpu::Device;
    fn queue(&self) -> &wgpu::Queue;
}

impl ContextDetail for Context {
    fn get_target(&self, tr: TargetRef) -> &Target {
        &self.targets[tr.0 as usize]
    }
    fn get_mesh(&self, mr: MeshRef) -> &Mesh {
        &self.meshes[mr.0 as usize]
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
    pub world: hecs::World,
    nodes: Vec<Node>,
    pub background: Color,
}

pub struct BakedSpace {
    pub pos_scale: [f32; 4],
    pub rot: [f32; 4],
}

pub struct BakedScene {
    nodes: Box<[BakedSpace]>,
}

impl ops::Index<NodeRef> for BakedScene {
    type Output = BakedSpace;
    fn index(&self, node: NodeRef) -> &BakedSpace {
        &self.nodes[node.0 as usize]
    }
}

pub struct EntityKind {
    raw: hecs::EntityBuilder,
    mesh: MeshRef,
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

    pub fn entity(&mut self, mesh: MeshRef) -> ObjectBuilder<EntityKind> {
        ObjectBuilder {
            scene: self,
            node: Node::default(),
            kind: EntityKind {
                raw: hecs::EntityBuilder::new(),
                mesh,
            },
        }
    }

    pub fn bake(&self) -> BakedScene {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for n in self.nodes.iter() {
            unimplemented!()
        }
        BakedScene {
            nodes: nodes.into_boxed_slice(),
        }
    }
}

pub struct Entity {
    pub node: NodeRef,
    pub mesh: MeshRef,
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

impl ObjectBuilder<'_, EntityKind> {
    /// Register a new material component with this entity.
    ///
    /// The following components are recognized by the library:
    ///   - [`Color`]
    pub fn component<T: hecs::Component>(mut self, component: T) -> Self {
        self.kind.raw.add(component);
        self
    }

    pub fn build(mut self) -> EntityRef {
        let entity = Entity {
            node: self.scene.add_node(self.node),
            mesh: self.kind.mesh,
        };
        let built = self.kind.raw.add(entity).build();
        self.scene.world.spawn(built)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeshRef(u32);

pub struct IndexStream {
    pub offset: wgpu::BufferAddress,
    pub format: wgpu::IndexFormat,
    pub count: u32,
}

pub struct VertexStream {
    pub type_id: TypeId,
    pub offset: wgpu::BufferAddress,
    pub stride: wgpu::BufferAddress,
}

pub struct Mesh {
    pub buffer: wgpu::Buffer,
    pub index_stream: Option<IndexStream>,
    vertex_streams: Box<[VertexStream]>,
    pub vertex_count: u32,
}

impl Mesh {
    pub fn vertex_stream<T: 'static>(&self) -> Option<&VertexStream> {
        self.vertex_streams
            .iter()
            .find(|vs| vs.type_id == TypeId::of::<T>())
    }
}

pub struct Vertex<T>(PhantomData<T>);

pub struct MeshBuilder<'a> {
    context: &'a mut Context,
    name: String,
    data: Vec<u8>, // could be moved up to the context
    index_stream: Option<IndexStream>,
    vertex_streams: Vec<VertexStream>,
    vertex_count: usize,
}

impl MeshBuilder<'_> {
    pub fn name(self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..self
        }
    }

    fn append<T: bytemuck::Pod>(&mut self, data: &[T]) -> wgpu::BufferAddress {
        let offset = self.data.len();
        self.data.extend(bytemuck::cast_slice(data));
        offset as _
    }

    pub fn index(mut self, data: &[u16]) -> Self {
        assert!(self.index_stream.is_none());
        let offset = self.append(data);
        Self {
            index_stream: Some(IndexStream {
                offset,
                format: wgpu::IndexFormat::Uint16,
                count: data.len() as u32,
            }),
            ..self
        }
    }

    pub fn vertex<T: bytemuck::Pod>(mut self, data: &[T]) -> Self {
        let offset = self.append(data);
        if self.vertex_count == 0 {
            self.vertex_count = data.len();
        } else {
            assert_eq!(self.vertex_count, data.len());
        }
        self.vertex_streams.push(VertexStream {
            type_id: TypeId::of::<T>(),
            offset,
            stride: mem::size_of::<T>() as _,
        });
        self
    }

    pub fn build(self) -> MeshRef {
        let index = self.context.meshes.len();

        let mut usage = wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX;
        usage.set(wgpu::BufferUsages::INDEX, self.index_stream.is_some());
        let buffer = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: if self.name.is_empty() {
                    None
                } else {
                    Some(&self.name)
                },
                contents: &self.data,
                usage,
            });

        self.context.meshes.push(Mesh {
            buffer,
            index_stream: self.index_stream,
            vertex_streams: self.vertex_streams.into_boxed_slice(),
            vertex_count: self.vertex_count as u32,
        });
        MeshRef(index as u32)
    }
}
