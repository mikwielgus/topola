use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::Type::Path;
use syn::{Data, DeriveInput, Field, Fields};

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
                let fields = fields.named.iter().map(|field| impl_field(field));

                quote! {
                    #(#fields)*

                    Ok(())
                }
            }
            _ => unimplemented!(),
        },
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
        },
        FieldType::AnonymousVec => {
            quote! {
                writer.write_array(&self.#name)?;
            }
        },
        FieldType::NamedVec(valid_aliases) => {
            let canonical_name = &valid_aliases[0];
            quote! {
                writer.write_named_array(#canonical_name, &self.#name)?;
            }
        },
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
