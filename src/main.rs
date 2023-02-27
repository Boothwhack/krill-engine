mod render;

use std::time::{Duration, SystemTime};
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event::ElementState::Pressed;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

type Vec2 = nalgebra::Vector2<f32>;

struct Player {
    position: Vec2,
}

#[derive(Default)]
struct InputState {
    move_up: bool,
    move_down: bool,
    move_left: bool,
    move_right: bool,
}

struct Game {
    player: Player,
    input: InputState,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            player: Player {
                position: Vec2::new(0.0, 0.0)
            },
            input: InputState::default(),
        }
    }
}

const PLAYER_MOVE_SPEED: f32 = 4.0;

impl Game {
    fn update(&mut self, elapsed: Duration) {
        self.player.position += Vec2::new(
            (if self.input.move_left { -1.0 } else { 0.0 }) +
                (if self.input.move_right { 1.0 } else { 0.0 }),
            (if self.input.move_down { -1.0 } else { 0.0 }) +
                (if self.input.move_up { 1.0 } else { 0.0 }),
        ) * PLAYER_MOVE_SPEED * elapsed.as_secs_f32()
    }
    fn render(&self, frame: &mut render::Frame) {
        let mut primary_pass = frame.begin_render_pass();

        frame.submit_render_pass(primary_pass);
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut render_api = render::RenderApi::new(&window);
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
                        match keycode {
                            VirtualKeyCode::Up => game.input.move_up = active,
                            VirtualKeyCode::Down => game.input.move_down = active,
                            VirtualKeyCode::Left => game.input.move_left = active,
                            VirtualKeyCode::Right => game.input.move_right = active,
                            _ => (),
                        }
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
                game.render(&mut frame);
                render_api.submit_frame(frame);
            }
            _ => (),
        }
    });
}
