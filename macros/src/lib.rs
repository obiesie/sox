use proc_macro::{self, TokenStream};


use syn::Item;
use syn::{parse_macro_input};

use crate::proc::soxtype_impl;

mod proc;

#[proc_macro_attribute]
pub fn soxtype(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let p_item = parse_macro_input!(item as Item);
    soxtype_impl(p_item).into()
}

#[proc_macro_attribute]
pub fn soxmethod(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
