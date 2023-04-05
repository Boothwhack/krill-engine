use std::mem::size_of_val;
use std::slice::from_raw_parts;

use futures::executor::block_on;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use render::{BufferUsages, Color, RenderPass, Target, VertexShader, WGPUContext};

const VERTICES: [f32; 2 * 3] = [
    0.5, -0.5,
    0.0, 0.5,
    -0.5, -0.5,
];

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let PhysicalSize { width, height } = window.inner_size();

    let context = block_on(WGPUContext::new())
        .expect("WGPU context");
    let mut surface = context.create_surface(&window);
    let mut device = block_on(context.request_device(&surface))
        .expect("WGPU device");
    surface.configure(&device, width, height);

    let pipeline = device.create_pipeline(&VertexShader {
        source: include_str!("triangle.wgsl"),
    });
    let buffer = device.create_buffer(size_of_val(&VERTICES), BufferUsages::VERTEX | BufferUsages::COPY_DST);

    let data = unsafe {
        from_raw_parts(VERTICES.as_ptr() as *const u8, size_of_val(&VERTICES))
    };
    device.submit_buffer(&buffer, 0, data);

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => control_flow.set_exit(),
                _ => (),
            }
            Event::RedrawRequested(id) if id == window.id() => {
                let frame = surface.get_frame();

                let mut encoder = device.command_encoder(&frame);
                encoder.render_pass(&RenderPass {
                    pipeline,
                    vertices: 0..3,
                    targets: vec![Target::ScreenTarget {
                        clear: Some(Color::new(30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0, 1.0)),
                    }],
                    vertex_buffers: vec![Some(buffer)],
                });
                device.submit_commands(encoder);

                surface.present(frame);
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => (),
        }
    })
}
