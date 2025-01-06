// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Field, Fields};

pub(super) fn impl_derive_actions(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let new_body = impl_new_body(&input.data);

    quote! {
        impl #name {
            pub fn new(tr: &Translator) -> Self {
                #new_body
            }
        }
    }
}

pub(super) fn impl_new_body(data: &Data) -> TokenStream {
    match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let fields = fields.named.iter().map(impl_new_field);

                quote! {
                    Self {
                        #(#fields)*
                    }
                }
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

pub(super) fn impl_new_field(field: &Field) -> TokenStream {
    let name = &field.ident;
    let typ = &field.ty;

    quote! {
        #name: #typ::new(tr),
    }
}
