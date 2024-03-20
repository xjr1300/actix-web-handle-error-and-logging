use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::{Attribute, Data, DataEnum, DeriveInput, Fields, Ident, Variant, Visibility};

use crate::utils::{expr_to_value, CommaPunctuatedNameValues};

pub(crate) fn use_case_error_impl(input: DeriveInput) -> syn::Result<TokenStream2> {
    let enum_ident = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let Data::Enum(data_enum) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "UseCaseError is expected enum",
        ));
    };

    // 列挙型のバリアントと`error_code`属性のフィールド値を取得
    let variants = retrieve_enum_variants(data_enum)?;

    // error_codeメソッドを実装
    let error_code_method = impl_error_code_method(vis, &variants);

    Ok(quote!(
        impl #impl_generics #enum_ident #ty_generics #where_clause {
            #error_code_method
        }
    ))
}

struct EnumVariant<'a> {
    ident: &'a Ident,
    is_unit: bool,
    error_code: u32,
}

/// 列挙型のバリアントと`use_case_error`属性のフィールド値を取得する。
///
/// #[use_case_error(error_code = 1000)]
///   ^^^^^^^^^^^^^^ ^^^^^^^^^^   ^^^^
///   属性            フィールド      フィールド値
fn retrieve_enum_variants(data_enum: &DataEnum) -> syn::Result<Vec<EnumVariant>> {
    let mut enum_variants = vec![];
    for variant in data_enum.variants.iter() {
        // use_case_error属性を取得
        let attr = retrieve_use_case_error_attribute(variant)?;
        // use_case_error属性のerror_codeフィールドを取得
        let error_code = retrieve_error_code_field_value(attr)?;
        // バリアントの情報を登録
        enum_variants.push(EnumVariant {
            ident: &variant.ident,
            is_unit: matches!(variant.fields, Fields::Unit),
            error_code,
        });
    }

    Ok(enum_variants)
}

/// 列挙型のバリアントに付与された`use_case_error`属性を取得する。
///
/// #[use_case_error(error_code = 1000)]
///   ^^^^^^^^^^^^^^
///   use_case_error属性
fn retrieve_use_case_error_attribute(variant: &Variant) -> syn::Result<&Attribute> {
    variant
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("use_case_error"))
        .ok_or(syn::Error::new(
            variant.ident.span(),
            "the variant of enum that derived UseCaseError must have a use_case_error attribute",
        ))
}

/// `use_case_error`属性に設定された`error_code`フィールドの値を取得する。
/// #[use_case_error(error_code = 1000)]
///                  ^^^^^^^^^^   ^^^^
///                  フィールド      フィールド値
fn retrieve_error_code_field_value(attr: &Attribute) -> syn::Result<u32> {
    // use_case_error属性に設定されたフィールドを取得
    let name_values: CommaPunctuatedNameValues = attr
        .parse_args_with(Punctuated::parse_terminated)
        .map_err(|_| {
            syn::Error::new_spanned(
                attr,
                "the use_case_error attribute must have the error_code fields",
            )
        })?;
    // error_codeフィールドを検索して、その値を取得
    for name_value in name_values.iter() {
        if name_value.path.is_ident("error_code") {
            let code = expr_to_value::<u32>(&name_value.value)
                .map_err(|_| syn::Error::new(name_value.span(), "error_code must be u32"))?;
            return Ok(code);
        }
    }

    Err(syn::Error::new(
        attr.span(),
        "the use_case_error attribute must have the error_code field",
    ))
}

/// error_codeメソッドを実装する。
fn impl_error_code_method(vis: &Visibility, variants: &[EnumVariant]) -> TokenStream2 {
    let mut arms: Vec<TokenStream2> = vec![];
    for EnumVariant {
        ident,
        is_unit,
        error_code,
    } in variants
    {
        let token_stream = if *is_unit {
            quote! { Self::#ident => #error_code, }
        } else {
            quote! { Self::#ident(..) => #error_code, }
        };
        arms.push(token_stream);
    }

    quote! {
        #vis fn error_code(&self) -> u32 {
            match *self {
                #(
                    #arms
                )*
            }
        }
    }
}
