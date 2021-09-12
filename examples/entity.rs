fn main() {
    use baryon::window::{Event, Window};

    let window = Window::new().title("Clear").build();
    let mut context = pollster::block_on(baryon::Context::new().screen(&window).build());
    let mut scene = baryon::Scene::default();
    scene.background = baryon::Color(0xFF203040);
    let _e = scene.entity().component(baryon::Color(0xFFFFFFFF)).build();
    let mut pass = baryon::pass::Clear; //TODO

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
