use proc_macro::{self, TokenStream};
use syn::DeriveInput;

#[proc_macro_derive(GetSize)]
pub fn derive_size(input: TokenStream) -> TokenStream {
    let a: DeriveInput = syn::parse(input).unwrap();

    let struct_name = a.ident;

    quote::quote! {
        impl #struct_name {
            pub fn size(&self) -> Result<(f64, f64), String> {
                Ok((self.size.thickness.get_num()?, self.size.length.get_num()?))
            }
        }
    }
    .into()
}
