use std::any::TypeId;
use std::collections::HashMap;
use std::mem::size_of_val;
use std::slice::from_raw_parts;

use futures::executor::block_on;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use assets::{AssetPipeline, AssetPipelines};
use assets::path::AssetPath;
use assets::desktop_fs::desktop_fs::DirectoryAssetSource;
use render::{BufferUsages, Color, RenderPass, Target, VertexShader, WGPUContext};
use render::pipeline::{RenderPipelineAsset, RenderPipelineAssetPipeline};

const VERTICES: [f32; 2 * 3] = [
    -0.5, -0.5,
    0.0, 0.5,
    0.5, -0.5,
];

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let context = WGPUContext::new().await.expect("WGPU context");
    let mut surface = context.create_surface(&window);
    let mut device = context.request_device(&surface).await.expect("WGPU device");

    let PhysicalSize { width, height } = window.inner_size();
    surface.configure(&device, width , height);

    let mut asset_pipelines = HashMap::new();
    asset_pipelines.insert(TypeId::of::<RenderPipelineAsset>(), Box::new(RenderPipelineAssetPipeline) as _);
    let asset_pipelines = AssetPipelines::new(asset_pipelines);

    let asset_source = DirectoryAssetSource::new("assets");

    let pipeline_asset = asset_pipelines
        .load_asset(AssetPath::new("/triangle.pipeline").unwrap(), TypeId::of::<RenderPipelineAsset>(), &asset_source)
        .await
        .expect("triangle render pipeline")
        .downcast::<RenderPipelineAsset>()
        .expect("render pipeline asset");

    let pipeline = device.create_pipeline(*pipeline_asset);
    let buffer = device.create_buffer(size_of_val(&VERTICES), BufferUsages::VERTEX | BufferUsages::COPY_DST);

    let data = unsafe {
        from_raw_parts(VERTICES.as_ptr() as *const u8, size_of_val(&VERTICES))
    };
    device.submit_buffer(buffer, 0, data);

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
                encoder.render_pass(RenderPass {
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
