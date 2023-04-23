use engine::asset_resource::AssetSourceResource;
use engine::assets::path::AssetPath;
use engine::assets::source::AssetSource;
use engine::assets::AssetPipelines;
use engine::ecs::world::World;
use engine::render::bindgroup::serial::{BindGroupAssetPipeline, BindGroupLayoutAsset};
use engine::render::pipeline::serial::{RenderPipelineAsset, RenderPipelineAssetPipeline};
use engine::render::{BindGroup, BindGroupBinding, Buffer, BufferUsages, Color, Handle, Pipeline, RenderPass, Target};
use instant::Instant;
use nalgebra::{Matrix4, Vector2, Vector3, Vector4};
use std::any::TypeId;
use std::collections::HashMap;
use std::mem::{size_of, size_of_val};
use std::ops::Deref;
use std::slice::from_raw_parts;
use winit::event::{DeviceEvent, ElementState, VirtualKeyCode};
use engine::surface::{SurfaceEvent, SurfaceEventResult};
use engine::utils::{HList, hlist};
use engine::utils::hlist::{Has, ToMut};
use engine::wgpu_render::WGPURenderResource;

#[derive(Debug, Default)]
struct InputState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

type Vec2 = Vector2<f32>;
type Vec3 = Vector3<f32>;
type Vec4 = Vector4<f32>;

#[derive(Debug, Default)]
struct Transform {
    position: Vec3,
}

struct Player;

enum Shape {
    Triangle,
}

pub struct GameResource {
    pipeline: Handle<Pipeline>,
    vertex_buffer: Handle<Buffer>,
    instance_buffer: Handle<Buffer>,
    uniform_buffer: Handle<Buffer>,
    camera_bind_group: BindGroup,
    start_time: Instant,
    previous_frame: Instant,
    input_state: InputState,
    world: World,
}

const VERTICES: [f32; 6 * 3] = [
    -0.3, -0.3, 1.0, 0.0, 0.0, 1.0, 0.0, 0.3, 0.0, 1.0, 0.0, 1.0, 0.3, -0.3, 0.0, 0.0, 1.0, 1.0,
];

pub async fn setup_game<A: AssetSource>(resources: HList!(WGPURenderResource, AssetSourceResource<A>)) -> HList!(GameResource, WGPURenderResource, AssetSourceResource<A>) {
    let (mut render, resources): (WGPURenderResource, _) = resources.pick();
    let (asset_source, _): (AssetSourceResource<A>, _) = resources.pick();

    let asset_pipelines = {
        let mut pipelines = HashMap::new();
        pipelines.insert(
            TypeId::of::<RenderPipelineAsset>(),
            Box::new(RenderPipelineAssetPipeline) as _,
        );
        pipelines.insert(
            TypeId::of::<BindGroupLayoutAsset>(),
            Box::new(BindGroupAssetPipeline) as _,
        );
        AssetPipelines::new(pipelines)
    };

    let pipeline_asset = asset_pipelines
        .load_asset(
            AssetPath::new("/triangle.pipeline").unwrap(),
            TypeId::of::<RenderPipelineAsset>(),
            asset_source.deref(),
        )
        .await
        .expect("triangle render pipeline")
        .downcast::<RenderPipelineAsset>()
        .expect("render pipeline asset");

    let camera_bind_group_asset = asset_pipelines
        .load_asset(
            AssetPath::new("/camera.bindgroup").unwrap(),
            TypeId::of::<BindGroupLayoutAsset>(),
            asset_source.deref(),
        )
        .await
        .expect("camera bind group layout")
        .downcast::<BindGroupLayoutAsset>()
        .expect("bind group layout asset");

    let surface_format = render.surface().format();

    let mut bind_group_layouts = HashMap::new();
    let camera_bind_group_layout = render
        .device_mut()
        .create_bind_group_layout_from_asset(*camera_bind_group_asset);
    bind_group_layouts.insert("camera".to_owned(), camera_bind_group_layout);

    let pipeline = render.device_mut().create_pipeline_from_asset(
        *pipeline_asset,
        surface_format,
        bind_group_layouts,
    );
    let vertex_buffer = render.device_mut().create_buffer(
        size_of_val(&VERTICES),
        BufferUsages::VERTEX | BufferUsages::COPY_DST,
    );

    let data = unsafe { from_raw_parts(VERTICES.as_ptr() as *const u8, size_of_val(&VERTICES)) };
    render.device().submit_buffer(vertex_buffer, 0, data);

    let uniform_buffer = render.device_mut().create_buffer(
        4 * size_of::<f32>(),
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    );
    let uniform_buffer_ref = render.device().get_buffer(uniform_buffer).unwrap();
    let camera_bind_group = render.device().create_bind_group(
        camera_bind_group_layout,
        &[BindGroupBinding::Buffer(uniform_buffer_ref)],
    );

    let instance_buffer = render.device_mut().create_buffer(
        4 * 4 * size_of::<f32>(),
        BufferUsages::VERTEX | BufferUsages::COPY_DST,
    );

    let mut world = World::default()
        .with_component::<Player>()
        .with_component::<Transform>()
        .with_component::<Shape>();
    {
        let triangle = world.new_entity();
        world.components_mut::<Player>().put(triangle, Player);
        world.components_mut::<Transform>().put(triangle, Transform::default());
        world.components_mut::<Shape>().put(triangle, Shape::Triangle);
    }

    let game = GameResource {
        pipeline,
        vertex_buffer,
        instance_buffer,
        uniform_buffer,
        camera_bind_group,
        start_time: Instant::now(),
        previous_frame: Instant::now(),
        input_state: Default::default(),
        world,
    };
    hlist!(game, render, asset_source)
}

fn data_bytes<T>(data: &T) -> &[u8] {
    unsafe { from_raw_parts(data as *const T as *const u8, size_of_val(data)) }
}

pub fn run_game<A: AssetSource>(event: SurfaceEvent, resources: &mut HList!(WGPURenderResource, GameResource, AssetSourceResource<A>)) -> SurfaceEventResult {
    let resources = resources.to_mut();
    let (game, resources): (&mut GameResource, _) = resources.pick();
    let (render, _): (&mut WGPURenderResource, _) = resources.pick();

    match event {
        SurfaceEvent::Resize { width, height } => {
            let (surface, device) = render.get_mut();
            surface.configure(device, width, height);

            SurfaceEventResult::Continue
        }
        SurfaceEvent::Draw => {
            // Update game state
            {
                let elapsed_since_previous_frame = game.previous_frame.elapsed().as_secs_f32();
                game.previous_frame = Instant::now();

                let move_direction = Vector3::new(
                    if game.input_state.left { -1.0 } else { 0.0 }
                        + if game.input_state.right { 1.0 } else { 0.0 },
                    if game.input_state.down { -1.0 } else { 0.0 }
                        + if game.input_state.up { 1.0 } else { 0.0 },
                    0.0,
                ) * 0.5 * elapsed_since_previous_frame;

                let mut transforms = game.world.components_mut::<Transform>();
                let players = game.world.components::<Player>();

                for entity in game
                    .world
                    .entity_iter()
                    .filter(|entity| players.has(*entity))
                {
                    // *transform = Transform { position: transform.position + move_direction * move_speed };
                    if let Some(transform) = transforms.get(entity) {
                        let transform = Transform {
                            position: transform.position + move_direction,
                        };
                        transforms.put(entity, transform);
                    }
                }
            }

            // Render game
            {
                let camera_transform = Vec4::zeros();
                let camera_transform_data = data_bytes(&camera_transform);
                render
                    .device()
                    .submit_buffer(game.uniform_buffer, 0, camera_transform_data);

                let frame = render.surface().get_frame();
                let mut encoder = render.device().command_encoder(&frame);

                let transforms = game.world.components::<Transform>();
                let shapes = game.world.components::<Shape>();

                let mut triangle_transform: Option<Vec3> = None;
                for (_, shape, transform) in game
                    .world
                    .entity_iter()
                    .filter_map(|entity| shapes.get(entity).map(|shape| (entity, shape)))
                    .filter_map(|(entity, shape)| {
                        transforms
                            .get(entity)
                            .map(|transform| (entity, shape, transform))
                    })
                {
                    if let Shape::Triangle = shape {
                        triangle_transform = Some(transform.position);
                        break;
                    }
                }

                if let Some(position) = triangle_transform {
                    let elapsed = game.start_time.elapsed().as_secs_f32() * 2.0;

                    let spin_radius = 0.1;
                    let position = position + Vec3::new(elapsed.sin(), elapsed.cos(), 0.0) * spin_radius;
                    let transform = Matrix4::new_translation(&position);

                    let transform_data = data_bytes(&transform);
                    render
                        .device()
                        .submit_buffer(game.instance_buffer, 0, transform_data);

                    encoder.render_pass(RenderPass {
                        pipeline: game.pipeline,
                        vertices: 0..3,
                        targets: vec![Target::ScreenTarget {
                            clear: Some(Color::rgb(0, 3, 22, 1.0)),
                        }],
                        vertex_buffers: vec![Some(game.vertex_buffer), Some(game.instance_buffer)],
                        bind_groups: vec![game.camera_bind_group.clone()],
                        instances: 0..1,
                    });
                    render.device().submit_commands(encoder);

                    render.surface().present(frame);
                }
            }

            SurfaceEventResult::Continue
        }
        SurfaceEvent::CloseRequested => SurfaceEventResult::Exit(None),
        SurfaceEvent::DeviceEvent(DeviceEvent::Key(key)) => {
            println!("Key event: {:?}", key);
            let state = key.state == ElementState::Pressed;
            match key.virtual_keycode {
                Some(VirtualKeyCode::Up) => game.input_state.up = state,
                Some(VirtualKeyCode::Down) => game.input_state.down = state,
                Some(VirtualKeyCode::Left) => game.input_state.left = state,
                Some(VirtualKeyCode::Right) => game.input_state.right = state,
                _ => (),
            }
            SurfaceEventResult::Continue
        }
        _ => SurfaceEventResult::Continue,
    }
}
