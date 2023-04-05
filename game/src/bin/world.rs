use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::Window;
use engine::ecs::world::World;
use engine::resource::{ResourceList, Resources};

#[derive(Default)]
struct InputResource();

#[derive(Default)]
struct SystemResource {
    exit: bool,
}

impl SystemResource {
    fn exit(&mut self) {
        self.exit = true;
    }
}

enum ProcessEvent<'a> {
    Setup(&'a Window),
    Draw,
    KeyEvent,
    Close,
}

fn run_winit<R, F>(resources: R, mut handler: F) -> !
    where
        R: ResourceList + 'static,
        F: FnMut(ProcessEvent, &mut R::WithResource<SystemResource>) + 'static,
{
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop)
        .expect("window");

    let mut resources = resources.with_resource(SystemResource::default());

    handler(ProcessEvent::Setup(&window), &mut resources);

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {}
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    handler(ProcessEvent::Close, &mut resources);
                    let system: &SystemResource = resources.get();
                    if system.exit {
                        control_flow.set_exit();
                    }
                }
                _ => ()
            }
            _ => ()
        }
    })
}

struct RenderResource;

struct Position {
    x: f32,
    y: f32,
}

enum Shape {
    Square {
        width: f32,
        height: f32,
    },
    Triangle(f32),
}

fn main() {
    let resources = Resources::new(InputResource::default())
        .with_resource(World::default().with_component::<Position>().with_component::<Shape>());

    run_winit(resources, |event, resources| match event {
        ProcessEvent::Setup(_window) => {
            // platform.add_resource(RenderResource);
        }
        ProcessEvent::Draw => {
            let world: &World = resources.get();
            let positions = world.components::<Position>();
            let shapes = world.components::<Shape>();

            // filter with hlist
            for (_, shape, position) in world
                .entity_iter()
                .filter_map(|entity| shapes.get(entity).map(|shape| (entity, shape)))
                .filter_map(|(entity, shape)| positions.get(entity).map(|position| (entity, shape, position)))
            {
                // TODO: Draw...
            }
        }
        ProcessEvent::Close => {
            let system: &mut SystemResource = resources.get_mut();
            system.exit();
        }
        _ => ()
    });
}
