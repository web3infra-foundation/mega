use crate::TokenStream2;
use syn::{spanned::Spanned, Attribute};

fn find_attribute_and_duplicates<'a>(
    attrs: &'a [Attribute],
    ident: &str,
) -> Option<(&'a Attribute, Vec<&'a Attribute>)> {
    let mut iter = attrs
        .iter()
        .filter(|attr| attr.path().get_ident().is_some_and(|i| i == ident));

    let first = iter.next()?;

    let vec = iter.collect();

    Some((first, vec))
}

pub fn find_attribute_unique<'l>(
    attrs: &'l [Attribute],
    ident: &str,
    error_tokens: &mut Vec<TokenStream2>,
) -> Option<&'l syn::Attribute> {
    let (attr, duplicates) = find_attribute_and_duplicates(attrs, ident)?;

    for duplicate in duplicates {
        error!(
            error_tokens,
            duplicate.meta.span(),
            "this attribute is already declared."
        );
    }

    Some(attr)
}
