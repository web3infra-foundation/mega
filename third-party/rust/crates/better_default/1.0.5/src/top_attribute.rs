use std::{
    collections::{hash_map, HashMap},
    fmt::Display,
};

use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Expr, Ident, LitInt, Token};

use crate::{Span2, TokenStream2};

enum FieldName {
    Ident(Ident),
    IntLiteral(LitInt),
}

impl FieldName {
    fn span(&self) -> Span2 {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::IntLiteral(int_literal) => int_literal.span(),
        }
    }
}

impl Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::Ident(ident) => ident.to_string(),
            Self::IntLiteral(int_literal) => int_literal.to_string(),
        };
        f.write_str(str.as_str())
    }
}

impl Parse for FieldName {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Ident) {
            Ok(Self::Ident(input.parse()?))
        } else {
            Ok(Self::IntLiteral(input.parse()?))
        }
    }
}

struct FieldAssign {
    ident: FieldName,
    _colon: Token![:],
    value: Expr,
}

impl Parse for FieldAssign {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            _colon: input.parse()?,
            value: input.parse()?,
        })
    }
}

fn parse_punctuated_unique(
    punctuated: Punctuated<FieldAssign, syn::token::Comma>,
    field_names: &[String],
    error_tokens: &mut Vec<TokenStream2>,
) -> HashMap<String, Expr> {
    let mut hash_map = HashMap::with_capacity(punctuated.len());
    for field in punctuated {
        let ident_str = field.ident.to_string();

        if !field_names.contains(&ident_str) {
            error!(
                error_tokens,
                field.ident.span(),
                "unknown field `{}`",
                ident_str
            );
            continue;
        }

        if let hash_map::Entry::Vacant(e) = hash_map.entry(ident_str) {
            e.insert(field.value);
        } else {
            error!(
                error_tokens,
                field.ident.span(),
                "this field is already declared."
            );
            continue;
        }
    }

    hash_map.shrink_to_fit();
    hash_map
}

pub fn get_default_values(
    attr: &Attribute,
    field_names: &[String],
    require_list: bool,
    error_tokens: &mut Vec<TokenStream2>,
) -> Option<HashMap<String, Expr>> {
    let list = if require_list {
        handle_error!(attr.meta.require_list(), error_tokens)?
    } else {
        match &attr.meta {
            syn::Meta::Path(_) => return None,
            syn::Meta::List(list) => list,
            syn::Meta::NameValue(nv) => {
                let ident = attr.path().get_ident().unwrap();
                error!(
                    error_tokens,
                    nv.span(),
                    "expected attribute arguments in parentheses (`{ident}(...)`) or single `{ident}`"
                );

                return None;
            }
        }
    };

    let punctuated: Punctuated<FieldAssign, Token![,]> = handle_error!(
        list.parse_args_with(Punctuated::parse_separated_nonempty),
        error_tokens
    )?;

    let hash_map = parse_punctuated_unique(punctuated, field_names, error_tokens);
    Some(hash_map)
}
