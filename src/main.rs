#[macro_use]
extern crate glium;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

fn main() {
    use glium::{glutin, Surface};

    let vertex1 = Vertex {
        position: [-0.5, -0.5],
    };
    let vertex2 = Vertex {
        position: [0.0, 0.5],
    };
    let vertex3 = Vertex {
        position: [0.5, -0.25],
    };
    let shape = vec![vertex1, vertex2, vertex3];

    let vertex_shader_src = r#"
    #version 140

    in vec2 position;

    void main() {
        gl_Position = vec4(position, 0.0, 1.0);
    }
"#;

    let fragment_shader_src = r#"
    #version 140

    out vec4 color;

    void main() {
        color = vec4(1.0, 0.0, 0.0, 1.0);
    }
"#;

    let mut events_loop = glium::glutin::EventsLoop::new();
    let window = glium::glutin::WindowBuilder::new();
    let context = glium::glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
    let program =
        glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
            .unwrap();

    let mut closed = false;
    while !closed {
        let mut target = display.draw();
        target.clear_color(0.1, 0.1, 0.15, 1.0);
        target.draw(&vertex_buffer, &indices, &program, &glium::uniforms::EmptyUniforms,
            &Default::default()).unwrap();
        target.finish().unwrap();

        events_loop.poll_events(|ev| match ev {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::Closed => closed = true,
                _ => (),
            },
            _ => (),
        });
    }
}
