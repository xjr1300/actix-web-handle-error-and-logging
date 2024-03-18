use actix_web::http::StatusCode;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Expr, Fields, Lit, MetaNameValue, Token, Visibility};

type CommaPunctuatedNameValues = Punctuated<MetaNameValue, Token![,]>;

struct VariantInfo<'a> {
    ident: &'a Ident,
    is_unit: bool,
    status_code: u16,
    error_code: u32,
}

pub(crate) fn response_error_impl(input: DeriveInput) -> syn::Result<TokenStream2> {
    let enum_ident = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let Data::Enum(data_enum) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "ResponseErrorImpl is expected enum",
        ));
    };

    // 列挙型のバリアントと`response_error`属性のフィールド値を取得
    let mut variant_infos: Vec<VariantInfo> = vec![];
    for variant in data_enum.variants.iter() {
        let attr = variant
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("response_error"));
        if attr.is_none() {
            return Err(syn::Error::new(
                variant.span(),
                "response_error attributes not found",
            ));
        }
        let field_values = retrieve_response_error_fields(attr.unwrap())?;
        let is_unit = matches!(variant.fields, Fields::Unit);
        variant_infos.push(VariantInfo {
            ident: &variant.ident,
            is_unit,
            status_code: field_values.status_code,
            error_code: field_values.error_code,
        });
    }

    let error_code_method = impl_error_code_method(vis, &variant_infos);
    let status_code_method = impl_status_code_method(&variant_infos)?;
    let error_response_method = impl_error_response_method();

    Ok(quote! {
        impl #impl_generics #enum_ident #ty_generics #where_clause {
            #error_code_method
        }

        impl #impl_generics actix_web::error::ResponseError for #enum_ident #ty_generics #where_clause {
            #status_code_method
            #error_response_method
        }

    })
}

fn impl_error_code_method(vis: &Visibility, variant_infos: &[VariantInfo]) -> TokenStream2 {
    let mut branches: Vec<TokenStream2> = vec![];
    for &VariantInfo {
        ident,
        is_unit,
        error_code,
        ..
    } in variant_infos
    {
        let token_stream = if is_unit {
            quote! { Self::#ident => #error_code, }
        } else {
            quote! { Self::#ident(..) => #error_code, }
        };
        branches.push(token_stream);
    }

    quote!(
        #vis fn error_code(&self) -> u32 {
            match *self {
                #(
                    #branches
                )*
            }
        }
    )
}

fn impl_status_code_method(variant_infos: &[VariantInfo]) -> syn::Result<TokenStream2> {
    let mut branches: Vec<TokenStream2> = vec![];
    for &VariantInfo {
        ident,
        is_unit,
        status_code,
        ..
    } in variant_infos
    {
        // `status_code`は、`StatusCode`に変換できることを確認済みであるため`unwrap()`
        let token_stream = if is_unit {
            quote! { Self::#ident => actix_web::http::StatusCode::from_u16(#status_code).unwrap(), }
        } else {
            quote! { Self::#ident(..) => actix_web::http::StatusCode::from_u16(#status_code).unwrap(), }
        };
        branches.push(token_stream);
    }

    Ok(quote!(
        fn status_code(&self) -> actix_web::http::StatusCode {
            match *self {
                #(
                    #branches
                )*
            }
        }
    ))
}

fn impl_error_response_method() -> TokenStream2 {
    quote!(
        fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
            let status_code = self.status_code();
            let error_code = self.error_code();
            let message = format!("{}", self);
            let body = ErrorResponseBody::new(status_code.as_u16(), Some(error_code), message);

            HttpResponseBuilder::new(status_code)
                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                .json(body)
        }
    )
}

struct ResponseErrorFields {
    status_code: u16,
    error_code: u32,
}

fn retrieve_response_error_fields(attr: &Attribute) -> syn::Result<ResponseErrorFields> {
    // 属性に記述されているカンマ区切りのフィールドを取得
    // #[response_error(status_code = 500, error_code = 1)]
    //   ^^^^^^^^^^^^^^ ^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^
    //       attr           field              field
    let name_values: CommaPunctuatedNameValues = attr
        .parse_args_with(Punctuated::parse_terminated)
        .map_err(|e| {
            syn::Error::new_spanned(
                attr,
                format!("failed to parse response_error attribute: {e}"),
            )
        })?;
    let mut status_code: Option<u16> = None;
    let mut error_code: Option<u32> = None;
    for name_value in name_values {
        if name_value.path.is_ident("status_code") {
            let code = expr_to_value::<u16>(&name_value.value)
                .map_err(|_| syn::Error::new(name_value.span(), "status_code must be u16"))?;
            // StatusCodeに変換できる値であるか確認
            if StatusCode::from_u16(code).is_err() {
                return Err(syn::Error::new(
                    name_value.span(),
                    "status_code must be well-known code",
                ));
            }
            status_code = Some(code);
        }
        if name_value.path.is_ident("error_code") {
            error_code =
                Some(expr_to_value::<u32>(&name_value.value).map_err(|_| {
                    syn::Error::new(name_value.span(), "error_code code must be u32")
                })?);
        }
    }
    if status_code.is_none() {
        return Err(syn::Error::new(
            attr.span(),
            "status_code not found in response_error fields",
        ));
    }
    if error_code.is_none() {
        return Err(syn::Error::new(
            attr.span(),
            "error_code not found in response_error fields",
        ));
    }

    Ok(ResponseErrorFields {
        status_code: status_code.unwrap(),
        error_code: error_code.unwrap(),
    })
}

fn expr_to_value<T>(expr: &Expr) -> Result<T, ()>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Int(lit_int) => Ok(lit_int.base10_parse::<T>().map_err(|_| ())?),
            _ => Err(()),
        },
        _ => Err(()),
    }
}
