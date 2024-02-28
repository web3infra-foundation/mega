#[macro_export]
macro_rules! create_characters {
    ($enum_name:ident, $struct_name:ident { $($field_name:ident: $field_value:expr),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $struct_name;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $enum_name {
            $(
                $field_name,
            )*
        }

        impl $struct_name {
            $(
                 const $field_name: &'static str = $field_value;
            )*

            pub fn get(character: $enum_name) -> &'static str {
                match character {
                    $(
                        $enum_name::$field_name => Self::$field_name,
                    )*
                }
            }
        }
    };
}