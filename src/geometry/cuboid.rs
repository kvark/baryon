use std::iter;

impl super::Geometry {
    pub fn cuboid(streams: super::Streams, half_extent: mint::Vector3<f32>) -> Self {
        let pos = |x, y, z| {
            crate::Position([
                (x as f32) * half_extent.x,
                (y as f32) * half_extent.y,
                (z as f32) * half_extent.z,
            ])
        };

        // bounding radius is half of the diagonal length
        let radius = (half_extent.x * half_extent.x
            + half_extent.y * half_extent.y
            + half_extent.z * half_extent.z)
            .sqrt();

        if streams.contains(super::Streams::NORMAL) {
            let positions = vec![
                // top (0, 0, 1)
                pos(-1, -1, 1),
                pos(1, -1, 1),
                pos(1, 1, 1),
                pos(-1, 1, 1),
                // bottom (0, 0, -1)
                pos(-1, 1, -1),
                pos(1, 1, -1),
                pos(1, -1, -1),
                pos(-1, -1, -1),
                // right (1, 0, 0)
                pos(1, -1, -1),
                pos(1, 1, -1),
                pos(1, 1, 1),
                pos(1, -1, 1),
                // left (-1, 0, 0)
                pos(-1, -1, 1),
                pos(-1, 1, 1),
                pos(-1, 1, -1),
                pos(-1, -1, -1),
                // front (0, 1, 0)
                pos(1, 1, -1),
                pos(-1, 1, -1),
                pos(-1, 1, 1),
                pos(1, 1, 1),
                // back (0, -1, 0)
                pos(1, -1, 1),
                pos(-1, -1, 1),
                pos(-1, -1, -1),
                pos(1, -1, -1),
            ];

            let normals = [
                crate::Normal([0.0, 0.0, 1.0]),
                crate::Normal([0.0, 0.0, -1.0]),
                crate::Normal([1.0, 0.0, 0.0]),
                crate::Normal([-1.0, 0.0, 0.0]),
                crate::Normal([0.0, 1.0, 0.0]),
                crate::Normal([0.0, -1.0, 0.0]),
            ]
            .iter()
            .flat_map(|&n| iter::repeat(n).take(4))
            .collect::<Vec<_>>();

            let indices = vec![
                0u16, 1, 2, 2, 3, 0, // top
                4, 5, 6, 6, 7, 4, // bottom
                8, 9, 10, 10, 11, 8, // right
                12, 13, 14, 14, 15, 12, // left
                16, 17, 18, 18, 19, 16, // front
                20, 21, 22, 22, 23, 20, // back
            ];

            Self {
                radius,
                positions,
                normals: Some(normals),
                indices: Some(indices),
            }
        } else {
            let positions = vec![
                // top (0, 0, 1)
                pos(-1, -1, 1),
                pos(1, -1, 1),
                pos(1, 1, 1),
                pos(-1, 1, 1),
                // bottom (0, 0, -1)
                pos(-1, 1, -1),
                pos(1, 1, -1),
                pos(1, -1, -1),
                pos(-1, -1, -1),
            ];

            let indices = vec![
                0u16, 1, 2, 2, 3, 0, // top
                4, 5, 6, 6, 7, 4, // bottom
                6, 5, 2, 2, 1, 6, // right
                0, 3, 4, 4, 7, 0, // left
                5, 4, 3, 3, 2, 5, // front
                1, 0, 7, 7, 6, 1, // back
            ];

            Self {
                radius,
                positions,
                normals: None,
                indices: Some(indices),
            }
        }
    }
}
