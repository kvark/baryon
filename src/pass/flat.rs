use bc::ContextDetail as _;
use std::mem;

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
    // x0,y0, x1,y1
    bounds: [f32; 4],
    // u0,v0, u1,v1
    tex_coords: [f32; 4],
}

#[derive(Eq, Hash, PartialEq)]
struct LocalKey {
    uniform_buf_index: usize,
    image: crate::ImageRef,
}

struct Pipelines {
    transparent: wgpu::RenderPipeline,
}

struct Instance {
    camera_distance: f32,
    locals_bl: super::BufferLocation,
    image: crate::ImageRef,
}

pub struct Flat {
    global_uniform_buf: wgpu::Buffer,
    global_bind_group: wgpu::BindGroup,
    local_bind_group_layout: wgpu::BindGroupLayout,
    local_bind_groups: fxhash::FxHashMap<LocalKey, wgpu::BindGroup>,
    uniform_pool: super::BufferPool,
    pipelines: Pipelines,
    temp: Vec<Instance>,
}

impl Flat {
    pub fn new(context: &crate::Context) -> Self {
        Self::new_offscreen(context.surface_info().unwrap(), context)
    }
    pub fn new_offscreen(target_info: crate::TargetInfo, context: &crate::Context) -> Self {
        let d = context.device();
        let shader_module = d.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("flat"),
            source: wgpu::ShaderSource::Wgsl(include_str!("flat.wgsl").into()),
        });

        let globals_size = mem::size_of::<Globals>() as wgpu::BufferAddress;
        let global_bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("flat globals"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(globals_size),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let global_uniform_buf = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flat globals"),
            size: globals_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = d.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("flat sampler"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let global_bind_group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("flat globals"),
            layout: &global_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: global_uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let locals_size = mem::size_of::<Locals>() as wgpu::BufferAddress;
        let local_bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("flat locals"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(locals_size),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("flat"),
            bind_group_layouts: &[&global_bgl, &local_bgl],
            push_constant_ranges: &[],
        });

        let pipelines = {
            let transparent = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("flat-transparent"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    buffers: &[],
                    module: &shader_module,
                    entry_point: "main_vs",
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: target_info.sample_count,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    targets: &[wgpu::ColorTargetState {
                        format: target_info.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::all(),
                    }],
                    module: &shader_module,
                    entry_point: "main_fs",
                }),
                multiview: None,
            });

            Pipelines { transparent }
        };

        Self {
            global_uniform_buf,
            global_bind_group,
            local_bind_group_layout: local_bgl,
            local_bind_groups: Default::default(),
            uniform_pool: super::BufferPool::uniform("flat locals", d),
            pipelines,
            temp: Vec::new(),
        }
    }
}

impl bc::Pass for Flat {
    fn draw(
        &mut self,
        targets: &[crate::TargetRef],
        scene: &crate::Scene,
        camera: &crate::Camera,
        context: &crate::Context,
    ) {
        let target = context.get_target(targets[0]);
        let device = context.device();

        let nodes = scene.bake();
        let cam_node = &nodes[camera.node];
        self.uniform_pool.reset();
        let queue = context.queue();

        {
            let m_proj = camera.projection_matrix(target.aspect());
            let m_view_inv = cam_node.inverse_matrix();
            let m_final = glam::Mat4::from(m_proj) * glam::Mat4::from(m_view_inv);
            let globals = Globals {
                view_proj: m_final.to_cols_array_2d(),
            };
            queue.write_buffer(&self.global_uniform_buf, 0, bytemuck::bytes_of(&globals));
        }

        // gather all sprites
        self.temp.clear();
        self.uniform_pool.reset();
        let cam_dir = glam::Quat::from_slice(&cam_node.rot) * -glam::Vec3::Z;

        for (_, (sprite,)) in scene.world.query::<(&bc::Sprite,)>().iter() {
            let space = &nodes[sprite.node];
            let cam_vector = glam::Vec3::from_slice(&space.pos_scale)
                - glam::Vec3::from_slice(&cam_node.pos_scale);
            let camera_distance = cam_vector.dot(cam_dir);

            let image = context.get_image(sprite.image);
            let locals = Locals {
                pos_scale: space.pos_scale,
                rot: space.rot,
                bounds: {
                    let (w, h) = match sprite.uv {
                        Some(ref uv) => (uv.end.x - uv.start.x, uv.end.y - uv.start.y),
                        None => (image.size.width as i16, image.size.height as i16),
                    };
                    [
                        -0.5 * w as f32,
                        -0.5 * h as f32,
                        0.5 * w as f32,
                        0.5 * w as f32,
                    ]
                },
                tex_coords: match sprite.uv {
                    Some(ref uv) => [
                        uv.start.x as f32 / image.size.width as f32,
                        uv.start.y as f32 / image.size.height as f32,
                        uv.end.x as f32 / image.size.width as f32,
                        uv.end.y as f32 / image.size.height as f32,
                    ],
                    None => [0.0, 0.0, 1.0, 1.0],
                },
            };
            let locals_bl = self.uniform_pool.alloc(&locals, queue);

            // pre-create local bind group, if needed
            let local_bgl = &self.local_bind_group_layout;
            let key = LocalKey {
                uniform_buf_index: locals_bl.index,
                image: sprite.image,
            };
            let binding = self.uniform_pool.binding::<Locals>(locals_bl.index);
            self.local_bind_groups.entry(key).or_insert_with(|| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("flat locals"),
                    layout: local_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(binding),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&image.view),
                        },
                    ],
                })
            });

            self.temp.push(Instance {
                camera_distance,
                locals_bl,
                image: sprite.image,
            });
        }

        // sort from back to front
        self.temp
            .sort_by_key(|s| (s.camera_distance * -1000.0) as i64);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("flat"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(camera.background.into()),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            pass.set_pipeline(&self.pipelines.transparent);
            pass.set_bind_group(0, &self.global_bind_group, &[]);

            for inst in self.temp.drain(..) {
                let key = LocalKey {
                    uniform_buf_index: inst.locals_bl.index,
                    image: inst.image,
                };
                let local_bg = &self.local_bind_groups[&key];
                pass.set_bind_group(1, local_bg, &[inst.locals_bl.offset]);
                pass.draw(0..4, 0..1);
            }
        }

        queue.submit(Some(encoder.finish()));
    }
}
