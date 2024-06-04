use idgenerator::*;

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
