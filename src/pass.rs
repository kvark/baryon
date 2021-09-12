use std::iter;

pub trait Pass {
    fn draw(
        &mut self,
        targets: &[crate::TargetRef],
        scene: &crate::Scene,
        context: &crate::Context,
    );
}

pub struct Clear;

impl Pass for Clear {
    fn draw(
        &mut self,
        targets: &[crate::TargetRef],
        scene: &crate::Scene,
        context: &crate::Context,
    ) {
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &context.get_target(targets[0]).view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(scene.background.into()),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
        }

        context.queue.submit(iter::once(encoder.finish()));
    }
}
