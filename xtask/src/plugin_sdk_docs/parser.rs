use syn::{Attribute, Expr, ExprLit, Lit, Meta, Type, Visibility};

use crate::XtaskError;

#[derive(Clone, Debug)]
/// Parsed Rustdoc block split into summary and titled sections.
pub(super) struct DocBlock {
    pub(super) summary: String,
    pub(super) sections: Vec<DocSection>,
}

#[derive(Clone, Debug)]
/// One titled section inside a parsed Rustdoc block.
pub(super) struct DocSection {
    pub(super) title: String,
    pub(super) body: String,
}

/// Reads a required SDK Rustdoc block and validates its basic shape.
pub(super) fn required_doc_block(
    source_path: &str,
    item_name: &str,
    attrs: &[Attribute],
) -> Result<DocBlock, XtaskError> {
    let raw = doc_lines(attrs).join("\n");
    if raw.trim().is_empty() {
        return Err(XtaskError::new(format!(
            "{}: SDK item `{}` is missing a doc-comment",
            source_path, item_name
        )));
    }
    let block = parse_doc_block(&raw);
    if block.summary.trim().is_empty() {
        return Err(XtaskError::new(format!(
            "{}: SDK item `{}` has no summary before the first section",
            source_path, item_name
        )));
    }
    require_any_section(source_path, item_name, &block)?;
    Ok(block)
}

/// Extracts an optional one-paragraph summary from doc comments.
pub(super) fn optional_doc_summary(attrs: &[Attribute]) -> Option<String> {
    let raw = doc_lines(attrs).join("\n");
    let summary = parse_doc_block(&raw).summary;
    if summary.trim().is_empty() {
        None
    } else {
        Some(summary)
    }
}

/// Requires at least one titled markdown section in a parsed Rustdoc block.
pub(super) fn require_any_section(
    source_path: &str,
    item_name: &str,
    docs: &DocBlock,
) -> Result<(), XtaskError> {
    if docs.sections.is_empty() {
        return Err(XtaskError::new(format!(
            "{}: SDK item `{}` must contain at least one `# ...` doc section",
            source_path, item_name
        )));
    }
    Ok(())
}

/// Requires a particular markdown heading to exist in a parsed Rustdoc block.
pub(super) fn require_section(
    source_path: &str,
    item_name: &str,
    docs: &DocBlock,
    section: &str,
) -> Result<(), XtaskError> {
    let title = section.trim_start_matches('#').trim();
    if docs
        .sections
        .iter()
        .any(|candidate| candidate.title == title)
    {
        return Ok(());
    }
    Err(XtaskError::new(format!(
        "{}: SDK item `{}` must contain `{}` doc section",
        source_path, item_name, section
    )))
}

/// Parses raw doc-comment text into a summary plus `# Heading` sections.
pub(super) fn parse_doc_block(raw: &str) -> DocBlock {
    let mut summary = Vec::new();
    let mut sections = Vec::<DocSection>::new();
    let mut current_title = None::<String>;
    let mut current_body = Vec::<String>::new();
    for line in raw.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            if let Some(previous) = current_title.replace(title.trim().to_string()) {
                sections.push(DocSection {
                    title: previous,
                    body: current_body.join("\n").trim().to_string(),
                });
                current_body.clear();
            }
            continue;
        }
        if current_title.is_some() {
            current_body.push(line.to_string());
        } else {
            summary.push(line.to_string());
        }
    }
    if let Some(title) = current_title {
        sections.push(DocSection {
            title,
            body: current_body.join("\n").trim().to_string(),
        });
    }
    DocBlock {
        summary: summary.join("\n").trim().to_string(),
        sections,
    }
}

/// Returns the simple owner type name from an inherent `impl` target.
pub(super) fn impl_owner_name(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

/// Returns `true` when a `syn` visibility is public.
pub(super) fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn doc_lines(attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            let Meta::NameValue(name_value) = &attr.meta else {
                return None;
            };
            let Expr::Lit(ExprLit {
                lit: Lit::Str(value),
                ..
            }) = &name_value.value
            else {
                return None;
            };
            Some(value.value().trim_start().to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_doc_block, require_section};

    #[test]
    fn parses_summary_and_sections() {
        let docs =
            parse_doc_block("Short summary.\n\n# SDK Example\n```rust\nlet ok = true;\n```\n");

        assert_eq!(docs.summary, "Short summary.");
        assert_eq!(docs.sections.len(), 1);
        assert_eq!(docs.sections[0].title, "SDK Example");
        assert!(docs.sections[0].body.contains("let ok"));
    }

    #[test]
    fn required_section_reports_missing_heading() {
        let docs = parse_doc_block("Short summary.\n\n# SDK Notes\nStable.");

        let error = require_section("src/plugins/abi.rs", "FluxStatus", &docs, "# SDK Example")
            .expect_err("missing section must fail");

        assert!(error.to_string().contains("must contain `# SDK Example`"));
    }
}
