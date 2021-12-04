use lyon::path::Path;
use lyon::tessellation::*;

// TODO is this always correct?
fn bounding_radius(path: Path) -> f32 {
    path.iter().fold(0.0, |accum, item| {
        let p = item.from();
        if p.x > accum {
          p.x
        } else if p.y > accum {
            p.y
        } else {
            accum
        }
    })
}

impl super::Geometry {
    pub fn shape(_streams: super::Streams, path: Path) -> Self {
        let mut buffer: VertexBuffers<crate::Position, u16> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        {
            // Compute the tessellation.
            tessellator
                .tessellate_path(
                    &path,
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(&mut buffer, |vertex: FillVertex| {
                        let p = vertex.position();
                        crate::Position([p.x, p.y, 0.0])
                    }),
                )
                .unwrap();
        }

        let radius = bounding_radius(path);

        // The tessellated geometry is ready to be uploaded to the GPU.
        println!(
            " -- {} vertices {} indices {} radius",
            buffer.vertices.len(),
            buffer.indices.len(),
            radius
        );

        Self {
            positions: buffer.vertices,
            indices: Some(buffer.indices),
            normals: None,
            radius,
        }
    }
}
