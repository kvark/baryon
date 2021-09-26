use bc::ContextDetail as _;
use std::ops;

pub struct Sprite {
    pub image: crate::ImageRef,
    pub texels: ops::Range<mint::Point2<i16>>,
    pub alpha: Option<f32>,
}

#[derive(Default)]
pub struct Flat;

impl bc::Pass for Flat {
    fn draw(
        &mut self,
        targets: &[crate::TargetRef],
        _scene: &crate::Scene,
        camera: &crate::Camera,
        context: &crate::Context,
    ) {
        let target = context.get_target(targets[0]);
        let mut encoder = context
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        }

        context.queue().submit(Some(encoder.finish()));
    }
}
