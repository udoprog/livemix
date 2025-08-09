use core::cell::RefCell;

use alloc::format;
use alloc::vec::Vec;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;

use crate::Toks;
use crate::attrs;

pub(crate) struct Ctxt {
    errors: RefCell<Vec<syn::Error>>,
}

impl Ctxt {
    /// Construct a new context.
    #[inline]
    pub(crate) fn new() -> Self {
        Ctxt {
            errors: RefCell::new(Vec::new()),
        }
    }

    /// Report an error to the context.
    #[inline]
    pub(crate) fn error(&self, e: syn::Error) {
        self.errors.borrow_mut().push(e);
    }

    /// Test if context contains errors.
    #[inline]
    pub(crate) fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    /// Coerce the context into a `TokenStream` of errors.
    pub(crate) fn into_errors(self) -> TokenStream {
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

struct Field<'field> {
    span: Span,
    accessor: syn::Member,
    attrs: attrs::FieldAttrs,
    data: &'field syn::Field,
}

fn fields<'field>(cx: &Ctxt, data: &'field syn::Data) -> Result<Vec<Field<'field>>, ()> {
    match data {
        syn::Data::Struct(s) => {
            let mut fields = Vec::new();

            for (index, f) in s.fields.iter().enumerate() {
                let attrs = attrs::field(cx, &f.attrs)?;

                let span;
                let accessor;

                match &f.ident {
                    Some(ident) => {
                        span = ident.span();
                        accessor = syn::Member::Named(ident.clone());
                    }
                    None => {
                        span = f.span();
                        accessor = syn::Member::Unnamed(syn::Index {
                            index: index as u32,
                            span: f.span(),
                        });
                    }
                };

                fields.push(Field {
                    span,
                    accessor,
                    attrs,
                    data: f,
                });
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

    let attrs = attrs::container(cx, &attrs)?;
    let base = attrs.path.unwrap_or_else(|| syn::parse_quote!(::pod));
    let core = syn::parse_quote!(::core);
    let toks = Toks::new(&core, &base);

    let Toks {
        result,
        option,
        readable_t,
        error,
        pod_stream_t,
        struct_,
        object,
        property,
        raw_id_t,
        default_t,
        pod_item_t,
        ..
    } = &toks;

    let fields = fields(&cx, &input.data)?;

    let (add, lt) = 'lt: {
        if let Some(lt) = generics.lifetimes().next() {
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

    let inner;

    match attrs.container {
        attrs::Container::Struct => {
            let accessor = fields.iter().map(|f| &f.accessor);

            inner = quote! {
                let mut st = #pod_item_t::read_struct(#pod_stream_t::next(pod)?)?;

                #result::Ok(Self {
                    #(#accessor: #struct_::read(&mut st)?,)*
                })
            };
        }
        attrs::Container::Object(o) => {
            let attrs::Object { ty, id } = &*o;

            let mut keys = Vec::new();
            let mut vars = Vec::new();
            let mut types = Vec::new();
            let mut fallback = Vec::new();

            for (n, f) in fields.iter().enumerate() {
                let Some(key) = &f.attrs.key else {
                    cx.error(syn::Error::new(
                        f.span,
                        "#[pod(key = ..)] Missing for field",
                    ));

                    continue;
                };

                let ty = &f.data.ty;

                keys.push(key);
                vars.push(syn::Ident::new(&format!("field{n}"), f.span));
                types.push(ty);
                fallback.push(quote!(<#ty as #default_t>::default()));
            }

            let match_fields = if !keys.is_empty() {
                quote! {
                    match #raw_id_t::from_id(#property::key(&prop)) {
                        #(#keys => {
                            #vars = #option::Some(#pod_item_t::read(#property::value(prop))?);
                        },)*
                        _ => {},
                    }
                }
            } else {
                quote!()
            };

            let accessor = fields.iter().map(|f| &f.accessor);

            inner = quote! {
                let mut obj = #pod_item_t::read_object(#pod_stream_t::next(pod)?)?;

                if #ty != #object::object_type::<u32>(&obj) {
                    return #result::Err(#error::__invalid_object_type(#ty, obj.object_type::<u32>()));
                }

                if #id != #object::object_id::<u32>(&obj) {
                    return #result::Err(#error::__invalid_object_id(#id, obj.object_id::<u32>()));
                }

                #(
                    let mut #vars = #option::<#types>::None;
                )*

                while !#object::is_empty(&obj) {
                    let prop = #object::property(&mut obj)?;
                    #match_fields
                }

                #result::Ok(Self {
                    #(#accessor: match #vars {
                        #option::Some(v) => v,
                        #option::None => #fallback,
                    },)*
                })
            };
        }
    }

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #readable_t<#lt> for #ident #ty_generics #where_generics {
            #[inline]
            fn read_from(pod: &mut impl #pod_stream_t<#lt>) -> #result<Self, #error> {
                #inner
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

    let attrs = attrs::container(cx, &attrs)?;
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
        object_builder,
        object,
        embeddable_t,
        writer_slice,
        writer_t,
        build_pod_t,
        ..
    } = &toks;

    let fields = fields(cx, &input.data)?;
    let accessor = fields.iter().map(|f| &f.accessor).collect::<Vec<_>>();

    let inner;
    let impl_embeddable;

    match attrs.container {
        attrs::Container::Struct => {
            inner = quote! {
                #builder::write_struct(#pod_sink_t::next(pod)?, |pod| {
                    #(#struct_builder::write(pod, &self.#accessor)?;)*
                    #result::Ok(())
                })?;

                #result::Ok(())
            };

            impl_embeddable = None;
        }
        attrs::Container::Object(o) => {
            let attrs::Object { ty, id } = &*o;

            let mut keys = Vec::new();

            for f in &fields {
                let Some(key) = &f.attrs.key else {
                    cx.error(syn::Error::new(
                        f.span,
                        "#[pod(key = ..)] Missing for field",
                    ));

                    continue;
                };

                keys.push(key);
            }

            inner = quote! {
                #builder::write_object(#pod_sink_t::next(pod)?, #ty, #id, |obj| {
                    #(
                        let prop = #object_builder::property(obj, #keys);
                        #builder::write(prop, &self.#accessor)?;
                    )*

                    #result::Ok(())
                })?;

                #result::Ok(())
            };

            let (impl_generics, ty_generics, where_generics) = generics.split_for_impl();

            impl_embeddable = Some(quote! {
                #[automatically_derived]
                impl #impl_generics #embeddable_t for #ident #ty_generics #where_generics {
                    type Embed<W> = #object<#writer_slice<W, 16>> where W: #writer_t;

                    #[inline]
                    fn embed_into<W, P>(&self, pod: #builder<W, P>) -> #result<Self::Embed<W>, #error>
                    where
                        W: #writer_t,
                        P: #build_pod_t,
                    {
                        #builder::embed_object(pod, #ty, #id, |obj| {
                            #(
                                let prop = #object_builder::property(obj, #keys);
                                #builder::write(prop, &self.#accessor)?;
                            )*

                            #result::Ok(())
                        })
                    }
                }
            });
        }
    }

    let (impl_generics, ty_generics, where_generics) = generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #writable_t for #ident #ty_generics #where_generics {
            #[inline]
            fn write_into(&self, pod: &mut impl #pod_sink_t) -> #result<(), #error> {
                #inner
            }
        }

        #impl_embeddable
    })
}
