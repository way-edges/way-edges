use proc_macro::{self, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, DeriveInput, ItemStruct, LitStr, Meta,
    MetaNameValue, Token,
};

#[proc_macro_derive(GetSize)]
pub fn derive_size(input: TokenStream) -> TokenStream {
    let a: DeriveInput = syn::parse(input).unwrap();

    let struct_name = a.ident;

    quote! {
        impl #struct_name {
            pub fn size(&self) -> Result<(f64, f64), String> {
                Ok((self.size.thickness.get_num()?, self.size.length.get_num()?))
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn wrap_rc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated<Meta, Token![,]>::parse_terminated);
    let a: DeriveInput = syn::parse(item.clone()).unwrap();
    let normal_name = a.ident;

    fn str_from_meta_name_value(nv: MetaNameValue) -> Option<proc_macro2::TokenStream> {
        if let syn::Expr::Lit(expr_lit) = nv.value {
            if let syn::Lit::Str(str) = &expr_lit.lit {
                return Some(str.value().parse().unwrap());
            }
        }
        None
    }

    let mut rc = None;
    let mut normal = None;
    for m in args.into_iter() {
        if let Meta::NameValue(meta_name_value) = m {
            let is = |s| meta_name_value.path.is_ident(s);
            if is("rc") {
                rc = str_from_meta_name_value(meta_name_value);
            } else if is("normal") {
                normal = str_from_meta_name_value(meta_name_value);
            };
        }
    }

    let pub_rc = rc;
    let pub_normal = normal;

    let rc_name = syn::Ident::new(&format!("{}Rc", normal_name), normal_name.span());
    let rc_weak_name = syn::Ident::new(&format!("{}RcWeak", normal_name), normal_name.span());

    let item = proc_macro2::TokenStream::from(item);

    quote! {
        #item
        impl #normal_name {
            #pub_normal fn make_rc(self) -> #rc_name {
                #rc_name::new(self)
            }
        }

        #[derive(Debug, Clone)]
        #pub_rc struct #rc_name(std::rc::Rc<std::cell::RefCell<#normal_name>>);

        impl #rc_name {
            #pub_rc fn new(normal: #normal_name) -> Self {
                use std::cell::RefCell;
                use std::rc::Rc;
                Self(Rc::new(RefCell::new(normal)))
            }
        }

        impl std::ops::Deref for #rc_name {
            type Target = std::cell::RefCell<#normal_name>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        #[derive(Debug, Clone)]
        #pub_rc struct #rc_weak_name(std::rc::Weak<std::cell::RefCell<#normal_name>>);
        impl #rc_weak_name {
            #pub_rc fn upgrade(&self) -> Option<#rc_name> {
                std::rc::Weak::upgrade(&self.0).map(#rc_name)
            }
        }
        impl #rc_name {
            #pub_rc fn downgrade(&self) -> #rc_weak_name {
                #rc_weak_name(std::rc::Rc::downgrade(&self.0))
            }
        }
    }
    .into()
}

use syn::{
    parse::{Parse, ParseStream},
    Attribute,
};

struct PropertyPair {
    name: LitStr,
    value: LitStr,
}

impl Parse for PropertyPair {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let value: LitStr = input.parse()?;
        Ok(PropertyPair { name, value })
    }
}

/// Parse the attributes to find the const_property attributes
fn extract_const_properties(attrs: &[Attribute]) -> Vec<PropertyPair> {
    let mut properties = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("const_property") {
            if let Ok(meta) = attr.parse_args::<PropertyPair>() {
                properties.push(meta);
            }
        }
    }

    properties
}

#[proc_macro_attribute]
pub fn const_property(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as PropertyPair);
    let mut input_ast = parse_macro_input!(input as DeriveInput);

    let struct_name = &input_ast.ident;
    let property_name = &args.name;
    let property_value = &args.value;

    // Generate the function name for the schema transformation
    let function_name = format_ident!("{}_generate_defs", struct_name);

    // Add the schemars transform attribute to the struct
    let schemars_path: syn::Path = syn::parse_str("schemars").unwrap();
    let transform_meta = syn::parse_quote! {
        #schemars_path(transform = #function_name)
    };

    // Add the schemars attribute
    input_ast.attrs.insert(
        0,
        syn::Attribute {
            pound_token: syn::token::Pound::default(),
            style: syn::AttrStyle::Outer,
            bracket_token: syn::token::Bracket::default(),
            meta: transform_meta,
        },
    );

    // Create the output with the transformed struct and the schema function
    let output = quote! {
        #input_ast

        #[allow(non_snake_case)]
        fn #function_name(schema: &mut Schema) {
            let root = schema.ensure_object();

            match root.get_mut("properties") {
                Some(Value::Object(map)) => map,
                _ => return,
            }
            .insert(
                #property_name.to_string(),
                Value::Object(serde_json::Map::from_iter(
                    vec![("const".to_string(), Value::String(#property_value.to_string()))].into_iter(),
                )),
            );
        }
    };

    output.into()
}

#[proc_macro_derive(ConstProperties, attributes(const_property))]
pub fn derive_const_properties(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = &input.ident;
    let properties = extract_const_properties(&input.attrs);

    if properties.is_empty() {
        return quote! {
            #input
        }
        .into();
    }

    // Generate the function name for the schema transformation
    let function_name = format_ident!("{}_generate_defs", struct_name);

    // Generate property insertions
    let property_insertions = properties.iter().map(|prop| {
        let name = &prop.name;
        let value = &prop.value;

        quote! {
            .insert(
                #name.to_string(),
                Value::Object(serde_json::Map::from_iter(
                    vec![("const".to_string(), Value::String(#value.to_string()))].into_iter(),
                )),
            )
        }
    });

    // Create the output with the transformed struct and the schema function
    let output = quote! {
        #[schemars(transform = #function_name)]
        #input

        #[allow(non_snake_case)]
        fn #function_name(schema: &mut Schema) {
            let root = schema.ensure_object();

            let props = match root.get_mut("properties") {
                Some(Value::Object(map)) => map,
                _ => return,
            };

            #(props #property_insertions;)*
        }
    };

    output.into()
}
