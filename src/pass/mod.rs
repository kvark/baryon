mod flat;
mod phong;
mod real;
mod solid;

pub use flat::Flat;
pub use phong::{Ambient, Phong, PhongConfig, Shader};
pub use real::{Material, Real, RealConfig};
pub use solid::{Solid, SolidConfig};

use std::mem;

fn align_up(offset: u32, align: u32) -> u32 {
    (offset + align - 1) & !(align - 1)
}

struct BufferPool {
    label: &'static str,
    usage: wgpu::BufferUsages,
    buffers: Vec<wgpu::Buffer>,
    chunk_size: u32,
    last_index: usize,
    last_offset: u32,
    alignment: u32,
}

struct BufferLocation {
    index: usize,
    offset: u32,
}

impl BufferPool {
    fn uniform(label: &'static str, device: &wgpu::Device) -> Self {
        let chunk_size = 0x10000;
        let usage = wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM;
        Self {
            label,
            buffers: vec![device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: chunk_size as wgpu::BufferAddress,
                usage,
                mapped_at_creation: false,
            })],
            chunk_size,
            last_index: 0,
            last_offset: 0,
            alignment: device.limits().min_uniform_buffer_offset_alignment,
            usage,
        }
    }

    fn prepare_for_count<T>(&mut self, count: usize, device: &wgpu::Device) -> usize {
        if count == 0 {
            return 0;
        }
        let size_per_element = align_up(mem::size_of::<T>() as u32, self.alignment);
        let elements_per_chunk = self.chunk_size / size_per_element;
        let buf_count = 1 + (count - 1) / (elements_per_chunk as usize);

        while self.buffers.len() < buf_count {
            self.buffers
                .push(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(self.label),
                    size: self.chunk_size as wgpu::BufferAddress,
                    usage: self.usage,
                    mapped_at_creation: false,
                }));
        }
        buf_count
    }

    //TODO: consider lifting `T` up
    fn binding<T>(&self, index: usize) -> wgpu::BufferBinding {
        wgpu::BufferBinding {
            buffer: &self.buffers[index],
            offset: 0,
            size: wgpu::BufferSize::new(mem::size_of::<T>() as _),
        }
    }

    fn alloc<T: bytemuck::Pod>(&mut self, object: &T, queue: &wgpu::Queue) -> BufferLocation {
        let size = mem::size_of::<T>() as u32;
        assert!(size <= self.chunk_size);
        if self.last_offset + size > self.chunk_size {
            self.last_index += 1;
            self.last_offset = 0;
        }

        let offset = self.last_offset;
        let buffer = &self.buffers[self.last_index];
        queue.write_buffer(
            buffer,
            offset as wgpu::BufferAddress,
            bytemuck::bytes_of(object),
        );

        self.last_offset = align_up(offset + size, self.alignment);

        BufferLocation {
            index: self.last_index,
            offset,
        }
    }

    fn reset(&mut self) {
        self.last_index = 0;
        self.last_offset = 0;
    }
}
