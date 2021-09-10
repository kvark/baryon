/// Order of components is: A, R, G, B
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

pub struct Window {
    event_loop: winit::event_loop::EventLoop<()>,
    raw: winit::window::Window,
}

#[derive(Default)]
pub struct WindowBuilder {
    title: Option<String>,
    size: Option<wgpu::Extent3d>,
}

impl Window {
    pub fn new() -> WindowBuilder {
        WindowBuilder::default()
    }
}

impl WindowBuilder {
    pub fn title(self, title: &str) -> Self {
        Self {
            title: Some(title.to_string()),
            ..self
        }
    }

    pub fn size(self, width: u32, height: u32) -> Self {
        Self {
            size: Some(wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            }),
            ..self
        }
    }

    pub fn build(self) -> Window {
        let event_loop = winit::event_loop::EventLoop::new();
        let mut builder = winit::window::WindowBuilder::new();
        if let Some(title) = self.title {
            builder = builder.with_title(title);
        }
        if let Some(size) = self.size {
            builder = builder
                .with_inner_size(winit::dpi::Size::Physical((size.width, size.height).into()));
        }
        let raw = builder.build(&event_loop).unwrap();
        Window { raw, event_loop }
    }
}

struct SurfaceContext {
    raw: wgpu::Surface,
    format: wgpu::TextureFormat,
    size: wgpu::Extent3d,
}

pub struct Context {
    instance: wgpu::Instance,
    surface: Option<SurfaceContext>,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

#[derive(Default)]
pub struct ContextBuilder<'a> {
    window: Option<&'a Window>,
    power_preference: wgpu::PowerPreference,
}

impl<'a> ContextBuilder<'a> {
    pub fn screen(self, win: &'a Window) -> Self {
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
        let mut surface = None;

        if let Some(win) = self.window {
            let size = win.raw.inner_size();
            let raw = unsafe { instance.create_surface(&win.raw) };
            surface = Some(SurfaceContext {
                raw,
                format: wgpu::TextureFormat::Rgba8Unorm,
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
            });
        }

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: self.power_preference,
                compatible_surface: surface.as_ref().map(|sc| &sc.raw),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        if let Some(ref mut suf) = surface {
            suf.format = suf.raw.get_preferred_format(&adapter).unwrap();
            suf.raw.configure(
                &device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: suf.format,
                    width: suf.size.width,
                    height: suf.size.height,
                    present_mode: wgpu::PresentMode::Mailbox,
                },
            );
        }

        Context {
            instance,
            surface,
            device,
            queue,
        }
    }
}

impl Context {
    pub fn new<'a>() -> ContextBuilder<'a> {
        ContextBuilder::default()
    }

    pub fn render_screen(&mut self, scene: &Scene) {
        let surface = self.surface.as_mut().expect("No scren is configured!");
        let frame = surface.raw.get_current_frame().unwrap();
        let view = frame
            .output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut comb = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let _pass = comb.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("screen"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(scene.background.into()),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
        }

        self.queue.submit(vec![comb.finish()]);
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Do we need explicit cleanup?
    }
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

#[derive(Default)]
pub struct Scene {
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

    pub fn object(&mut self) -> ObjectBuilder<()> {
        ObjectBuilder {
            scene: self,
            node: Node::default(),
            kind: (),
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
