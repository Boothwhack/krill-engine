use std::any::TypeId;
use std::collections::HashMap;
use std::mem::size_of_val;
use std::slice::from_raw_parts;
use engine::asset_resource::AssetSourceResource;
use engine::assets::AssetPipelines;
use engine::assets::path::AssetPath;
use engine::assets::source::AssetSource;
use engine::render::{Buffer, BufferUsages, Color, Handle, Pipeline, RenderPass, Target};
use engine::render::pipeline::{RenderPipelineAsset, RenderPipelineAssetPipeline};
use engine::resource::frunk::hlist::{HList, Selector};
use engine::resource::ResourceList;
use engine::winit_surface::{SurfaceEvent, WGPURenderResource};

pub struct TriangleResource {
    pipeline: Handle<Pipeline>,
    buffer: Handle<Buffer>,
}

const VERTICES: [f32; 2 * 3] = [
    -0.5, -0.5,
    0.0, 0.5,
    0.5, -0.5,
];

pub async fn setup_game<R, A, IRender, IAssets>(mut resources: R) -> R::WithResource<TriangleResource>
    where R: ResourceList,
          A: AssetSource,
          R::Resources: Selector<WGPURenderResource, IRender>,
          R::Resources: Selector<AssetSourceResource<A>, IAssets> {
    let asset_pipelines = {
        let mut pipelines = HashMap::new();
        pipelines.insert(TypeId::of::<RenderPipelineAsset>(), Box::new(RenderPipelineAssetPipeline) as _);
        AssetPipelines::new(pipelines)
    };

    let asset_source: &AssetSourceResource<A> = resources.get();

    let pipeline_asset = asset_pipelines
        .load_asset(AssetPath::new("/triangle.pipeline").unwrap(), TypeId::of::<RenderPipelineAsset>(), asset_source.get())
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

    resources.with_resource(TriangleResource { pipeline, buffer })
}

pub fn run_game<R, IRender, ITriangle>(event: SurfaceEvent, resources: &mut R)
    where R: HList + Selector<WGPURenderResource, IRender> + Selector<TriangleResource, ITriangle>, {
    match event {
        SurfaceEvent::Resize { width, height } => {
            let render: &mut WGPURenderResource = resources.get_mut();
            let (surface, device) = render.get();
            surface.configure(device, width, height);
        }
        SurfaceEvent::Draw => {
            let render: &WGPURenderResource = resources.get();
            let triangle: &TriangleResource = resources.get();

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
    }
}
