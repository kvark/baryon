use std::{ops, path::Path};

#[derive(Default)]
struct MeshScratch {
    indices: Vec<u16>,
    positions: Vec<crate::Position>,
    tex_coords: Vec<crate::TexCoords>,
    normals: Vec<crate::Normal>,
}

struct Texture {
    image: crate::ImageRef,
}

struct Primitive {
    prototype: crate::Prototype,
    color: crate::Color,
    shader: crate::pass::Shader,
    material: crate::pass::Material,
}

fn load_texture(mut data: gltf::image::Data, context: &mut crate::Context) -> Texture {
    let format = match data.format {
        gltf::image::Format::R8 => wgpu::TextureFormat::R8Unorm,
        gltf::image::Format::R8G8 => wgpu::TextureFormat::Rg8Unorm,
        gltf::image::Format::R8G8B8 | gltf::image::Format::B8G8R8 => {
            log::warn!(
                "Converting {}x{} texture from RGB to RGBA...",
                data.width,
                data.height
            );
            let original = data.pixels;
            data.pixels = Vec::with_capacity(original.len() * 4 / 3);
            for chunk in original.chunks(3) {
                data.pixels.push(chunk[0]);
                data.pixels.push(chunk[1]);
                data.pixels.push(chunk[2]);
                data.pixels.push(0xFF);
            }
            if data.format == gltf::image::Format::R8G8B8 {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Bgra8UnormSrgb
            }
        }
        gltf::image::Format::R16G16B16 => panic!("RGB16 is outdated"),
        gltf::image::Format::R8G8B8A8 => wgpu::TextureFormat::Rgba8UnormSrgb,
        gltf::image::Format::B8G8R8A8 => wgpu::TextureFormat::Bgra8UnormSrgb,
        gltf::image::Format::R16 => wgpu::TextureFormat::R16Float,
        gltf::image::Format::R16G16 => wgpu::TextureFormat::Rg16Float,
        gltf::image::Format::R16G16B16A16 => wgpu::TextureFormat::Rgba16Float,
    };

    let desc = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: data.width,
            height: data.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
    };
    let image = context.add_image_from_data(&desc, &data.pixels);
    Texture { image }
}

fn load_primitive<'a>(
    primitive: gltf::Primitive<'a>,
    buffers: &[gltf::buffer::Data],
    textures: &[Texture],
    context: &mut crate::Context,
    scratch: &mut MeshScratch,
) -> Primitive {
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
    let mut mesh_builder = context.add_mesh();

    if let Some(indices) = reader.read_indices() {
        scratch.indices.clear();
        scratch.indices.extend(indices.into_u32().map(|i| i as u16));
        mesh_builder.index(&scratch.indices);
    }

    if let Some(positions) = reader.read_positions() {
        scratch.positions.clear();
        scratch.positions.extend(positions.map(crate::Position));
        mesh_builder.vertex(&scratch.positions);
    }

    if let Some(tex_coords) = reader.read_tex_coords(0) {
        scratch.tex_coords.clear();
        scratch
            .tex_coords
            .extend(tex_coords.into_u16().map(crate::TexCoords));
        mesh_builder.vertex(&scratch.tex_coords);
    }

    if let Some(normals) = reader.read_normals() {
        scratch.normals.clear();
        scratch.normals.extend(normals.map(crate::Normal));
        mesh_builder.vertex(&scratch.normals);
    }

    let mat = primitive.material();
    let pbr = mat.pbr_metallic_roughness();
    let base_color = pbr.base_color_factor();
    let material = crate::pass::Material {
        base_color_map: pbr
            .base_color_texture()
            .map(|t| textures[t.texture().index()].image),
        emissive_color: crate::Color::from_rgb_alpha(mat.emissive_factor(), 0.0),
        metallic_factor: pbr.metallic_factor(),
        roughness_factor: pbr.roughness_factor(),
        normal_scale: 1.0,
        occlusion_strength: 1.0,
    };

    Primitive {
        prototype: mesh_builder.build(),
        color: crate::Color::from_rgba(base_color),
        shader: crate::pass::Shader::Gouraud { flat: true }, //TODO
        material,
    }
}

#[derive(Debug)]
struct Named<T> {
    data: T,
    name: Option<String>,
}

#[derive(Debug)]
pub struct NamedVec<T>(Vec<Named<T>>);

impl<T> Default for NamedVec<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> ops::Index<usize> for NamedVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.0[index].data
    }
}

impl<T> NamedVec<T> {
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter().map(|elem| &elem.data)
    }

    pub fn find(&self, name: &str) -> Option<&T> {
        self.0
            .iter()
            .find(|elem| elem.name.as_deref() == Some(name))
            .map(|elem| &elem.data)
    }
}

#[derive(Default)]
pub struct Module {
    pub entities: NamedVec<bc::EntityRef>,
    pub cameras: NamedVec<bc::Camera>,
}

/// Load mesh from glTF 2.0 format.
pub fn load_gltf(
    path: impl AsRef<Path>,
    scene: &mut crate::Scene,
    parent: crate::NodeRef,
    context: &mut crate::Context,
) -> Module {
    let mut module = Module::default();
    let (gltf, buffers, images) = gltf::import(path).expect("invalid glTF 2.0");

    let mut textures = Vec::with_capacity(images.len());
    for (_texture, data) in gltf.textures().zip(images.into_iter()) {
        let texture = load_texture(data, context);
        textures.push(texture);
    }

    let mut prototypes = Vec::with_capacity(gltf.meshes().len());
    let mut scratch = MeshScratch::default();
    for gltf_mesh in gltf.meshes() {
        let mut primitives = Vec::new();
        for gltf_primitive in gltf_mesh.primitives() {
            let primitive =
                load_primitive(gltf_primitive, &buffers, &textures, context, &mut scratch);
            primitives.push(primitive);
        }
        prototypes.push(primitives);
    }

    #[derive(Clone)]
    struct TempNode {
        parent: crate::NodeRef,
        node: crate::NodeRef,
    }
    let mut nodes = vec![
        TempNode {
            parent,
            node: crate::NodeRef::default()
        };
        gltf.nodes().len()
    ];

    for gltf_node in gltf.nodes() {
        let (translation, rotation, scale) = gltf_node.transform().decomposed();
        let uniform_scale = if scale[1] != scale[0] || scale[2] != scale[0] {
            log::warn!(
                "Node[{}] scale {:?} is non-uniform",
                gltf_node.index(),
                scale
            );
            (scale[0] + scale[1] + scale[2]) / 3.0
        } else {
            scale[0]
        };
        log::debug!("Node {:?}", gltf_node.name());

        let cur = &mut nodes[gltf_node.index()];
        let node = scene
            .add_node()
            .parent(cur.parent)
            .position(translation.into())
            .orientation(rotation.into())
            .scale(uniform_scale)
            .build();
        cur.node = node;

        for gltf_child in gltf_node.children() {
            nodes[gltf_child.index()].parent = node;
        }

        if let Some(gltf_mesh) = gltf_node.mesh() {
            log::debug!("Mesh {:?}", gltf_mesh.name());
            for primitive in prototypes[gltf_mesh.index()].iter() {
                let entity = scene
                    .add_entity(&primitive.prototype)
                    .parent(node)
                    .component(primitive.color)
                    .component(primitive.shader)
                    .component(primitive.material)
                    .build();
                module.entities.0.push(Named {
                    data: entity,
                    name: gltf_mesh.name().map(str::to_string),
                });
            }
        }

        if let Some(gltf_camera) = gltf_node.camera() {
            let (depth, projection) = match gltf_camera.projection() {
                gltf::camera::Projection::Orthographic(p) => (
                    p.znear()..p.zfar(),
                    bc::Projection::Orthographic {
                        center: [0.0; 2].into(),
                        //Note: p.xmag() is ignored
                        extent_y: p.ymag(),
                    },
                ),
                gltf::camera::Projection::Perspective(p) => (
                    p.znear()..p.zfar().unwrap_or(f32::INFINITY),
                    bc::Projection::Perspective {
                        fov_y: p.yfov().to_degrees(),
                    },
                ),
            };
            log::debug!(
                "Camera {:?} depth {:?} proj {:?} at {:?}",
                gltf_camera.name(),
                depth,
                projection,
                scene[node]
            );
            module.cameras.0.push(Named {
                data: bc::Camera {
                    projection,
                    depth,
                    node,
                    background: bc::Color::default(),
                },
                name: gltf_camera.name().map(str::to_string),
            });
        }
    }

    module
}
