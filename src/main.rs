#[macro_use]
extern crate glium;
extern crate image;

use std::io::Cursor;

mod teapot;

fn main() {
    use glium::{glutin, Surface};

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new();
    let context = glutin::ContextBuilder::new().with_depth_buffer(24);
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let image = image::load(
        Cursor::new(&include_bytes!("/Users/malcolm/Projects/rust/rscraft/assets/kitty.jpg")[..]),
        image::JPEG,
    ).unwrap()
        .to_rgba();
    let image_dimensions = image.dimensions();
    let image =
        glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let _texture = glium::texture::Texture2d::new(&display, image).unwrap();

    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
        tex_coords: [f32; 2],
    }

    implement_vertex!(Vertex, position, tex_coords);

    let positions = glium::VertexBuffer::new(&display, &teapot::VERTICES).unwrap();
    let normals = glium::VertexBuffer::new(&display, &teapot::NORMALS).unwrap();
    let indices = glium::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &teapot::INDICES,
    ).unwrap();

    let vertex_shader_src = r#"
#version 330

in vec3 position;
in vec3 normal;

out vec3 v_normal;
out vec3 v_position;

uniform mat4 matrix;
uniform mat4 perspective;

void main() {
    v_normal = transpose(inverse(mat3(matrix))) * normal;
    gl_Position = perspective * matrix * vec4(position, 1.0);
    v_position = gl_Position.xyz / gl_Position.w;
}
    "#;

    let fragment_shader_src = r#"
#version 330

in vec3 v_normal;
in vec3 v_position;

out vec4 color;

uniform vec3 u_light;

const vec3 ambient_color = vec3(0.0, 0.0, 0.2);
const vec3 diffuse_color = vec3(0.0, 0.0, 0.6);
const vec3 specular_color = vec3(0.8, 0.8, 0.8);

void main() {
    float diffuse = max(dot(normalize(v_normal), normalize(u_light)), 0.0);

    vec3 camera_dir = normalize(-v_position);
    vec3 half_direction = normalize(normalize(u_light) + camera_dir);
    float specular = pow(max(dot(half_direction, normalize(v_normal)), 0.0), 16.0);

    color = vec4(ambient_color + diffuse * diffuse_color + specular * specular_color, 1.0);
}
    "#;

    let program =
        glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
            .unwrap();

    let mut t: f32 = -2.0;
    let mut closed = false;
    while !closed {
        // we update `t`
        t += 0.004;
        if t > 2.0 {
            t = -2.0;
        }

        let mut target = display.draw();
        target.clear_color_and_depth((0.1, 0.1, 0.2, 1.0), 1.0);

        let matrix = [
            [t.cos() * 0.01, t.sin() * 0.01, 0.0, 0.0],
            [-t.sin() * 0.01, t.cos() * 0.01, 0.0, 0.0],
            [0.0, 0.0, 0.01, 0.0],
            [0.0, 0.0, 4.0 + t.cos() * 10.0, 1.0f32],
        ];

        let perspective = {
            let (width, height) = target.get_dimensions();
            let aspect_ratio = height as f32 / width as f32;

            let fov: f32 = 3.141592 / 3.0;
            let zfar = 1024.0;
            let znear = 0.1;

            let f = 1.0 / (fov / 2.0).tan();

            [
                [f * aspect_ratio, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
                [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
            ]
        };

        // the direction of the light
        let light = [-1.0, 0.4, 0.9f32];

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        };

        target
            .draw(
                (&positions, &normals),
                &indices,
                &program,
                &uniform! { matrix: matrix, perspective: perspective, u_light: light },
                &params,
            )
            .unwrap();
        target.finish().unwrap();

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => closed = true,
                _ => (),
            },
            _ => (),
        });
    }
}
