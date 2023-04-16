/*use std::any::TypeId;
use std::collections::HashMap;
use std::mem::size_of_val;
use std::slice::from_raw_parts;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use engine::assets::AssetPipelines;
use engine::assets::path::AssetPath;
use engine::assets::source::desktop_fs::DirectoryAssetSource;
use engine::process::{ProcessBuilder, ProcessInfo};
use engine::render::{Buffer, BufferUsages, Color, DeviceContext, Handle, Pipeline, RenderPass, SurfaceContext, Target, WGPUContext};
use engine::render::pipeline::{RenderPipelineAsset, RenderPipelineAssetPipeline};
use engine::resource::ResourceList;
use engine::winit_surface::{RunWinitSurfaceExt, SurfaceEvent, WGPURenderExt, WGPURenderResource, WithWinitSurfaceExt};

#[tokio::main]
async fn main() {
    ProcessBuilder::new(ProcessInfo)
        .with_winit_surface()
        .with_wgpu_render().await
        .setup_async(|mut resources| async move {
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

            let render: &mut WGPURenderResource = resources.get_mut();

            let pipeline = render.device_mut().create_pipeline(*pipeline_asset);
            let buffer = render.device_mut().create_buffer(size_of_val(&VERTICES), BufferUsages::VERTEX | BufferUsages::COPY_DST);

            let data = unsafe {
                from_raw_parts(VERTICES.as_ptr() as *const u8, size_of_val(&VERTICES))
            };
            render.device().submit_buffer(buffer, 0, data);

            resources.with_resource(TriangleResource { pipeline,buffer })
        }).await
        .run(|event, resourcs| match event {
            SurfaceEvent::Draw => {
                let render: &WGPURenderResource = resourcs.get();
                let triangle: &TriangleResource = resourcs.get();

                let frame = render.surface().get_frame();

                let mut encoder = render.device().command_encoder(&frame);
                encoder.render_pass(RenderPass {
                    pipeline: triangle.pipeline,
                    vertices: 0..3,
                    targets: vec![Target::ScreenTarget {
                        clear: Some(Color::new(30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0, 1.0)),
                    }],
                    vertex_buffers: vec![Some(triangle.buffer)],
                });
                render.device().submit_commands(encoder);

                render.surface().present(frame);
            }
            SurfaceEvent::Close => {}
        });
}*/
