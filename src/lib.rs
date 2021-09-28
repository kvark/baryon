pub use bc::{
    Camera, Color, Context, EntityRef, ImageRef, Light, LightBuilder, LightRef, MeshBuilder,
    MeshRef, NodeRef, Pass, Projection, Prototype, Scene, Sprite, SpriteBuilder, TargetInfo,
    TargetRef, UvRange,
};
use std::mem;

pub mod asset;
pub mod geometry;
pub mod pass;
#[cfg(feature = "window")]
pub mod window;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Position(pub [f32; 3]);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Normal(pub [f32; 3]);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TexCoords(pub [u16; 2]);

impl Position {
    const fn layout<const LOCATION: u32>() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: LOCATION,
            }],
        }
    }
}

impl Normal {
    const fn layout<const LOCATION: u32>() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: LOCATION,
            }],
        }
    }
}

impl TexCoords {
    const fn layout<const LOCATION: u32>() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Unorm16x2,
                offset: 0,
                shader_location: LOCATION,
            }],
        }
    }
}
