use std::{collections::BTreeMap, fs, path::Path};

use quote::ToTokens;
use syn::{
    Fields, FnArg, ImplItem, Item, ItemConst, ItemEnum, ItemFn, ItemImpl, ItemStruct, ItemType,
    Pat, ReturnType, Type, TypeBareFn,
};

use crate::{
    plugin_sdk_docs::{
        model::{
            ApiArgumentDoc, ApiFieldDoc, ApiGroup, ApiItemDoc, ApiVariantDoc, SdkCategory,
            SdkItemKind, SdkSource, SDK_SOURCES,
        },
        parser::{
            impl_owner_name, is_public, optional_doc_summary, require_any_section, require_section,
            required_doc_block, DocBlock,
        },
    },
    XtaskError,
};

/// Collects all generated Plugin SDK item docs from configured Rust sources.
pub(super) fn collect_api_items(repo_root: &Path) -> Result<Vec<ApiItemDoc>, XtaskError> {
    let mut items = Vec::new();
    let mut event_payloads = BTreeMap::<String, Vec<ApiArgumentDoc>>::new();
    for source in SDK_SOURCES {
        let path = repo_root.join(source.path);
        let text = fs::read_to_string(&path).map_err(|error| {
            XtaskError::new(format!(
                "failed to read SDK source '{}': {}",
                path.display(),
                error
            ))
        })?;
        let parsed = syn::parse_file(&text).map_err(|error| {
            XtaskError::new(format!(
                "failed to parse SDK source '{}': {}",
                path.display(),
                error
            ))
        })?;
        collect_source_items(*source, parsed.items, &mut items, &mut event_payloads)?;
    }
    for item in &mut items {
        if item.group == ApiGroup::Events {
            let variant = item
                .name
                .strip_prefix("PluginEventKind::")
                .unwrap_or(&item.name);
            item.arguments = event_payloads.remove(variant).unwrap_or_default();
        }
    }
    Ok(items)
}

fn collect_source_items(
    source: SdkSource,
    source_items: Vec<Item>,
    items: &mut Vec<ApiItemDoc>,
    event_payloads: &mut BTreeMap<String, Vec<ApiArgumentDoc>>,
) -> Result<(), XtaskError> {
    for item in source_items {
        match item {
            Item::Struct(item) if is_public(&item.vis) => push_struct_doc(source, items, &item)?,
            Item::Enum(item) if is_public(&item.vis) => {
                if item.ident == "PluginEvent" {
                    collect_event_payloads(&item, event_payloads);
                }
                push_enum_doc(source, items, &item)?;
            }
            Item::Type(item) if is_public(&item.vis) => push_type_doc(source, items, &item)?,
            Item::Const(item) if is_public(&item.vis) => push_const_doc(source, items, &item)?,
            Item::Fn(item) if is_public(&item.vis) => push_fn_doc(source, items, &item)?,
            Item::Impl(item) => push_impl_docs(source, items, &item)?,
            _ => {}
        }
    }
    Ok(())
}

fn push_struct_doc(
    source: SdkSource,
    items: &mut Vec<ApiItemDoc>,
    item: &ItemStruct,
) -> Result<(), XtaskError> {
    let name = item.ident.to_string();
    let docs = required_doc_block(source.path, &name, &item.attrs)?;
    if matches!(item.fields, Fields::Unit) {
        require_section(source.path, &name, &docs, "# SDK Notes")?;
    } else {
        require_section(source.path, &name, &docs, "# Fields")?;
    }
    let mut api_item = base_item(
        source,
        name,
        docs,
        ApiGroup::Structures,
        SdkItemKind::Struct,
    );
    api_item.docs = without_generated_sections(api_item.docs, &["Fields", "SDK Notes"]);
    api_item.signature = Some(format!("pub struct {}", item.ident));
    api_item.fields = struct_fields(item);
    items.push(api_item);
    Ok(())
}

fn push_enum_doc(
    source: SdkSource,
    items: &mut Vec<ApiItemDoc>,
    item: &ItemEnum,
) -> Result<(), XtaskError> {
    let name = item.ident.to_string();
    let docs = required_doc_block(source.path, &name, &item.attrs)?;
    require_section(source.path, &name, &docs, "# Variants")?;
    let mut api_item = base_item(source, name, docs, ApiGroup::Enums, SdkItemKind::Enum);
    api_item.docs = without_generated_sections(api_item.docs, &["Variants"]);
    api_item.signature = Some(format!("pub enum {}", item.ident));
    api_item.variants = enum_variants(item);
    items.push(api_item);
    if item.ident == "PluginEventKind" {
        push_event_docs(source, items, item)?;
    }
    Ok(())
}

fn push_event_docs(
    source: SdkSource,
    items: &mut Vec<ApiItemDoc>,
    item: &ItemEnum,
) -> Result<(), XtaskError> {
    for variant in &item.variants {
        let event_name = format!("PluginEventKind::{}", variant.ident);
        let docs = required_doc_block(source.path, &event_name, &variant.attrs)?;
        require_section(source.path, &event_name, &docs, "# SDK Example")?;
        items.push(base_item(
            source,
            event_name,
            docs,
            ApiGroup::Events,
            SdkItemKind::Event,
        ));
    }
    Ok(())
}

fn push_type_doc(
    source: SdkSource,
    items: &mut Vec<ApiItemDoc>,
    item: &ItemType,
) -> Result<(), XtaskError> {
    let name = item.ident.to_string();
    let docs = required_doc_block(source.path, &name, &item.attrs)?;
    if name.ends_with("Fn") {
        require_section(source.path, &name, &docs, "# SDK Example")?;
    } else {
        require_any_section(source.path, &name, &docs)?;
    }
    let bare_fn = match item.ty.as_ref() {
        Type::BareFn(bare) => Some(bare),
        _ => None,
    };
    let group = if bare_fn.is_some() {
        ApiGroup::Methods
    } else {
        ApiGroup::Structures
    };
    let mut api_item = base_item(source, name, docs, group, SdkItemKind::TypeAlias);
    api_item.signature = Some(format!("pub type {} = {};", item.ident, tokens(&item.ty)));
    if let Some(bare) = bare_fn {
        api_item.arguments = bare_fn_arguments(bare);
        api_item.return_value = return_type(&bare.output);
    }
    items.push(api_item);
    Ok(())
}

fn push_const_doc(
    source: SdkSource,
    items: &mut Vec<ApiItemDoc>,
    item: &ItemConst,
) -> Result<(), XtaskError> {
    let name = item.ident.to_string();
    let docs = required_doc_block(source.path, &name, &item.attrs)?;
    if name.starts_with("FLUX_PLUGIN_") || name.contains("PLUGIN_API_VERSION") {
        require_section(source.path, &name, &docs, "# SDK Example")?;
    } else {
        require_any_section(source.path, &name, &docs)?;
    }
    let mut api_item = base_item(source, name, docs, ApiGroup::Constants, SdkItemKind::Const);
    api_item.signature = Some(format!(
        "pub const {}: {} = ...;",
        item.ident,
        tokens(&item.ty)
    ));
    items.push(api_item);
    Ok(())
}

fn push_fn_doc(
    source: SdkSource,
    items: &mut Vec<ApiItemDoc>,
    item: &ItemFn,
) -> Result<(), XtaskError> {
    let name = item.sig.ident.to_string();
    let docs = required_doc_block(source.path, &name, &item.attrs)?;
    require_section(source.path, &name, &docs, "# SDK Example")?;
    let mut api_item = base_item(source, name, docs, ApiGroup::Methods, SdkItemKind::Function);
    api_item.signature = Some(tokens(&item.sig));
    api_item.arguments = signature_arguments(&item.sig.inputs);
    api_item.return_value = return_type(&item.sig.output);
    items.push(api_item);
    Ok(())
}

fn push_impl_docs(
    source: SdkSource,
    items: &mut Vec<ApiItemDoc>,
    item: &ItemImpl,
) -> Result<(), XtaskError> {
    let Some(owner) = impl_owner_name(&item.self_ty) else {
        return Ok(());
    };
    for impl_item in &item.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };
        if !is_public(&method.vis) {
            continue;
        }
        let name = format!("{}::{}", owner, method.sig.ident);
        let docs = required_doc_block(source.path, &name, &method.attrs)?;
        require_section(source.path, &name, &docs, "# SDK Example")?;
        let mut api_item = base_item(source, name, docs, ApiGroup::Methods, SdkItemKind::Method);
        api_item.signature = Some(tokens(&method.sig));
        api_item.arguments = signature_arguments(&method.sig.inputs);
        api_item.return_value = return_type(&method.sig.output);
        items.push(api_item);
    }
    Ok(())
}

fn base_item(
    source: SdkSource,
    name: String,
    docs: DocBlock,
    group: ApiGroup,
    kind: SdkItemKind,
) -> ApiItemDoc {
    ApiItemDoc {
        file_name: format!("{}.md", slugify(&name)),
        name,
        group,
        kind,
        category: if group == ApiGroup::Events {
            SdkCategory::Event
        } else {
            source.category
        },
        source_path: source.path.to_string(),
        source_title: source.title.to_string(),
        docs,
        signature: None,
        fields: Vec::new(),
        variants: Vec::new(),
        arguments: Vec::new(),
        return_value: None,
    }
}

fn struct_fields(item: &ItemStruct) -> Vec<ApiFieldDoc> {
    match &item.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter(|field| is_public(&field.vis))
            .filter_map(|field| {
                let name = field.ident.as_ref()?.to_string();
                let ty = tokens(&field.ty);
                Some(ApiFieldDoc {
                    description: optional_doc_summary(&field.attrs)
                        .unwrap_or_else(|| default_field_description(&name, &ty)),
                    name,
                    ty,
                })
            })
            .collect(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .filter(|(_, field)| is_public(&field.vis))
            .map(|(index, field)| {
                let name = index.to_string();
                let ty = tokens(&field.ty);
                ApiFieldDoc {
                    description: optional_doc_summary(&field.attrs)
                        .unwrap_or_else(|| default_field_description(&name, &ty)),
                    name,
                    ty,
                }
            })
            .collect(),
        Fields::Unit => Vec::new(),
    }
}

fn enum_variants(item: &ItemEnum) -> Vec<ApiVariantDoc> {
    item.variants
        .iter()
        .map(|variant| ApiVariantDoc {
            name: variant.ident.to_string(),
            payload: variant_payload(&variant.fields),
            description: optional_doc_summary(&variant.attrs)
                .unwrap_or_else(|| format!("`{}` variant.", variant.ident)),
        })
        .collect()
}

fn collect_event_payloads(item: &ItemEnum, payloads: &mut BTreeMap<String, Vec<ApiArgumentDoc>>) {
    for variant in &item.variants {
        payloads.insert(
            variant.ident.to_string(),
            variant_fields_as_arguments(&variant.fields),
        );
    }
}

fn variant_fields_as_arguments(fields: &Fields) -> Vec<ApiArgumentDoc> {
    match fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter_map(|field| {
                let name = field.ident.as_ref()?.to_string();
                let ty = tokens(&field.ty);
                Some(ApiArgumentDoc {
                    description: optional_doc_summary(&field.attrs)
                        .unwrap_or_else(|| default_argument_description(&name, &ty)),
                    name,
                    ty,
                })
            })
            .collect(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let name = index.to_string();
                let ty = tokens(&field.ty);
                ApiArgumentDoc {
                    description: optional_doc_summary(&field.attrs)
                        .unwrap_or_else(|| default_argument_description(&name, &ty)),
                    name,
                    ty,
                }
            })
            .collect(),
        Fields::Unit => Vec::new(),
    }
}

fn signature_arguments(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> Vec<ApiArgumentDoc> {
    inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Receiver(_) => None,
            FnArg::Typed(pat_type) => {
                let name = pat_name(&pat_type.pat);
                let ty = tokens(&pat_type.ty);
                Some(ApiArgumentDoc {
                    description: default_argument_description(&name, &ty),
                    name,
                    ty,
                })
            }
        })
        .collect()
}

fn bare_fn_arguments(bare: &TypeBareFn) -> Vec<ApiArgumentDoc> {
    bare.inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let name = input
                .name
                .as_ref()
                .map(|(name, _)| name.to_string())
                .unwrap_or_else(|| index.to_string());
            let ty = tokens(&input.ty);
            ApiArgumentDoc {
                description: default_argument_description(&name, &ty),
                name,
                ty,
            }
        })
        .collect()
}

fn return_type(output: &ReturnType) -> Option<String> {
    match output {
        ReturnType::Default => Some("()".to_string()),
        ReturnType::Type(_, ty) => Some(tokens(ty)),
    }
}

fn variant_payload(fields: &Fields) -> String {
    match fields {
        Fields::Unit => "none".to_string(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .map(|field| tokens(&field.ty))
            .collect::<Vec<_>>()
            .join(", "),
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter_map(|field| Some(format!("{}: {}", field.ident.as_ref()?, tokens(&field.ty))))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn without_generated_sections(mut docs: DocBlock, section_names: &[&str]) -> DocBlock {
    docs.sections
        .retain(|section| !section_names.contains(&section.title.as_str()));
    docs
}

fn pat_name(pat: &Pat) -> String {
    match pat {
        Pat::Ident(ident) => ident.ident.to_string(),
        _ => "_".to_string(),
    }
}

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn default_field_description(name: &str, ty: &str) -> String {
    format!("`{}` field stored as `{}`.", name, ty)
}

fn default_argument_description(name: &str, ty: &str) -> String {
    format!("`{}` argument passed as `{}`.", name, ty)
}
