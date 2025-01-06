// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use proc_macro::TokenStream;
use syn::DeriveInput;

mod implementation;

#[proc_macro_derive(Actions)]
pub fn derive_actions(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    implementation::impl_derive_actions(&input).into()
}
