/// Basic event trait. A [Event] gets passed into the event system, and 
/// passed to the first handler. Handlers can delegate the event further up 
/// the chain, or return an output immedietly. 
pub trait Event {
    type Output;
}
