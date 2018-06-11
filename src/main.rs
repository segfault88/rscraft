#[macro_use]
extern crate vulkano;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate cgmath;
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
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::swapchain;
use vulkano::swapchain::AcquireError;
use vulkano::swapchain::PresentMode;
use vulkano::swapchain::SurfaceTransform;
use vulkano::swapchain::Swapchain;
use vulkano::swapchain::SwapchainCreationError;
use vulkano::sync::now;
use vulkano::sync::GpuFuture;
use vulkano_win::VkSurfaceBuild;

use std::mem;
use std::sync::Arc;

mod teapot;

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

        println!(
            "caps image extent min: {:?} max: {:?}, current_extent: {:?}",
            caps.min_image_extent, caps.max_image_extent, caps.current_extent
        );

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

    let mut depth_buffer = vulkano::image::attachment::AttachmentImage::transient(
        device.clone(),
        dimensions,
        vulkano::format::D16Unorm,
    ).unwrap();

    let vertex_buffer = vulkano::buffer::cpu_access::CpuAccessibleBuffer::from_iter(
        device.clone(),
        vulkano::buffer::BufferUsage::all(),
        teapot::VERTICES.iter().cloned(),
    ).expect("failed to create vertex buffer");

    let normals_buffer = vulkano::buffer::cpu_access::CpuAccessibleBuffer::from_iter(
        device.clone(),
        vulkano::buffer::BufferUsage::all(),
        teapot::NORMALS.iter().cloned(),
    ).expect("failed to create normals buffer");

    let index_buffer = vulkano::buffer::cpu_access::CpuAccessibleBuffer::from_iter(
        device.clone(),
        vulkano::buffer::BufferUsage::all(),
        teapot::INDICES.iter().cloned(),
    ).expect("failed to create index buffer");

    // note: this teapot was meant for OpenGL where the origin is at the lower left
    //       instead the origin is at the upper left in vulkan, so we reverse the Y axis
    let mut proj = cgmath::perspective(
        cgmath::Rad(std::f32::consts::FRAC_PI_2),
        { dimensions[0] as f32 / dimensions[1] as f32 },
        0.01,
        100.0,
    );
    let view = cgmath::Matrix4::look_at(
        cgmath::Point3::new(0.3, 0.3, 1.0),
        cgmath::Point3::new(0.0, 0.0, 0.0),
        cgmath::Vector3::new(0.0, -1.0, 0.0),
    );
    let scale = cgmath::Matrix4::from_scale(0.01);

    let uniform_buffer = vulkano::buffer::cpu_pool::CpuBufferPool::<vs::ty::Data>::new(
        device.clone(),
        vulkano::buffer::BufferUsage::all(),
    );

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
            },
            depth: {
                    load: Clear,
                    store: DontCare,
                    format: vulkano::format::Format::D16Unorm,
                    samples: 1,
                }
        },
        pass: {
            color: [color],
            depth_stencil: {depth}
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
    let rotation_start = std::time::Instant::now();

    loop {
        previous_frame_end.cleanup_finished();

        if recreate_swapchain {
            dimensions = {
                // using this causes it to flip out and get stuck with
                // SwapchainCreationError::UnsupportedDimensions errors
                // seems like high dpi issues ???
                let window_inner = window.window().get_inner_size().unwrap();
                // [new_width, new_height]
                let current_extent = window
                    .capabilities(physical)
                    .unwrap()
                    .current_extent
                    .unwrap();

                println!(
                    "in recreate swapchain, window_inner: {:?}, current_extent: {:?}",
                    window_inner, current_extent
                );
                current_extent
            };

            println!("Recreate Dimensions: {}, {}", dimensions[0], dimensions[1]);

            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => {
                    continue;
                }
                Err(err) => panic!("{:?}", err),
            };

            mem::replace(&mut swapchain, new_swapchain);
            mem::replace(&mut images, new_images);

            let new_depth_buffer = vulkano::image::attachment::AttachmentImage::transient(
                device.clone(),
                dimensions,
                vulkano::format::D16Unorm,
            ).unwrap();
            std::mem::replace(&mut depth_buffer, new_depth_buffer);

            framebuffers = None;

            proj = cgmath::perspective(
                cgmath::Rad(std::f32::consts::FRAC_PI_2),
                { dimensions[0] as f32 / dimensions[1] as f32 },
                0.01,
                100.0,
            );

            recreate_swapchain = false;
        }
        if framebuffers.is_none() {
            let new_framebuffers = Some(
                images
                    .iter()
                    .map(|image| {
                        Arc::new(
                            vulkano::framebuffer::Framebuffer::start(render_pass.clone())
                                .add(image.clone())
                                .unwrap()
                                .add(depth_buffer.clone())
                                .unwrap()
                                .build()
                                .unwrap(),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            std::mem::replace(&mut framebuffers, new_framebuffers);
        }

        let uniform_buffer_subbuffer = {
            let elapsed = rotation_start.elapsed();
            let rotation =
                elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0;
            let rotation = cgmath::Matrix3::from_angle_y(cgmath::Rad(rotation as f32));

            let uniform_data = vs::ty::Data {
                world: cgmath::Matrix4::from(rotation).into(),
                view: (view * scale).into(),
                proj: proj.into(),
            };

            uniform_buffer.next(uniform_data).unwrap()
        };

        let set = Arc::new(
            vulkano::descriptor::descriptor_set::PersistentDescriptorSet::start(
                pipeline.clone(),
                0,
            ).add_buffer(uniform_buffer_subbuffer)
                .unwrap()
                .build()
                .unwrap(),
        );

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
            vulkano::command_buffer::AutoCommandBufferBuilder::primary_one_time_submit(
                device.clone(),
                queue.family(),
            ).unwrap()
                .begin_render_pass(
                    framebuffers.as_ref().unwrap()[image_num].clone(),
                    false,
                    vec![[0.0, 0.0, 1.0, 1.0].into(), 1f32.into()],
                )
                .unwrap()
                .draw_indexed(
                    pipeline.clone(),
                    vulkano::command_buffer::DynamicState {
                        line_width: None,
                        viewports: Some(vec![vulkano::pipeline::viewport::Viewport {
                            origin: [0.0, 0.0],
                            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                            depth_range: 0.0..1.0,
                        }]),
                        scissors: None,
                    },
                    (vertex_buffer.clone(), normals_buffer.clone()),
                    index_buffer.clone(),
                    set.clone(),
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
            x => println!("event: {:?}", x),
        });
        if done {
            return;
        }
    }
}

mod vs {
    #[derive(VulkanoShader)]
    #[ty = "vertex"]
    #[src = "
#version 450
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 0) out vec3 v_normal;
layout(set = 0, binding = 0) uniform Data {
    mat4 world;
    mat4 view;
    mat4 proj;
} uniforms;
void main() {
    mat4 worldview = uniforms.view * uniforms.world;
    v_normal = transpose(inverse(mat3(worldview))) * normal;
    gl_Position = uniforms.proj * worldview * vec4(position, 1.0);
}
"]
    struct Dummy;
}

mod fs {
    #[derive(VulkanoShader)]
    #[ty = "fragment"]
    #[src = "
#version 450
layout(location = 0) in vec3 v_normal;
layout(location = 0) out vec4 f_color;
const vec3 LIGHT = vec3(0.0, 0.0, 1.0);
void main() {
    float brightness = dot(normalize(v_normal), normalize(LIGHT));
    vec3 dark_color = vec3(0.6, 0.0, 0.0);
    vec3 regular_color = vec3(1.0, 0.0, 0.0);
    f_color = vec4(mix(dark_color, regular_color, brightness), 1.0);
}
"]
    struct Dummy;
}
