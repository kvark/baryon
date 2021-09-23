fn main() {
    use baryon::window::{Event, Window};

    let window = Window::new().title("Empty").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let scene = baryon::Scene::new();
    let mut camera = baryon::Camera::default();
    camera.background = baryon::Color(0xFF203040);
    let mut pass = baryon::pass::Clear;

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
