use proc_macro::TokenStream;
use syn::{Attribute, DeriveInput, LitStr};

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

fn attr_present(attrs: &Vec<Attribute>, name: &str) -> bool {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident(name))
        .is_some()
}

fn attr_content(attrs: &Vec<Attribute>, name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident(name))
        .and_then(|attr| Some(attr
            .parse_args::<LitStr>()
            .expect("string literal")
            .value()
        ))
}
