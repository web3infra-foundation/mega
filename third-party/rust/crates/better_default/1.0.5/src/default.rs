use std::collections::HashMap;

use quote::{quote, ToTokens};
use syn::{spanned::Spanned, Expr, Fields, Ident, Token};

use crate::{attrs, constants::{self, DefaultTraitPath}, traits::JoinTokens, Span2, TokenStream2};

pub struct DefaultValue {
    ident: Option<Ident>,
    value: TokenStream2,
}

impl ToTokens for DefaultValue {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        if let Some(ident) = &self.ident {
            ident.to_tokens(tokens);
            Token![:](Span2::call_site()).to_tokens(tokens);
        }

        self.value.to_tokens(tokens);
    }
}

fn get_field_default_values(
    top_default_values: Option<&HashMap<String, Expr>>,
    fields: &Fields,
    error_tokens: &mut Vec<TokenStream2>,
) -> Vec<DefaultValue> {
    let mut default_values_vec = Vec::with_capacity(fields.len());
    for (i, field) in fields.iter().enumerate() {
        let ident = field.ident.clone();
        let ident_str = ident
            .as_ref()
            .map_or_else(|| i.to_string(), ToString::to_string);

        let ty = &field.ty;

        let default_tokens = attrs::find_attribute_unique(
            &field.attrs,
            constants::DEFAULT_IDENT,
            error_tokens,
        )
            .and_then(|attr| handle_error!(attr.meta.require_list(), error_tokens));

        let top_default_tokens = top_default_values
            .and_then(|h| h.get(&ident_str))
            .map(ToTokens::to_token_stream);

        if let Some(meta) = default_tokens {
            if top_default_tokens.is_some() {
                error!(
                    error_tokens,
                    meta.path.span(),
                    "a default value for this field already exists in the top default attribute."
                );
            }
        }

        let default_tokens = default_tokens.and_then(|meta| handle_error!(meta.parse_args::<Expr>(), error_tokens));

        let default_tokens = default_tokens
            .map(ToTokens::into_token_stream)
            .or(top_default_tokens)
            .unwrap_or(quote! { <#ty as #DefaultTraitPath>::default() });

        let default_value = DefaultValue {
            ident,
            value: default_tokens,
        };
        default_values_vec.push(default_value);
    }

    default_values_vec
}

pub fn derive_body(
    top_default_values: Option<&HashMap<String, Expr>>,
    fields: &Fields,
    error_tokens: &mut Vec<TokenStream2>,
) -> TokenStream2 {
    let delimiter = match fields {
        Fields::Named(_) => proc_macro2::Delimiter::Brace,
        Fields::Unnamed(_) => proc_macro2::Delimiter::Parenthesis,
        Fields::Unit => return TokenStream2::new(),
    };

    let default_value_vec = get_field_default_values(top_default_values, fields, error_tokens);

    let flattened_tokens = default_value_vec.join_tokens(&Token![,](Span2::call_site()));
    proc_macro2::Group::new(delimiter, flattened_tokens).into_token_stream()
}
