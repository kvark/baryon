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
        MeshBuilder {
            context: self,
            name: String::new(),
            data: Vec::new(),
            vertex_count: 0,
            index_stream: None,
            vertex_streams: Vec::new(),
            type_infos: Vec::new(),
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
    fn draw(&mut self, targets: &[TargetRef], scene: &Scene, camera: &Camera, context: &Context);
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeRef(u32);

#[derive(Clone, Debug, PartialEq)]
struct Space {
    position: glam::Vec3,
    scale: f32,
    orientation: glam::Quat,
}

impl Default for Space {
    fn default() -> Self {
        Self {
            position: glam::Vec3::ZERO,
            scale: 1.0,
            orientation: glam::Quat::IDENTITY,
        }
    }
}

impl Space {
    fn combine(&self, other: &Self) -> Self {
        Self {
            scale: self.scale * other.scale,
            orientation: self.orientation * other.orientation,
            position: self.scale * (self.orientation * other.position) + self.position,
        }
    }

    fn inverse(&self) -> Self {
        let scale = 1.0 / self.scale;
        let orientation = self.orientation.inverse();
        let position = -scale * (orientation * self.position);
        Self {
            position,
            scale,
            orientation,
        }
    }

    fn to_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::splat(self.scale),
            self.orientation,
            self.position,
        )
    }
}

#[derive(Default, Debug, PartialEq)]
struct Node {
    parent: NodeRef,
    local: Space,
}

pub type EntityRef = hecs::Entity;

pub struct Scene {
    pub world: hecs::World,
    nodes: Vec<Node>,
}

pub struct BakedNode {
    pub pos_scale: [f32; 4],
    pub rot: [f32; 4],
}

impl From<Space> for BakedNode {
    fn from(s: Space) -> Self {
        BakedNode {
            pos_scale: [s.position.x, s.position.y, s.position.z, s.scale],
            rot: s.orientation.into(),
        }
    }
}

impl BakedNode {
    fn to_space(&self) -> Space {
        Space {
            position: glam::Vec3::new(self.pos_scale[0], self.pos_scale[1], self.pos_scale[2]),
            scale: self.pos_scale[3],
            orientation: glam::Quat::from_array(self.rot),
        }
    }

    pub fn inverse_matrix(&self) -> mint::ColumnMatrix4<f32> {
        self.to_space().inverse().to_matrix().into()
    }
}

pub struct BakedScene {
    nodes: Box<[BakedNode]>,
}

impl ops::Index<NodeRef> for BakedScene {
    type Output = BakedNode;
    fn index(&self, node: NodeRef) -> &BakedNode {
        &self.nodes[node.0 as usize]
    }
}

#[derive(Clone, Debug)]
pub enum Projection {
    Orthographic {
        /// The center of the projection.
        center: mint::Vector2<f32>,
        /// Vertical extent from the center point. The height is double the extent.
        /// The width is derived from the height based on the current aspect ratio.
        extent_y: f32,
    },
    Perspective {
        /// Vertical field of view, in degrees.
        /// Note: the horizontal FOV is computed based on the aspect.
        fov_y: f32,
    },
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub projection: Projection,
    /// Specify the depth range as seen by the camera.
    /// `depth.start` maps to 0.0, and `depth.end` maps to 1.0.
    pub depth: ops::Range<f32>,
    pub node: NodeRef,
    pub background: Color,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            projection: Projection::Orthographic {
                center: mint::Vector2 { x: 0.0, y: 0.0 },
                extent_y: 1.0,
            },
            depth: 0.0..1.0,
            node: NodeRef::default(),
            background: Color::default(),
        }
    }
}

const DEGREES_TO_RADIANS: f32 = std::f32::consts::PI / 180.0;

impl Camera {
    pub fn projection_matrix(&self, aspect: f32) -> mint::ColumnMatrix4<f32> {
        let matrix = match self.projection {
            Projection::Orthographic { center, extent_y } => {
                let extent_x = aspect * extent_y;
                glam::Mat4::orthographic_rh(
                    center.x - extent_x,
                    center.x + extent_x,
                    center.y - extent_y,
                    center.y + extent_y,
                    self.depth.start,
                    self.depth.end,
                )
            }
            Projection::Perspective { fov_y } => {
                let fov = fov_y * DEGREES_TO_RADIANS;
                if self.depth.end == f32::INFINITY {
                    assert!(self.depth.start.is_finite());
                    glam::Mat4::perspective_infinite_rh(fov, aspect, self.depth.start)
                } else if self.depth.start == f32::INFINITY {
                    glam::Mat4::perspective_infinite_reverse_rh(fov, aspect, self.depth.end)
                } else {
                    glam::Mat4::perspective_rh(fov, aspect, self.depth.start, self.depth.end)
                }
            }
        };
        matrix.into()
    }
}

pub struct EntityKind {
    raw: hecs::EntityBuilder,
    mesh: MeshRef,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            world: Default::default(),
            nodes: vec![Node::default()],
        }
    }

    fn add_node(&mut self, node: Node) -> NodeRef {
        if node.local == Space::default() {
            node.parent
        } else {
            let index = self.nodes.len();
            self.nodes.push(node);
            NodeRef(index as u32)
        }
    }

    pub fn node(&mut self) -> ObjectBuilder<()> {
        ObjectBuilder {
            scene: self,
            node: Node::default(),
            kind: (),
        }
    }

    pub fn entity(&mut self, prototype: &Prototype) -> ObjectBuilder<EntityKind> {
        let mut raw = hecs::EntityBuilder::new();
        raw.add_bundle(prototype);
        ObjectBuilder {
            scene: self,
            node: Node::default(),
            kind: EntityKind {
                raw,
                mesh: prototype.reference,
            },
        }
    }

    pub fn bake(&self) -> BakedScene {
        let mut nodes: Vec<BakedNode> = Vec::with_capacity(self.nodes.len());
        for n in self.nodes.iter() {
            let space = if n.parent == NodeRef::default() {
                n.local.clone()
            } else {
                let parent_space = nodes[n.parent.0 as usize].to_space();
                parent_space.combine(&n.local)
            };
            nodes.push(space.into());
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
    pub fn parent(mut self, parent: NodeRef) -> Self {
        self.node.parent = parent;
        self
    }

    //TODO: should we accept `V: Into<mint::...>` here?
    pub fn position(mut self, position: mint::Vector3<f32>) -> Self {
        self.node.local.position = position.into();
        self
    }

    pub fn look_at(mut self, target: mint::Vector3<f32>, up: mint::Vector3<f32>) -> Self {
        /* // This path just doesn't work well
        let dir = (glam::Vec3::from(target) - self.node.local.position).normalize();
        self.node.local.orientation = glam::Quat::from_rotation_arc(-glam::Vec3::Z, dir);
            * glam::Quat::from_rotation_arc(glam::Vec3::Y, up.into());
        let temp = glam::Quat::from_rotation_arc(glam::Vec3::Y, up.into();
        let new_dir = temp * -glam::Vec3::Z;
        self.node.local.orientation = glam::Quat::from_rotation_arc(-glam::Vec3::Z, dir);
        */

        let affine = glam::Affine3A::look_at_rh(self.node.local.position, target.into(), up.into());
        let (_, rot, _) = affine.inverse().to_scale_rotation_translation();
        // translation here is expected to match `self.node.local.position`
        self.node.local.orientation = rot;

        /* // Blocked on https://github.com/bitshifter/glam-rs/issues/235
        let dir = self.node.local.position - glam::Vec3::from(target);
        let f = dir.normalize();
        let s = glam::Vec3::from(up).cross(f).normalize();
        let u = f.cross(s);
        self.node.local.orientation = glam::Quat::from_rotation_axes(s, u, f);
        */
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

pub struct Prototype {
    pub reference: MeshRef,
    type_ids: Box<[TypeId]>,
    type_infos: Box<[hecs::TypeInfo]>,
}

pub struct IndexStream {
    pub offset: wgpu::BufferAddress,
    pub format: wgpu::IndexFormat,
    pub count: u32,
}

pub struct VertexStream {
    type_id: TypeId,
    pub offset: wgpu::BufferAddress,
    pub stride: wgpu::BufferAddress,
}

//HACK: `hecs` doesn't want anybody to implement this, but we have no choice.
unsafe impl<'a> hecs::DynamicBundle for &'a Prototype {
    fn with_ids<T>(&self, f: impl FnOnce(&[TypeId]) -> T) -> T {
        f(&self.type_ids)
    }
    fn type_info(&self) -> Vec<hecs::TypeInfo> {
        self.type_infos.to_vec()
    }
    unsafe fn put(self, mut f: impl FnMut(*mut u8, hecs::TypeInfo)) {
        const DUMMY_SIZE: usize = 1;
        let mut v = [0u8; DUMMY_SIZE];
        assert!(mem::size_of::<Vertex<()>>() <= DUMMY_SIZE);
        for ts in self.type_infos.iter() {
            f(v.as_mut_ptr(), ts.clone());
        }
    }
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
    type_infos: Vec<hecs::TypeInfo>,
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
        self.type_infos.push(hecs::TypeInfo::of::<Vertex<T>>());
        self
    }

    pub fn build(self) -> Prototype {
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

        let type_ids = self
            .vertex_streams
            .iter()
            .map(|vs| vs.type_id)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        self.context.meshes.push(Mesh {
            buffer,
            index_stream: self.index_stream,
            vertex_streams: self.vertex_streams.into_boxed_slice(),
            vertex_count: self.vertex_count as u32,
        });

        Prototype {
            reference: MeshRef(index as u32),
            type_ids,
            type_infos: self.type_infos.into_boxed_slice(),
        }
    }
}
