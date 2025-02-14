// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::ext::IdentExt;
use syn::Type::Path;
use syn::{punctuated::Punctuated, Data, DeriveInput, Field, Fields, Ident, Variant};

use crate::parse_attributes;
use crate::FieldType;

pub fn impl_write(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;

    let body = impl_body(&input.data);

    quote! {
        impl<W: std::io::Write> WriteSes<W> for #name {
            fn write_dsn(&self, writer: &mut ListWriter<W>)
                -> std::io::Result<()>
            {
                #body
            }
        }
    }
}

fn impl_body(data: &Data) -> TokenStream {
    match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let fields = fields.named.iter().map(impl_field);

                quote! {
                    #(#fields)*

                    Ok(())
                }
            }
            _ => unimplemented!(),
        },
        Data::Enum(data) => {
            let variants = data.variants.iter().map(impl_variant);

            quote! {
                match self {
                    #(#variants)*
                }

                Ok(())
            }
        }
        _ => unimplemented!(),
    }
}

fn impl_field(field: &Field) -> TokenStream {
    let name = &field.ident;
    let name_str = name.as_ref().expect("field name").unraw();
    let field_type = parse_attributes(&field.attrs);

    match field_type {
        FieldType::Anonymous => {
            quote! {
                writer.write_value(&self.#name)?;
            }
        }
        FieldType::AnonymousVec => {
            quote! {
                writer.write_array(&self.#name)?;
            }
        }
        FieldType::NamedVec(valid_aliases) => {
            let canonical_name = &valid_aliases[0];
            quote! {
                writer.write_named_array(#canonical_name, &self.#name)?;
            }
        }
        FieldType::NotSpecified => {
            if let Path(type_path) = &field.ty {
                let segments = &type_path.path.segments;
                if segments.len() == 1 {
                    let ident = &segments.first().unwrap().ident;
                    if ident == "Option" {
                        return quote! {
                            writer
                                .write_optional(stringify!(#name_str), &self.#name)?;
                        };
                    }
                }
            }

            quote! {
                writer.write_named(stringify!(#name_str), &self.#name)?;
            }
        }
    }
}

fn impl_variant(variant: &Variant) -> TokenStream {
    let name = &variant.ident;
    let mut name_str = name.unraw().to_string();
    name_str.make_ascii_lowercase();

    match &variant.fields {
        Fields::Unnamed(fields) => {
            let names: Vec<_> = (0..fields.unnamed.len())
                .map(|i| Ident::new(&format!("inner__{}", i), Span::mixed_site()))
                .collect();
            let mut select = Punctuated::<_, syn::Token![,]>::new();
            for i in &names {
                select.push(i.clone());
            }
            let fields = names.into_iter().map(|name| {
                quote! { writer.write_value(#name)?; }
            });
            quote! { Self::#name(#select) => {
                writer.write_token(ListToken::Start { name: #name_str.to_string() })?;
                #(#fields)*
                writer.write_token(ListToken::End)?;
            }, }
        }
        _ => unimplemented!(),
    }
}
