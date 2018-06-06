#[macro_use]
extern crate vulkano;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate vulkano_win;
extern crate winit;

use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::DynamicState;
use vulkano::device::Device;
use vulkano::framebuffer::Framebuffer;
use vulkano::framebuffer::Subpass;
use vulkano::instance::Instance;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain;
use vulkano::swapchain::AcquireError;
use vulkano::swapchain::PresentMode;
use vulkano::swapchain::SurfaceTransform;
use vulkano::swapchain::Swapchain;
use vulkano::swapchain::SwapchainCreationError;
use vulkano::sync::GpuFuture;
use vulkano::sync::now;
use vulkano_win::VkSurfaceBuild;

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

    let mut events_loop = winit::EventsLoop::new();
    let window = winit::WindowBuilder::new()
        .build_vk_surface(&events_loop, instance.clone())
        .unwrap();

    let mut dimensions = {
        let (width, height) = window.window().get_inner_size().unwrap();
        [width, height]
    };

    println!("Dimensions: {}, {}", dimensions[0], dimensions[1]);

    let queue = physical
        .queue_families()
        .find(|&q| {
            // We take the first queue that supports drawing to our window.
            q.supports_graphics() && window.is_supported(q).unwrap_or(false)
        })
        .expect("couldn't find a graphical queue family");

    let (device, mut queues) = {
        let device_ext = vulkano::device::DeviceExtensions {
            khr_swapchain: true,
            ..vulkano::device::DeviceExtensions::none()
        };

        Device::new(
            physical,
            physical.supported_features(),
            &device_ext,
            [(queue, 0.5)].iter().cloned(),
        ).expect("failed to create device")
    };

    let queue = queues.next().unwrap();

    let (mut swapchain, mut images) = {
        let caps = window
            .capabilities(physical)
            .expect("failed to get surface capabilities");

        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        dimensions = caps.current_extent.unwrap_or(dimensions);

        let format = caps.supported_formats[0].0;

        Swapchain::new(
            device.clone(),
            window.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            caps.supported_usage_flags,
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            true,
            None,
        ).expect("failed to create swapchain")
    };

    let vertex_buffer = {
        #[derive(Debug, Clone)]
        struct Vertex {
            position: [f32; 2],
        }
        impl_vertex!(Vertex, position);

        CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            [
                Vertex {
                    position: [-0.5, -0.25],
                },
                Vertex {
                    position: [0.0, 0.5],
                },
                Vertex {
                    position: [0.25, -0.1],
                },
            ].iter()
                .cloned(),
        ).expect("failed to create buffer")
    };

    mod vs {
        #[derive(VulkanoShader)]
        #[ty = "vertex"]
        #[src = "
#version 450
layout(location = 0) in vec2 position;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}
"]
        struct Dummy;
    }

    mod fs {
        #[derive(VulkanoShader)]
        #[ty = "fragment"]
        #[src = "
#version 450
layout(location = 0) out vec4 f_color;
void main() {
    f_color = vec4(1.0, 0.0, 0.0, 1.0);
}
"]
        struct Dummy;
    }

    let vs = vs::Shader::load(device.clone()).expect("failed to create shader module");
    let fs = fs::Shader::load(device.clone()).expect("failed to create shader module");

    let render_pass = Arc::new(
        single_pass_renderpass!(device.clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    ).unwrap(),
    );

    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let mut framebuffers: Option<Vec<Arc<vulkano::framebuffer::Framebuffer<_, _>>>> = None;

    let mut recreate_swapchain = false;

    let mut previous_frame_end = Box::new(now(device.clone())) as Box<GpuFuture>;

    loop {
        previous_frame_end.cleanup_finished();

        if recreate_swapchain {
            dimensions = {
                let (new_width, new_height) = window.window().get_inner_size().unwrap();
                [new_width, new_height]
            };

            println!("Dimensions: {}, {}", dimensions[0], dimensions[1]);

            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => {
                    //  { max_supported }
                    // println!("continue... {:?}", max_supported);
                    continue;
                }
                Err(err) => panic!("{:?}", err),
            };

            mem::replace(&mut swapchain, new_swapchain);
            mem::replace(&mut images, new_images);

            framebuffers = None;

            recreate_swapchain = false;
        }
        if framebuffers.is_none() {
            let new_framebuffers = Some(
                images
                    .iter()
                    .map(|image| {
                        Arc::new(
                            Framebuffer::start(render_pass.clone())
                                .add(image.clone())
                                .unwrap()
                                .build()
                                .unwrap(),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            mem::replace(&mut framebuffers, new_framebuffers);
        }
        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    recreate_swapchain = true;
                    continue;
                }
                Err(err) => panic!("{:?}", err),
            };

        let command_buffer =
            AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
                .unwrap()
                .begin_render_pass(
                    framebuffers.as_ref().unwrap()[image_num].clone(),
                    false,
                    vec![[0.0, 0.0, 1.0, 1.0].into()],
                )
                .unwrap()
                .draw(
                    pipeline.clone(),
                    DynamicState {
                        line_width: None,
                        viewports: Some(vec![
                            Viewport {
                                origin: [0.0, 0.0],
                                dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                                depth_range: 0.0..1.0,
                            },
                        ]),
                        scissors: None,
                    },
                    vertex_buffer.clone(),
                    (),
                    (),
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap();

        let future = previous_frame_end
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
            .then_signal_fence_and_flush()
            .unwrap();
        previous_frame_end = Box::new(future) as Box<_>;

        let mut done = false;
        events_loop.poll_events(|ev| match ev {
            winit::Event::WindowEvent {
                event: winit::WindowEvent::Closed,
                ..
            } => done = true,
            winit::Event::WindowEvent {
                event: winit::WindowEvent::Resized(_, _),
                ..
            } => recreate_swapchain = true,
            _ => (),
        });
        if done {
            return;
        }
    }
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
