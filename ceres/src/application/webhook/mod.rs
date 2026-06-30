//! Webhook delivery facade (orchestration entry point for CL lifecycle events).

use callisto::mega_cl;
use common::errors::MegaError;
pub use jupiter::service::webhook_service::{
    ClPayload, RepositoryPayload, WebhookEvent, WebhookPayload,
};
use jupiter::{service::webhook_service::WebhookService, storage::Storage};

/// Dispatches CL webhook events asynchronously.
#[derive(Clone)]
pub struct WebhookDispatcher {
    service: WebhookService,
}

impl WebhookDispatcher {
    pub fn from_storage(storage: &Storage) -> Result<Self, MegaError> {
        Ok(Self {
            service: storage.webhook_service.clone(),
        })
    }

    pub fn dispatch(&self, event_type: WebhookEvent, cl_model: &mega_cl::Model) {
        self.service.dispatch(event_type, cl_model);
    }
}

pub fn dispatch_cl_webhook(storage: &Storage, event_type: WebhookEvent, cl_model: &mega_cl::Model) {
    storage.webhook_service.dispatch(event_type, cl_model);
}
