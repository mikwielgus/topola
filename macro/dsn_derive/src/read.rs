use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Field};
use syn::Type::Path;
use syn::ext::IdentExt;

use crate::attr_present;
use crate::attr_content;

pub fn impl_read(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let body = impl_body(&input.data);

    quote! {
        impl<R: std::io::BufRead> ReadDsn<R> for #name {
            fn read_dsn(tokenizer: &mut ListTokenizer<R>)
                 -> Result<Self, ParseError>
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
                        Ok(Self {
                            #(#fields)*
                        })
                    }
                }
                _ => unimplemented!()
            }
        }
        Data::Enum(_data) => {
            todo!();
        }
        _ => unimplemented!()
    }
}

fn impl_field(field: &Field) -> TokenStream {
    let name = &field.ident;
    let name_str = name.as_ref().expect("field name").unraw();

    if attr_present(&field.attrs, "anon") {
        quote! {
            #name: tokenizer.read_value()?,
        }
    } else if let Some(dsn_name) = attr_content(&field.attrs, "vec") {
        quote! {
            #name: tokenizer.read_named_array(#dsn_name)?,
        }
    } else if attr_present(&field.attrs, "anon_vec") {
        quote! {
            #name: tokenizer.read_array()?,
        }
    } else {
        if let Path(type_path) = &field.ty {
            let segments = &type_path.path.segments;
            if segments.len() == 1 {
                let ident = &segments.first().unwrap().ident;
                if ident == "Option" {
                    return quote! {
                        #name: tokenizer.read_optional(stringify!(#name_str))?,
                    }
                }
            }
        }

        quote! {
            #name: tokenizer.read_named(stringify!(#name_str))?,
        }
    }
}

