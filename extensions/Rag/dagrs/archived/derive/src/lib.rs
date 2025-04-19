use proc_macro::TokenStream;

extern crate proc_macro2;
extern crate quote;
extern crate syn;

#[cfg(feature = "derive")]
mod relay;
#[cfg(feature = "derive")]
mod task;

/// [`CustomTask`] is a derived macro that may be used when customizing tasks. It can only be
/// marked on the structure, and the user needs to specify four attributes of the custom task
/// type, which are task(attr="id"), task(attr = "name"), task(attr = "precursors ") and
/// task(attr = "action"), which are used in the `derive_task` example.
#[cfg(feature = "derive")]
#[proc_macro_derive(CustomTask, attributes(task))]
pub fn derive_task(input: TokenStream) -> TokenStream {
    use crate::task::parse_task;
    use syn::{parse_macro_input, DeriveInput};
    let input = parse_macro_input!(input as DeriveInput);
    parse_task(&input).into()
}

/// The [`dependencies!`] macro allows users to specify all task dependencies in an easy-to-understand
/// way. It will return to the user a series of `DefaultTask` in the order of tasks given by the user.
#[cfg(feature = "derive")]
#[proc_macro]
pub fn dependencies(input: TokenStream) -> TokenStream {
    use crate::relay::generate_task;
    use relay::Tasks;
    let tasks = syn::parse_macro_input!(input as Tasks);
    let relies = tasks.resolve_dependencies();
    if let Err(err) = relies {
        return err.into_compile_error().into();
    }
    let token = generate_task(relies.unwrap());
    token.into()
}
