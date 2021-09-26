fn main() {
    use baryon::window::{Event, Window};

    let window = Window::new().title("Sprite").build();
    let mut context = pollster::block_on(baryon::Context::init().build(&window));
    let scene = baryon::Scene::new();
    let camera = baryon::Camera {
        projection: baryon::Projection::Orthographic {
            center: [0.0, 0.0].into(),
            extent_y: 10.0,
        },
        ..Default::default()
    };
    let mut pass = baryon::pass::Flat;

    let _image = context.load_image("../three/test_data/pikachu_anim.png");
    //let _entity = scene.add_entity()

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
