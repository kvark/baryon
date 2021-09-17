fn create_cube(builder: baryon::MeshBuilder) -> baryon::Prototype {
    fn vertex(x: i8, y: i8, z: i8) -> baryon::Position {
        baryon::Position([x as f32, y as f32, z as f32])
    }

    let vertex_data = [
        // top (0, 0, 1)
        vertex(-1, -1, 1),
        vertex(1, -1, 1),
        vertex(1, 1, 1),
        vertex(-1, 1, 1),
        // bottom (0, 0, -1)
        vertex(-1, 1, -1),
        vertex(1, 1, -1),
        vertex(1, -1, -1),
        vertex(-1, -1, -1),
        // right (1, 0, 0)
        vertex(1, -1, -1),
        vertex(1, 1, -1),
        vertex(1, 1, 1),
        vertex(1, -1, 1),
        // left (-1, 0, 0)
        vertex(-1, -1, 1),
        vertex(-1, 1, 1),
        vertex(-1, 1, -1),
        vertex(-1, -1, -1),
        // front (0, 1, 0)
        vertex(1, 1, -1),
        vertex(-1, 1, -1),
        vertex(-1, 1, 1),
        vertex(1, 1, 1),
        // back (0, -1, 0)
        vertex(1, -1, 1),
        vertex(-1, -1, 1),
        vertex(-1, -1, -1),
        vertex(1, -1, -1),
    ];

    let index_data = [
        0u16, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    builder.vertex(&vertex_data).index(&index_data).build()
}

fn main() {
    use baryon::window::{Event, Window};

    env_logger::init();
    let window = Window::new().title("Clear").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let mut scene = baryon::Scene::new();

    let camera = baryon::Camera {
        projection: baryon::Projection::Perspective { fov_y: 45.0 },
        depth: 1.0..10.0,
        node: scene
            .node()
            .position([1.5f32, -5.0, 3.0].into())
            .look_at([0.0f32; 3].into(), [0.0f32, 0.0, 1.0].into())
            .build(),
        background: baryon::Color(0xFF203040),
    };

    let cube = create_cube(context.add_mesh());
    let _e = scene
        .entity(&cube)
        .component(baryon::Color(0xFFFFFFFF))
        .build();
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
