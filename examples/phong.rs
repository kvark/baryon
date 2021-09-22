fn main() {
    use baryon::{
        geometry::{Geometry, Streams},
        window::{Event, Window},
    };

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

    let prototype = Geometry::cuboid(
        Streams::NORMAL,
        mint::Vector3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    )
    .bake(&mut context);
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
