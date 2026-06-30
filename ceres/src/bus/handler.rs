use async_trait::async_trait;
use common::errors::MegaError;

use super::event::TransportEvent;

#[async_trait]
pub trait ApplicationEventHandler: Send + Sync {
    async fn handle(&self, event: TransportEvent) -> Result<(), MegaError>;
}
