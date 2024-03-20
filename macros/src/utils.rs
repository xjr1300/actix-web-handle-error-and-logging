use syn::{punctuated::Punctuated, Expr, Lit, MetaNameValue, Token};

/// 属性内にあるカンマで区切られたフィールドで、名前と値を等号で連結
///
/// version = "1.0.0", features = ["derive", "macro"]
pub(crate) type CommaPunctuatedNameValues = Punctuated<MetaNameValue, Token![,]>;

/// リテラルを表現した式から、リテラルの値を取得する。
pub(crate) fn expr_to_value<T>(expr: &Expr) -> Result<T, ()>
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
