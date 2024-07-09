use proc_macro::{self, TokenStream};
use syn::DeriveInput;

#[proc_macro_derive(GetSize)]
pub fn derive_size(input: TokenStream) -> TokenStream {
    let a: DeriveInput = syn::parse(input).unwrap();

    let struct_name = a.ident;

    quote::quote! {
        impl #struct_name {
            pub fn get_size(&self) -> Result<(f64, f64), String> {
                Ok((self.width.get_num()?, self.height.get_num()?))
            }
        }
    }
    .into()
}
