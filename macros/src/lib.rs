use proc_macro::{self, TokenStream};

use quote::{quote, ToTokens};
use syn::Item;
use syn::{parse_macro_input, ItemFn};

use crate::proc::soxtypeimpl;

mod proc;

#[proc_macro_attribute]
pub fn show_streams(attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn trace(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    TokenStream::from(quote!(#input))
}

#[proc_macro_attribute]
pub fn soxtype(attr: TokenStream, item: TokenStream) -> TokenStream {
    let p_item = parse_macro_input!(item as Item);
    soxtypeimpl(p_item).into()
}

#[proc_macro_attribute]
pub fn soxmethod(attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
