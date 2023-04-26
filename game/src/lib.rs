use engine::asset_resource::AssetSourceResource;
use engine::assets::path::AssetPath;
use engine::assets::source::AssetSource;
use engine::assets::AssetPipelines;
use engine::ecs::world::{View, World};
use engine::render::bindgroup::serial::{BindGroupAssetPipeline, BindGroupLayoutAsset};
use engine::render::pipeline::serial::{RenderPipelineAsset, RenderPipelineAssetPipeline};
use engine::render::{BindGroup, BindGroupBinding, Buffer, BufferUsages, Color, DeviceContext, Handle, Pipeline, RenderPass, Target};
use instant::Instant;
use nalgebra::{Matrix4, RealField, Rotation3, Vector2, Vector3, Vector4};
use std::any::TypeId;
use std::collections::HashMap;
use std::mem::{size_of, size_of_val};
use std::ops::Deref;
use std::slice::from_raw_parts;
use std::time::Duration;
use float_ord::FloatOrd;
use rand::{random, Rng, SeedableRng};
use rand::distributions::Standard;
use rand::rngs::StdRng;
use winit::event::{DeviceEvent, ElementState, VirtualKeyCode};
use engine::surface::{SurfaceEvent, SurfaceEventResult};
use engine::utils::{delist, HList, hlist};
use engine::utils::hlist::ToMut;
use engine::wgpu_render::WGPURenderResource;

#[derive(Debug, Default)]
struct InputState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    shoot: bool,
    has_shot: bool,
}

type Vec2 = Vector2<f32>;
type Vec3 = Vector3<f32>;
type Vec4 = Vector4<f32>;

#[derive(Clone, Debug, Default)]
struct Transform {
    position: Vec3,
    rotation: f32,
    velocity: Vec3,
    angular_velocity: f32,
    transient: bool,
    size: f32,
}

impl Transform {
    pub fn to_matrix(&self) -> Matrix4<f32> {
        let translation = Matrix4::new_translation(&self.position);
        let rotation = Rotation3::from_euler_angles(0.0, 0.0, self.rotation);
        translation * rotation.to_homogeneous() * Matrix4::new_scaling(0.1 * self.size)
    }
}

// Marker component that denotes the player entity
struct Player;

// Marker component that denotes a bullet in flight
struct Bullet;

// Marker component that denotes a meteor
struct Meteor;

#[derive(Copy, Clone)]
enum Shape {
    Ship,
    Meteor,
    Bullet,
}

struct Sprite {
    vertex_buffer: Handle<Buffer>,
    instance_buffer: Handle<Buffer>,
    vertices: u32,
}

fn data_bytes<T>(data: &[T]) -> &[u8] {
    unsafe { from_raw_parts(data.as_ptr() as *const u8, size_of_val(data)) }
}

impl Sprite {
    fn new(device: &mut DeviceContext, vertices: &[Vec2]) -> Self {
        let vertex_buffer = device.create_buffer(size_of_val(vertices), BufferUsages::VERTEX | BufferUsages::COPY_DST);
        device.submit_buffer(vertex_buffer, 0, data_bytes(vertices));

        let instance_buffer = device.create_buffer(4 * 4 * size_of::<f32>(), BufferUsages::VERTEX | BufferUsages::COPY_DST);

        Sprite { vertex_buffer, instance_buffer, vertices: vertices.len() as _ }
    }
}

#[derive(Clone, Debug)]
pub struct Collider {
    size: f32,
}

fn collides(a: &Collider, a_pos: &Vec3, b: &Collider, b_pos: &Vec3) -> bool {
    let distance = (a_pos - b_pos).magnitude();
    distance < (a.size + b.size)
}

pub struct GameState {
    world: World,
    previous_meteor: Instant,
    time_until_meteor: Duration,
    meteor_timer: Duration,
}

impl Default for GameState {
    fn default() -> Self {
        let mut world = World::default()
            .with_component::<Player>()
            .with_component::<Meteor>()
            .with_component::<Bullet>()
            .with_component::<Transform>()
            .with_component::<Shape>()
            .with_component::<Collider>();

        {
            let player = world.new_entity();

            world.components_mut::<Player>().put(player, Player);
            world.components_mut::<Transform>().put(player, Transform { size: 1.0, ..Transform::default() });
            world.components_mut::<Shape>().put(player, Shape::Ship);
            world.components_mut::<Collider>().put(player, Collider { size: 0.025 });
        }

        GameState {
            world,
            previous_meteor: Instant::now(),
            time_until_meteor: Duration::from_secs(3),
            meteor_timer: Duration::from_secs(10),
        }
    }
}

pub struct GameResource {
    pipeline: Handle<Pipeline>,
    ship_sprite: Sprite,
    meteor_sprite: Sprite,
    bullet_sprite: Sprite,
    camera_uniform_buffer: Handle<Buffer>,
    color_scheme_uniform_buffer: Handle<Buffer>,
    camera_bind_group: BindGroup,
    color_scheme_bind_group: BindGroup,
    previous_frame: Instant,
    input_state: InputState,
    state: GameState,
    bounds: Vec2,
    restart_timer: Option<(Instant, Duration)>,
}

const SHIP_VERTICES: [Vec2; 4] = [
    Vec2::new(-0.3, -0.3),
    Vec2::new(0.0, -0.2),
    Vec2::new(0.0, 0.3),
    Vec2::new(0.3, -0.3),
];

const BULLET_VERTICES: [Vec2; 4] = [
    Vec2::new(0.04, -0.08),
    Vec2::new(0.04, 0.08),
    Vec2::new(-0.04, -0.08),
    Vec2::new(-0.04, 0.08),
];

fn calculate_game_bounds(width: u32, height: u32) -> Vec2 {
    let aspect_ratio = width as f32 / height as f32;

    if aspect_ratio > 1.0 {
        Vec2::new(1.0, height as f32 / width as f32)
    } else {
        Vec2::new(aspect_ratio, 1.0)
    }
}

pub async fn setup_game<A: AssetSource>(resources: HList!(WGPURenderResource, AssetSourceResource<A>)) -> HList!(GameResource, WGPURenderResource, AssetSourceResource<A>) {
    let (mut render, resources) = resources;
    let (asset_source, _) = resources;

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

    let pipeline_asset: RenderPipelineAsset = asset_pipelines
        .load_asset(AssetPath::new("/game.pipeline").unwrap(), asset_source.deref())
        .await
        .expect("game render pipeline");
    let camera_bind_group_asset: BindGroupLayoutAsset = asset_pipelines
        .load_asset(AssetPath::new("/camera.bindgroup").unwrap(), asset_source.deref())
        .await
        .expect("camera bind group layout");
    let color_scheme_bind_group_asset: BindGroupLayoutAsset = asset_pipelines
        .load_asset(AssetPath::new("/color-scheme.bindgroup").unwrap(), asset_source.deref())
        .await
        .expect("color scheme bind group layout");

    let surface_format = render.surface().format();

    let mut bind_group_layouts = HashMap::new();
    let camera_bind_group_layout = render
        .device_mut()
        .create_bind_group_layout_from_asset(camera_bind_group_asset);
    bind_group_layouts.insert("camera".to_owned(), camera_bind_group_layout);
    let color_scheme_bind_group_layout = render
        .device_mut()
        .create_bind_group_layout_from_asset(color_scheme_bind_group_asset);
    bind_group_layouts.insert("color-scheme".to_owned(), color_scheme_bind_group_layout);

    let pipeline = render.device_mut().create_pipeline_from_asset(
        pipeline_asset,
        surface_format,
        bind_group_layouts,
    );

    // generate meteor shapes
    let meteor_vertices = {
        let radius = 0.5;
        let mut vertices: [Vec2; 10] = Default::default();

        let mut indices = vec![0];
        for i in 1..=vertices.len() / 2 {
            indices.push(i);
            indices.push(vertices.len() - i);
        }

        let mut rng = StdRng::seed_from_u64(0).sample_iter::<f32, _>(Standard);

        let vertex_count = vertices.len();
        for (i, vertex) in vertices.iter_mut().enumerate() {
            let progress = (indices[i] as f32 / vertex_count as f32) * f32::pi() * 2.0;

            let random_x = rng.next().unwrap() * 0.09;
            let random_y = rng.next().unwrap() * 0.08;

            *vertex = Vec2::new(
                progress.sin() * radius + random_x,
                progress.cos() * radius + random_y,
            );
        }

        vertices
    };

    let ship_sprite = Sprite::new(render.device_mut(), &SHIP_VERTICES);
    let meteor_sprite = Sprite::new(render.device_mut(), &meteor_vertices);
    let bullet_sprite = Sprite::new(render.device_mut(), &BULLET_VERTICES);

    let camera_uniform_buffer = render.device_mut().create_buffer(
        4 * 4 * size_of::<f32>(),
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    );
    let camera_uniform_buffer_ref = render.device().get_buffer(camera_uniform_buffer).unwrap();
    let camera_bind_group = render.device().create_bind_group(
        camera_bind_group_layout,
        &[BindGroupBinding::Buffer(camera_uniform_buffer_ref)],
    );

    let color_scheme_uniform_buffer = render.device_mut().create_buffer(
        4 * size_of::<f32>(),
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    );
    render.device().submit_buffer(color_scheme_uniform_buffer, 0, data_bytes(&[Color::rgb(250, 235, 215, 1.0)]));
    let color_scheme_uniform_buffer_ref = render.device().get_buffer(color_scheme_uniform_buffer).unwrap();
    let color_scheme_bind_group = render.device().create_bind_group(
        color_scheme_bind_group_layout,
        &[BindGroupBinding::Buffer(color_scheme_uniform_buffer_ref)],
    );

    let bounds = if let Some((width, height)) = render.surface().size() {
        calculate_game_bounds(width, height)
    } else { Vec2::new(1.0, 1.0) };

    let game = GameResource {
        pipeline,
        ship_sprite,
        meteor_sprite,
        bullet_sprite,
        camera_uniform_buffer,
        color_scheme_uniform_buffer,
        camera_bind_group,
        color_scheme_bind_group,
        previous_frame: Instant::now(),
        input_state: InputState::default(),
        state: GameState::default(),
        bounds,
        restart_timer: None,
    };
    hlist!(game, render, asset_source)
}

pub fn run_game<A: AssetSource>(event: SurfaceEvent, resources: &mut HList!(WGPURenderResource, GameResource, AssetSourceResource<A>)) -> SurfaceEventResult {
    let (render, resources) = resources.to_mut();
    let (game, _) = resources;

    match event {
        SurfaceEvent::Resize { width, height } => {
            let (surface, device) = render.get_mut();
            surface.configure(device, width, height);

            game.bounds = calculate_game_bounds(width, height);

            SurfaceEventResult::Continue
        }
        SurfaceEvent::Draw => {
            // Update game state
            {
                if let Some((time, duration)) = game.restart_timer.as_ref() {
                    if time.elapsed() > *duration {
                        game.state = GameState::default();
                        game.restart_timer = None;
                    }
                }

                // list of which entities will be deleted at the end of the game tick
                let mut remove = Vec::new();
                // components for new entities that will be spawned at the ent of the tick
                let mut create = Vec::new();

                //
                {
                    if game.state.previous_meteor.elapsed() >= game.state.time_until_meteor {
                        game.state.previous_meteor = Instant::now();
                        game.state.time_until_meteor = game.state.meteor_timer;
                        game.state.meteor_timer = Duration::from_secs_f32(game.state.meteor_timer.as_secs_f32() * 0.90);

                        let position: f32 = random();
                        let position = if position <= 0.25 {
                            Vec3::new(position * 8.0 - 1.0, 1.0, 0.0)
                        } else if position <= 0.5 {
                            Vec3::new(1.0, (position - 0.25) * 8.0 - 1.0, 0.0)
                        } else if position <= 0.75 {
                            Vec3::new((position - 0.5) * 8.0 - 1.0, -1.0, 0.0)
                        } else {
                            Vec3::new(-1.0, (position - 0.75) * 8.0 - 1.0, 0.0)
                        }.component_mul(&Vec3::new(game.bounds.x, game.bounds.y, 0.0));

                        let players = game.state.world.components::<Player>();
                        let transforms = game.state.world.components::<Transform>();

                        let velocity = game.state.world.entity_iter()
                            .filter(|entity| players.has(*entity))
                            .filter_map(|entity| transforms.get(entity))
                            .map(|transform| transform.position - position)
                            .min_by_key(|target| FloatOrd(target.magnitude()))
                            .unwrap_or(-position)
                            .normalize()
                            .scale(0.2);

                        let size = 1.0 - (random::<f32>() * 0.5 - 0.5);
                        let rotation = random::<f32>() * f32::pi() * 2.0;
                        let angular_velocity = random::<f32>() * 0.4;
                        create.push((
                            Transform {
                                position,
                                rotation,
                                size: 1.5 * size,
                                velocity,
                                angular_velocity,
                                ..Transform::default()
                            },
                            Shape::Meteor,
                            Collider { size: 0.05 * size },
                        ));
                    }
                }
                {
                    let elapsed_since_previous_frame = game.previous_frame.elapsed().as_secs_f32();
                    game.previous_frame = Instant::now();

                    let shoot = if game.input_state.shoot && !game.input_state.has_shot {
                        game.input_state.has_shot = true;
                        true
                    } else { false };

                    let mut transforms = game.state.world.components_mut::<Transform>();
                    let players = game.state.world.components::<Player>();

                    let rotation_speed = 2.1;
                    let player_rotation = (if game.input_state.left { 1.0 } else { 0.0 } +
                        if game.input_state.right { -1.0 } else { 0.0 }) * rotation_speed * elapsed_since_previous_frame;

                    let max_speed = 1.2;
                    let thrust_amount = 0.7;
                    let thrust_vec = Vec3::new(0.0, if game.input_state.up { 1.0 } else { 0.0 } + if game.input_state.down { -1.0 } else { 0.0 }, 0.0);

                    let bullet_speed = 2.0;

                    let mut player_count = 0;
                    for (entity, player) in game.state
                        .world
                        .entity_iter()
                        .map(|entity| (entity, players.get(entity)))
                    {
                        if let Some(transform) = transforms.get(entity) {
                            let mut velocity = transform.velocity;
                            let mut rotation = transform.rotation;
                            if player.is_some() {
                                player_count += 1;
                                rotation += player_rotation;

                                let thrust_angle = Rotation3::from_axis_angle(&Vec3::z_axis(), rotation);
                                let thrust = thrust_angle * thrust_vec * thrust_amount * elapsed_since_previous_frame;

                                velocity += thrust;
                                if velocity.magnitude() > max_speed {
                                    velocity = velocity.normalize() * max_speed;
                                }
                            }

                            rotation += transform.angular_velocity * elapsed_since_previous_frame;

                            let position = transform.position + velocity * elapsed_since_previous_frame;
                            let x = if position.x > game.bounds.x {
                                if transform.transient {
                                    remove.push(entity);
                                    continue;
                                }
                                -game.bounds.x
                            } else if position.x < -game.bounds.x {
                                if transform.transient {
                                    remove.push(entity);
                                    continue;
                                }
                                game.bounds.x
                            } else {
                                position.x
                            };
                            let y = if position.y > game.bounds.y {
                                if transform.transient {
                                    remove.push(entity);
                                    continue;
                                }
                                -game.bounds.y
                            } else if position.y < -game.bounds.y {
                                if transform.transient {
                                    remove.push(entity);
                                    continue;
                                }
                                game.bounds.y
                            } else {
                                position.y
                            };
                            let position = Vec3::new(x, y, 0.0);

                            if player.is_some() && shoot {
                                let angle = Rotation3::from_axis_angle(&Vec3::z_axis(), rotation);
                                let angle = angle * Vec3::y_axis();
                                create.push((
                                    Transform {
                                        position: position + angle.scale(0.02),
                                        velocity: angle.scale(bullet_speed),
                                        rotation,
                                        transient: true,
                                        size: 1.0,
                                        ..Default::default()
                                    },
                                    Shape::Bullet,
                                    Collider { size: 0.01 },
                                ));
                            }

                            let transform = Transform { position, rotation, velocity, ..transform.clone() };
                            transforms.put(entity, transform);
                        }
                    }
                    if player_count == 0 && game.restart_timer.is_none() {
                        game.restart_timer = Some((Instant::now(), Duration::from_secs(3)));
                    }
                }

                {
                    let players = View::builder()
                        .marked::<Player>().required::<Collider>().required::<Transform>()
                        .build(&game.state.world);
                    let meteors = View::builder()
                        .marked::<Meteor>().required::<Collider>().required::<Transform>()
                        .build(&game.state.world);
                    let bullets = View::builder()
                        .marked::<Bullet>().required::<Collider>().required::<Transform>()
                        .build(&game.state.world);

                    // check if a player is colliding with a meteor
                    for (player, (player_collider, (player_transform, _))) in players.iter() {
                        for (_, (meteor_collider, (meteor_transform, _))) in meteors.iter() {
                            if collides(player_collider, &player_transform.position, meteor_collider, &meteor_transform.position) {
                                remove.push(player);
                            }
                        }
                    }

                    let split_size = 0.6;
                    let split_angle = 0.5;
                    let split_velocity = 1.2;
                    let split_min_size = 0.5;

                    // check if a bullet is colliding with a meteor
                    for (bullet, (bullet_collider, (bullet_transform, _))) in bullets.iter() {
                        for (meteor, (meteor_collider, (meteor_transform, _))) in meteors.iter() {
                            if collides(bullet_collider, &bullet_transform.position, meteor_collider, &meteor_transform.position) {
                                remove.push(bullet);
                                remove.push(meteor);

                                if meteor_transform.size > split_min_size {
                                    let size_distribution = (random::<f32>() * 2.0 - 1.0) * 0.2;

                                    let rotation = random::<f32>() * f32::pi() * 2.0;
                                    // ±0.25
                                    let angle_random = random::<f32>() * 0.5 - 0.25;
                                    let size_random = 1.0 + size_distribution;
                                    let spin_direction = random::<f32>().signum();
                                    create.push((
                                        Transform {
                                            rotation,
                                            velocity: Rotation3::from_axis_angle(&Vec3::z_axis(), split_angle + angle_random) * meteor_transform.velocity * split_velocity,
                                            size: meteor_transform.size * split_size * size_random,
                                            angular_velocity: meteor_transform.angular_velocity * spin_direction + spin_direction * (random::<f32>() * 0.2 + 0.1),
                                            ..meteor_transform.clone()
                                        },
                                        Shape::Meteor,
                                        Collider { size: meteor_collider.size * split_size * size_random },
                                    ));
                                    let rotation = random::<f32>() * f32::pi() * 2.0;
                                    let angle_random = random::<f32>() * 0.5 - 0.25;
                                    let size = 1.0 - size_distribution;
                                    let spin_direction = random::<f32>().signum();
                                    create.push((
                                        Transform {
                                            rotation,
                                            velocity: Rotation3::from_axis_angle(&Vec3::z_axis(), -split_angle + angle_random) * meteor_transform.velocity * split_velocity,
                                            size: meteor_transform.size * split_size * size,
                                            angular_velocity: meteor_transform.angular_velocity * spin_direction + spin_direction * (random::<f32>() * 0.2 + 0.1),
                                            ..meteor_transform.clone()
                                        },
                                        Shape::Meteor,
                                        Collider { size: meteor_collider.size * split_size * size },
                                    ));
                                }
                            }
                        }
                    }
                }

                for (transform, shape, collider) in create {
                    let entity = game.state.world.new_entity();
                    game.state.world.components_mut::<Transform>().put(entity, transform);
                    game.state.world.components_mut::<Collider>().put(entity, collider);
                    game.state.world.components_mut::<Shape>().put(entity, shape);
                    match shape {
                        Shape::Ship => game.state.world.components_mut::<Player>().put(entity, Player),
                        Shape::Meteor => game.state.world.components_mut::<Meteor>().put(entity, Meteor),
                        Shape::Bullet => game.state.world.components_mut::<Bullet>().put(entity, Bullet),
                    }
                }
                for entity in remove {
                    game.state.world.drop_entity(entity)
                }
            }

            // Render game
            {
                let camera_scale = Vec2::new(1.0 / game.bounds.x, 1.0 / game.bounds.y);
                let view_matrix: Matrix4<f32> = Matrix4::new_nonuniform_scaling(&Vec3::new(camera_scale.x, camera_scale.y, 1.0));

                render.device().submit_buffer(game.camera_uniform_buffer, 0, data_bytes(&[view_matrix]));

                let transforms = game.state.world.components::<Transform>();
                let shapes = game.state.world.components::<Shape>();

                let mut player_transforms = Vec::new();
                let mut meteor_transforms = Vec::new();
                let mut bullet_transforms = Vec::new();

                for (_, shape, transform) in game.state
                    .world
                    .entity_iter()
                    .filter_map(|entity| shapes.get(entity).map(|shape| (entity, shape)))
                    .filter_map(|(entity, shape)| {
                        transforms
                            .get(entity)
                            .map(|transform| (entity, shape, transform))
                    })
                {
                    match shape {
                        Shape::Ship => player_transforms.push(transform),
                        Shape::Meteor => meteor_transforms.push(transform),
                        Shape::Bullet => bullet_transforms.push(transform),
                    }
                }

                let mut clear = true;
                let render_passes = [
                    (&game.ship_sprite, player_transforms),
                    (&game.meteor_sprite, meteor_transforms),
                    (&game.bullet_sprite, bullet_transforms),
                ].into_iter().map(|(sprite, transforms)| {
                    let instances = transforms.into_iter().map(Transform::to_matrix).collect::<Vec<_>>();
                    let instances_data = data_bytes(&instances);
                    render.device_mut().resize_buffer(sprite.instance_buffer, instances_data.len());
                    render.device().submit_buffer(sprite.instance_buffer, 0, instances_data);

                    RenderPass {
                        pipeline: game.pipeline,
                        vertices: 0..sprite.vertices,
                        targets: vec![Target::ScreenTarget {
                            clear: if clear {
                                clear = false;
                                Some(Color::rgb(0, 3, 22, 1.0))
                            } else { None },
                        }],
                        vertex_buffers: vec![Some(sprite.vertex_buffer), Some(sprite.instance_buffer)],
                        bind_groups: vec![game.camera_bind_group.clone(), game.color_scheme_bind_group.clone()],
                        instances: 0..instances.len() as _,
                    }
                }).collect::<Vec<_>>();

                let frame = render.surface().get_frame();
                let mut encoder = render.device().command_encoder(&frame);

                for render_pass in render_passes {
                    encoder.render_pass(render_pass);
                }

                render.device().submit_commands(encoder);

                render.surface().present(frame);
            }

            SurfaceEventResult::Continue
        }
        SurfaceEvent::CloseRequested => SurfaceEventResult::Exit(None),
        SurfaceEvent::DeviceEvent(DeviceEvent::Key(key)) => {
            let state = key.state == ElementState::Pressed;
            match key.virtual_keycode {
                Some(VirtualKeyCode::Up) => game.input_state.up = state,
                Some(VirtualKeyCode::Down) => game.input_state.down = state,
                Some(VirtualKeyCode::Left) => game.input_state.left = state,
                Some(VirtualKeyCode::Right) => game.input_state.right = state,
                Some(VirtualKeyCode::Space) => {
                    game.input_state.shoot = state;
                    if !state {
                        game.input_state.has_shot = false;
                    }
                }
                _ => (),
            }
            SurfaceEventResult::Continue
        }
        _ => SurfaceEventResult::Continue,
    }
}
