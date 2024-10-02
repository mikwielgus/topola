use proc_macro::TokenStream;
use syn::{Attribute, DeriveInput, LitStr, Meta, Token};
use syn::punctuated::Punctuated;

mod read;
mod write;

#[proc_macro_derive(ReadDsn, attributes(anon, vec, anon_vec))]
pub fn derive_read(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    read::impl_read(&input).into()
}

#[proc_macro_derive(WriteSes, attributes(anon, vec, anon_vec))]
pub fn derive_write(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    write::impl_write(&input).into()
}

enum FieldType {
    Anonymous,
    AnonymousVec,
    NamedVec(Vec<LitStr>),
    NotSpecified,
}

fn parse_attributes(attrs: &Vec<Attribute>) -> FieldType {
    for attr in attrs {
        match &attr.meta {
            Meta::Path(path) => {
                if path.is_ident("anon") {
                    return FieldType::Anonymous;
                } else if path.is_ident("anon_vec") {
                    return FieldType::AnonymousVec;
                }
            },
            Meta::List(list) if list.path.is_ident("vec") => {
                return FieldType::NamedVec(list
                    .parse_args_with(
                        Punctuated::<LitStr, Token![,]>::parse_terminated
                    )
                    .expect("#[vec(...)] must contain a list of string literals")
                    .iter()
                    .cloned()
                    .collect()
                );
            },
            _ => (),
        }
    }

    FieldType::NotSpecified
}
