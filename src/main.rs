extern crate image;
extern crate nalgebra as na;
#[macro_use]
extern crate vulkano;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate vulkano_win;
extern crate winit;

use vulkano::instance::Instance;

use std::mem;
use std::sync::Arc;

fn main() {
    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
    };

    let physical = vulkano::instance::PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no device available");
    println!(
        "Using device: {} (type: {:?})",
        physical.name(),
        physical.ty()
    );
}

// #[macro_use]
// extern crate glium;
// extern crate cgmath;
// extern crate image;
// extern crate nalgebra as na;

// // use cgmath::Zero;
// // use cgmath::{Matrix4, Point3, Vector3};
// use na::geometry::Perspective3;
// use na::{Matrix4, Point3, Vector3};
// use std::io::Cursor;

// mod teapot;

// fn view_matrix(position: &[f32; 3], direction: &[f32; 3], up: &[f32; 3]) -> [[f32; 4]; 4] {
//     let f = {
//         let f = direction;
//         let len = f[0] * f[0] + f[1] * f[1] + f[2] * f[2];
//         let len = len.sqrt();
//         [f[0] / len, f[1] / len, f[2] / len]
//     };

//     let s = [
//         up[1] * f[2] - up[2] * f[1],
//         up[2] * f[0] - up[0] * f[2],
//         up[0] * f[1] - up[1] * f[0],
//     ];

//     let s_norm = {
//         let len = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
//         let len = len.sqrt();
//         [s[0] / len, s[1] / len, s[2] / len]
//     };

//     let u = [
//         f[1] * s_norm[2] - f[2] * s_norm[1],
//         f[2] * s_norm[0] - f[0] * s_norm[2],
//         f[0] * s_norm[1] - f[1] * s_norm[0],
//     ];

//     let p = [
//         -position[0] * s_norm[0] - position[1] * s_norm[1] - position[2] * s_norm[2],
//         -position[0] * u[0] - position[1] * u[1] - position[2] * u[2],
//         -position[0] * f[0] - position[1] * f[1] - position[2] * f[2],
//     ];

//     [
//         [s_norm[0], u[0], f[0], 0.0],
//         [s_norm[1], u[1], f[1], 0.0],
//         [s_norm[2], u[2], f[2], 0.0],
//         [p[0], p[1], p[2], 1.0],
//     ]
// }

// fn main() {
//     use glium::{glutin, Surface};

//     let mut events_loop = glutin::EventsLoop::new();
//     let window = glutin::WindowBuilder::new();
//     let context = glutin::ContextBuilder::new().with_depth_buffer(24);
//     let display = glium::Display::new(window, context, &events_loop).unwrap();

//     let image = image::load(
//         Cursor::new(&include_bytes!("/Users/malcolm/Projects/rust/rscraft/assets/kitty.jpg")[..]),
//         image::JPEG,
//     ).unwrap()
//         .to_rgba();
//     let image_dimensions = image.dimensions();
//     let image =
//         glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
//     let _texture = glium::texture::Texture2d::new(&display, image).unwrap();

//     #[derive(Copy, Clone)]
//     struct Vertex {
//         position: [f32; 2],
//         tex_coords: [f32; 2],
//     }

//     implement_vertex!(Vertex, position, tex_coords);

//     let positions = glium::VertexBuffer::new(&display, &teapot::VERTICES).unwrap();
//     let normals = glium::VertexBuffer::new(&display, &teapot::NORMALS).unwrap();
//     let indices = glium::IndexBuffer::new(
//         &display,
//         glium::index::PrimitiveType::TrianglesList,
//         &teapot::INDICES,
//     ).unwrap();

//     let vertex_shader_src = r#"
// #version 330

// in vec3 position;
// in vec3 normal;

// out vec3 v_normal;

// uniform mat4 perspective;
// uniform mat4 view;
// uniform mat4 model;

// void main() {
//     mat4 modelview = view * model;
//     v_normal = transpose(inverse(mat3(modelview))) * normal;
//     gl_Position = perspective * modelview * vec4(position, 1.0);
// }
//     "#;

//     let fragment_shader_src = r#"
// #version 330

// in vec3 v_normal;
// out vec4 color;
// uniform vec3 u_light;
// void main() {
//     float brightness = dot(normalize(v_normal), normalize(u_light));
//     vec3 dark_color = vec3(0.6, 0.0, 0.0);
//     vec3 regular_color = vec3(1.0, 0.0, 0.0);
//     color = vec4(mix(dark_color, regular_color, brightness), 1.0);
// }
//     "#;

//     let program =
//         glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
//             .unwrap();

//     let mut t: f32 = -2.0;
//     let mut closed = false;
//     let mut mode = true;
//     while !closed {
//         // we update `t`
//         t += 0.004;
//         if t > 2.0 {
//             t = -2.0;
//         }

//         let mut target = display.draw();
//         target.clear_color_and_depth((0.1, 0.1, 0.2, 1.0),  -99999999.0);

//         // let model = [
//         //     [t.cos() * 0.01, t.sin() * 0.01, 0.0, 0.0],
//         //     [-t.sin() * 0.01, t.cos() * 0.01, 0.0, 0.0],
//         //     [0.0, 0.0, 0.01, 0.0],
//         //     [0.0, 0.0, 4.0, 1.0f32],
//         // ];

//         let rot = Matrix4::from_scaled_axis(&Vector3::x() * 3.14 * t);

//         let model_matrix =
//             Matrix4::new_scaling(0.1).append_translation(&Vector3::new(1.0f32, t * 10.0, 90.0f32))
//                 * rot;

//         let model: [[f32; 4]; 4] = model_matrix.into();

//         let perspective = {
//             let (width, height) = target.get_dimensions();
//             let aspect_ratio = height as f32 / width as f32;

//             let fov: f32 = 3.141592 / 3.0;
//             let zfar = 1024.0;
//             let znear = 0.1;

//             let f = 1.0 / (fov / 2.0).tan();

//             [
//                 [f * aspect_ratio, 0.0, 0.0, 0.0],
//                 [0.0, f, 0.0, 0.0],
//                 [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
//                 [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
//             ]
//         };

//         let (width, height) = target.get_dimensions();
//         let aspect_ratio = height as f32 / width as f32;
//         let fov: f32 = 3.141592 / 3.0;
//         // let perspective2: [[f32; 4]; 4] = Matrix4::new_perspective(aspect_ratio, fov, 0.1f32, 1024.0f32).into();

//         let perspective2: [[f32; 4]; 4] =
//             Perspective3::new(aspect_ratio, fov, 0.1f32, 1024.0f32).to_homogeneous().into();

//         println!("p1: {:?}\np2: {:?}", perspective, perspective2);

//         // let x = Matrix4::zero<fl>();
//         // let eye: Point3<f32> = Point3::new(2.0f32, 0.0f32, 2.0f32);
//         // let dir: Vector3<f32> = Vector3::new(10f32, 0f32, 2.0f32);
//         // let up: Vector3<f32> = Vector3::new(0f32, 0f32, 1f32);
//         // let look_at_dir: Matrix4<f32> = Matrix4::look_at_dir(eye, dir, up);

//         let eye = Point3::new(1.0, 0.0, 1.0);
//         let look_target = Point3::new(1.0, 0.0, 0.0);
//         let naview = Matrix4::look_at_lh(&eye, &look_target, &Vector3::y());

//         // the direction of the light
//         let light = [-1.0, 0.4, 0.9f32];

//         let params = glium::DrawParameters {
//             depth: glium::Depth {
//                 test: glium::draw_parameters::DepthTest::IfMoreOrEqual,
//                 write: true,
//                 ..Default::default()
//             },
//             backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
//             ..Default::default()
//         };

//         let view = view_matrix(&[2.0, -1.0, 1.0], &[-2.0, 1.0, 1.0], &[0.0, 1.0, 0.0]);

//         // println!("x: {:?}\nview:      {:?}\n", naview, view);

//         let view_param: [[f32; 4]; 4];
//         if mode {
//             view_param = naview.into();
//         } else {
//             view_param = view;
//         }

//         target
//             .draw(
//                 (&positions, &normals),
//                 &indices,
//                 &program,
//                 &uniform! { model: model, view: view_param, perspective: perspective2, u_light: light },
//                 &params,
//             )
//             .unwrap();
//         target.finish().unwrap();

//         events_loop.poll_events(|event| match event {
//             glutin::Event::WindowEvent { event, .. } => match event {
//                 glutin::WindowEvent::Closed => closed = true,
//                 glutin::WindowEvent::ReceivedCharacter('m') => {
//                     mode = !mode;
//                 }
//                 glutin::WindowEvent::ReceivedCharacter('q') => {
//                     println!("Received q, quitting");
//                     closed = true
//                 }
//                 glutin::WindowEvent::ReceivedCharacter(a) => println!("ReceivedCharacter: {:?}", a),
//                 x => println!("Glutin Window Event: {:?}", x),
//             },
//             _ => (),
//         });
//     }
// }
