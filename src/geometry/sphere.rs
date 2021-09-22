// See https://github.com/gfx-rs/genmesh/blob/master/src/icosphere.rs

const F: f32 = 1.618034; // 0.5 * (1.0 + 5f32.sqrt());

// Base icosahedron positions
const BASE_POSITIONS: [[f32; 3]; 12] = [
    [-1.0, F, 0.0],
    [1.0, F, 0.0],
    [-1.0, -F, 0.0],
    [1.0, -F, 0.0],
    [0.0, -1.0, F],
    [0.0, 1.0, F],
    [0.0, -1.0, -F],
    [0.0, 1.0, -F],
    [F, 0.0, -1.0],
    [F, 0.0, 1.0],
    [-F, 0.0, -1.0],
    [-F, 0.0, 1.0],
];

// Base icosahedron faces
const BASE_FACES: [[u16; 3]; 20] = [
    [0, 11, 5],
    [0, 5, 1],
    [0, 1, 7],
    [0, 7, 10],
    [0, 10, 11],
    [11, 10, 2],
    [5, 11, 4],
    [1, 5, 9],
    [7, 1, 8],
    [10, 7, 6],
    [3, 9, 4],
    [3, 4, 2],
    [3, 2, 6],
    [3, 6, 8],
    [3, 8, 9],
    [9, 8, 1],
    [4, 9, 5],
    [2, 4, 11],
    [6, 2, 10],
    [8, 6, 7],
];

impl super::Geometry {
    pub fn sphere(streams: super::Streams, radius: f32, detail: usize) -> Self {
        assert!(detail < 30); // just a sanity check
        let mut lookup = fxhash::FxHashMap::default();
        let mut prev_faces = Vec::new();
        let mut vertices = BASE_POSITIONS
            .iter()
            .map(|p| glam::Vec3::from_slice(p))
            .collect::<Vec<_>>();
        let mut faces = BASE_FACES.to_vec();

        for _ in 1..detail {
            lookup.clear();
            prev_faces.clear();
            prev_faces.append(&mut faces);

            for face in prev_faces.iter() {
                let mut mid = [0u16; 3];
                for (pair, index) in face
                    .iter()
                    .cloned()
                    .zip(face[1..].iter().chain(face.first()).cloned())
                    .zip(mid.iter_mut())
                {
                    *index = match lookup.get(&pair) {
                        Some(i) => *i,
                        None => {
                            let i = vertices.len() as u16;
                            lookup.insert(pair, i);
                            lookup.insert((pair.1, pair.0), i);
                            let v = 0.5 * (vertices[pair.0 as usize] + vertices[pair.1 as usize]);
                            vertices.push(v);
                            i
                        }
                    };
                }

                faces.push([face[0], mid[0], mid[2]]);
                faces.push([face[1], mid[1], mid[0]]);
                faces.push([face[2], mid[2], mid[1]]);
                faces.push([mid[0], mid[1], mid[2]]);
            }
        }

        let indices = faces.into_iter().flat_map(|face| face).collect::<Vec<_>>();
        let mut positions = Vec::with_capacity(vertices.len());
        let mut normals = if streams.contains(super::Streams::NORMAL) {
            Some(Vec::with_capacity(vertices.len()))
        } else {
            None
        };

        for v in vertices {
            let n = v.normalize();
            positions.push(crate::Position((n * radius).into()));
            if let Some(ref mut normals) = normals {
                normals.push(crate::Normal(n.into()));
            }
        }

        Self {
            positions,
            normals,
            radius,
            indices: Some(indices),
        }
    }
}
