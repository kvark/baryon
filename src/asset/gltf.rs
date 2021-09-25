use std::path::Path;

#[derive(Default)]
struct MeshScratch {
    indices: Vec<u16>,
    positions: Vec<crate::Position>,
    normals: Vec<crate::Normal>,
}

struct Primitive {
    prototype: crate::Prototype,
    color: crate::Color,
    shader: crate::pass::Shader,
    material: crate::pass::Material,
}

fn load_primitive<'a>(
    primitive: gltf::Primitive<'a>,
    buffers: &[gltf::buffer::Data],
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

    if let Some(normals) = reader.read_normals() {
        scratch.normals.clear();
        scratch.normals.extend(normals.map(crate::Normal));
        mesh_builder.vertex(&scratch.normals);
    }

    let mat = primitive.material();
    let pbr = mat.pbr_metallic_roughness();
    let base_color = pbr.base_color_factor();
    let material = crate::pass::Material {
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

/// Load mesh from glTF 2.0 format.
pub fn load_gltf(
    path: impl AsRef<Path>,
    scene: &mut crate::Scene,
    parent: crate::NodeRef,
    context: &mut crate::Context,
) {
    let (gltf, buffers, _images) = gltf::import(path).expect("invalid glTF 2.0");

    let mut prototypes = Vec::with_capacity(gltf.meshes().len());
    let mut scratch = MeshScratch::default();
    for gltf_mesh in gltf.meshes() {
        let mut primitives = Vec::new();
        for gltf_primitive in gltf_mesh.primitives() {
            let primitive = load_primitive(gltf_primitive, &buffers, context, &mut scratch);
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
            for primitive in prototypes[gltf_mesh.index()].iter() {
                let _entity = scene
                    .add_entity(&primitive.prototype)
                    .parent(node)
                    .component(primitive.color)
                    .component(primitive.shader)
                    .component(primitive.material)
                    .build();
            }
        }
    }
}
