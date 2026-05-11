use std::collections::BTreeMap;

use crate::plugin_sdk_docs::model::{
    ApiArgumentDoc, ApiFieldDoc, ApiGroup, ApiItemDoc, ApiVariantDoc,
};

type ItemLinkIndex = BTreeMap<String, (ApiGroup, String)>;

/// Renders one generated group index page.
pub(super) fn render_index(contents: &mut String, title: &str, items: &[ApiItemDoc]) {
    render_generated_header(contents, title);
    contents.push_str(&format!("# {}\n\n", title));
    contents.push_str("| API item | Kind | Source |\n");
    contents.push_str("| --- | --- | --- |\n");
    for item in items {
        contents.push_str(&format!(
            "| [`{}`]({}) | {} | `{}` |\n",
            item.name,
            item.relative_link(),
            item.kind.label(),
            item.source_path
        ));
    }
    contents.push('\n');
}

/// Renders one concrete API item page.
pub(super) fn render_item_page(contents: &mut String, item: &ApiItemDoc, items: &[ApiItemDoc]) {
    let link_index = build_item_link_index(items);

    render_generated_header(contents, &item.name);
    contents.push_str(&format!("# `{}`\n\n", item.name));
    contents.push_str(&format!(
        "<span class=\"sdk-badge sdk-badge-{}\">{}</span> <span class=\"sdk-kind\">{}</span>\n\n",
        item.category.badge(),
        item.category.badge(),
        item.kind.label()
    ));
    contents.push_str(&format!(
        "Source: **{}** (`{}`). Generated group: **{}**.\n\n",
        item.source_title,
        item.source_path,
        item.group.label()
    ));

    match item.group {
        ApiGroup::Structures => render_structure_page(contents, item, &link_index),
        ApiGroup::Enums => render_enum_page(contents, item, &link_index),
        ApiGroup::Constants => render_constant_page(contents, item),
        ApiGroup::Methods => render_method_page(contents, item, &link_index),
        ApiGroup::Events => render_event_page(contents, item, &link_index),
    }
}

/// Renders the mdBook summary with generated API groups and item links.
pub(super) fn render_summary(contents: &mut String, groups: &[(ApiGroup, Vec<&ApiItemDoc>)]) {
    contents.push_str("# Summary\n\n");
    contents.push_str("[Introduction](README.md)\n\n");
    contents.push_str("# Guide\n\n");
    contents.push_str("- [Plugin system overview](overview.md)\n");
    contents.push_str("- [Runtime plugin lifecycle](lifecycle.md)\n");
    contents.push_str("- [Plugin package structure](package-structure.md)\n");
    contents.push_str("- [Manifest](manifest-guide.md)\n");
    contents.push_str("- [Build and dev reload](build-and-reload.md)\n\n");
    contents.push_str("# Generated API\n\n");
    for (group, items) in groups {
        contents.push_str(&format!(
            "- [{}](generated/{})\n",
            group.label(),
            group.index_file_name()
        ));
        for item in items {
            contents.push_str(&format!(
                "  - [`{}`](generated/{}/{})\n",
                item.name,
                group.directory_name(),
                item.file_name
            ));
        }
    }
}

fn render_structure_page(contents: &mut String, item: &ApiItemDoc, links: &ItemLinkIndex) {
    contents.push_str("## Description\n\n");
    contents.push_str(&item.docs.summary);
    contents.push_str("\n\n");
    if let Some(signature) = &item.signature {
        contents.push_str("## Declaration\n\n");
        contents.push_str("```rust\n");
        contents.push_str(signature);
        contents.push_str("\n```\n\n");
    }
    if !item.fields.is_empty() {
        contents.push_str("## Fields\n\n");
        render_field_table(contents, item.group, &item.fields, links);
    }
    if !item.methods.is_empty() {
        contents.push_str("## Methods\n\n");
        for method in &item.methods {
            contents.push_str(&format!(
                "- [`{}`](../methods/{})\n",
                method.name, method.file_name
            ));
        }
        contents.push('\n');
    }
    if !item.variants.is_empty() {
        contents.push_str("## Variants\n\n");
        render_variant_table(contents, item.group, &item.variants, links);
    }
    render_extra_sections(contents, item);
    render_example(contents, item);
}

fn render_enum_page(contents: &mut String, item: &ApiItemDoc, links: &ItemLinkIndex) {
    contents.push_str("## Description\n\n");
    contents.push_str(&item.docs.summary);
    contents.push_str("\n\n");
    if let Some(signature) = &item.signature {
        contents.push_str("## Declaration\n\n");
        contents.push_str("```rust\n");
        contents.push_str(signature);
        contents.push_str("\n```\n\n");
    }
    if !item.variants.is_empty() {
        contents.push_str("## Variants\n\n");
        render_variant_table(contents, item.group, &item.variants, links);
    }
    render_extra_sections(contents, item);
    render_example(contents, item);
}

fn render_constant_page(contents: &mut String, item: &ApiItemDoc) {
    contents.push_str("## Description\n\n");
    contents.push_str(&item.docs.summary);
    contents.push_str("\n\n");
    if let Some(signature) = &item.signature {
        contents.push_str("## Declaration\n\n");
        contents.push_str("```rust\n");
        contents.push_str(signature);
        contents.push_str("\n```\n\n");
    }
    render_extra_sections(contents, item);
    render_example(contents, item);
}

fn render_method_page(contents: &mut String, item: &ApiItemDoc, links: &ItemLinkIndex) {
    contents.push_str("## Description\n\n");
    contents.push_str(&item.docs.summary);
    contents.push_str("\n\n");
    if let Some(signature) = &item.signature {
        contents.push_str("## Signature\n\n");
        contents.push_str("```rust\n");
        contents.push_str(signature);
        contents.push_str("\n```\n\n");
    }
    contents.push_str("## Arguments\n\n");
    if item.arguments.is_empty() {
        contents.push_str("This method does not take plugin-supplied arguments.\n\n");
    } else {
        render_argument_table(contents, item.group, &item.arguments, links);
    }
    contents.push_str("## Return Value\n\n");
    contents.push_str(&render_type_markdown(
        item.return_value.as_deref().unwrap_or("()"),
        item.group,
        links,
    ));
    contents.push_str("\n\n");
    render_extra_sections(contents, item);
    render_example(contents, item);
}

fn render_event_page(contents: &mut String, item: &ApiItemDoc, links: &ItemLinkIndex) {
    contents.push_str("## When It Fires\n\n");
    contents.push_str(&item.docs.summary);
    contents.push_str("\n\n");
    contents.push_str("## Arguments\n\n");
    if item.arguments.is_empty() {
        contents.push_str("This event does not carry additional payload fields.\n\n");
    } else {
        render_argument_table(contents, item.group, &item.arguments, links);
    }
    render_extra_sections(contents, item);
    render_example(contents, item);
}

fn render_extra_sections(contents: &mut String, item: &ApiItemDoc) {
    for section in &item.docs.sections {
        contents.push_str(&format!("## {}\n\n", section.title));
        contents.push_str(&section.body);
        contents.push_str("\n\n");
    }
}

fn render_example(contents: &mut String, item: &ApiItemDoc) {
    let Some(example) = &item.example else {
        return;
    };
    let source_path = example
        .relative_path
        .display()
        .to_string()
        .replace('\\', "/");
    contents.push_str("## SDK Example\n\n");
    contents.push_str(&format!(
        "_Source: [`{}`](../../{})_\n\n",
        source_path, source_path
    ));
    contents.push_str(&example.contents);
    contents.push_str("\n\n");
}

fn render_field_table(
    contents: &mut String,
    from_group: ApiGroup,
    fields: &[ApiFieldDoc],
    links: &ItemLinkIndex,
) {
    contents.push_str("| Field | Type | Description |\n");
    contents.push_str("| --- | --- | --- |\n");
    for field in fields {
        contents.push_str(&format!(
            "| `{}` | {} | {} |\n",
            field.name,
            escape_table_cell(&render_type_markdown(&field.ty, from_group, links)),
            escape_table_cell(&field.description)
        ));
    }
    contents.push('\n');
}

fn render_argument_table(
    contents: &mut String,
    from_group: ApiGroup,
    arguments: &[ApiArgumentDoc],
    links: &ItemLinkIndex,
) {
    contents.push_str("| Argument | Type | Description |\n");
    contents.push_str("| --- | --- | --- |\n");
    for argument in arguments {
        contents.push_str(&format!(
            "| `{}` | {} | {} |\n",
            argument.name,
            escape_table_cell(&render_type_markdown(&argument.ty, from_group, links)),
            escape_table_cell(&argument.description)
        ));
    }
    contents.push('\n');
}

fn render_variant_table(
    contents: &mut String,
    from_group: ApiGroup,
    variants: &[ApiVariantDoc],
    links: &ItemLinkIndex,
) {
    contents.push_str("| Variant | Payload | Description |\n");
    contents.push_str("| --- | --- | --- |\n");
    for variant in variants {
        contents.push_str(&format!(
            "| `{}` | {} | {} |\n",
            variant.name,
            escape_table_cell(&render_type_markdown(&variant.payload, from_group, links)),
            escape_table_cell(&variant.description)
        ));
    }
    contents.push('\n');
}

fn render_generated_header(contents: &mut String, title: &str) {
    contents.push_str(
        "<!-- generated by cargo xtask generate-plugin-sdk-docs; do not edit by hand -->\n\n",
    );
    contents.push_str(&format!("<!-- {} -->\n\n", title));
}

fn build_item_link_index(items: &[ApiItemDoc]) -> ItemLinkIndex {
    let mut links = BTreeMap::new();
    for item in items {
        if item.group == ApiGroup::Events || item.group == ApiGroup::Constants {
            continue;
        }
        if item.name.contains("::") && item.group != ApiGroup::Structures {
            continue;
        }
        links.insert(item.name.clone(), (item.group, item.file_name.clone()));
    }
    links
}

fn render_type_markdown(raw: &str, from_group: ApiGroup, links: &ItemLinkIndex) -> String {
    let mut rendered = String::new();
    let mut token = String::new();
    for character in raw.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            token.push(character);
            continue;
        }
        flush_type_token(&mut rendered, &mut token, from_group, links);
        rendered.push(character);
    }
    flush_type_token(&mut rendered, &mut token, from_group, links);
    rendered
}

fn flush_type_token(
    rendered: &mut String,
    token: &mut String,
    from_group: ApiGroup,
    links: &ItemLinkIndex,
) {
    if token.is_empty() {
        return;
    }
    if let Some((target_group, file_name)) = links.get(token) {
        rendered.push_str(&format!(
            "[`{}`]({})",
            token,
            relative_item_link(from_group, *target_group, file_name)
        ));
    } else {
        rendered.push_str(token);
    }
    token.clear();
}

fn relative_item_link(from_group: ApiGroup, target_group: ApiGroup, file_name: &str) -> String {
    let _ = from_group;
    format!("../{}/{}", target_group.directory_name(), file_name)
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}
