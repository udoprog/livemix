use core::cell::RefCell;

use alloc::vec::Vec;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Token;
use syn::spanned::Spanned;

use crate::Toks;

pub(super) struct Ctxt {
    errors: RefCell<Vec<syn::Error>>,
}

impl Ctxt {
    /// Construct a new context.
    pub(super) fn new() -> Self {
        Ctxt {
            errors: RefCell::new(Vec::new()),
        }
    }

    /// Report an error to the context.
    fn error(&self, e: syn::Error) {
        self.errors.borrow_mut().push(e);
    }

    /// Coerce the context into a `TokenStream` of errors.
    pub(super) fn into_errors(self) -> TokenStream {
        let errors = self.errors.borrow();

        if errors.is_empty() {
            return syn::Error::new(
                Span::call_site(),
                "Macro expansion failed, but no errors reported",
            )
            .to_compile_error();
        }

        let mut stream = TokenStream::new();

        for error in errors.iter() {
            stream.extend(error.to_compile_error());
        }

        stream.into()
    }
}

struct ContainerAttrs {
    path: Option<syn::Path>,
}

fn container_attrs(cx: &Ctxt, inputs: &[syn::Attribute]) -> Result<ContainerAttrs, ()> {
    let mut attrs = ContainerAttrs { path: None };

    for a in inputs {
        if !a.path().is_ident("pod") {
            continue;
        }

        let result = a.parse_nested_meta(|meta| {
            if meta.path.is_ident("crate") {
                if meta.input.parse::<Option<Token![=]>>()?.is_some() {
                    attrs.path = Some(meta.input.parse()?);
                } else {
                    attrs.path = Some(syn::parse_quote!(crate));
                }

                return Ok(());
            }

            Err(syn::Error::new(meta.path.span(), "Unsupported attribute"))
        });

        if let Err(e) = result {
            cx.error(e);
            continue;
        }
    }

    Ok(attrs)
}

struct FieldAttrs {}

fn field_attrs(cx: &Ctxt, inputs: &[syn::Attribute]) -> Result<FieldAttrs, ()> {
    let attrs = FieldAttrs {};

    for a in inputs {
        if !a.path().is_ident("pod") {
            continue;
        }

        let result = a.parse_nested_meta(|meta| {
            Err(syn::Error::new(meta.path.span(), "Unsupported attribute"))
        });

        if let Err(e) = result {
            cx.error(e);
            continue;
        }
    }

    Ok(attrs)
}

struct Field {
    accessor: syn::Member,
}

fn fields(cx: &Ctxt, data: &syn::Data) -> Result<Vec<Field>, ()> {
    match data {
        syn::Data::Struct(s) => {
            let mut fields = Vec::new();

            for (index, f) in s.fields.iter().enumerate() {
                let _ = field_attrs(cx, &f.attrs)?;

                let accessor = match &f.ident {
                    Some(ident) => syn::Member::Named(ident.clone()),
                    None => syn::Member::Unnamed(syn::Index {
                        index: index as u32,
                        span: f.span(),
                    }),
                };

                fields.push(Field { accessor });
            }

            Ok(fields)
        }
        syn::Data::Enum(..) => {
            cx.error(syn::Error::new(
                Span::call_site(),
                "Enums are not supported",
            ));
            Err(())
        }
        syn::Data::Union(..) => {
            cx.error(syn::Error::new(
                Span::call_site(),
                "Unions are not supported",
            ));
            Err(())
        }
    }
}

pub fn readable(cx: &Ctxt, input: syn::DeriveInput) -> Result<TokenStream, ()> {
    let syn::DeriveInput {
        ident,
        generics,
        attrs,
        ..
    } = input;

    let attrs = container_attrs(cx, &attrs)?;
    let base = attrs.path.unwrap_or_else(|| syn::parse_quote!(::pod));
    let core = syn::parse_quote!(::core);
    let toks = Toks::new(&core, &base);

    let Toks {
        result,
        readable_t,
        error,
        pod_stream_t,
        typed_pod,
        struct_,
        ..
    } = &toks;

    let fields = fields(&cx, &input.data)?;

    let (add, lt) = 'lt: {
        for lt in generics.lifetimes() {
            break 'lt (false, lt.lifetime.clone());
        }

        (true, syn::parse_quote!('__de))
    };

    let mut with_lifetime;

    let with_lifetime = if add {
        with_lifetime = generics.clone();

        with_lifetime
            .params
            .push(syn::GenericParam::Lifetime(syn::LifetimeParam {
                attrs: Vec::new(),
                lifetime: lt.clone(),
                colon_token: None,
                bounds: syn::punctuated::Punctuated::new(),
            }));

        &with_lifetime
    } else {
        &generics
    };

    let (impl_generics, _, where_generics) = with_lifetime.split_for_impl();
    let (_, ty_generics, _) = generics.split_for_impl();
    let accessor = fields.iter().map(|f| &f.accessor);

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #readable_t<#lt> for #ident #ty_generics #where_generics {
            #[inline]
            fn read_from(pod: &mut impl #pod_stream_t<#lt>) -> #result<Self, #error> {
                let mut st = #typed_pod::read_struct(#pod_stream_t::next(pod)?)?;

                #result::Ok(Self {
                    #(#accessor: #struct_::read(&mut st)?,)*
                })
            }
        }
    })
}

pub fn writable(cx: &Ctxt, input: syn::DeriveInput) -> Result<TokenStream, ()> {
    let syn::DeriveInput {
        ident,
        generics,
        attrs,
        ..
    } = input;

    let attrs = container_attrs(cx, &attrs)?;
    let base = attrs.path.unwrap_or_else(|| syn::parse_quote!(::pod));
    let core = syn::parse_quote!(::core);
    let toks = Toks::new(&core, &base);

    let Toks {
        result,
        writable_t,
        error,
        pod_sink_t,
        builder,
        struct_builder,
        ..
    } = &toks;

    let fields = fields(&cx, &input.data)?;
    let accessor = fields.iter().map(|f| &f.accessor);

    let (impl_generics, ty_generics, where_generics) = generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #writable_t for #ident #ty_generics #where_generics {
            #[inline]
            fn write_into(&self, pod: &mut impl #pod_sink_t) -> #result<(), #error> {
                #builder::write_struct(#pod_sink_t::next(pod)?, |pod| {
                    #(#struct_builder::write(pod, &self.#accessor)?;)*
                    #result::Ok(())
                })?;

                #result::Ok(())
            }
        }
    })
}
