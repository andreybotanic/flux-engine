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
            impl_owner_name, is_doc_hidden, is_public, optional_doc_summary, required_doc_block,
            section_body, DocBlock,
        },
    },
    XtaskError,
};

/// Collects all generated Plugin SDK item docs from configured Rust sources.
pub(super) fn collect_api_items(repo_root: &Path) -> Result<Vec<ApiItemDoc>, XtaskError> {
    let mut items = Vec::new();
    let mut payload_structs = BTreeMap::<String, Vec<ApiArgumentDoc>>::new();
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
        collect_source_items(
            *source,
            parsed.items,
            &mut items,
            &mut payload_structs,
            &mut event_payloads,
        )?;
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
            match fs::read_to_string(&absolute_path) {
                Ok(contents) => {
                    if contents.trim().is_empty() {
                        return Err(XtaskError::new(format!(
                            "Plugin SDK example for `{}` is empty: '{}'",
                            item.name,
                            absolute_path.display()
                        )));
                    }
                    if contains_legacy_plugin_api_markers(&contents) {
                        continue;
                    }
                    item.example = Some(ApiExampleDoc {
                        relative_path,
                        contents: contents.trim().to_string(),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(XtaskError::new(format!(
                        "failed to read Plugin SDK example for `{}` from '{}': {}",
                        item.name,
                        absolute_path.display(),
                        error
                    )));
                }
            }
        }
    }

    Ok(items)
}

fn collect_source_items(
    source: SdkSource,
    source_items: Vec<Item>,
    items: &mut Vec<ApiItemDoc>,
    payload_structs: &mut BTreeMap<String, Vec<ApiArgumentDoc>>,
    event_payloads: &mut BTreeMap<String, Vec<ApiArgumentDoc>>,
) -> Result<(), XtaskError> {
    for item in source_items {
        match item {
            Item::Struct(item)
                if is_public(&item.vis)
                    && !is_doc_hidden(&item.attrs)
                    && source.includes(&item.ident.to_string()) =>
            {
                push_struct_doc(source, items, &item)?;
                if source.category == SdkCategory::Event {
                    payload_structs.insert(
                        item.ident.to_string(),
                        struct_fields_as_arguments(source.path, &item.ident.to_string(), &item)?,
                    );
                }
            }
            Item::Enum(item) => {
                if is_public(&item.vis)
                    && !is_doc_hidden(&item.attrs)
                    && source.includes(&item.ident.to_string())
                {
                    push_enum_doc(source, items, &item)?;
                }
            }
            Item::Type(item)
                if is_public(&item.vis)
                    && !is_doc_hidden(&item.attrs)
                    && source.includes(&item.ident.to_string()) =>
            {
                push_type_doc(source, items, &item)?
            }
            Item::Const(item)
                if is_public(&item.vis)
                    && !is_doc_hidden(&item.attrs)
                    && source.includes(&item.ident.to_string()) =>
            {
                push_const_doc(source, items, &item)?
            }
            Item::Fn(item)
                if is_public(&item.vis)
                    && !is_doc_hidden(&item.attrs)
                    && source.includes(&item.sig.ident.to_string()) =>
            {
                push_fn_doc(source, items, &item)?
            }
            Item::Impl(item) => {
                collect_abi_event_payload_impl(&item, payload_structs, event_payloads);
                push_impl_docs(source, items, &item)?
            }
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
    let event_variants = api_item.variants.clone();
    items.push(api_item);
    if item.ident == "PluginEvent" {
        push_event_docs(source, items, item, &event_variants);
    }
    Ok(())
}

fn push_event_docs(
    source: SdkSource,
    items: &mut Vec<ApiItemDoc>,
    item: &ItemEnum,
    variants: &[ApiVariantDoc],
) {
    for variant in &item.variants {
        let event_name = format!("PluginEvent::{}", variant.ident);
        let summary = optional_doc_summary(&variant.attrs)
            .or_else(|| {
                variants
                    .iter()
                    .find(|candidate| candidate.name == variant.ident.to_string())
                    .map(|candidate| candidate.description.clone())
            })
            .unwrap_or_else(|| {
                format!(
                    "Raised when the engine dispatches the `{}` runtime event.",
                    variant.ident
                )
            });
        items.push(base_item(
            source,
            event_name,
            DocBlock {
                summary,
                sections: Vec::new(),
            },
            ApiGroup::Events,
            SdkItemKind::Event,
        ));
    }
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
        if is_doc_hidden(&method.attrs) {
            continue;
        }
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
    _source_path: &str,
    item_name: &str,
    item: &ItemStruct,
    docs: &DocBlock,
) -> Result<Vec<ApiFieldDoc>, XtaskError> {
    let descriptions = if matches!(item.fields, Fields::Unit) || !has_public_fields(item) {
        BTreeMap::new()
    } else {
        optional_section_entries(docs, "Fields")
    };
    match &item.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter(|field| is_public(&field.vis))
            .map(|field| {
                let name = field.ident.as_ref().expect("named field").to_string();
                let ty = tokens(&field.ty);
                Ok(ApiFieldDoc {
                    name: name.clone(),
                    ty: ty.clone(),
                    description: explicit_field_description(
                        item_name,
                        &name,
                        &ty,
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
                let ty = tokens(&field.ty);
                Ok(ApiFieldDoc {
                    name: name.clone(),
                    ty: ty.clone(),
                    description: explicit_field_description(
                        item_name,
                        &name,
                        &ty,
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
    _source_path: &str,
    item_name: &str,
    item: &ItemEnum,
    docs: &DocBlock,
) -> Result<Vec<ApiVariantDoc>, XtaskError> {
    let descriptions = optional_section_entries(docs, "Variants");
    item.variants
        .iter()
        .map(|variant| {
            let name = variant.ident.to_string();
            let payload = variant_payload(&variant.fields);
            let description = optional_doc_summary(&variant.attrs)
                .or_else(|| descriptions.get(&name).cloned())
                .unwrap_or_else(|| default_variant_description(item_name, &name, &payload));
            Ok(ApiVariantDoc {
                name,
                payload,
                description,
            })
        })
        .collect()
}

fn explicit_field_description(
    item_name: &str,
    field_name: &str,
    field_ty: &str,
    attrs: &[syn::Attribute],
    descriptions: &BTreeMap<String, String>,
) -> Result<String, XtaskError> {
    Ok(optional_doc_summary(attrs)
        .or_else(|| descriptions.get(field_name).cloned())
        .unwrap_or_else(|| default_field_description(item_name, field_name, field_ty)))
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

fn struct_fields_as_arguments(
    _source_path: &str,
    item_name: &str,
    item: &ItemStruct,
) -> Result<Vec<ApiArgumentDoc>, XtaskError> {
    let descriptions = BTreeMap::<String, String>::new();
    match &item.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter(|field| is_public(&field.vis))
            .map(|field| {
                let name = field.ident.as_ref().expect("named field").to_string();
                let ty = tokens(&field.ty);
                Ok(ApiArgumentDoc {
                    name: name.clone(),
                    ty: ty.clone(),
                    description: optional_doc_summary(&field.attrs)
                        .or_else(|| descriptions.get(&name).cloned())
                        .unwrap_or_else(|| default_field_description(item_name, &name, &ty)),
                })
            })
            .collect(),
        Fields::Unnamed(fields) => Ok(variant_fields_as_arguments(&Fields::Unnamed(fields.clone()))),
        Fields::Unit => Ok(Vec::new()),
    }
}

fn collect_abi_event_payload_impl(
    item: &ItemImpl,
    payload_structs: &BTreeMap<String, Vec<ApiArgumentDoc>>,
    event_payloads: &mut BTreeMap<String, Vec<ApiArgumentDoc>>,
) {
    let Some((_, trait_path, _)) = &item.trait_ else {
        return;
    };
    let Some(trait_name) = trait_path.segments.last().map(|segment| segment.ident.to_string()) else {
        return;
    };
    if trait_name != "AbiEventPayload" {
        return;
    }
    let Some(owner) = impl_owner_name(&item.self_ty) else {
        return;
    };
    let Some(event_name) = item.items.iter().find_map(|impl_item| {
        let ImplItem::Const(impl_const) = impl_item else {
            return None;
        };
        if impl_const.ident != "KIND" {
            return None;
        }
        let syn::Expr::Path(path) = &impl_const.expr else {
            return None;
        };
        path.path.segments.last().map(|segment| segment.ident.to_string())
    }) else {
        return;
    };
    let arguments = payload_structs.get(&owner).cloned().unwrap_or_default();
    for alias in event_payload_aliases(&event_name) {
        event_payloads.insert(alias.to_string(), arguments.clone());
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

fn optional_section_entries(docs: &DocBlock, section_name: &str) -> BTreeMap<String, String> {
    section_body(docs, section_name)
        .and_then(|body| parse_named_entries("", "", section_name, body).ok())
        .unwrap_or_default()
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

fn default_field_description(item_name: &str, field_name: &str, field_ty: &str) -> String {
    if field_name == "0" {
        return format!("Wrapped `{}` value stored by `{}`.", field_ty, item_name);
    }
    format!("`{}` field stored as `{}` on `{}`.", field_name, field_ty, item_name)
}

fn default_variant_description(item_name: &str, variant_name: &str, payload: &str) -> String {
    if payload == "none" {
        return format!("`{}` variant of `{}`.", variant_name, item_name);
    }
    format!(
        "`{}` variant of `{}` carrying `{}`.",
        variant_name, item_name, payload
    )
}

fn event_payload_aliases(event_name: &str) -> &'static [&'static str] {
    match event_name {
        "MouseDownCell" => &[
            "MouseDownCell",
            "MouseMoveCell",
            "MouseUpCell",
            "MouseEnterCell",
            "MouseLeaveCell",
        ],
        "KeyPressed" => &["KeyPressed", "KeyReleased"],
        "EntityPlaced" => &["EntityPlaced", "EntityRemoved"],
        "WorldCreated" => &["WorldCreated"],
        "WorldLoaded" => &["WorldLoaded"],
        "WorldBeforeSave" => &["WorldBeforeSave"],
        "WorldAfterSave" => &["WorldAfterSave"],
        "WorldUnloaded" => &["WorldUnloaded"],
        "SimulationPreCellGasStep" => &["SimulationPreCellGasStep"],
        "SimulationPostCellGasStep" => &["SimulationPostCellGasStep"],
        "SimulationPausedChanged" => &["SimulationPausedChanged"],
        "ToolSelected" => &["ToolSelected"],
        "OverlayChanged" => &["OverlayChanged"],
        "BuildHudForCell" => &["BuildHudForCell"],
        "BuildPanel" => &["BuildPanel"],
        "RenderOverlay" => &["RenderOverlay"],
        _ => &[],
    }
}

fn contains_legacy_plugin_api_markers(contents: &str) -> bool {
    [
        "FluxRuntimeHost",
        "FluxRegistrar",
        "FluxPluginHandle",
        "FluxStatus",
        "FluxUtf8Slice",
        "extern \"C\"",
    ]
    .iter()
    .any(|marker| contents.contains(marker))
}
