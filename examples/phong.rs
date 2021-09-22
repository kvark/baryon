use std::iter;

fn create_prototype(builder: baryon::MeshBuilder) -> baryon::Prototype {
    fn vertex(x: i8, y: i8, z: i8) -> baryon::Position {
        baryon::Position([x as f32, y as f32, z as f32])
    }

    let positions = [
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
    let normals = [
        baryon::Normal([0.0, 0.0, 1.0]),
        baryon::Normal([0.0, 0.0, -1.0]),
        baryon::Normal([1.0, 0.0, 0.0]),
        baryon::Normal([-1.0, 0.0, 0.0]),
        baryon::Normal([0.0, 1.0, 0.0]),
        baryon::Normal([0.0, -1.0, 0.0]),
    ]
    .iter()
    .flat_map(|&n| iter::repeat(n).take(4))
    .collect::<Vec<_>>();

    let index_data = [
        0u16, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    builder
        .vertex(&positions)
        .vertex(&normals)
        .index(&index_data)
        .radius(1.5)
        .build()
}

fn main() {
    use baryon::window::{Event, Window};

    env_logger::init();
    let window = Window::new().title("Phong").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let mut scene = baryon::Scene::new();

    let camera = baryon::Camera {
        projection: baryon::Projection::Perspective { fov_y: 45.0 },
        depth: 1.0..10.0,
        node: scene
            .add_node()
            .position([-1.8f32, 5.0, 2.0].into())
            .look_at([0f32; 3].into(), [0f32, 0.0, 1.0].into())
            .build(),
        background: baryon::Color(0xFF203040),
    };

    let _point_light = scene
        .add_point_light()
        .position([3.0, 3.0, 3.0].into())
        .color(baryon::Color(0xFFFF8080))
        .build();
    let _dir_light = scene
        .add_directional_light()
        .position([0.0, 0.0, 5.0].into())
        .intensity(3.0)
        .color(baryon::Color(0xFF8080FF))
        .build();

    let prototype = create_prototype(context.add_mesh());
    let _cube = scene
        .add_entity(&prototype)
        .component(baryon::Color(0xFF808080))
        .component(baryon::pass::Shader::Flat)
        .build();

    let mut pass = baryon::pass::Phong::new(
        &baryon::pass::PhongConfig {
            cull_back_faces: true,
            max_lights: 10,
            ambient: baryon::pass::Ambient {
                color: baryon::Color(0xFFFFFFFF),
                intensity: 0.2,
            },
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
