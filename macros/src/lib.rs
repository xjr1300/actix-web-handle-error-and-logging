use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod response_error;
use response_error::response_error_impl;

/// ResponseError導出マクロ
#[proc_macro_derive(ResponseErrorImpl, attributes(response_error))]
pub fn derive_response_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match response_error_impl(input) {
        Ok(token_stream) => TokenStream::from(token_stream),
        Err(err) => TokenStream::from(err.into_compile_error()),
    }
}
