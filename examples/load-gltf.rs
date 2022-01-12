fn main() {
    use baryon::window::{Event, Window};

    let window = Window::new().title("Load GLTF").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let mut scene = baryon::Scene::new();

    let node = scene.add_node().build();
    let module = baryon::asset::load_gltf(
        format!(
            //"{}/examples/assets/Duck/Duck.gltf",
            "{}/../rendering-demo-scenes/pbr-test/pbr-test.glb",
            env!("CARGO_MANIFEST_DIR")
        ),
        &mut scene,
        node,
        &mut context,
    );

    let mut pass = baryon::pass::Real::new(
        &baryon::pass::RealConfig {
            cull_back_faces: true,
            max_lights: 4,
        },
        &context,
    );

    window.run(move |event| match event {
        Event::Resize { width, height } => {
            context.resize(width, height);
        }
        Event::Draw => {
            let camera = module.cameras.find("Camera").unwrap();
            context.present(&mut pass, &scene, camera);
        }
        _ => {}
    })
}
