use std::any::Any;

pub type Message = Box<dyn EventBase>;

pub trait EventBase: Send + Sync + Any + std::fmt::Display {
    // async fn process(&self);
}

// impl std::fmt::Display for Event {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Event::Api(evt) => write!(f, "{}", evt),

//             #[allow(unreachable_patterns)]
//             _ => write!(f, "Unknown Event Type")
//         }
//     }
// }
