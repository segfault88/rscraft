#[macro_use]
extern crate glium;
extern crate image;

use std::io::Cursor;

mod teapot;

fn main() {
    use glium::{glutin, Surface};

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new();
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let image = image::load(
        Cursor::new(&include_bytes!("/Users/malcolm/Projects/rust/rscraft/assets/kitty.jpg")[..]),
        image::JPEG,
    ).unwrap()
        .to_rgba();
    let image_dimensions = image.dimensions();
    let image =
        glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let texture = glium::texture::Texture2d::new(&display, image).unwrap();

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
#version 150

in vec3 position;
in vec3 normal;

out vec3 v_normal;

uniform mat4 matrix;

void main() {
    v_normal = transpose(inverse(mat3(matrix))) * normal;
    gl_Position = matrix * vec4(position, 1.0);
}
    "#;

    let fragment_shader_src = r#"
#version 150

in vec3 v_normal;
out vec4 color;
uniform vec3 u_light;

void main() {
    float brightness = dot(normalize(v_normal), normalize(u_light));
    vec3 dark_color = vec3(0.0, 0.0, 0.6);
    vec3 regular_color = vec3(0.0, 0.0, 1.0);
    color = vec4(mix(dark_color, regular_color, brightness), 1.0);
}
    "#;

    let program =
        glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
            .unwrap();

    let mut t: f32 = -1.5;
    let mut closed = false;
    while !closed {
        // we update `t`
        t += 0.002;
        if t > 1.5 {
            t = -1.5;
        }

        let mut target = display.draw();
        target.clear_color(0.1, 0.1, 0.15, 1.0);

        let oldUniforms = uniform! {
            matrix: [
                [t.cos(), t.sin(), 0.0, 0.0],
                [-t.sin(), t.cos(), 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [ t , 0.0, 0.0, 1.0f32],
            ],
            tex: &texture,
        };

        let matrix = [
            [t.cos() * 0.01, t.sin() * 0.01, 0.0, 0.0],
            [-t.sin() * 0.01, t.cos() * 0.01, 0.0, 0.0],
            [0.0, 0.0, 0.01, 0.0],
            [0.0, 0.0, 0.0, 1.0f32],
        ];

        // the direction of the light
        let light = [-1.0, 0.4, 0.9f32];

        target
            .draw(
                (&positions, &normals),
                &indices,
                &program,
                &uniform! { matrix: matrix, u_light: light },
                &Default::default(),
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
