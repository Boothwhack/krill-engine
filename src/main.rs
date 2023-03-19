use std::time::{Duration, SystemTime};

use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event::ElementState::Pressed;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

mod ecs;
mod engine;
mod events;
mod genvec;
mod render;

type Vec2 = nalgebra::Vector2<f32>;

struct Player;

#[derive(Default, Clone, Debug)]
struct Position {
    pos: Vec2,
}

#[derive(Default, Clone, Debug)]
struct InputState {
    move_up: bool,
    move_down: bool,
    move_left: bool,
    move_right: bool,
}

struct Game {
    world: ecs::World,
    player: ecs::EntityHandle,
}

impl Default for Game {
    fn default() -> Self {
        let mut world = ecs::World::default();
        let player = world.new_entity();
        Game { world, player }
    }
}

const PLAYER_MOVE_SPEED: f32 = 4.0;

impl Game {
    fn update(&mut self, elapsed: Duration) {
        let mut position = match self.world.component::<Position>(self.player) {
            Some(position) => position.clone(),
            None => Position::default(),
        };
        let input = self.world.component::<InputState>(self.player).unwrap();

        position.pos += Vec2::new(
            (if input.move_left { -1.0 } else { 0.0 }) +
                (if input.move_right { 1.0 } else { 0.0 }),
            (if input.move_down { -1.0 } else { 0.0 }) +
                (if input.move_up { 1.0 } else { 0.0 }),
        ) * PLAYER_MOVE_SPEED * elapsed.as_secs_f32();

        self.world.attach(self.player, position);
    }

    fn render(&self) -> Vec<render::RenderPass> {
        /*let mut primary_pass = frame.begin_render_pass();

        frame.submit_render_pass(primary_pass);*/
        vec!()
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut render_api = render::Renderer::new(&window);
    let size = window.inner_size();
    render_api.configure_surface(size.width, size.height);

    let mut game = Game::default();

    let mut last_update: Option<SystemTime> = None;

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        match event {
            Event::WindowEvent {
                event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => control_flow.set_exit(),
                WindowEvent::Resized(size) => render_api.configure_surface(size.width, size.height),
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        let active = input.state == Pressed;
                        let mut input = game.world.component::<InputState>(game.player).unwrap().clone();
                        match keycode {
                            VirtualKeyCode::Up => input.move_up = active,
                            VirtualKeyCode::Down => input.move_down = active,
                            VirtualKeyCode::Left => input.move_left = active,
                            VirtualKeyCode::Right => input.move_right = active,
                            _ => (),
                        }
                        game.world.attach(game.player, input);
                    }
                }
                _ => (),
            }
            Event::MainEventsCleared => {
                let since_last_update = last_update.map_or_else(
                    || Duration::default(),
                    |it| it.elapsed().unwrap(),
                );
                last_update = Some(SystemTime::now());
                game.update(since_last_update)
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let mut frame = render_api.begin_frame().unwrap();
                game.render();
                render_api.submit_frame(frame);
            }
            _ => (),
        }
    });
}
