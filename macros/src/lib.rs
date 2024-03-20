use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod utils;

mod use_case_error;
use use_case_error::use_case_error_impl;

/// UseCaseError導出マクロ
#[proc_macro_derive(UseCaseError, attributes(use_case_error))]
pub fn derive_use_case_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match use_case_error_impl(input) {
        Ok(token_stream) => TokenStream::from(token_stream),
        Err(err) => TokenStream::from(err.into_compile_error()),
    }
}
