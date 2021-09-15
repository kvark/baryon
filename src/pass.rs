use baryon_core::ContextDetail as _;
use std::iter;

pub struct Clear;

impl super::Pass for Clear {
    fn draw(
        &mut self,
        targets: &[super::TargetRef],
        scene: &crate::Scene,
        context: &crate::Context,
    ) {
        let mut encoder = context
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: context.get_target(targets[0]),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(scene.background.into()),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
        }

        context.queue().submit(iter::once(encoder.finish()));
    }
}

pub struct Solid {}
