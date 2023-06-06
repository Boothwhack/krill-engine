use std::collections::HashMap;
use std::mem::size_of;
use std::time::Duration;

use bytemuck::bytes_of;
use float_ord::FloatOrd;
use instant::Instant;
use log::debug;
use nalgebra::{Matrix4, RealField, Rotation3, Vector2, Vector3};
use rand::random;

use engine::asset_resource::AssetSourceResource;
use engine::assets::source::AssetSource;
use engine::ecs::world::{View, World};
use engine::events::{Context, ContextWith};
use engine::render::{Batch, BufferUsages, Color, Handle, Model, VecBuf};
use engine::render::geometry::VertexFormat;
use engine::render::material::{AttributeDefinition, AttributeSemantics, AttributeType, Material, MaterialDefinition, PipelineDefinition, Shader, UniformDefinition, UniformEntryDefinition, UniformEntryTypeDefinition, UniformVisibility};
use engine::render::uniform::{UniformInstance, UniformInstanceEntry};
use engine::surface::{Exit, RunnableSurface, SurfaceEvent, SurfaceResource};
use engine::surface::input::{DeviceEvent, ElementState, VirtualKeyCode};
use engine::utils::{HList, hlist};
use engine::wgpu_render::WGPURenderResource;

use crate::graphics::{BACKGROUND_COLOR, FOREGROUND_COLOR, Graphics, Shape};
use crate::text::Text;

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
        let scale = Matrix4::new_scaling(0.1 * self.size);
        translation * rotation.to_homogeneous() * scale
    }
}

// Marker component that denotes the player entity
struct Player;

// Marker component that denotes a bullet in flight
struct Bullet;

// Marker component that denotes a meteor
struct Meteor;

#[derive(Debug)]
enum Type {
    Player,
    Bullet,
    Meteor,
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
    score: u32,
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
            score: 0,
        }
    }
}

pub struct GameResource {
    pub material: Handle<Material>,
    pub graphics: Graphics,
    pub camera_uniform: UniformInstance,
    pub camera_uniform_buffer: Handle<VecBuf>,
    pub previous_frame: Instant,
    input_state: InputState,
    pub state: GameState,
    pub bounds: Vec2,
    pub restart_timer: Option<(Instant, Duration)>,
    pub text: Text,
}

fn calculate_game_bounds(width: u32, height: u32) -> Vec2 {
    let aspect_ratio = width as f32 / height as f32;

    if aspect_ratio > 1.0 {
        Vec2::new(1.0, height as f32 / width as f32)
    } else {
        Vec2::new(aspect_ratio, 1.0)
    }
}

pub async fn setup_game_resources<A: AssetSource>(resources: HList!(WGPURenderResource, AssetSourceResource<A>)) -> HList!(GameResource, WGPURenderResource, AssetSourceResource<A>) {
    let (mut render, resources) = resources;
    let (asset_source, _) = resources;

    render.register_uniform("camera", UniformDefinition {
        entries: vec![UniformEntryDefinition {
            visibility: UniformVisibility::Vertex,
            typ: UniformEntryTypeDefinition::Buffer,
        }],
    });
    let material = render.new_material(
        MaterialDefinition {
            attributes: vec![
                AttributeDefinition {
                    name: None,
                    semantics: AttributeSemantics::Position { transform: Default::default() },
                    typ: AttributeType::Float32(3),
                },
                AttributeDefinition {
                    name: None,
                    semantics: AttributeSemantics::Color,
                    typ: AttributeType::Float32(4),
                },
            ],
            uniforms: vec!["camera".to_owned()],
        },
        PipelineDefinition {
            shader_modules: vec![include_str!("assets/game.wgsl").to_owned()],
            vertex_shader: Shader { index: 0, entrypoint: "vs_main".to_owned() },
            fragment_shader: Shader { index: 0, entrypoint: "fs_main".to_owned() },
            attribute_locations: {
                let mut attributes = HashMap::new();
                attributes.insert("position".to_owned(), 0);
                attributes.insert("color".to_owned(), 1);
                attributes
            },
        },
    );

    let vertex_format = VertexFormat::from(vec![
        AttributeDefinition {
            name: Some("position".to_owned()),
            semantics: AttributeSemantics::Position { transform: Default::default() },
            typ: AttributeType::Float32(3),
        },
        AttributeDefinition {
            name: Some("color".to_owned()),
            semantics: AttributeSemantics::Color,
            typ: AttributeType::Float32(4),
        },
    ]);

    let camera_uniform_buffer = render.new_buffer(size_of::<Matrix4<f32>>(), BufferUsages::UNIFORM | BufferUsages::COPY_DST);
    let camera_uniform = render.instantiate_uniform("camera", vec![Some(UniformInstanceEntry::Buffer(camera_uniform_buffer.into()))]);

    let bounds = if let Some((width, height)) = render.surface_size() {
        calculate_game_bounds(width, height)
    } else {
        Vec2::new(1.0, 1.0)
    };

    let text = Text::new(render.render_mut(), &vertex_format);
    let graphics = Graphics::new(render.render_mut(), &vertex_format);

    let game = GameResource {
        material,
        graphics,
        camera_uniform,
        camera_uniform_buffer,
        previous_frame: Instant::now(),
        input_state: InputState::default(),
        state: GameState::default(),
        bounds,
        restart_timer: None,
        text,
    };
    hlist!(game, render, asset_source)
}

const MAX_METEOR_SIZE: f32 = 2.0;
const SIZE_BIAS: f32 = 1.8;

fn calculate_score(size: f32) -> u32 {
    let size = (MAX_METEOR_SIZE - size) / SIZE_BIAS;
    let size_multiplier = size.powf(2.0);
    let score = 50 + (size * size_multiplier * 100.0).round() as u32;
    debug!(target:"meteors", "Scored: {score} for hit: {size} ({size_multiplier})");
    score
}

pub fn on_surface_event<R, S, I>(event: SurfaceEvent, mut context: Context<SurfaceEvent, R>) -> ()
    where S: RunnableSurface,
          for<'a> Context<'a, SurfaceEvent, R>: ContextWith<HList!(GameResource, WGPURenderResource, SurfaceResource<S>), I> {
    let (game, resources) = context.resources_mut();
    let (render, resources) = resources;
    let (surface, _) = resources;

    match event {
        SurfaceEvent::Resize { width, height } => {
            render.configure_surface(width, height);
            game.bounds = calculate_game_bounds(width, height);
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
                            Type::Meteor,
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
                                    Type::Bullet,
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

                                game.state.score += calculate_score(meteor_transform.size);

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
                                        Type::Meteor,
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
                                        Type::Meteor,
                                        Collider { size: meteor_collider.size * split_size * size },
                                    ));
                                }
                            }
                        }
                    }
                }

                for (transform, shape, typ, collider) in create {
                    debug!(target:"meteors", "Spawning entity: {shape:?} {transform:?} {typ:?} {collider:?}");

                    let entity = game.state.world.new_entity();
                    game.state.world.components_mut::<Transform>().put(entity, transform);
                    game.state.world.components_mut::<Collider>().put(entity, collider);
                    game.state.world.components_mut::<Shape>().put(entity, shape);
                    match typ {
                        Type::Player => game.state.world.components_mut::<Player>().put(entity, Player),
                        Type::Meteor => game.state.world.components_mut::<Meteor>().put(entity, Meteor),
                        Type::Bullet => game.state.world.components_mut::<Bullet>().put(entity, Bullet),
                    }
                }
                for entity in remove {
                    game.state.world.drop_entity(entity)
                }
            }

            // Render game
            {
                // setup camera uniform buffer
                let camera_scale = Vec2::new(1.0 / game.bounds.x, 1.0 / game.bounds.y);
                let view_matrix: Matrix4<f32> = Matrix4::new_nonuniform_scaling(&Vec3::new(camera_scale.x, camera_scale.y, 1.0));

                render.get_buffer(game.camera_uniform_buffer)
                    .unwrap()
                    .upload(0, bytes_of(&view_matrix));

                //let mut shapes_instances: HashMap<Shape, Vec<Matrix4<f32>>> = HashMap::new();
                let mut models: Vec<Model> = vec![];

                // collect shapes from the ecs (player, meteors and bullets)
                let shapes = View::builder()
                    .required::<Shape>()
                    .required::<Transform>()
                    .build(&game.state.world);
                for (_, (shape, (transform, ()))) in shapes.iter() {
                    game.graphics.submit_models(shape, transform.to_matrix(), &mut models);
                }

                // score text
                let score = format!("{}", game.state.score);
                let score = score
                    .chars()
                    .filter(|c| c.is_ascii())
                    .flat_map(|c| c.to_uppercase());
                let mut offset = 0.0;
                const FONT_SIZE: f32 = 0.05;
                const LETTER_SPACING: f32 = 0.3;
                const SAFE_AREA: Vec2 = Vec2::new(0.05, 0.05);
                let text_translation = Matrix4::new_translation(&Vec3::new(
                    -game.bounds.x + SAFE_AREA.x,
                    game.bounds.y - SAFE_AREA.y,
                    0.0,
                )) * Matrix4::new_scaling(FONT_SIZE);
                for char in score {
                    if let Some(character) = game.text.character(char) {
                        let char_translation = Matrix4::new_translation(&Vec3::new(
                            offset - character.bounds.0,
                            -1.0,
                            0.0,
                        ));

                        offset += character.size() + LETTER_SPACING;
                        models.push(Model::new(
                            character.data,
                            text_translation * char_translation,
                            FOREGROUND_COLOR,
                        ));
                    }
                }

                let frame = render.request_frame();

                let mut drawer = render.new_drawer(&frame);

                let mut batch = Batch::new(game.material, vec![&game.camera_uniform]);
                batch.clear(BACKGROUND_COLOR);
                batch.models(models);

                drawer.submit_batch(batch);
                drawer.finish();

                render.present_frame(frame);
            }
        }
        SurfaceEvent::CloseRequested => surface.set_exit(Exit::Exit),
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
        }
        _ => {}
    }
}
