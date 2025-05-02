use quote::quote;

pub const DEFAULT_IDENT: &str = "default";

macro_rules! create_const_tokens {
    ($ident: ident = $($tt: tt)*) => {
        pub struct $ident;
        impl quote::ToTokens for $ident {
            fn to_tokens(&self, tokens: &mut crate::TokenStream2) {
                quote! { $($tt)* }.to_tokens(tokens)
            }
        }
    }
}

create_const_tokens!(DefaultTraitPath = core::default::Default);
