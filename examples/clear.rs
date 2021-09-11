use baryon::window::{Event, Window};

fn main() {
    let window = Window::new().title("Clear").build();
    let mut context = pollster::block_on(baryon::Context::new().screen(&window).build());
    let mut scene = baryon::Scene::default();
    scene.background = baryon::Color(0xFF203040);
    window.run(move |event| match event {
        Event::Draw => {
            context.render_screen(&scene);
        }
        _ => {}
    })
}
