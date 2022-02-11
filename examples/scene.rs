fn main() {
    use baryon::{
        geometry::{Geometry, Streams},
        pass::{Phong, PhongConfig, Shader},
        window::{Event, Window},
        Camera, Color, Context, Projection, Scene,
    };

    let window = Window::new().title("Scene").build();
    let mut context = pollster::block_on(Context::init().build(&window));
    let mut scene = Scene::new();

    let camera = Camera {
        projection: Projection::Perspective { fov_y: 45.0 },
        depth: 1.0..10.0,
        node: scene
            .add_node()
            .position([-2.0, 2.5, 5.0].into())
            .look_at([0f32; 3].into(), [0f32, 1.0, 0.0].into())
            .build(),
        background: Color::BLACK_OPAQUE,
    };

    scene
        .add_point_light()
        .position([4.0, 8.0, 4.0].into())
        .color(Color(0x00AAAAAA))
        .build();
    scene
        .add_entity(&Geometry::plane(5.0).bake(&mut context))
        .position([0.0, 0.0, 0.0].into())
        .component(Color(0xFF006400))
        .component(Shader::Phong { glossiness: 100 })
        .build();
    scene
        .add_entity(&Geometry::cuboid(Streams::NORMAL, [0.5, 0.5, 0.5].into()).bake(&mut context))
        .position([0.0, 0.25, 0.0].into())
        .component(Color::new(0.8, 0.7, 0.6, 1.0))
        .component(Shader::Phong { glossiness: 100 })
        .build();

    let mut pass = Phong::new(
        &PhongConfig {
            ..Default::default()
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
