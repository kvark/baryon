use std::{iter, path::Path};

/// Load entities from Wavefront Obj format.
pub fn load_obj(
    path: impl AsRef<Path>,
    scene: &mut crate::Scene,
    node: crate::NodeRef,
    context: &mut crate::Context,
) -> fxhash::FxHashMap<String, (crate::EntityRef, crate::Prototype)> {
    let mut obj = obj::Obj::load(path).unwrap();
    obj.load_mtls().unwrap();

    let mut entities = fxhash::FxHashMap::default();
    let mut positions = Vec::new();
    let mut normals = Vec::new();

    for object in obj.data.objects {
        for group in object.groups {
            positions.clear();
            normals.clear();

            for poly in group.polys.iter() {
                let tr0 = [0usize, 1, 2];
                let tr1 = if poly.0.len() > 3 {
                    if poly.0.len() > 4 {
                        log::warn!("Object geometry contains pentagons!");
                    }
                    Some([2usize, 3, 0])
                } else {
                    None
                };
                for triangle in iter::once(tr0).chain(tr1) {
                    for &elem_index in triangle.iter() {
                        let obj::IndexTuple(pos_id, _tex_id, nor_id) = poly.0[elem_index];
                        positions.push(crate::Position(obj.data.position[pos_id]));
                        if let Some(index) = nor_id {
                            normals.push(crate::Normal(obj.data.normal[index]));
                        }
                    }
                }
            }

            let mut mesh_builder = context.add_mesh();
            mesh_builder.vertex(&positions);
            if !normals.is_empty() {
                mesh_builder.vertex(&normals);
            }
            let prototype = mesh_builder.build();
            let mut entity_builder = scene.add_entity(&prototype);
            entity_builder.parent(node);

            log::info!(
                "\tmaterial {} with {} positions and {} normals",
                group.name,
                positions.len(),
                normals.len()
            );
            if let Some(obj::ObjMaterial::Mtl(ref mat)) = group.material {
                if let Some(cf) = mat.kd {
                    let color = cf.iter().fold(0xFF, |u, c| {
                        (u << 8) + (c * 255.0).max(0.0).min(255.0) as u32
                    });
                    entity_builder.component(crate::Color(color));
                }
                if !normals.is_empty() {
                    if let Some(glossiness) = mat.ns {
                        entity_builder.component(crate::pass::Shader::Phong {
                            glossiness: glossiness as u8,
                        })
                    } else {
                        entity_builder.component(crate::pass::Shader::Gouraud { flat: false })
                    };
                }
            }

            entities.insert(group.name, (entity_builder.build(), prototype));
        }
    }

    entities
}
