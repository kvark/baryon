fn main() {
    use baryon::window::{Event, Window};

    env_logger::init();
    let window = Window::new().title("Clear").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let mut scene = baryon::Scene::new();
    scene.background = baryon::Color(0xFF203040);
    let mut pass = baryon::pass::Clear;

    window.run(move |event| match event {
        Event::Resize { width, height } => {
            context.resize(width, height);
        }
        Event::Draw => {
            context.present(&mut pass, &scene);
        }
        _ => {}
    })
}
