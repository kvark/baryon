fn main() {
    use baryon::window::{Event, Window};

    let window = Window::new().title("Load OBJ").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let mut scene = baryon::Scene::new();

    let camera = baryon::Camera {
        projection: baryon::Projection::Perspective { fov_y: 45.0 },
        depth: 1.0..10.0,
        node: scene
            .add_node()
            .position([-3f32, 2.0, 5.0].into())
            .look_at([1.0, 0.0, 0.0].into(), [0f32, 1.0, 0.0].into())
            .build(),
        background: baryon::Color(0xFF203040),
    };

    let _entities = baryon::asset::load_obj(
        format!("{}/examples/assets/car.obj", env!("CARGO_MANIFEST_DIR")),
        &mut scene,
        baryon::NodeRef::default(),
        &mut context,
    );

    let mut pass = baryon::pass::Solid::new(
        &baryon::pass::SolidConfig {
            cull_back_faces: true,
        },
        &context,
    );

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
