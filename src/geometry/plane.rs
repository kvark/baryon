use crate::{Normal, Position};

impl super::Geometry {
    pub fn plane(size: f32) -> Self {
        let extent = size / 2.0;
        let extent2 = extent.powf(2.0);
        let radius = (extent2 + extent2).sqrt();

        let vertices = [
            ([extent, 0.0, -extent], [0.0, 1.0, 0.0], [1.0, 1.0]),
            ([extent, 0.0, extent], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ([-extent, 0.0, extent], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ([-extent, 0.0, -extent], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ];

        let indices = vec![0, 2, 1, 0, 3, 2];

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        // let mut uvs = Vec::new();
        for (position, normal, _uv) in vertices.iter() {
            positions.push(Position(*position));
            normals.push(Normal(*normal));
            // uvs.push(*uv);
        }

        Self {
            radius,
            positions,
            normals: Some(normals),
            indices: Some(indices),
        }
    }
}
