use syn::*;
use quote::quote;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

#[derive(Debug, Clone, Copy)]
enum NamingStyle {
    SnakeCase,
    CamelCase,
    None,
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

fn format_variant(v: &Variant, ns: NamingStyle) -> String {
    match get_naming_style(v.attrs.iter()) {
        NamingStyle::SnakeCase => return to_snake_case(v.ident.to_string().as_str()),
        NamingStyle::CamelCase => return to_camel_case(v.ident.to_string().as_str()),
        _ => {}
    }

    match ns {
        NamingStyle::SnakeCase => to_snake_case(v.ident.to_string().as_str()),
        NamingStyle::CamelCase => to_camel_case(v.ident.to_string().as_str()),
        NamingStyle::None => v.ident.to_string(),
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
