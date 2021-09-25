use bc::ContextDetail as _;
use fxhash::FxHashMap;
use std::mem;

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub emissive_color: crate::Color,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            emissive_color: crate::Color(0),
            metallic_factor: 1.0,
            roughness_factor: 0.0,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
        }
    }
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Light {
    pos: [f32; 4],
    rot: [f32; 4],
    color_intensity: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Locals {
    pos_scale: [f32; 4],
    rot: [f32; 4],
    base_color_factor: [f32; 4],
    emissive_factor: [f32; 4],
    metallic_roughness_values: [f32; 2],
    normal_scale: f32,
    occlusion_strength: f32,
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
pub struct RealConfig {
    pub cull_back_faces: bool,
    pub max_lights: usize,
}

impl Default for RealConfig {
    fn default() -> Self {
        Self {
            cull_back_faces: true,
            max_lights: 16,
        }
    }
}

struct PipelineInfo {
    layout: wgpu::PipelineLayout,
    shader_module: wgpu::ShaderModule,
    primitive_state: wgpu::PrimitiveState,
}

/// Realistic renderer.
/// Follows Disney PBR.
pub struct Real {
    depth_texture: Option<(wgpu::TextureView, wgpu::Extent3d)>,
    global_uniform_buf: wgpu::Buffer,
    light_buf: wgpu::Buffer,
    light_capacity: usize,
    global_bind_group: wgpu::BindGroup,
    local_bind_group_layout: wgpu::BindGroupLayout,
    local_bind_groups: FxHashMap<LocalKey, wgpu::BindGroup>,
    uniform_pool: super::BufferPool,
    pipeline_info: PipelineInfo,
    pipelines: FxHashMap<PipelineKey, wgpu::RenderPipeline>,
}

impl Real {
    pub fn new(config: &RealConfig, context: &crate::Context) -> Self {
        let d = context.device();
        let shader_module = d.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("real"),
            source: wgpu::ShaderSource::Wgsl(include_str!("real.wgsl").into()),
        });

        let globals_size = mem::size_of::<Globals>() as wgpu::BufferAddress;
        let global_bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("real globals"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            mem::size_of::<Light>() as wgpu::BufferAddress
                        ),
                    },
                    count: None,
                },
            ],
        });
        let global_uniform_buf = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("real globals"),
            size: globals_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let light_buf = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("real lights"),
            size: (config.max_lights * mem::size_of::<Light>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let global_bind_group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("real globals"),
            layout: &global_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: global_uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buf.as_entire_binding(),
                },
            ],
        });

        let locals_size = mem::size_of::<Locals>() as wgpu::BufferAddress;
        let local_bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("real locals"),
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
            label: Some("real"),
            bind_group_layouts: &[&global_bgl, &local_bgl],
            push_constant_ranges: &[],
        });

        Self {
            depth_texture: None,
            global_uniform_buf,
            light_capacity: config.max_lights,
            light_buf,
            global_bind_group,
            local_bind_group_layout: local_bgl,
            local_bind_groups: Default::default(),
            uniform_pool: super::BufferPool::uniform("real locals", d),
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

impl bc::Pass for Real {
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
        //TODO: abstract this part away
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
                label: Some("real"),
                layout: Some(&info.layout),
                vertex: wgpu::VertexState {
                    buffers: &[crate::Position::layout::<0>(), crate::Normal::layout::<2>()],
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
            let node = &nodes[camera.node];
            let m_view_inv = node.inverse_matrix();
            let m_final = glam::Mat4::from(m_proj) * glam::Mat4::from(m_view_inv);
            let globals = Globals {
                view_proj: m_final.to_cols_array_2d(),
                camera_pos: node.pos_scale,
            };
            queue.write_buffer(&self.global_uniform_buf, 0, bytemuck::bytes_of(&globals));
        }

        let lights = scene
            .lights()
            .map(|(_, light)| {
                let space = &nodes[light.node];
                let mut pos = space.pos_scale;
                pos[3] = match light.kind {
                    bc::LightKind::Directional => 1.0,
                    bc::LightKind::Point => 0.0,
                };
                let mut color_intensity = light.color.into_vec4();
                color_intensity[3] = light.intensity;
                Light {
                    pos,
                    rot: space.rot,
                    color_intensity,
                }
            })
            .collect::<Vec<_>>();
        let light_count = lights.len().min(self.light_capacity);
        queue.write_buffer(
            &self.light_buf,
            0,
            bytemuck::cast_slice(&lights[..light_count]),
        );

        //TODO: abstract this away
        // pre-create the bind groups so that we don't need to do it on the fly
        let local_bgl = &self.local_bind_group_layout;
        let entity_count = scene
            .world
            .query::<(&bc::Entity, &bc::Color, &Material)>()
            .with::<bc::Vertex<crate::Position>>()
            .with::<bc::Vertex<crate::Normal>>()
            .iter()
            .count();
        let uniform_pool_size = self
            .uniform_pool
            .prepare_for_count::<Locals>(entity_count, device);
        for uniform_buf_index in 0..uniform_pool_size {
            let key = LocalKey { uniform_buf_index };
            let binding = self.uniform_pool.binding::<Locals>(uniform_buf_index);

            self.local_bind_groups.entry(key).or_insert_with(|| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("real locals"),
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
                label: Some("real"),
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

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.global_bind_group, &[]);

            for (_, (entity, &color, mat)) in scene
                .world
                .query::<(&bc::Entity, &bc::Color, &Material)>()
                .with::<bc::Vertex<crate::Position>>()
                .with::<bc::Vertex<crate::Normal>>()
                .iter()
            {
                let space = &nodes[entity.node];
                let mesh = context.get_mesh(entity.mesh);

                let locals = Locals {
                    pos_scale: space.pos_scale,
                    rot: space.rot,
                    base_color_factor: color.into_vec4(),
                    emissive_factor: mat.emissive_color.into_vec4(),
                    metallic_roughness_values: [mat.metallic_factor, mat.roughness_factor],
                    normal_scale: mat.normal_scale,
                    occlusion_strength: mat.occlusion_strength,
                };
                let bl = self.uniform_pool.alloc(&locals, queue);

                let key = LocalKey {
                    uniform_buf_index: bl.index,
                };
                let local_bg = &self.local_bind_groups[&key];
                pass.set_bind_group(1, local_bg, &[bl.offset]);

                pass.set_vertex_buffer(0, mesh.vertex_slice::<crate::Position>());
                pass.set_vertex_buffer(1, mesh.vertex_slice::<crate::Normal>());

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