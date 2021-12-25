use lyon::path::Path;
use lyon::tessellation::*;

type PositionBuilder = VertexBuffers<crate::Position, u16>;

fn fill_position(vertex: FillVertex) -> crate::Position {
    let p = vertex.position();
    crate::Position([p.x, p.y, 0.0])
}

fn stroke_position(vertex: StrokeVertex) -> crate::Position {
    let p = vertex.position();
    crate::Position([p.x, p.y, 0.0])
}

fn bounding_radius(path: &Path) -> f32 {
    path.iter().fold(0.0, |accum, item| {
        let p = item.from();
        accum.max(p.x.abs().max(p.y.abs()))
    })
}

impl super::Geometry {
    pub fn fill(path: &Path) -> Self {
        let mut buffer = PositionBuilder::new();
        let builder = &mut BuffersBuilder::new(&mut buffer, fill_position);
        let mut tessellator = FillTessellator::new();
        tessellator
            .tessellate_path(path, &FillOptions::default(), builder)
            .unwrap();

        let radius = bounding_radius(path);

        Self {
            positions: buffer.vertices,
            indices: Some(buffer.indices),
            normals: None,
            radius,
        }
    }

    pub fn stroke(path: &Path, options: &StrokeOptions) -> Self {
        let mut buffer = PositionBuilder::new();
        let builder = &mut BuffersBuilder::new(&mut buffer, stroke_position);
        let mut tessellator = StrokeTessellator::new();
        tessellator.tessellate_path(path, options, builder).unwrap();

        let radius = bounding_radius(path);

        Self {
            positions: buffer.vertices,
            indices: Some(buffer.indices),
            normals: None,
            radius,
        }
    }
}
