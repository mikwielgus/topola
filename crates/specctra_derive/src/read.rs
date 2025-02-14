// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::Type::Path;
use syn::{Data, DeriveInput, Field, Fields, Variant};

use crate::parse_attributes;
use crate::FieldType;

pub fn impl_read(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let body = impl_body(&input.data);

    quote! {
        impl<R: std::io::BufRead> ReadDsn<R> for #name {
            fn read_dsn(tokenizer: &mut ListTokenizer<R>)
                 -> Result<Self, ParseErrorContext>
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
                    Ok(Self {
                        #(#fields)*
                    })
                }
            }
            _ => unimplemented!(),
        },
        Data::Enum(data) => {
            let (variantnames, variants): (TokenStream, TokenStream) =
                data.variants.iter().map(impl_variant).unzip();
            quote! {
                let ctx = tokenizer.context();
                let name = tokenizer.consume_token()?.expect_any_start()?;
                let value = Ok(match name.as_str() {
                    #variants
                    _ => return Err(ParseError::ExpectedStartOfListOneOf(&[#variantnames]).add_context(ctx)),
                });
                tokenizer.consume_token()?.expect_end()?;
                value
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
                #name: tokenizer.read_value()?,
            }
        }
        FieldType::AnonymousVec => {
            quote! {
                #name: tokenizer.read_array()?,
            }
        }
        FieldType::NamedVec(valid_aliases) => {
            quote! {
                #name: tokenizer.read_named_array(&[#(#valid_aliases),*])?,
            }
        }
        FieldType::NotSpecified => {
            if let Path(type_path) = &field.ty {
                let segments = &type_path.path.segments;
                if segments.len() == 1 {
                    let ident = &segments.first().unwrap().ident;
                    if ident == "Option" {
                        return quote! {
                            #name: tokenizer
                                .read_optional(&[stringify!(#name_str)])?,
                        };
                    }
                }
            }

            quote! {
                #name: tokenizer.read_named(&[stringify!(#name_str)])?,
            }
        }
    }
}

fn impl_variant(variant: &Variant) -> (TokenStream, TokenStream) {
    let name = &variant.ident;
    let mut name_str = name.unraw().to_string();
    name_str.make_ascii_lowercase();

    let inner = match &variant.fields {
        Fields::Unnamed(fields) => {
            let all_parts =
                core::iter::repeat(quote! { tokenizer.read_value()?, }).take(fields.unnamed.len());
            quote! { Self::#name(#(#all_parts)*) }
        }
        Fields::Named(fields) => {
            let fields = fields.named.iter().map(impl_field);

            quote! {
                Self::#name {
                    #(#fields)*
                }
            }
        }
        Fields::Unit => unimplemented!(),
    };
    (
        quote! {
            #name_str,
        },
        quote! {
            #name_str => #inner,
        },
    )
}
