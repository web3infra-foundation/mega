use quote::quote;
use syn::{
    spanned::Spanned, Attribute, DataEnum, DataStruct, DeriveInput,
    Fields,
};

use crate::{
    attrs, default,
    top_attribute,
    Span2, TokenStream2, constants::{self, DefaultTraitPath}
};

fn search_and_mark_default_attribute_on_fields(
    fields: &Fields,
    error_tokens: &mut Vec<TokenStream2>,
) {
    for field in fields {
        if let Some(attribute) = attrs::find_attribute_unique(
            &field.attrs,
            constants::DEFAULT_IDENT,
            error_tokens,
        ) {
            error!(
                error_tokens,
                attribute.meta.span(),
                "You can't use the default attribute on variant fields if the variant is not declared as default."
            );
        }
    }
}

fn get_fields_name(fields: &Fields) -> Vec<String> {
    match fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|i| i.ident.as_ref().unwrap().to_string())
            .collect(),
        Fields::Unnamed(_) => (0..fields.len()).map(|i| i.to_string()).collect(),
        Fields::Unit => Vec::new(),
    }
}

fn derive_struct(
    top_attribute: Option<&Attribute>,
    data: &DataStruct,
    error_tokens: &mut Vec<TokenStream2>,
) -> TokenStream2 {
    let field_names = get_fields_name(&data.fields);
    let top_attribute =
        top_attribute.and_then(|attr| top_attribute::get_default_values(attr, &field_names, true, error_tokens));

    let body_tokens = default::derive_body(top_attribute.as_ref(), &data.fields, error_tokens);

    quote! { Self #body_tokens }
}

fn derive_enum(
    top_attribute: Option<&Attribute>,
    data: &DataEnum,
    error_tokens: &mut Vec<TokenStream2>,
) -> TokenStream2 {
    if let Some(attr) = top_attribute {
        error!(
            error_tokens,
            attr.meta.span(),
            "top default attributes are not allowed on enums."
        );
    }

    let mut default_variant = None;
    for variant in &data.variants {
        let Some(attr) = attrs::find_attribute_unique(
            &variant.attrs,
            constants::DEFAULT_IDENT,
            error_tokens,
        ) else {
            search_and_mark_default_attribute_on_fields(&variant.fields, error_tokens);

            continue;
        };

        if let Some((ident, _)) = default_variant.as_ref() {
            error!(
                error_tokens,
                attr.meta.span(),
                "the default value is already assigned to `{}`",
                ident
            );

            continue;
        }

        let field_names = get_fields_name(&variant.fields);
        let top_attribute = top_attribute::get_default_values(attr, &field_names, false, error_tokens);

        let headless_default_tokens =
            default::derive_body(top_attribute.as_ref(), &variant.fields, error_tokens);
        let ident = variant.ident.clone();
        // FIXME: for some reason the "value holding a reference to a value owned by the current function"
        //  error has the Span::call_site() span, and idk why.
        let default_tokens: TokenStream2 = quote! { Self::#ident #headless_default_tokens };

        default_variant = Some((ident, default_tokens));
    }

    if let Some((_, tokens)) = default_variant {
        tokens
    } else {
        error!(
            error_tokens,
            Span2::call_site(),
            "the default variant has not been set."
        );

        quote! { panic!() }
    }
}

pub fn derive(input: &DeriveInput) -> TokenStream2 {
    let mut error_tokens = Vec::new();

    let top_attribute = attrs::find_attribute_unique(
        &input.attrs,
        constants::DEFAULT_IDENT,
        &mut error_tokens,
    );

    let tokens = match &input.data {
        syn::Data::Struct(data) => derive_struct(top_attribute, data, &mut error_tokens),
        syn::Data::Enum(data) => derive_enum(top_attribute, data, &mut error_tokens),
        syn::Data::Union(data) => {
            return error!(
                data.union_token.span(),
                "this derive is not implemented for unions."
            )
            .into_compile_error();
        }
    };

    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let error_tokens: TokenStream2 = error_tokens.into_iter().collect();

    quote! {
        impl #impl_generics #DefaultTraitPath for #ident #type_generics #where_clause {
            fn default() -> Self {
                #tokens
            }
        }

        #error_tokens
    }
}
