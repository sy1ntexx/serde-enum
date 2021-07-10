use quote::quote;
use syn::*;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum NamingStyle {
    SnakeCase,
    CamelCase,
    ScreamingSnakeCase,
    None,
}

lazy_static! {
    static ref NAME_MAP: HashMap<NamingStyle, fn(&str) -> String> = {
        let mut m = HashMap::new();

        // I have no actual idea why this is working like that, and why I need this cast
        m.insert(NamingStyle::SnakeCase, to_snake_case as fn(&str) -> String);
        m.insert(NamingStyle::CamelCase, to_camel_case);
        m.insert(NamingStyle::ScreamingSnakeCase, to_screaming_snake_case);
        m
    };
}

#[proc_macro_derive(Serialize_enum, attributes(serde))]
pub fn serialize_enum(item: TokenStream) -> TokenStream {
    let target = parse_macro_input!(item as DeriveInput);
    let data = get_enum_from_input(&target);

    let style = get_naming_style(target.attrs.iter());

    let target_ident = &target.ident;
    let ser_arms = create_ser_arms(&data, style);
    let out = quote! {
        impl serde::Serialize for #target_ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer
            {
                match self {
                    #(#ser_arms),*
                }
            }
        }
    };
    out.into()
}

#[proc_macro_derive(Deserialize_enum, attributes(serde))]
pub fn deserialize_enum(item: TokenStream) -> TokenStream {
    let target = parse_macro_input!(item as DeriveInput);
    let data = get_enum_from_input(&target);

    let style = get_naming_style(target.attrs.iter());

    let target_ident = &target.ident;
    let de_arms = create_de_arms(&data, style);
    let out = quote! {
        impl<'de> serde::Deserialize<'de> for #target_ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>
            {
                Ok(
                    match <&str>::deserialize(deserializer)? {
                        #(#de_arms),*,
                        _ => { unimplemented!() }
                    }
                )
            }
        }
    };
    out.into()
}

fn get_naming_style<'a>(target: impl Iterator<Item = &'a Attribute>) -> NamingStyle {
    for a in target {
        if let Some(i) = a.path.get_ident() {
            if i == "serde" {
                if let Ok(ExprParen { expr, .. }) = parse2::<ExprParen>(a.tokens.clone()) {
                    if let Expr::Assign(ea) = expr.as_ref() {
                        if let Expr::Path(ep) = ea.left.as_ref() {
                            if let Some(i) = ep.path.get_ident() {
                                if i == "rename" || i == "rename_all" {
                                    if let Expr::Lit(ExprLit {
                                        lit: Lit::Str(s), ..
                                    }) = ea.right.as_ref()
                                    {
                                        return match s.value().as_str() {
                                            "snake_case" => NamingStyle::SnakeCase,
                                            "camelCase" => NamingStyle::CamelCase,
                                            "SCREAMING_SNAKE_CASE" => {
                                                NamingStyle::ScreamingSnakeCase
                                            }
                                            _ => {
                                                panic!(
                                                    "Unsupported style. \
                                                    Available: `snake_case`, `camelCase`"
                                                )
                                            }
                                        };
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    NamingStyle::None
}

fn get_variant_alias(v: &Variant) -> Option<String> {
    for a in v.attrs.iter() {
        if let Some(i) = a.path.get_ident() {
            if i == "serde" {
                if let Ok(ExprParen { expr, .. }) = parse2::<ExprParen>(a.tokens.clone()) {
                    if let Expr::Assign(ea) = expr.as_ref() {
                        if let Expr::Path(ep) = ea.left.as_ref() {
                            if let Some(i) = ep.path.get_ident() {
                                if i == "name" {
                                    if let Expr::Lit(ExprLit {
                                        lit: Lit::Str(s), ..
                                    }) = ea.right.as_ref()
                                    {
                                        return Some(s.value());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_enum_from_input(target: &DeriveInput) -> DataEnum {
    if !target.generics.params.is_empty() {
        panic!("`Serialize_enum` target cannot have any generics parameters!");
    }

    if let Data::Enum(ref e) = target.data {
        e.clone()
    } else {
        panic!("`Serialize_enum` can only be applied to enums!");
    }
}

fn create_ser_arms(target: &DataEnum, n: NamingStyle) -> impl Iterator<Item = TokenStream2> {
    target.variants.clone().into_iter().map(move |v| {
        assert!(matches!(v.fields, Fields::Unit));
        let ident = &v.ident;
        let value = format_variant(&v, n);

        quote! {
            Self::#ident => { serializer.serialize_str(#value) }
        }
    })
}

fn create_de_arms(target: &DataEnum, n: NamingStyle) -> impl Iterator<Item = TokenStream2> {
    target.variants.clone().into_iter().map(move |v| {
        assert!(matches!(v.fields, Fields::Unit));

        let ident = &v.ident;
        let value = format_variant(&v, n);

        quote! {
            #value => Self::#ident
        }
    })
}

fn format_variant(v: &Variant, parent_style: NamingStyle) -> String {
    if let Some(s) = get_variant_alias(v) {
        return s;
    }
    
    let own_style = get_naming_style(v.attrs.iter());

    match own_style {
        NamingStyle::None => match parent_style {
            NamingStyle::None => v.ident.to_string(),
            ps => NAME_MAP.get(&ps).unwrap()(&v.ident.to_string()),
        },
        os => NAME_MAP.get(&os).unwrap()(&v.ident.to_string()),
    }
}

fn to_snake_case(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    if v.is_empty() {
        out.push(v.chars().next().unwrap().to_ascii_lowercase());
    }

    for c in v.chars().skip(1) {
        if c.is_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }

    out
}

fn to_camel_case(v: &str) -> String {
    v.to_string()
        .char_indices()
        .map(|(i, c)| if i == 0 { c.to_ascii_lowercase() } else { c })
        .collect()
}

fn to_screaming_snake_case(v: &str) -> String {
    v.char_indices()
        .fold(String::with_capacity(v.len()), |mut s, (i, c)| {
            if c.is_uppercase() && i != 0 {
                s.push('_');
            }
            s.push(c.to_ascii_uppercase());
            s
        })
}
