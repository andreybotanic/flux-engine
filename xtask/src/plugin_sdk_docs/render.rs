use crate::plugin_sdk_docs::model::{
    ApiArgumentDoc, ApiFieldDoc, ApiGroup, ApiItemDoc, ApiVariantDoc,
};

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
pub(super) fn render_item_page(contents: &mut String, item: &ApiItemDoc) {
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
        ApiGroup::Structures => render_structure_page(contents, item),
        ApiGroup::Enums => render_enum_page(contents, item),
        ApiGroup::Constants => render_constant_page(contents, item),
        ApiGroup::Methods => render_method_page(contents, item),
        ApiGroup::Events => render_event_page(contents, item),
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

fn render_structure_page(contents: &mut String, item: &ApiItemDoc) {
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
        render_field_table(contents, &item.fields);
    }
    if !item.variants.is_empty() {
        contents.push_str("## Variants\n\n");
        render_variant_table(contents, &item.variants);
    }
    render_extra_sections(contents, item);
}

fn render_enum_page(contents: &mut String, item: &ApiItemDoc) {
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
        render_variant_table(contents, &item.variants);
    }
    render_extra_sections(contents, item);
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
}

fn render_method_page(contents: &mut String, item: &ApiItemDoc) {
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
        render_argument_table(contents, &item.arguments);
    }
    contents.push_str("## Return Value\n\n");
    contents.push_str(item.return_value.as_deref().unwrap_or("()"));
    contents.push_str("\n\n");
    render_extra_sections(contents, item);
}

fn render_event_page(contents: &mut String, item: &ApiItemDoc) {
    contents.push_str("## When It Fires\n\n");
    contents.push_str(&item.docs.summary);
    contents.push_str("\n\n");
    contents.push_str("## Arguments\n\n");
    if item.arguments.is_empty() {
        contents.push_str("This event does not carry additional payload fields.\n\n");
    } else {
        render_argument_table(contents, &item.arguments);
    }
    render_extra_sections(contents, item);
}

fn render_extra_sections(contents: &mut String, item: &ApiItemDoc) {
    for section in &item.docs.sections {
        contents.push_str(&format!("## {}\n\n", section.title));
        contents.push_str(&section.body);
        contents.push_str("\n\n");
    }
}

fn render_field_table(contents: &mut String, fields: &[ApiFieldDoc]) {
    contents.push_str("| Field | Type | Description |\n");
    contents.push_str("| --- | --- | --- |\n");
    for field in fields {
        contents.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            field.name,
            escape_table_cell(&field.ty),
            escape_table_cell(&field.description)
        ));
    }
    contents.push('\n');
}

fn render_argument_table(contents: &mut String, arguments: &[ApiArgumentDoc]) {
    contents.push_str("| Argument | Type | Description |\n");
    contents.push_str("| --- | --- | --- |\n");
    for argument in arguments {
        contents.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            argument.name,
            escape_table_cell(&argument.ty),
            escape_table_cell(&argument.description)
        ));
    }
    contents.push('\n');
}

fn render_variant_table(contents: &mut String, variants: &[ApiVariantDoc]) {
    contents.push_str("| Variant | Payload | Description |\n");
    contents.push_str("| --- | --- | --- |\n");
    for variant in variants {
        contents.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            variant.name,
            escape_table_cell(&variant.payload),
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

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}
