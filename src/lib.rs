pub use bc::{
    Camera, Color, Context, EntityRef, MeshBuilder, MeshRef, NodeRef, Pass, Projection, Prototype,
    Scene, TargetRef,
};
use std::mem;

pub mod pass;
pub mod window;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Position(pub [f32; 3]);

impl Position {
    const fn layout<const LOCATION: u32>() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Position>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: LOCATION + 0,
            }],
        }
    }
}
