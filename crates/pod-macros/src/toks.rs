use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::Token;

pub(crate) struct Toks<'base> {
    pub(crate) builder: P<'base>,
    pub(crate) error: P<'base>,
    pub(crate) pod_sink_t: P<'base>,
    pub(crate) pod_stream_t: P<'base>,
    pub(crate) readable_t: P<'base>,
    pub(crate) result: Nested<'base>,
    pub(crate) option: Nested<'base>,
    pub(crate) struct_: P<'base>,
    pub(crate) object: P<'base>,
    pub(crate) property: P<'base>,
    pub(crate) struct_builder: Nested<'base>,
    pub(crate) object_builder: Nested<'base>,
    pub(crate) writable_t: P<'base>,
    pub(crate) raw_id_t: P<'base>,
    pub(crate) pod_item_t: P<'base>,
    pub(crate) default_t: Nested<'base>,
}

impl<'base> Toks<'base> {
    pub(super) fn new(core: &'base syn::Path, base: &'base syn::Path) -> Self {
        macro_rules! p {
            ($module:ident :: $ident:ident) => {
                Nested {
                    base,
                    module: syn::Ident::new(stringify!($module), Span::call_site()),
                    ident: syn::Ident::new(stringify!($ident), Span::call_site()),
                }
            };

            ($ident:ident) => {
                P {
                    base,
                    ident: syn::Ident::new(stringify!($ident), Span::call_site()),
                }
            };
        }

        macro_rules! core {
            ($module:ident :: $ident:ident) => {
                Nested {
                    base: core,
                    module: syn::Ident::new(stringify!($module), Span::call_site()),
                    ident: syn::Ident::new(stringify!($ident), Span::call_site()),
                }
            };
        }

        Toks {
            builder: p!(Builder),
            error: p!(Error),
            pod_sink_t: p!(PodSink),
            pod_stream_t: p!(PodStream),
            readable_t: p!(Readable),
            result: core!(result::Result),
            option: core!(option::Option),
            struct_: p!(Struct),
            object: p!(Object),
            property: p!(Property),
            struct_builder: p!(builder::StructBuilder),
            object_builder: p!(builder::ObjectBuilder),
            writable_t: p!(Writable),
            raw_id_t: p!(RawId),
            pod_item_t: p!(PodItem),
            default_t: core!(default::Default),
        }
    }
}

/// A type referenced from `::core`.
pub(crate) struct Nested<'base> {
    base: &'base syn::Path,
    module: syn::Ident,
    ident: syn::Ident,
}

impl ToTokens for Nested<'_> {
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.base.to_tokens(tokens);
        <Token![::]>::default().to_tokens(tokens);
        self.module.to_tokens(tokens);
        <Token![::]>::default().to_tokens(tokens);
        self.ident.to_tokens(tokens);
    }
}

/// A more memory efficient path.
pub(crate) struct P<'base> {
    base: &'base syn::Path,
    ident: syn::Ident,
}

impl ToTokens for P<'_> {
    #[inline]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.base.to_tokens(tokens);
        <Token![::]>::default().to_tokens(tokens);
        self.ident.to_tokens(tokens);
    }
}
