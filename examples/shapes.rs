// https://nical.github.io/posts/lyon-intro.html

use baryon::{
    geometry::{Geometry, Streams},
    window::{Event, Window},
    pass, Camera, Color,
};
use lyon::{algorithms::aabb::bounding_rect, math::point, path::Path};

fn main() {
    let window = Window::new().title("Shape").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let mut scene = baryon::Scene::new();

    // Build a Path.
    let mut builder = Path::builder();
    builder.begin(point(5.0, 5.0));
    builder.cubic_bezier_to(point(5.0, 5.0), point(4.0, 0.0), point(0.0, 0.0));
    builder.cubic_bezier_to(point(-6.0, 0.0), point(-6.0, 7.0), point(-6.0, 7.0));
    builder.cubic_bezier_to(point(-6.0, 11.0), point(-3.0, 15.4), point(5.0, 19.0));
    builder.cubic_bezier_to(point(12.0, 15.4), point(16.0, 11.0), point(16.0, 7.0));
    builder.cubic_bezier_to(point(16.0, 7.0), point(16.0, 0.0), point(10.0, 0.0));
    builder.cubic_bezier_to(point(7.0, 0.0), point(5.0, 5.0), point(5.0, 5.0));
    builder.end(true);
    let path = builder.build();
    let bbox = bounding_rect(path.iter());
    let shape_prototype = Geometry::shape(Streams::empty(), path).bake(&mut context);
    scene
        .add_entity(&shape_prototype)
        .component(Color::RED)
        .position(mint::Vector3 {
            x: bbox.size.width / -4.0,
            y: bbox.size.height / -2.0,
            z: 0.0,
        })
        .build();

    let camera = baryon::Camera {
        projection: baryon::Projection::Perspective { fov_y: 70.0 },
        depth: 1.0..100.0,
        node: scene
            .add_node()
            .position([0.0f32, 0.0, -30.0].into())
            .look_at([0f32; 3].into(), [0f32, -1.0, 0.0].into())
            .build(),
        background: baryon::Color(0xFF203040),
    };

    let mut pass = pass::Solid::new(
        &pass::SolidConfig {
            cull_back_faces: false,
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
