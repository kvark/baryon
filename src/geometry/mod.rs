pub mod cuboid;
pub mod sphere;

bitflags::bitflags!(
    /// Types of optional vertex streams.
    pub struct Streams: u32 {
        const NORMAL = 1 << 1;
    }
);

pub struct Geometry {
    pub positions: Vec<crate::Position>,
    pub normals: Option<Vec<crate::Normal>>,
    pub indices: Option<Vec<u16>>,
    pub radius: f32,
}

impl Geometry {
    pub fn bake(&self, context: &mut bc::Context) -> bc::Prototype {
        let mut mb = context.add_mesh();
        mb.radius(self.radius);
        mb.vertex(&self.positions);
        if let Some(ref stream) = self.normals {
            mb.vertex(stream);
        }
        if let Some(ref indices) = self.indices {
            mb.index(indices);
        }
        mb.build()
    }
}
