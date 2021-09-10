fn main() {
    let window = baryon::Window::new().title("Clear").build();
    let mut context = pollster::block_on(baryon::Context::new().screen(&window).build());
    let mut scene = baryon::Scene::default();
    scene.background = baryon::Color(0xFF203040);
    context.render_screen(&scene);
}
