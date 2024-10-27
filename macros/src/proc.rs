use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{ImplItem, Item};

pub fn soxtype_impl(item: Item) -> TokenStream {
    let mut methods = Vec::new();

    let tokens = match item.clone() {
        Item::Impl(item_impl) => {
            let ident = item_impl.self_ty.as_ref().into_token_stream();

            for i in item_impl.items.iter() {

                match i {
                    ImplItem::Fn(v) => {
                        let fn_name = v.sig.ident.clone();

                        for attr in v.attrs.iter() {
                            if attr.path().is_ident("soxmethod") {
                                methods.push((fn_name.to_string(), fn_name.clone()));
                            }
                        }
                    }

                    _ => {}
                }
            }
            let mut inner_tokens = TokenStream::new();
            let mut tokens = TokenStream::new();
            for (method_name, method) in methods {
                inner_tokens.extend(quote! [
                    (#method_name, SoxMethod{
                        func: static_func(#ident::#method)
                    }),
                ]);
            }
            let array: TokenTree = Group::new(Delimiter::Bracket, inner_tokens).into();
            tokens.extend([array]);

            quote! {
                #item_impl
                impl SoxClassImpl for #ident{
                    const METHOD_DEFS: &'static [(&'static str, SoxMethod)] = &#tokens;
                }
            }
        }
        _ => item.into_token_stream(),
    };
    tokens.into_token_stream()
}
