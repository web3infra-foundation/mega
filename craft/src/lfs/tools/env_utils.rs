use crate::lfs::errors::install_error::ENVINSTALLError;
#[cfg(target_os = "windows")]
pub mod env_utils {
    use winreg::{RegKey, enums::*};
    use crate::lfs::errors::install_error::ENVINSTALLError;
    use crate::lfs::tools::constant_table::env_utils_table;
    pub struct Environment {
        key: RegKey,
    }

    impl Environment {
        pub fn new() -> Result<Self, ENVINSTALLError> {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let env_key = hkcu.open_subkey_with_flags(
                env_utils_table::ENVIRONMENTCharacters::get(
                    env_utils_table::ENVIRONMENTEnum::ENVIRONMENT
                )
                , KEY_READ | KEY_WRITE)?;
            Ok(Environment { key: env_key })
        }

        pub fn get_variable(&self, name: &str) -> Result<String, ENVINSTALLError> {
            Ok(self.key.get_value(name)?)
        }

        pub fn set_variable(&self, name: &str, value: &str) -> Result<(), ENVINSTALLError> {
            self.key.set_value(name, &value)?;
            Ok(())
        }
    }
}