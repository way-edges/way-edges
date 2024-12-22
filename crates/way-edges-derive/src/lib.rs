use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Error, Ident, ItemTrait, Meta,
    MetaNameValue, Token, Type, Visibility,
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
        impl gtk::glib::clone::Upgrade for #rc_weak_name {
            type Strong = #rc_name;
            fn upgrade(&self) -> Option<Self::Strong> {
                std::rc::Weak::upgrade(&self.0).map(#rc_name)
            }
        }
        impl gtk::glib::clone::Downgrade for #rc_name {
            type Weak = #rc_weak_name;
            fn downgrade(&self) -> Self::Weak {
                #rc_weak_name(std::rc::Rc::downgrade(&self.0))
            }
        }
    }
    .into()
}
