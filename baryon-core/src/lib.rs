#![allow(
    // We use loops for getting early-out of scope without closures.
    clippy::never_loop,
    // We don't use syntax sugar where it's not necessary.
    clippy::match_like_matches_macro,
    // Redundant matching is more explicit.
    clippy::redundant_pattern_matching,
    // Explicit lifetimes are often easier to reason about.
    clippy::needless_lifetimes,
    // No need for defaults in the internal types.
    clippy::new_without_default,
    // For some reason `rustc` can warn about these in const generics even
    // though they are required.
    unused_braces,
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_qualifications,
    // We don't match on a reference, unless required.
    clippy::pattern_type_mismatch,
)]

mod color;
mod mesh;
mod space;

use raw_window_handle::HasRawWindowHandle;
use std::{mem, ops};

pub use color::Color;
pub use mesh::{IndexStream, Mesh, MeshBuilder, Prototype, Vertex, VertexStream};
pub use space::{Camera, Projection, RawSpace};

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

impl Target {
    pub fn aspect(&self) -> f32 {
        self.size.width as f32 / self.size.height as f32
    }
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
    }

    pub fn present<P: Pass>(&mut self, pass: &mut P, scene: &Scene, camera: &Camera) {
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

        pass.draw(&[tr], scene, camera, self);

        self.targets.pop();
    }

    pub fn add_mesh(&mut self) -> MeshBuilder {
        MeshBuilder::new(self)
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
    fn draw(&mut self, targets: &[TargetRef], scene: &Scene, camera: &Camera, context: &Context);
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeRef(u32);

#[derive(Default, Debug, PartialEq)]
pub struct Node {
    parent: NodeRef,
    local: space::Space,
}

pub type EntityRef = hecs::Entity;

pub struct Scene {
    pub world: hecs::World,
    nodes: Vec<Node>,
    lights: Vec<Light>,
}

impl ops::Index<NodeRef> for Scene {
    type Output = Node;
    fn index(&self, node: NodeRef) -> &Node {
        &self.nodes[node.0 as usize]
    }
}

impl ops::IndexMut<NodeRef> for Scene {
    fn index_mut(&mut self, node: NodeRef) -> &mut Node {
        &mut self.nodes[node.0 as usize]
    }
}

impl ops::Index<LightRef> for Scene {
    type Output = Light;
    fn index(&self, light: LightRef) -> &Light {
        &self.lights[light.0 as usize]
    }
}

impl ops::IndexMut<LightRef> for Scene {
    fn index_mut(&mut self, light: LightRef) -> &mut Light {
        &mut self.lights[light.0 as usize]
    }
}

pub struct BakedScene {
    spaces: Box<[RawSpace]>,
}

impl ops::Index<NodeRef> for BakedScene {
    type Output = RawSpace;
    fn index(&self, node: NodeRef) -> &RawSpace {
        &self.spaces[node.0 as usize]
    }
}

pub struct EntityBuilder {
    raw: hecs::EntityBuilder,
    mesh: MeshRef,
}

pub struct LightBuilder {
    color: Color,
    intensity: f32,
    kind: LightKind,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            world: Default::default(),
            nodes: vec![Node::default()],
            lights: Vec::new(),
        }
    }

    fn add_node_impl(&mut self, node: &mut Node) -> NodeRef {
        let index = self.nodes.len();
        self.nodes.push(mem::take(node));
        NodeRef(index as u32)
    }

    pub fn add_node(&mut self) -> ObjectBuilder<()> {
        ObjectBuilder {
            scene: self,
            node: Node::default(),
            kind: (),
        }
    }

    pub fn add_entity(&mut self, prototype: &Prototype) -> ObjectBuilder<EntityBuilder> {
        let mut raw = hecs::EntityBuilder::new();
        raw.add_bundle(prototype);
        ObjectBuilder {
            scene: self,
            node: Node::default(),
            kind: EntityBuilder {
                raw,
                mesh: prototype.reference,
            },
        }
    }

    pub fn add_light(&mut self, kind: LightKind) -> ObjectBuilder<LightBuilder> {
        ObjectBuilder {
            scene: self,
            node: Node::default(),
            kind: LightBuilder {
                color: Color(0xFFFFFFFF),
                intensity: 1.0,
                kind,
            },
        }
    }

    pub fn add_directional_light(&mut self) -> ObjectBuilder<LightBuilder> {
        self.add_light(LightKind::Directional)
    }

    pub fn add_point_light(&mut self) -> ObjectBuilder<LightBuilder> {
        self.add_light(LightKind::Point)
    }

    pub fn lights<'a>(&'a self) -> impl Iterator<Item = (LightRef, &'a Light)> {
        self.lights
            .iter()
            .enumerate()
            .map(|(i, light)| (LightRef(i as u32), light))
    }

    pub fn bake(&self) -> BakedScene {
        let mut spaces: Vec<RawSpace> = Vec::with_capacity(self.nodes.len());
        for n in self.nodes.iter() {
            let space = if n.parent == NodeRef::default() {
                n.local.clone()
            } else {
                let parent_space = spaces[n.parent.0 as usize].to_space();
                parent_space.combine(&n.local)
            };
            spaces.push(space.into());
        }
        BakedScene {
            spaces: spaces.into_boxed_slice(),
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
    pub fn parent(&mut self, parent: NodeRef) -> &mut Self {
        self.node.parent = parent;
        self
    }
}

impl ObjectBuilder<'_, ()> {
    pub fn build(&mut self) -> NodeRef {
        self.scene.add_node_impl(&mut self.node)
    }
}

impl ObjectBuilder<'_, EntityBuilder> {
    /// Register a new material component with this entity.
    ///
    /// The following components are recognized by the library:
    ///   - [`Color`]
    pub fn component<T: hecs::Component>(&mut self, component: T) -> &mut Self {
        self.kind.raw.add(component);
        self
    }

    pub fn build(&mut self) -> EntityRef {
        let entity = Entity {
            node: if self.node.local == space::Space::default() {
                self.node.parent
            } else {
                self.scene.add_node_impl(&mut self.node)
            },
            mesh: self.kind.mesh,
        };
        let built = self.kind.raw.add(entity).build();
        self.scene.world.spawn(built)
    }
}

impl ObjectBuilder<'_, LightBuilder> {
    pub fn intensity(&mut self, intensity: f32) -> &mut Self {
        self.kind.intensity = intensity;
        self
    }

    pub fn color(&mut self, color: Color) -> &mut Self {
        self.kind.color = color;
        self
    }

    pub fn build(&mut self) -> LightRef {
        let light = Light {
            node: if self.node.local == space::Space::default() {
                self.node.parent
            } else {
                self.scene.add_node_impl(&mut self.node)
            },
            color: self.kind.color,
            intensity: self.kind.intensity,
            kind: self.kind.kind,
        };
        let index = self.scene.lights.len();
        self.scene.lights.push(light);
        LightRef(index as u32)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeshRef(u32);

#[derive(Clone, Copy, Debug)]
pub enum LightKind {
    Directional,
    Point,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LightRef(u32);

#[derive(Debug)]
pub struct Light {
    pub node: NodeRef,
    pub color: Color,
    pub intensity: f32,
    pub kind: LightKind,
}
