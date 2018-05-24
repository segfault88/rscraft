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
#version 140

in vec3 position;
in vec3 normal;

uniform mat4 matrix;

void main() {
    gl_Position = matrix * vec4(position, 1.0);
}
    "#;

    let fragment_shader_src = r#"
#version 140

out vec4 color;

void main() {
    color = vec4(0.8, 0.0, 0.0, 1.0);
}
    "#;

    let program =
        glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
            .unwrap();

    let mut t: f32 = -0.5;
    let mut closed = false;
    while !closed {
        // we update `t`
        t += 0.002;
        if t > 0.5 {
            t = -0.5;
        }

        let mut target = display.draw();
        target.clear_color(0.1, 0.1, 0.15, 1.0);

        let uniforms = uniform! {
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

        target
            .draw(
                (&positions, &normals),
                &indices,
                &program,
                &uniform! { matrix: matrix },
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
