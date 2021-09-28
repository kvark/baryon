fn main() {
    use baryon::window::{Event, Window};

    let window = Window::new().title("Load GLTF").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let mut scene = baryon::Scene::new();

    let camera = baryon::Camera {
        projection: baryon::Projection::Perspective { fov_y: 45.0 },
        depth: 1.0..100.0,
        node: scene
            .add_node()
            .position([1f32, 2.0, 3.0].into())
            .look_at([0.0, 0.8, 0.0].into(), [0f32, 1.0, 0.0].into())
            .build(),
        background: baryon::Color(0xFF203040),
    };

    let node = scene.add_node().build();
    let _ = baryon::asset::load_gltf(
        format!(
            "{}/examples/assets/Duck/Duck.gltf",
            env!("CARGO_MANIFEST_DIR")
        ),
        &mut scene,
        node,
        &mut context,
    );

    let _point_light = scene
        .add_point_light()
        .position([3.0, 3.0, 3.0].into())
        .color(baryon::Color(0xFFFFFFFF))
        .intensity(2.0)
        .build();

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
            scene[node].pre_rotate([0.0, 1.0, 0.0].into(), 1.0);
            context.present(&mut pass, &scene, &camera);
        }
        _ => {}
    })
}
