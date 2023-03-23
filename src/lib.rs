use std::time::{Duration, SystemTime};

use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event::ElementState::Pressed;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

mod ecs;
mod engine;
mod events;
mod genvec;
pub mod render;
