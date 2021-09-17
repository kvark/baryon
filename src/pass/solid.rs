use bc::ContextDetail as _;
use fxhash::FxHashMap;
use std::mem;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Locals {
    pos_scale: [f32; 4],
    rot: [f32; 4],
    color: [f32; 4],
}

#[derive(Eq, Hash, PartialEq)]
struct PipelineKey {
    target_format: wgpu::TextureFormat,
}

#[derive(Eq, Hash, PartialEq)]
struct LocalKey {
    uniform_buf_index: usize,
}

#[derive(Debug)]
pub struct SolidConfig {
    pub cull_back_faces: bool,
}

impl Default for SolidConfig {
    fn default() -> Self {
        Self {
            cull_back_faces: true,
        }
    }
}

struct PipelineInfo {
    layout: wgpu::PipelineLayout,
    shader_module: wgpu::ShaderModule,
    primitive_state: wgpu::PrimitiveState,
}

pub struct Solid {
    depth_texture: Option<(wgpu::TextureView, wgpu::Extent3d)>,
    global_uniform_buf: wgpu::Buffer,
    global_bind_group: wgpu::BindGroup,
    local_bind_group_layout: wgpu::BindGroupLayout,
    local_bind_groups: FxHashMap<LocalKey, wgpu::BindGroup>,
    uniform_pool: super::BufferPool,
    pipeline_info: PipelineInfo,
    pipelines: FxHashMap<PipelineKey, wgpu::RenderPipeline>,
}

impl Solid {
    pub fn new(config: &SolidConfig, context: &crate::Context) -> Self {
        let d = context.device();
        let shader_module = d.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("solid"),
            source: wgpu::ShaderSource::Wgsl(include_str!("solid.wgsl").into()),
        });

        let globals_size = mem::size_of::<Globals>() as wgpu::BufferAddress;
        let global_bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("solid globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(globals_size),
                },
                count: None,
            }],
        });
        let global_uniform_buf = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("solid globals"),
            size: globals_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let global_bind_group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("solid globals"),
            layout: &global_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: global_uniform_buf.as_entire_binding(),
            }],
        });

        let locals_size = mem::size_of::<Locals>() as wgpu::BufferAddress;
        let local_bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("solid locals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(locals_size),
                },
                count: None,
            }],
        });

        let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("solid"),
            bind_group_layouts: &[&global_bgl, &local_bgl],
            push_constant_ranges: &[],
        });

        Self {
            depth_texture: None,
            global_uniform_buf,
            global_bind_group,
            local_bind_group_layout: local_bgl,
            local_bind_groups: Default::default(),
            uniform_pool: super::BufferPool::uniform("solid locals", d),
            pipeline_info: PipelineInfo {
                layout: pipeline_layout,
                shader_module,
                primitive_state: wgpu::PrimitiveState {
                    cull_mode: if config.cull_back_faces {
                        Some(wgpu::Face::Back)
                    } else {
                        None
                    },
                    ..Default::default()
                },
            },
            pipelines: Default::default(),
        }
    }
}

impl bc::Pass for Solid {
    fn draw(
        &mut self,
        targets: &[crate::TargetRef],
        scene: &crate::Scene,
        camera: &crate::Camera,
        context: &crate::Context,
    ) {
        let target = context.get_target(targets[0]);
        let device = context.device();

        let reset_depth = match self.depth_texture {
            Some((_, size)) => size != target.size,
            None => true,
        };
        if reset_depth {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                size: target.size,
                sample_count: 1,
                mip_level_count: 1,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.depth_texture = Some((view, target.size));
        }

        let info = &self.pipeline_info;
        let key = PipelineKey {
            target_format: target.format,
        };
        let pipeline = self.pipelines.entry(key).or_insert_with(|| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("solid"),
                layout: Some(&info.layout),
                vertex: wgpu::VertexState {
                    buffers: &[super::Position::layout::<0>()],
                    module: &info.shader_module,
                    entry_point: "main_vs",
                },
                primitive: info.primitive_state,
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    depth_write_enabled: true,
                    bias: Default::default(),
                    stencil: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    targets: &[target.format.into()],
                    module: &info.shader_module,
                    entry_point: "main_fs",
                }),
            })
        });

        let nodes = scene.bake();
        self.uniform_pool.reset();
        let queue = context.queue();

        {
            let m_proj = camera.projection_matrix(target.aspect());
            let m_view_inv = nodes[camera.node].inverse_matrix();
            let globals = Globals {
                view_proj: (glam::Mat4::from_cols_array_2d(&m_view_inv)
                    * glam::Mat4::from_cols_array_2d(&m_proj))
                .to_cols_array_2d(),
            };
            queue.write_buffer(&self.global_uniform_buf, 0, bytemuck::bytes_of(&globals));
        }

        // pre-create the bind groups so that we don't need to do it on the fly
        let local_bgl = &self.local_bind_group_layout;
        let entity_count = scene
            .world
            .query::<(&bc::Entity, &bc::Color)>()
            .with::<bc::Vertex<super::Position>>()
            .iter()
            .count();
        let uniform_pool_size = self.uniform_pool.buffer_count::<Locals>(entity_count);
        for uniform_buf_index in 0..uniform_pool_size {
            let key = LocalKey { uniform_buf_index };
            let binding = self.uniform_pool.binding::<Locals>(uniform_buf_index);

            self.local_bind_groups.entry(key).or_insert_with(|| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("solid locals"),
                    layout: local_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(binding),
                    }],
                })
            });
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("solid"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(camera.background.into()),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.as_ref().unwrap().0,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &self.global_bind_group, &[]);

            for (_, (entity, color)) in scene
                .world
                .query::<(&bc::Entity, &bc::Color)>()
                .with::<bc::Vertex<super::Position>>()
                .iter()
            {
                let space = &nodes[entity.node];
                let locals = Locals {
                    pos_scale: space.pos_scale,
                    rot: space.rot,
                    color: color.into_vec4(),
                };
                let bl = self.uniform_pool.alloc(&locals, device, queue);

                let key = LocalKey {
                    uniform_buf_index: bl.index,
                };
                let local_bg = &self.local_bind_groups[&key];
                pass.set_bind_group(1, local_bg, &[bl.offset]);

                let mesh = context.get_mesh(entity.mesh);
                let pos_vs = mesh.vertex_stream::<super::Position>().unwrap();
                pass.set_vertex_buffer(0, mesh.buffer.slice(pos_vs.offset..));

                if let Some(ref is) = mesh.index_stream {
                    pass.set_index_buffer(mesh.buffer.slice(is.offset..), is.format);
                    pass.draw_indexed(0..is.count, 0, 0..1);
                } else {
                    pass.draw(0..mesh.vertex_count, 0..1);
                }
            }
        }

        queue.submit(Some(encoder.finish()));
    }
}
