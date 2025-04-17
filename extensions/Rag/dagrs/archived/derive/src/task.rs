use proc_macro2::TokenStream;
use syn::{
    Data, DeriveInput, Expr, ExprLit, Field, Fields, GenericArgument, Ident, Lit, MetaNameValue,
    PathArguments, Type,
};

const ID: &str = "id";
const NAME: &str = "name";
const PRECURSORS: &str = "precursors";
const ACTION: &str = "action";

#[allow(unused)]
pub(crate) fn parse_task(input: &DeriveInput) -> TokenStream {
    let struct_ident = &input.ident;
    let fields = match input.data {
        Data::Struct(ref str) => &str.fields,
        _ => {
            return syn::Error::new_spanned(
                struct_ident,
                "Task macros can only be annotated on struct.",
            )
            .into_compile_error();
        }
    };
    let attr_token = generate_field_function(fields);
    if let Err(e) = attr_token {
        return e.into_compile_error();
    }
    generate_impl(struct_ident, attr_token.unwrap())
}

fn generate_field_function(fields: &Fields) -> syn::Result<proc_macro2::TokenStream> {
    let mut token = proc_macro2::TokenStream::new();
    for field in fields.iter() {
        for attr in field.attrs.iter() {
            if attr.path().is_ident("task") {
                let kv: MetaNameValue = attr.parse_args()?;
                if kv.path.is_ident("attr") {
                    let err_msg = format!(
                        "The optional value of attr is [{},{},{},{}]",
                        ID, NAME, PRECURSORS, ACTION
                    );
                    let err = Err(syn::Error::new_spanned(&kv.value, err_msg));
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) = kv.value
                    {
                        let tk = match lit.value().as_str() {
                            ID => validate_id(field),
                            NAME => validate_name(field),
                            PRECURSORS => validate_precursors(field),
                            ACTION => validate_action(field),
                            _ => return err,
                        }?;
                        token.extend(tk);
                    } else {
                        return err;
                    }
                } else {
                    return Err(syn::Error::new_spanned(kv, "expect `task(attr = \"...\")`"));
                }
            }
        }
    }
    Ok(token)
}

fn validate_id(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &field.ident;
    let err = Err(syn::Error::new_spanned(
        &field.ty,
        "The type of `id` should be `usize`",
    ));
    if let Type::Path(ref str) = field.ty {
        let ty = str.path.segments.last().unwrap();
        if ty.ident.eq("usize") {
            Ok(quote::quote!(
                fn id(&self) -> usize {
                    self.#ident
                }
            ))
        } else {
            err
        }
    } else {
        err
    }
}

fn validate_name(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &field.ident;
    let err = Err(syn::Error::new_spanned(
        &field.ty,
        "The type of `name` should be `String`",
    ));
    if let Type::Path(ref str) = field.ty {
        let ty = str.path.segments.last().unwrap();
        if ty.ident.eq("String") {
            Ok(quote::quote!(
                fn name(&self) -> &str {
                    &self.#ident
                }
            ))
        } else {
            err
        }
    } else {
        err
    }
}

fn validate_precursors(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &field.ident;
    let err = Err(syn::Error::new_spanned(
        &field.ty,
        "The type of `id` should be `Vec<usize>`",
    ));
    if let Type::Path(ref str) = field.ty {
        let ty = str.path.segments.last().unwrap();
        if ty.ident.eq("Vec") {
            match ty.arguments {
                PathArguments::AngleBracketed(ref inner) => {
                    if let GenericArgument::Type(Type::Path(inner_ty)) = inner.args.last().unwrap()
                    {
                        if inner_ty.path.is_ident("usize") {
                            Ok(quote::quote!(
                                fn precursors(&self) -> &[usize] {
                                    &self.#ident
                                }
                            ))
                        } else {
                            err
                        }
                    } else {
                        err
                    }
                }
                _ => err,
            }
        } else {
            err
        }
    } else {
        err
    }
}

fn validate_action(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &field.ident;
    let err = Err(syn::Error::new_spanned(
        &field.ty,
        "The type of `id` should be `Action`",
    ));
    if let Type::Path(ref str) = field.ty {
        let ty = str.path.segments.last().unwrap();
        if ty.ident.eq("Action") {
            Ok(quote::quote!(
                fn action(&self) -> Action {
                    self.#ident.clone()
                }
            ))
        } else {
            err
        }
    } else {
        err
    }
}

fn generate_impl(struct_ident: &Ident, fields_function: proc_macro2::TokenStream) -> TokenStream {
    quote::quote!(
        impl dagrs::Task for #struct_ident{
            #fields_function
        }
        unsafe impl Send for #struct_ident{}
        unsafe impl Sync for #struct_ident{}
    )
}
