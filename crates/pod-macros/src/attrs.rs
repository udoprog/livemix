use alloc::boxed::Box;
use alloc::format;

use syn::Token;
use syn::spanned::Spanned;

use crate::Ctxt;

pub(crate) struct Object {
    pub(crate) ty: syn::Expr,
    pub(crate) id: syn::Expr,
}

#[derive(Default)]
pub(crate) enum Container {
    #[default]
    Struct,
    Object(Box<Object>),
}

#[derive(Default)]
pub(crate) struct ContainerAttrs {
    pub(crate) container: Container,
    pub(crate) path: Option<syn::Path>,
}

pub(crate) fn container(cx: &Ctxt, inputs: &[syn::Attribute]) -> Result<ContainerAttrs, ()> {
    let mut attrs = ContainerAttrs::default();

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

            if meta.path.is_ident("object") {
                let content;
                syn::parenthesized!(content in meta.input);

                let mut object_type = None;
                let mut object_id = None;

                loop {
                    if content.is_empty() {
                        break;
                    }

                    let out = 'out: {
                        if content.parse::<Option<Token![type]>>()?.is_some() {
                            break 'out &mut object_type;
                        }

                        let ident = content.parse::<syn::Ident>()?;

                        if ident == "id" {
                            break 'out &mut object_id;
                        }

                        return Err(syn::Error::new(
                            ident.span(),
                            format!("#[pod(object({ident}))] Unknown object attribute"),
                        ));
                    };

                    content.parse::<Token![=]>()?;
                    *out = Some(content.parse()?);

                    if content.is_empty() {
                        break;
                    }

                    _ = content.parse::<Token![,]>()?;
                }

                let object_type = object_type.ok_or_else(|| {
                    syn::Error::new(
                        meta.path.span(),
                        "#[pod(object(..))] Missing `type` attribute",
                    )
                })?;

                let object_id = object_id.ok_or_else(|| {
                    syn::Error::new(
                        meta.path.span(),
                        "#[pod(object(..))] Missing `id` attribute",
                    )
                })?;

                attrs.container = Container::Object(Box::new(Object {
                    ty: object_type,
                    id: object_id,
                }));
                return Ok(());
            }

            Err(syn::Error::new(
                meta.path.span(),
                "#[pod(..)] Unsupported container attribute",
            ))
        });

        if let Err(e) = result {
            cx.error(e);
            continue;
        }
    }

    Ok(attrs)
}

#[derive(Default)]
pub(crate) struct FieldAttrs {
    pub(crate) key: Option<syn::Expr>,
}

pub(crate) fn field(cx: &Ctxt, inputs: &[syn::Attribute]) -> Result<FieldAttrs, ()> {
    let mut attrs = FieldAttrs::default();

    for a in inputs {
        if !a.path().is_ident("pod") {
            continue;
        }

        let result = a.parse_nested_meta(|meta| {
            if meta.path.is_ident("property") {
                if meta.input.parse::<Option<Token![=]>>()?.is_some() {
                    attrs.key = Some(meta.input.parse()?);
                    return Ok(());
                }

                let content;
                syn::parenthesized!(content in meta.input);

                loop {
                    if content.is_empty() {
                        break;
                    }

                    let out = 'out: {
                        let ident = content.parse::<syn::Ident>()?;

                        if ident == "key" {
                            break 'out &mut attrs.key;
                        }

                        return Err(syn::Error::new(
                            ident.span(),
                            format!("#[pod(property({}))] Unknown key", ident),
                        ));
                    };

                    content.parse::<Token![=]>()?;
                    *out = Some(content.parse()?);

                    if content.is_empty() {
                        break;
                    }

                    _ = content.parse::<Token![,]>()?;
                }

                return Ok(());
            }

            Err(syn::Error::new(
                meta.path.span(),
                "#[pod(..)] Unsupported attribute",
            ))
        });

        if let Err(e) = result {
            cx.error(e);
            continue;
        }
    }

    Ok(attrs)
}
