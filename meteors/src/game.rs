use std::mem::swap;
use std::time::Duration;

use bytemuck::bytes_of;
use float_ord::FloatOrd;
use instant::Instant;
use log::debug;
use nalgebra::{matrix, Matrix4, RealField, Rotation3, vector, Vector2, Vector3};
use rand::random;

use engine::asset_resource::AssetSourceResource;
use engine::assets::source::AssetSource;
use engine::ecs::world::{EntityId, View, World};
use engine::events::{Context, ContextWith};
use engine::render::{Batch, Color, Model, RenderApi};
use engine::surface::{Exit, RunnableSurface, SurfaceEvent, SurfaceResource};
use engine::surface::input::{DeviceEvent, ElementState, VirtualKeyCode};
use engine::utils::{HList, hlist};
use engine::wgpu_render::WGPURenderResource;

use crate::graphics::{BACKGROUND_COLOR, FOREGROUND_COLOR, Graphics, Shape};

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
pub struct Body {
    transform: Transform,
    velocity: Vec3,
    angular_velocity: f32,
    transient: bool,
}

#[derive(Clone, Debug)]
pub struct Transform {
    position: Vec3,
    rotation: f32,
    size: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            position: Vector3::zeros(),
            rotation: 0.0,
            size: 1.0,
        }
    }
}

impl Transform {
    pub fn position(&self) -> &Vector3<f32> {
        &self.position
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn to_matrix(&self) -> Matrix4<f32> {
        let translation = Matrix4::new_translation(&self.position);
        let rotation = Rotation3::from_euler_angles(0.0, 0.0, self.rotation);
        let scale = Matrix4::new_scaling(self.size);
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

fn default_world() -> World {
    World::default()
        .with_component::<Player>()
        .with_component::<Meteor>()
        .with_component::<Bullet>()
        .with_component::<Body>()
        .with_component::<Shape>()
        .with_component::<Collider>()
}

fn add_player(world: &mut World) -> EntityId {
    let player = world.new_entity();

    world.components_mut::<Player>().put(player, Player);
    world.components_mut::<Body>().put(player, Default::default());
    world.components_mut::<Shape>().put(player, Shape::Ship);
    world.components_mut::<Collider>().put(player, Collider { size: 0.025 });

    player
}

pub enum GameState {
    Empty,
    MainMenu(MainMenuState),
    InGame(IngameState),
    GameOver(GameOverState),
}

impl Default for GameState {
    fn default() -> Self {
        GameState::Empty
    }
}

impl GameState {
    fn new() -> Self {
        GameState::MainMenu(Default::default())
    }

    fn take(&mut self) -> Self {
        let mut value = GameState::Empty;
        swap(self, &mut value);
        value
    }
}

pub struct IngameState {
    world: World,
    previous_meteor: Instant,
    meteor_timer: Duration,
    score: u32,
}

impl From<World> for IngameState {
    fn from(value: World) -> Self {
        IngameState {
            world: value,
            ..Default::default()
        }
    }
}

impl Default for IngameState {
    fn default() -> Self {
        let mut world = default_world();
        add_player(&mut world);

        IngameState {
            world,
            previous_meteor: Instant::now(),
            meteor_timer: Duration::from_secs(10),
            score: 0,
        }
    }
}

pub struct GameOverState {
    score: u32,
    world: World,
    dead_time: Instant,
    fade_out: Duration,
}

pub struct GlobalState {
    input_state: InputState,
    previous_update: Instant,
    bounds: Vec2,
}

pub struct MainMenuState {
    world: World,
    fade_out: Option<(Instant, Duration)>,
}

impl Default for MainMenuState {
    fn default() -> Self {
        let mut world = default_world();
        add_player(&mut world);

        const START_METEOR_SIZE:f32 = 1.5;
        let start_meteor = world.new_entity();
        world.components_mut::<Meteor>().put(start_meteor, Meteor);
        world.components_mut::<Body>().put(start_meteor, Body {
            transform: Transform {
                position: vector!(4.5, -1.8, 0.0),
                size: START_METEOR_SIZE,
                rotation: 0.0,
            },
            angular_velocity: 0.2,
            ..Default::default()
        });
        world.components_mut::<Shape>().put(start_meteor, Shape::Meteor);
        world.components_mut::<Collider>().put(start_meteor, Collider { size: START_METEOR_SIZE*0.75 });

        MainMenuState {
            world,
            fade_out: None,
        }
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        GlobalState {
            input_state: Default::default(),
            previous_update: Instant::now(),
            bounds: vector!(Self::VIEWPORT_SCALE, Self::VIEWPORT_SCALE),
        }
    }
}

impl GlobalState {
    const VIEWPORT_SCALE: f32 = 10.0;

    fn calculate_bounds(&mut self, width: u32, height: u32) {
        let aspect_ratio = width as f32 / height as f32;

        self.bounds = if aspect_ratio > 1.0 {
            Vec2::new(1.0, height as f32 / width as f32)
        } else {
            Vec2::new(aspect_ratio, 1.0)
        } * Self::VIEWPORT_SCALE;
    }
}

pub struct GameResource {
    pub graphics: Graphics,
    pub state: GameState,
    pub global: GlobalState,
}

impl GameResource {
    fn new(render: &mut RenderApi) -> Self {
        GameResource {
            graphics: Graphics::new(render),
            state: Default::default(),
            global: Default::default(),
        }
    }
}

pub async fn setup_game_resources<A: AssetSource>(resources: HList!(WGPURenderResource, AssetSourceResource<A>)) -> HList!(GameResource, WGPURenderResource, AssetSourceResource<A>) {
    let (mut render, (asset_source, ..)) = resources;

    let mut game = GameResource::new(render.render_mut());
    if let Some((width, height)) = render.surface_size() {
        game.global.calculate_bounds(width, height);
    }
    hlist!(game, render, asset_source)
}

const MAX_METEOR_SIZE: f32 = 2.0;
const SIZE_BIAS: f32 = 1.8;

pub fn on_surface_event<R, S, I>(event: SurfaceEvent, mut context: Context<SurfaceEvent, R>) -> ()
    where S: RunnableSurface,
          for<'a> Context<'a, SurfaceEvent, R>: ContextWith<HList!(GameResource, WGPURenderResource, SurfaceResource<S>), I> {
    let (game, resources) = context.resources_mut();
    let (render, resources) = resources;
    let (surface, _) = resources;

    match event {
        SurfaceEvent::Resize { width, height } => {
            render.configure_surface(width, height);
            game.global.calculate_bounds(width, height);
        }
        SurfaceEvent::Draw => {
            let mut models = vec![];

            game.state = match game.state.take() {
                GameState::Empty => GameState::new(),
                GameState::MainMenu(mut state) => {
                    let mut create = vec![];
                    let mut remove = vec![];
                    common_update_world(GameContext {
                        global: &mut game.global,
                        world: &mut state.world,
                        create: &mut create,
                        remove: &mut remove,
                    });
                    let mut hit_start_meteor = false;
                    check_collisions_between::<Bullet, Meteor, _>(&state.world, |((bullet, bullet_body, _), (meteor, meteor_body, meteor_collider))| {
                        hit_start_meteor = true;
                        let velocity = vector!(0.0, 1.8, 0.0);
                        split_meteor(meteor_body, meteor_collider, Rotation3::from_euler_angles(0.0, 0.0, bullet_body.transform.rotation) * velocity, &mut create);
                        remove.push(meteor);
                        remove.push(bullet);
                    });
                    remove_entities(remove, &mut state.world);
                    create_entities(create, &mut state.world);

                    draw_world(&state.world, &mut game.graphics, &mut models);
                    draw_logo(&game.global, &game.graphics, &mut models);

                    game.graphics.draw_arrow_keys(
                        Matrix4::new_scaling(0.3).append_translation(&vector!(-4.5, -2.0, 0.0)),
                        FOREGROUND_COLOR,
                        &mut models,
                    );
                    game.graphics.draw_text(
                        "MOVE",
                        Matrix4::new_scaling(0.3).append_translation(&vector!(-5.55, -2.8, 0.0)),
                        FOREGROUND_COLOR,
                        &mut models,
                    );

                    game.graphics.draw_spacebar(
                        Matrix4::new_scaling(0.3).append_translation(&vector!(0.0, -2.0, 0.0)),
                        FOREGROUND_COLOR,
                        &mut models,
                    );
                    game.graphics.draw_text(
                        "SHOOT",
                        Matrix4::new_scaling(0.3).append_translation(&vector!(-1.4, -2.8, 0.0)),
                        FOREGROUND_COLOR,
                        &mut models,
                    );

                    game.graphics.draw_text(
                        "DESTROY",
                        Matrix4::new_scaling(0.3).append_translation(&vector!(2.6, -2.8, 0.0)),
                        FOREGROUND_COLOR,
                        &mut models,
                    );

                    if hit_start_meteor {
                        GameState::InGame(IngameState::from(state.world))
                    } else {
                        GameState::MainMenu(state)
                    }
                }
                GameState::InGame(mut state) => {
                    // update game state
                    let mut create = vec![];
                    let mut remove = vec![];
                    common_update_world(GameContext {
                        global: &mut game.global,
                        world: &mut state.world,
                        create: &mut create,
                        remove: &mut remove,
                    });

                    if state.previous_meteor.elapsed() >= state.meteor_timer {
                        spawn_meteor(&state.world, &game.global, &mut create);
                        state.previous_meteor = Instant::now();
                        // spawn next meteor 10% sooner to increase difficulty
                        state.meteor_timer = Duration::from_secs_f32(state.meteor_timer.as_secs_f32() * 0.90);
                    }

                    // handle collisions
                    check_collisions_between::<Player, Meteor, _>(&state.world, |((player, ..), ..)| {
                        remove.push(player);
                    });
                    check_collisions_between::<Bullet, Meteor, _>(&state.world, |((bullet, ..), (meteor, body, collider))| {
                        state.score += calculate_score(body.transform.size);
                        remove.push(bullet);
                        remove.push(meteor);
                        split_meteor(body, collider, Vec3::zeros(), &mut create);
                    });

                    remove_entities(remove, &mut state.world);
                    create_entities(create, &mut state.world);

                    draw_world(&state.world, &mut game.graphics, &mut models);
                    draw_score(state.score, &game.global, &game.graphics, &mut models);

                    // transition to game over state if all players are dead
                    let player_count = View::builder().marked::<Player>().build(&state.world).iter().count();
                    if player_count == 0 {
                        debug!(target: "meteors", "Game over, score: {}", state.score);
                        GameState::GameOver(GameOverState {
                            score: state.score,
                            world: state.world,
                            dead_time: Instant::now(),
                            fade_out: Duration::from_secs(3),
                        })
                    } else {
                        GameState::InGame(state)
                    }
                }
                GameState::GameOver(mut state) => {
                    common_update_world(GameContext {
                        global: &mut game.global,
                        world: &mut state.world,
                        remove: &mut vec![],
                        create: &mut vec![],
                    });

                    draw_world(&state.world, &mut game.graphics, &mut models);
                    draw_score(state.score, &game.global, &game.graphics, &mut models);

                    if state.dead_time.elapsed() > state.fade_out {
                        debug!(target:"meteors", "Restarting game...");
                        GameState::InGame(Default::default())
                    } else {
                        GameState::GameOver(state)
                    }
                }
            };

            game.global.previous_update = Instant::now();

            // setup camera uniform buffer
            let camera_scale = vector!(1.0 / game.global.bounds.x, 1.0 / game.global.bounds.y);
            let view_matrix: Matrix4<f32> = Matrix4::new_nonuniform_scaling(&vector!(camera_scale.x, camera_scale.y, 1.0));

            render.get_buffer(game.graphics.camera_uniform_buffer)
                .unwrap()
                .upload(0, bytes_of(&view_matrix));

            // draw game
            let frame = render.request_frame();

            let mut drawer = render.new_drawer(&frame);

            let mut batch = Batch::new(game.graphics.material, vec![&game.graphics.camera_uniform]);
            batch.clear(BACKGROUND_COLOR);
            batch.models(models);

            drawer.submit_batch(batch);
            drawer.finish();

            render.present_frame(frame);
        }
        SurfaceEvent::CloseRequested => surface.set_exit(Exit::Exit),
        SurfaceEvent::DeviceEvent(DeviceEvent::Key(key)) => {
            let state = key.state == ElementState::Pressed;
            match key.virtual_keycode {
                Some(VirtualKeyCode::Up) => game.global.input_state.up = state,
                Some(VirtualKeyCode::Down) => game.global.input_state.down = state,
                Some(VirtualKeyCode::Left) => game.global.input_state.left = state,
                Some(VirtualKeyCode::Right) => game.global.input_state.right = state,
                Some(VirtualKeyCode::Space) => {
                    game.global.input_state.shoot = state;
                    if !state {
                        game.global.input_state.has_shot = false;
                    }
                }
                _ => (),
            }
        }
        _ => {}
    }
}

#[derive(Default)]
struct Components {
    body: Option<Body>,
    shape: Option<Shape>,
    collider: Option<Collider>,
}

struct GameContext<'a> {
    global: &'a mut GlobalState,
    world: &'a mut World,
    create: &'a mut Vec<(Type, Components)>,
    remove: &'a mut Vec<EntityId>,
}

fn remove_entities(entities: Vec<EntityId>, world: &mut World) {
    for entity in entities {
        world.drop_entity(entity);
    }
}

fn create_entities(entities: Vec<(Type, Components)>, world: &mut World) {
    for (typ, Components { body, shape, collider }) in entities {
        let entity = world.new_entity();
        match typ {
            Type::Player => world.components_mut::<Player>().put(entity, Player),
            Type::Bullet => world.components_mut::<Bullet>().put(entity, Bullet),
            Type::Meteor => world.components_mut::<Meteor>().put(entity, Meteor),
        }

        if let Some(body) = body {
            world.components_mut::<Body>().put(entity, body);
        }
        if let Some(shape) = shape {
            world.components_mut::<Shape>().put(entity, shape);
        }
        if let Some(collider) = collider {
            world.components_mut::<Collider>().put(entity, collider);
        }
    }
}

fn calculate_score(size: f32) -> u32 {
    let size = (MAX_METEOR_SIZE - size) / SIZE_BIAS;
    let size_multiplier = size.powf(2.0);
    let score = 50 + (size * size_multiplier * 100.0).round() as u32;
    debug!(target:"meteors", "Scored: {score} for hit: {size} ({size_multiplier})");
    score
}

/// Common operations that need to occur every frame regardless of game state
fn common_update_world(mut context: GameContext) {
    let elapsed_since_previous_frame = context.global.previous_update.elapsed().as_secs_f32();

    let mut bodies = context.world.components_mut::<Body>();

    // update player
    const MAX_SPEED: f32 = 12.0;
    const THRUST_AMOUNT: f32 = 7.0;
    let thrust_direction = vector!(
        0.0,
        if context.global.input_state.up { THRUST_AMOUNT } else { 0.0 }
            + if context.global.input_state.down { -THRUST_AMOUNT } else { 0.0 },
        0.0
    );

    const ROTATION_SPEED: f32 = 2.1;
    let player_rotation = (if context.global.input_state.left { 1.0 } else { 0.0 } +
        if context.global.input_state.right { -1.0 } else { 0.0 }) * ROTATION_SPEED;

    const BULLET_SPEED: f32 = 20.0;
    let shoot = if context.global.input_state.shoot && !context.global.input_state.has_shot {
        context.global.input_state.has_shot = true;
        true
    } else { false };

    for (player, ..) in View::builder()
        .marked::<Player>()
        .build(context.world)
        .iter() {
        if let Some(mut body) = bodies.get(player).cloned() {
            body.transform.rotation += player_rotation * elapsed_since_previous_frame;

            let thrust_angle = Rotation3::from_axis_angle(&Vec3::z_axis(), body.transform.rotation);
            let thrust = thrust_angle * thrust_direction;
            body.velocity += thrust * elapsed_since_previous_frame;

            if shoot {
                let angle = Rotation3::from_axis_angle(&Vec3::z_axis(), body.transform.rotation);
                let angle = angle * Vec3::y_axis();
                context.create.push((
                    Type::Bullet,
                    Components {
                        body: Some(Body {
                            transform: Transform {
                                position: body.transform.position + angle.scale(0.2),
                                rotation: body.transform.rotation,
                                size: 1.0,
                            },
                            velocity: angle.scale(BULLET_SPEED),
                            transient: true,

                            ..Default::default()
                        }),
                        shape: Some(Shape::Bullet),
                        collider: Some(Collider { size: 0.1 }),
                    },
                ));
            }

            bodies.put(player, body);
        }
    }

    // update physics
    for entity in context.world.entity_iter() {
        if let Some(body) = bodies.get(entity) {
            let mut body = body.clone();
            body.transform.rotation += body.angular_velocity * elapsed_since_previous_frame;
            body.transform.position += body.velocity * elapsed_since_previous_frame;

            if body.transient {
                if body.transform.position.x.abs() > context.global.bounds.x || body.transform.position.y.abs() > context.global.bounds.y {
                    context.remove.push(entity);
                }
            } else {
                // wraps position to screen bounds
                body.transform.position.x = (body.transform.position.x + context.global.bounds.x) % (context.global.bounds.x * 2.0) - context.global.bounds.x;
                body.transform.position.y = (body.transform.position.y + context.global.bounds.y) % (context.global.bounds.y * 2.0) - context.global.bounds.y;
                if body.transform.position.x < -context.global.bounds.x {
                    body.transform.position.x += context.global.bounds.x * 2.0;
                }
                if body.transform.position.y < -context.global.bounds.y {
                    body.transform.position.y += context.global.bounds.y * 2.0;
                }
            }

            bodies.put(entity, body);
        }
    }
}

/// Spawns a meteor at a random position at the screens edge, with randomized size and rotation.
fn spawn_meteor(world: &World, global: &GlobalState, create: &mut Vec<(Type, Components)>) {
    let position: f32 = random();
    let position = if position <= 0.25 {
        Vec3::new(position * 8.0 - 1.0, 1.0, 0.0)
    } else if position <= 0.5 {
        Vec3::new(1.0, (position - 0.25) * 8.0 - 1.0, 0.0)
    } else if position <= 0.75 {
        Vec3::new((position - 0.5) * 8.0 - 1.0, -1.0, 0.0)
    } else {
        Vec3::new(-1.0, (position - 0.75) * 8.0 - 1.0, 0.0)
    }.component_mul(&Vec3::new(global.bounds.x, global.bounds.y, 0.0));

    let players = View::builder()
        .marked::<Player>()
        .required::<Body>()
        .build(world);
    let direction = players.iter()
        .map(|(_, (body, ..))| &body.transform)
        .map(|transform| transform.position - position)
        .min_by_key(|target| FloatOrd(target.magnitude()))
        .unwrap_or(-position)
        .normalize();

    let velocity = direction * 2.0;
    let size = 1.0 - (random::<f32>() * 0.5 - 0.5);
    let rotation = random::<f32>() * f32::pi() * 2.0;
    let angular_velocity = random::<f32>() * 0.4;

    create.push((Type::Meteor, Components {
        body: Some(Body {
            transform: Transform {
                position,
                rotation,
                size: 1.5 * size,
            },
            velocity,
            angular_velocity,
            ..Default::default()
        }),
        shape: Some(Shape::Meteor),
        collider: Some(Collider { size: size * 0.75 }),
    }));
}

fn split_meteor(body: &Body, collider: &Collider, velocity: Vec3, create: &mut Vec<(Type, Components)>) {
    const SPLIT_MIN_SIZE: f32 = 0.5;
    const SPLIT_SIZE: f32 = 0.6;
    const SPLIT_ANGLE: f32 = 0.5;
    const SPLIT_VELOCITY: f32 = 1.2;

    if body.transform.size > SPLIT_MIN_SIZE {
        let size_distribution = (random::<f32>() * 2.0 - 1.0) * 0.2;

        for sign in [1.0, -1.0] {
            let size = 1.0 + sign * size_distribution;
            let size_multiplier = SPLIT_SIZE * size;
            let rotation = random::<f32>() * f32::pi() * 2.0;
            let angle_random = random::<f32>() * 0.5 - 0.25;
            let spin_direction = (random::<f32>() - 0.5).signum();
            let general_velocity = velocity + body.velocity * SPLIT_VELOCITY;
            create.push((Type::Meteor, Components {
                body: Some(Body {
                    transform: Transform {
                        position: body.transform.position,
                        rotation,
                        size: body.transform.size * size_multiplier,
                    },
                    velocity: Rotation3::from_axis_angle(&Vec3::z_axis(), sign * SPLIT_ANGLE + angle_random) * general_velocity,
                    angular_velocity: body.angular_velocity * spin_direction + spin_direction * (random::<f32>() * 0.2 + 0.1),
                    ..body.clone()
                }),
                shape: Some(Shape::Meteor),
                collider: Some(Collider { size: collider.size * size_multiplier }),
            }));
        }
    }
}

fn check_collisions_between<A: 'static, B: 'static, F>(world: &World, f: F)
    where F: FnMut(((EntityId, &Body, &Collider), (EntityId, &Body, &Collider))) {
    let a = View::builder()
        .marked::<A>()
        .required::<Body>()
        .required::<Collider>()
        .build(world);
    let b = View::builder()
        .marked::<B>()
        .required::<Body>()
        .required::<Collider>()
        .build(world);

    a.iter().flat_map(|(a, (body_a, (collider_a, ..)))|
        b.iter().filter_map(move |(b, (body_b, (collider_b, ..)))| {
            collides(collider_a, body_a.transform.position(), collider_b, body_b.transform.position())
                .then(|| ((a, body_a, collider_a), (b, body_b, collider_b)))
        })
    ).for_each(f);
}

fn draw_world(world: &World, graphics: &Graphics, models: &mut Vec<Model>) {
    // collect shapes from the ecs (player, meteors and bullets)
    let shapes = View::builder()
        .required::<Shape>()
        .required::<Body>()
        .build(world);
    for (_, (shape, (body, ..))) in shapes.iter() {
        graphics.draw_shape(shape, &body.transform, models);
    }
}

fn draw_score(score: u32, global: &GlobalState, graphics: &Graphics, models: &mut Vec<Model>) {
    const SAFE_AREA: Vec2 = Vec2::new(0.5, 0.5);
    const FONT_SIZE: f32 = 0.5;

    let score = format!("{}", score);
    let text_translation = Matrix4::new_translation(&Vec3::new(
        -global.bounds.x + SAFE_AREA.x,
        global.bounds.y - SAFE_AREA.y,
        0.0,
    )) * Matrix4::new_scaling(FONT_SIZE);
    graphics.draw_text(&score, text_translation, FOREGROUND_COLOR, models);
}

fn draw_logo(global: &GlobalState, graphics: &Graphics, models: &mut Vec<Model>) {
    let skew = matrix![
        1.0, 0.0, 0.0, 0.0;
        0.0, 1.0, 0.0, 0.0;
        0.0, 0.0, 1.0, 0.0;
        0.0, 0.1, 0.0, 1.0];
    const LOGO_SIZE: f32 = 0.8;
    // approximately centered text
    let transform = skew
        .prepend_translation(&vector!(-6.2, 0.0, 0.0)) // center text for skew effect
        .append_nonuniform_scaling(&vector!(1.0 * LOGO_SIZE, 1.2 * LOGO_SIZE, 1.0))
        .append_translation(&vector!(0.0, 5.0, 0.0));
    graphics.draw_text("METEORS", transform, FOREGROUND_COLOR, models);
}
