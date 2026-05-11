use std::{collections::BTreeMap, fs, path::Path};

use quote::ToTokens;
use syn::{
    Fields, FnArg, ImplItem, Item, ItemConst, ItemEnum, ItemFn, ItemImpl, ItemStruct, ItemType,
    Pat, ReturnType, Type, TypeBareFn,
};

use crate::{
    plugin_sdk_docs::{
        model::{
            ApiArgumentDoc, ApiExampleDoc, ApiFieldDoc, ApiGroup, ApiItemDoc, ApiMethodLink,
            ApiVariantDoc, SdkCategory, SdkItemKind, SdkSource, DOCS_SRC_ROOT, SDK_SOURCES,
        },
        parser::{
            impl_owner_name, is_public, optional_doc_summary, require_section, required_doc_block,
            section_body, DocBlock,
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
        if item.group != ApiGroup::Events {
            continue;
        }
        let variant = item
            .name
            .strip_prefix("PluginEvent::")
            .unwrap_or(&item.name);
        item.arguments = event_payloads.remove(variant).unwrap_or_default();
    }

    let mut methods_by_owner = BTreeMap::<String, Vec<ApiMethodLink>>::new();
    for item in &items {
        if item.group != ApiGroup::Methods {
            continue;
        }
        let Some((owner, _)) = item.name.split_once("::") else {
            continue;
        };
        methods_by_owner
            .entry(owner.to_string())
            .or_default()
            .push(ApiMethodLink {
                name: item.name.clone(),
                file_name: item.file_name.clone(),
            });
    }
    for methods in methods_by_owner.values_mut() {
        methods.sort_by(|left, right| left.name.cmp(&right.name));
    }

    for item in &mut items {
        item.methods = methods_by_owner.remove(&item.name).unwrap_or_default();
        if let Some(relative_path) = ApiExampleDoc::relative_path_for(item) {
            let absolute_path = repo_root.join(DOCS_SRC_ROOT).join(&relative_path);
            let contents = fs::read_to_string(&absolute_path).map_err(|error| {
                XtaskError::new(format!(
                    "missing Plugin SDK example for `{}`; expected '{}': {}",
                    item.name,
                    absolute_path.display(),
                    error
                ))
            })?;
            if contents.trim().is_empty() {
                return Err(XtaskError::new(format!(
                    "Plugin SDK example for `{}` is empty: '{}'",
                    item.name,
                    absolute_path.display()
                )));
            }
            item.example = Some(ApiExampleDoc {
                relative_path,
                contents: contents.trim().to_string(),
            });
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
            Item::Struct(item)
                if is_public(&item.vis) && source.includes(&item.ident.to_string()) =>
            {
                push_struct_doc(source, items, &item)?
            }
            Item::Enum(item) => {
                if item.ident == "PluginRuntimeEvent" {
                    collect_event_payloads(&item, event_payloads);
                }
                if is_public(&item.vis) && source.includes(&item.ident.to_string()) {
                    push_enum_doc(source, items, &item)?;
                }
            }
            Item::Type(item)
                if is_public(&item.vis) && source.includes(&item.ident.to_string()) =>
            {
                push_type_doc(source, items, &item)?
            }
            Item::Const(item)
                if is_public(&item.vis) && source.includes(&item.ident.to_string()) =>
            {
                push_const_doc(source, items, &item)?
            }
            Item::Fn(item)
                if is_public(&item.vis) && source.includes(&item.sig.ident.to_string()) =>
            {
                push_fn_doc(source, items, &item)?
            }
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
    } else if has_public_fields(item) {
        require_section(source.path, &name, &docs, "# Fields")?;
    }

    let fields = struct_fields(source.path, &name, item, &docs)?;
    let mut api_item = base_item(
        source,
        name,
        without_generated_sections(docs, &["Fields", "SDK Notes", "SDK Example"]),
        ApiGroup::Structures,
        SdkItemKind::Struct,
    );
    api_item.signature = Some(format!("pub struct {}", item.ident));
    api_item.fields = fields;
    items.push(api_item);
    Ok(())
}

fn has_public_fields(item: &ItemStruct) -> bool {
    match &item.fields {
        Fields::Named(fields) => fields.named.iter().any(|field| is_public(&field.vis)),
        Fields::Unnamed(fields) => fields.unnamed.iter().any(|field| is_public(&field.vis)),
        Fields::Unit => false,
    }
}

fn push_enum_doc(
    source: SdkSource,
    items: &mut Vec<ApiItemDoc>,
    item: &ItemEnum,
) -> Result<(), XtaskError> {
    let name = item.ident.to_string();
    let docs = required_doc_block(source.path, &name, &item.attrs)?;
    require_section(source.path, &name, &docs, "# Variants")?;

    let variants = enum_variants(source.path, &name, item, &docs)?;
    let mut api_item = base_item(
        source,
        name,
        without_generated_sections(docs, &["Variants", "SDK Example"]),
        ApiGroup::Enums,
        SdkItemKind::Enum,
    );
    api_item.signature = Some(format!("pub enum {}", item.ident));
    api_item.variants = variants;
    items.push(api_item);
    if item.ident == "PluginEvent" {
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
        let event_name = format!("PluginEvent::{}", variant.ident);
        let docs = required_doc_block(source.path, &event_name, &variant.attrs)?;
        items.push(base_item(
            source,
            event_name,
            without_generated_sections(docs, &["SDK Example"]),
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

    let bare_fn = match item.ty.as_ref() {
        Type::BareFn(bare) => Some(bare),
        _ => None,
    };
    let group = if bare_fn.is_some() {
        ApiGroup::Methods
    } else {
        ApiGroup::Structures
    };
    let mut api_item = base_item(
        source,
        name,
        without_generated_sections(docs, &["SDK Example"]),
        group,
        SdkItemKind::TypeAlias,
    );
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

    let mut api_item = base_item(
        source,
        name,
        without_generated_sections(docs, &["SDK Example"]),
        ApiGroup::Constants,
        SdkItemKind::Const,
    );
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

    let mut api_item = base_item(
        source,
        name,
        without_generated_sections(docs, &["SDK Example"]),
        ApiGroup::Methods,
        SdkItemKind::Function,
    );
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
    if !source.includes_owner(&owner) {
        return Ok(());
    }

    for impl_item in &item.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };
        if !is_public(&method.vis) {
            continue;
        }
        let name = format!("{}::{}", owner, method.sig.ident);
        let docs = required_doc_block(source.path, &name, &method.attrs)?;
        let mut api_item = base_item(
            source,
            name,
            without_generated_sections(docs, &["SDK Example"]),
            ApiGroup::Methods,
            SdkItemKind::Method,
        );
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
        methods: Vec::new(),
        example: None,
    }
}

fn struct_fields(
    source_path: &str,
    item_name: &str,
    item: &ItemStruct,
    docs: &DocBlock,
) -> Result<Vec<ApiFieldDoc>, XtaskError> {
    let descriptions = if matches!(item.fields, Fields::Unit) || !has_public_fields(item) {
        BTreeMap::new()
    } else {
        section_entries(source_path, item_name, docs, "Fields")?
    };
    match &item.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter(|field| is_public(&field.vis))
            .map(|field| {
                let name = field.ident.as_ref().expect("named field").to_string();
                Ok(ApiFieldDoc {
                    name: name.clone(),
                    ty: tokens(&field.ty),
                    description: explicit_field_description(
                        source_path,
                        item_name,
                        &name,
                        &field.attrs,
                        &descriptions,
                    )?,
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
                Ok(ApiFieldDoc {
                    name: name.clone(),
                    ty: tokens(&field.ty),
                    description: explicit_field_description(
                        source_path,
                        item_name,
                        &name,
                        &field.attrs,
                        &descriptions,
                    )?,
                })
            })
            .collect(),
        Fields::Unit => Ok(Vec::new()),
    }
}

fn enum_variants(
    source_path: &str,
    item_name: &str,
    item: &ItemEnum,
    docs: &DocBlock,
) -> Result<Vec<ApiVariantDoc>, XtaskError> {
    let descriptions = section_entries(source_path, item_name, docs, "Variants")?;
    item.variants
        .iter()
        .map(|variant| {
            let name = variant.ident.to_string();
            let description = optional_doc_summary(&variant.attrs)
                .or_else(|| descriptions.get(&name).cloned())
                .ok_or_else(|| {
                    XtaskError::new(format!(
                        "{}: SDK item `{}` is missing a description for variant `{}`",
                        source_path, item_name, name
                    ))
                })?;
            Ok(ApiVariantDoc {
                name,
                payload: variant_payload(&variant.fields),
                description,
            })
        })
        .collect()
}

fn explicit_field_description(
    source_path: &str,
    item_name: &str,
    field_name: &str,
    attrs: &[syn::Attribute],
    descriptions: &BTreeMap<String, String>,
) -> Result<String, XtaskError> {
    optional_doc_summary(attrs)
        .or_else(|| descriptions.get(field_name).cloned())
        .ok_or_else(|| {
            XtaskError::new(format!(
                "{}: SDK item `{}` is missing a description for field `{}`",
                source_path, item_name, field_name
            ))
        })
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
                let name = if fields.unnamed.len() == 1 {
                    "payload".to_string()
                } else {
                    format!("payload{}", index)
                };
                let ty = tokens(&field.ty);
                ApiArgumentDoc {
                    description: optional_doc_summary(&field.attrs).unwrap_or_else(|| {
                        format!("Typed payload passed to this event as `{}`.", ty)
                    }),
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

fn section_entries(
    source_path: &str,
    item_name: &str,
    docs: &DocBlock,
    section_name: &str,
) -> Result<BTreeMap<String, String>, XtaskError> {
    let Some(body) = section_body(docs, section_name) else {
        return Err(XtaskError::new(format!(
            "{}: SDK item `{}` must contain `# {}` doc section",
            source_path, item_name, section_name
        )));
    };
    parse_named_entries(source_path, item_name, section_name, body)
}

fn parse_named_entries(
    source_path: &str,
    item_name: &str,
    section_name: &str,
    body: &str,
) -> Result<BTreeMap<String, String>, XtaskError> {
    let mut entries = BTreeMap::new();
    for raw_line in body.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(rest) = line.strip_prefix("- `") else {
            return Err(XtaskError::new(format!(
                "{}: SDK item `{}` section `# {}` must use `- `name`: description` entries",
                source_path, item_name, section_name
            )));
        };
        let Some((name, description)) = rest.split_once("`: ") else {
            return Err(XtaskError::new(format!(
                "{}: SDK item `{}` section `# {}` has invalid entry `{}`",
                source_path, item_name, section_name, line
            )));
        };
        let description = description.trim();
        if description.is_empty() {
            return Err(XtaskError::new(format!(
                "{}: SDK item `{}` section `# {}` has empty description for `{}`",
                source_path, item_name, section_name, name
            )));
        }
        entries.insert(name.to_string(), description.to_string());
    }
    Ok(entries)
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

fn default_argument_description(name: &str, ty: &str) -> String {
    if name == "payload" {
        return format!("Typed payload passed to this callback as `{}`.", ty);
    }
    if name.starts_with("out_") {
        return format!("Output pointer filled by the callee as `{}`.", ty);
    }
    format!("`{}` argument passed as `{}`.", name, ty)
}
