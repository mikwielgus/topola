use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Field};
use syn::Type::Path;
use syn::ext::IdentExt;

use crate::attr_present;
use crate::attr_content;

pub fn impl_write(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;

    let body = impl_body(&input.data);

    quote! {
        impl<W: std::io::Write> WriteDsn<W> for #name {
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
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => {
                    let fields = fields.named.iter().map(|field| {
                        impl_field(field)
                    });

                    quote! {
                        #(#fields)* 

                        Ok(())
                    }
                }
                _ => unimplemented!()
            }
        }
        _ => unimplemented!()
    }
}

fn impl_field(field: &Field) -> TokenStream {
    let name = &field.ident;
    let name_str = name.as_ref().expect("field name").unraw();

    if attr_present(&field.attrs, "anon") {
        quote! {
            writer.write_value(&self.#name)?;
        }
    } else if let Some(dsn_name) = attr_content(&field.attrs, "vec") {
        quote! {
            writer.write_named_array(#dsn_name, &self.#name)?;
        }
    } else if attr_present(&field.attrs, "anon_vec") {
        quote! {
            writer.write_array(&self.#name)?;
        }

    } else {
        if let Path(type_path) = &field.ty {
            let segments = &type_path.path.segments;
            if segments.len() == 1 {
                let ident = &segments.first().unwrap().ident;
                if ident == "Option" {
                    return quote! {
                        writer.write_optional(stringify!(#name_str), &self.#name)?;
                    }
                }
            }
        }

        quote! {
            writer.write_named(stringify!(#name_str), &self.#name)?;
        }
    }
}


