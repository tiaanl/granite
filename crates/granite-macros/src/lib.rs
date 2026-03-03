use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Expr, ExprLit, Fields, Lit, Path, Type, parse_macro_input,
};

#[proc_macro_derive(AsUniformBuffer, attributes(uniform_visibility))]
pub fn derive_as_uniform_buffer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_as_uniform_buffer(input).into()
}

#[proc_macro_derive(AsVertexLayout, attributes(layout))]
pub fn derive_as_vertex_layout(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_as_layout(input, LayoutTarget::Vertex).into()
}

#[proc_macro_derive(AsInstanceLayout, attributes(layout))]
pub fn derive_as_instance_layout(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_as_layout(input, LayoutTarget::Instance).into()
}

fn expand_as_uniform_buffer(input: DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let visibility = match parse_uniform_visibility(&input.attrs) {
        Ok(visibility) => normalize_shader_visibility_path(visibility),
        Err(error) => return error.to_compile_error(),
    };

    quote! {
        impl #impl_generics ::granite::renderer::AsUniformBuffer for #name #ty_generics #where_clause {
            const VISIBILITY: ::granite::renderer::ShaderVisibility = #visibility;
        }
    }
}

#[derive(Clone, Copy)]
enum LayoutTarget {
    Vertex,
    Instance,
}

fn expand_as_layout(input: DeriveInput, target: LayoutTarget) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let data = match &input.data {
        Data::Struct(data) => data,
        _ => {
            return syn::Error::new_spanned(name, "derive only supports structs")
                .to_compile_error();
        }
    };

    let mut attributes = Vec::new();
    let fields_iter = match &data.fields {
        Fields::Named(fields) => fields.named.iter().collect::<Vec<_>>(),
        Fields::Unnamed(fields) => fields.unnamed.iter().collect::<Vec<_>>(),
        Fields::Unit => Vec::new(),
    };

    for field in fields_iter {
        let config = match parse_layout_config(&field.attrs) {
            Ok(config) => config,
            Err(error) => return error.to_compile_error(),
        };

        if config.skip {
            continue;
        }

        let format_tokens = if let Some(path) = config.format {
            normalize_vertex_format_path(path)
        } else {
            match infer_vertex_format_path(&field.ty) {
                Some(path) => path,
                None => {
                    return syn::Error::new(
                        field.ty.span(),
                        "could not infer vertex format; use #[layout(format = Float32xN)]",
                    )
                    .to_compile_error();
                }
            }
        };

        attributes.push(quote_spanned! {
            field.span() => ::granite::renderer::VertexAttribute { format: #format_tokens }
        });
    }

    let layout_trait = match target {
        LayoutTarget::Vertex => quote!(::granite::renderer::AsVertexBufferLayout),
        LayoutTarget::Instance => quote!(::granite::renderer::AsInstanceBufferLayout),
    };

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics #layout_trait for #name #ty_generics #where_clause {
            fn layout() -> ::granite::renderer::VertexBufferLayout {
                ::granite::renderer::VertexBufferLayout {
                    size: ::std::mem::size_of::<Self>() as u64,
                    attributes: vec![#(#attributes),*],
                }
            }
        }
    }
}

#[derive(Default)]
struct LayoutConfig {
    skip: bool,
    format: Option<Path>,
}

fn parse_layout_config(attributes: &[Attribute]) -> syn::Result<LayoutConfig> {
    let mut config = LayoutConfig::default();

    for attribute in attributes {
        if !attribute.path().is_ident("layout") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                if config.skip {
                    return Err(meta.error("duplicate `skip`"));
                }

                if config.format.is_some() {
                    return Err(meta.error("`skip` cannot be combined with `format`"));
                }

                config.skip = true;
                return Ok(());
            }

            if meta.path.is_ident("format") {
                if config.skip {
                    return Err(meta.error("`format` cannot be combined with `skip`"));
                }
                if config.format.is_some() {
                    return Err(meta.error("duplicate `format`"));
                }

                let value = meta.value()?;
                let path: Path = value.parse()?;
                config.format = Some(path);
                return Ok(());
            }

            Err(meta.error("unsupported layout option; expected `skip` or `format = ...`"))
        })?;
    }

    Ok(config)
}

fn parse_uniform_visibility(attributes: &[Attribute]) -> syn::Result<Path> {
    let mut visibility: Option<Path> = None;

    for attribute in attributes {
        if !attribute.path().is_ident("uniform_visibility") {
            continue;
        }

        let parsed: Path = attribute.parse_args()?;
        if visibility.is_some() {
            return Err(syn::Error::new_spanned(
                attribute,
                "duplicate `uniform_visibility` attribute",
            ));
        }
        visibility = Some(parsed);
    }

    visibility.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "missing #[uniform_visibility(...)] attribute",
        )
    })
}

fn normalize_shader_visibility_path(path: Path) -> proc_macro2::TokenStream {
    if path.segments.len() == 1 {
        let ident = &path.segments[0].ident;
        quote!(::granite::renderer::ShaderVisibility::#ident)
    } else {
        quote!(#path)
    }
}

fn normalize_vertex_format_path(path: Path) -> proc_macro2::TokenStream {
    if path.segments.len() == 1 {
        let ident = &path.segments[0].ident;
        quote!(::granite::renderer::VertexFormat::#ident)
    } else {
        quote!(#path)
    }
}

fn infer_vertex_format_path(ty: &Type) -> Option<proc_macro2::TokenStream> {
    match ty {
        Type::Path(type_path) => {
            let ident = &type_path.path.segments.last()?.ident;
            let format_ident = match ident.to_string().as_str() {
                "f32" => "Float32",
                "u32" => "Uint32",
                "i32" => "Sint32",
                "Vec2" => "Float32x2",
                "Vec3" => "Float32x3",
                "Vec4" => "Float32x4",
                "UVec2" => "Uint32x2",
                "UVec3" => "Uint32x3",
                "UVec4" => "Uint32x4",
                "IVec2" => "Sint32x2",
                "IVec3" => "Sint32x3",
                "IVec4" => "Sint32x4",
                _ => return None,
            };
            let ident = syn::Ident::new(format_ident, ident.span());
            Some(quote!(::granite::renderer::VertexFormat::#ident))
        }
        Type::Array(type_array) => {
            let len = parse_array_len(&type_array.len)?;
            let Type::Path(element_path) = &*type_array.elem else {
                return None;
            };
            let element_ident = element_path.path.segments.last()?.ident.to_string();
            let format_ident = match (element_ident.as_str(), len) {
                ("f32", 1) => "Float32",
                ("f32", 2) => "Float32x2",
                ("f32", 3) => "Float32x3",
                ("f32", 4) => "Float32x4",
                ("u32", 1) => "Uint32",
                ("u32", 2) => "Uint32x2",
                ("u32", 3) => "Uint32x3",
                ("u32", 4) => "Uint32x4",
                ("i32", 1) => "Sint32",
                ("i32", 2) => "Sint32x2",
                ("i32", 3) => "Sint32x3",
                ("i32", 4) => "Sint32x4",
                _ => return None,
            };
            let ident = syn::Ident::new(format_ident, type_array.span());
            Some(quote!(::granite::renderer::VertexFormat::#ident))
        }
        _ => None,
    }
}

fn parse_array_len(expr: &Expr) -> Option<usize> {
    let Expr::Lit(ExprLit {
        lit: Lit::Int(int_lit),
        ..
    }) = expr
    else {
        return None;
    };
    int_lit.base10_parse::<usize>().ok()
}
