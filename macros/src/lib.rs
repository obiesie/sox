use proc_macro::{self, TokenStream};

use quote::quote;
use syn::Item;
use syn::{parse_macro_input, ItemFn};

use crate::proc::soxtypeimpl;

mod proc;

#[proc_macro_attribute]
pub fn show_streams(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn trace(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    TokenStream::from(quote!(#input))
}

#[proc_macro_attribute]
pub fn soxtype(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let p_item = parse_macro_input!(item as Item);
    soxtypeimpl(p_item).into()
}

#[proc_macro_attribute]
pub fn soxmethod(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
