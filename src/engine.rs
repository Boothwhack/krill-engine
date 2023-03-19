use crate::events::{Event, EventBusDrain};
use crate::render::RenderPass;

pub trait Game {
    fn update(&mut self, events: EventBusDrain<Event>);

    fn render(&self) -> Vec<RenderPass>;
}
