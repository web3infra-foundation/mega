use anyhow::{Result, anyhow};

pub fn default_config_template(bin_name: &str) -> Result<&'static str> {
    match bin_name {
        "mono" => Ok(include_str!("../../../config/config.toml")),
        "orion-server" => Ok(include_str!("../../../config/config.toml")),
        _ => Err(anyhow!("unknown binary `{}`", bin_name)),
    }
}
