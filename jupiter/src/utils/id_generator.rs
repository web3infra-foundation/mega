use std::sync::Once;

use idgenerator::*;

static ID_GENERATOR_INIT: Once = Once::new();

/// Ensures [`IdInstance`] is configured (idempotent; safe if [`set_up_options`] already ran, e.g. via [`crate::storage::init::database_connection`]).
pub fn ensure_initialized() {
    ID_GENERATOR_INIT.call_once(|| {
        if let Err(e) = set_up_options() {
            tracing::debug!(
                ?e,
                "id_generator::set_up_options (ignored if already initialized)"
            );
        }
    });
}

pub fn set_up_options() -> Result<(), OptionError> {
    // Setup the option for the id generator instance.
    let options = IdGeneratorOptions::new().worker_id(1).worker_id_bit_len(6);

    // Initialize the id generator instance with the option.
    // Other options not set will be given the default value.
    IdInstance::init(options)?;

    // Get the option from the id generator instance.
    let options = IdInstance::get_options();
    tracing::info!("First setting: {:?}", options);
    Ok(())
}
