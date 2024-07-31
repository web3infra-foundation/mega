use std::sync::Arc;

use serde::de::DeserializeOwned;
use async_trait::async_trait;

use crate::api::ApiServiceState;

pub(crate) type Message = Arc<Box<dyn Event<Type = EventType>>>;

pub(crate) enum EventType {
    ApiRequestEvent,

}

#[async_trait]
pub trait Event: Send + Sync {
    type Type: Into<EventType>;
    fn event_type(&self) -> Self::Type;

    async fn process(&self);
    async fn done(&self);
}

// A common event stores how to perform a async action.

pub(crate) struct ApiRequestEvent<T>
    where T: DeserializeOwned
{
    state: ApiServiceState,
    handler: fn() -> T,
}

impl<T> ApiRequestEvent<T>
    where T: DeserializeOwned
{
    fn new(state: ApiServiceState, handler: fn() -> T) -> Arc<Self> {
        Arc::new(ApiRequestEvent {
            state,
            handler
        })
    }
}

#[async_trait]
impl<T> Event for ApiRequestEvent<T>
    where T: DeserializeOwned
{
    type Type = EventType;
    fn event_type(&self) -> Self::Type { EventType::ApiRequestEvent }

    async fn process(&self) {
        (self.handler)();
    }

    async fn done(&self) {
        todo!()
    }
}
