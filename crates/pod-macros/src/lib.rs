//! Macros used to interact with pods.

#![no_std]
#![allow(clippy::needless_late_init)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

use proc_macro::TokenStream;

mod pod;
use self::pod::Ctxt;

mod toks;
use self::toks::Toks;

mod attrs;

#[proc_macro_derive(Readable, attributes(pod))]
pub fn derive_readable(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let cx = pod::Ctxt::new();

    if let Ok(stream) = pod::readable(&cx, input)
        && !cx.has_errors()
    {
        return stream.into();
    }

    cx.into_errors().into()
}

#[proc_macro_derive(Writable, attributes(pod))]
pub fn derive_writable(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let cx = pod::Ctxt::new();

    if let Ok(stream) = pod::writable(&cx, input)
        && !cx.has_errors()
    {
        return stream.into();
    }

    cx.into_errors().into()
}
