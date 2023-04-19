use std::any::TypeId;
use std::collections::HashMap;
use std::mem::{size_of, size_of_val};
use std::slice::from_raw_parts;
use instant::Instant;
use nalgebra::Vector2;
use winit::event::{DeviceEvent, ElementState, VirtualKeyCode};
use engine::asset_resource::AssetSourceResource;
use engine::assets::AssetPipelines;
use engine::assets::path::AssetPath;
use engine::assets::source::AssetSource;
use engine::render::{BindGroup, BindGroupBinding, Buffer, BufferUsages, Color, Handle, Pipeline, RenderPass, Target};
use engine::render::bindgroup::serial::{BindGroupLayoutAsset, BindGroupAssetPipeline};
use engine::render::pipeline::serial::{RenderPipelineAsset, RenderPipelineAssetPipeline};
use engine::resource::frunk::hlist::{HList, Selector};
use engine::resource::ResourceList;
use engine::winit_surface::{SurfaceEvent, SurfaceEventResult, WGPURenderResource};

#[derive(Debug, Default)]
struct InputState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

pub struct TriangleResource {
    pipeline: Handle<Pipeline>,
    vertex_buffer: Handle<Buffer>,
    uniform_buffer: Handle<Buffer>,
    camera_bind_group: BindGroup,
    start_time: Instant,
    previous_frame: Instant,
    input_state: InputState,
    position: Vector2<f32>,
}

const VERTICES: [f32; 6 * 3] = [
    -0.3, -0.3, 1.0, 0.0, 0.0, 1.0,
    0.0, 0.3, 0.0, 1.0, 0.0, 1.0,
    0.3, -0.3, 0.0, 0.0, 1.0, 1.0,
];

pub async fn setup_game<R, A, IRender, IAssets>(mut resources: R) -> R::WithResource<TriangleResource>
    where R: ResourceList,
          A: AssetSource,
          R::Resources: Selector<WGPURenderResource, IRender>,
          R::Resources: Selector<AssetSourceResource<A>, IAssets> {
    let asset_pipelines = {
        let mut pipelines = HashMap::new();
        pipelines.insert(TypeId::of::<RenderPipelineAsset>(), Box::new(RenderPipelineAssetPipeline) as _);
        pipelines.insert(TypeId::of::<BindGroupLayoutAsset>(), Box::new(BindGroupAssetPipeline) as _);
        AssetPipelines::new(pipelines)
    };

    let asset_source: &AssetSourceResource<A> = resources.get();

    let pipeline_asset = asset_pipelines
        .load_asset(AssetPath::new("/triangle.pipeline").unwrap(), TypeId::of::<RenderPipelineAsset>(), asset_source.get())
        .await
        .expect("triangle render pipeline")
        .downcast::<RenderPipelineAsset>()
        .expect("render pipeline asset");

    let camera_bind_group_asset = asset_pipelines
        .load_asset(AssetPath::new("/camera.bindgroup").unwrap(), TypeId::of::<BindGroupLayoutAsset>(), asset_source.get())
        .await
        .expect("camera bind group layout")
        .downcast::<BindGroupLayoutAsset>()
        .expect("bind group layout asset");

    let render: &mut WGPURenderResource = resources.get_mut();
    let surface_format = render.surface().format();

    let mut bind_group_layouts = HashMap::new();
    let camera_bind_group_layout = render.device_mut().create_bind_group_layout_from_asset(*camera_bind_group_asset);
    bind_group_layouts.insert("camera".to_owned(), camera_bind_group_layout);

    let pipeline = render.device_mut().create_pipeline_from_asset(*pipeline_asset, surface_format, bind_group_layouts);
    let vertex_buffer = render.device_mut().create_buffer(size_of_val(&VERTICES), BufferUsages::VERTEX | BufferUsages::COPY_DST);

    let data = unsafe {
        from_raw_parts(VERTICES.as_ptr() as *const u8, size_of_val(&VERTICES))
    };
    render.device().submit_buffer(vertex_buffer, 0, data);

    let uniform_buffer = render.device_mut().create_buffer(4 * size_of::<f32>(), BufferUsages::UNIFORM | BufferUsages::COPY_DST);
    let uniform_buffer_ref = render.device().get_buffer(uniform_buffer).unwrap();
    let camera_bind_group = render.device().create_bind_group(camera_bind_group_layout, &[
        BindGroupBinding::Buffer(uniform_buffer_ref),
    ]);

    resources.with_resource(TriangleResource {
        pipeline,
        vertex_buffer,
        uniform_buffer,
        camera_bind_group,
        start_time: Instant::now(),
        previous_frame: Instant::now(),
        input_state: Default::default(),
        position: Default::default(),
    })
}

pub fn run_game<R, IRender, ITriangle>(event: SurfaceEvent, resources: &mut R) -> SurfaceEventResult
    where R: HList + Selector<WGPURenderResource, IRender> + Selector<TriangleResource, ITriangle>, {
    match event {
        SurfaceEvent::Resize { width, height } => {
            let render: &mut WGPURenderResource = resources.get_mut();
            let (surface, device) = render.get();
            surface.configure(device, width, height);

            SurfaceEventResult::Continue
        }
        SurfaceEvent::Draw => {
            let mut triangle: &mut TriangleResource = resources.get_mut();
            let mut move_direction = Vector2::new(
                if triangle.input_state.left { -1.0 } else { 0.0 } + if triangle.input_state.right { 1.0 } else { 0.0 },
                if triangle.input_state.down { -1.0 } else { 0.0 } + if triangle.input_state.up { 1.0 } else { 0.0 },
            );
            let elapsed_since_previous_frame = triangle.previous_frame.elapsed().as_secs_f32();
            let move_speed = 0.5;
            move_direction *= elapsed_since_previous_frame;
            triangle.position += move_direction * move_speed;
            triangle.previous_frame = Instant::now();

            let triangle: &TriangleResource = resources.get();
            let elapsed = triangle.start_time.elapsed().as_secs_f32() * 2.0;

            let spin_radius = 0.1;
            let transform: [f32; 4] = [
                triangle.position.x + elapsed.sin() * spin_radius,
                triangle.position.y + elapsed.cos() * spin_radius,
                0.0, 0.0,
            ];
            let transform_data = unsafe {
                from_raw_parts(transform.as_ptr() as *const u8, size_of_val(&transform))
            };

            let render: &WGPURenderResource = resources.get();
            render.device().submit_buffer(triangle.uniform_buffer, 0, transform_data);

            let frame = render.surface().get_frame();

            let mut encoder = render.device().command_encoder(&frame);
            encoder.render_pass(RenderPass {
                pipeline: triangle.pipeline,
                vertices: 0..3,
                targets: vec![Target::ScreenTarget {
                    clear: Some(Color::new(30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0, 1.0)),
                }],
                vertex_buffers: vec![Some(triangle.vertex_buffer)],
                bind_groups: vec![triangle.camera_bind_group.clone()],
            });
            render.device().submit_commands(encoder);

            render.surface().present(frame);

            SurfaceEventResult::Continue
        }
        SurfaceEvent::CloseRequested => {
            SurfaceEventResult::Exit(None)
        }
        SurfaceEvent::DeviceEvent(DeviceEvent::Key(key)) => {
            println!("Key event: {:?}", key);
            let mut triangle: &mut TriangleResource = resources.get_mut();
            let state = key.state == ElementState::Pressed;
            match key.virtual_keycode {
                Some(VirtualKeyCode::Up) => triangle.input_state.up = state,
                Some(VirtualKeyCode::Down) => triangle.input_state.down = state,
                Some(VirtualKeyCode::Left) => triangle.input_state.left = state,
                Some(VirtualKeyCode::Right) => triangle.input_state.right = state,
                _ => ()
            }
            SurfaceEventResult::Continue
        }
        _ => SurfaceEventResult::Continue,
    }
}
