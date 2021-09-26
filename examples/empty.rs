// Custom implementation of a `Pass` that just clears.
pub struct Clear;
impl baryon::Pass for Clear {
    fn draw(
        &mut self,
        targets: &[baryon::TargetRef],
        _scene: &baryon::Scene,
        camera: &baryon::Camera,
        context: &baryon::Context,
    ) {
        use bc::ContextDetail as _;

        let target = context.get_target(targets[0]);
        let mut encoder = context
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
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

fn main() {
    use baryon::window::{Event, Window};

    let window = Window::new().title("Empty").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let scene = baryon::Scene::new();
    let mut camera = baryon::Camera::default();
    camera.background = baryon::Color(0xFF203040);
    let mut pass = Clear;

    window.run(move |event| match event {
        Event::Resize { width, height } => {
            context.resize(width, height);
        }
        Event::Draw => {
            context.present(&mut pass, &scene, &camera);
        }
        _ => {}
    })
}
