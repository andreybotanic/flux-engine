use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use bevy_math::{UVec2, Vec2, Vec4};
use serde::{Deserialize, Serialize};

use crate::{ContentId, ContentTag};

/// Stable overlay material identifier.
pub type OverlayMaterialId = ContentId;

/// Final output node of one overlay graph.
pub type OverlayOutput = OverlayNodeId;

/// Public ergonomic alias for overlay selector expressions.
pub type Selector = OverlaySelectorExpr;

/// Stable node identifier inside one overlay graph.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OverlayNodeId(String);

impl OverlayNodeId {
    /// Parses one overlay node identifier.
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.is_empty() {
            return Err("overlay node id must not be empty".to_string());
        }
        let bytes = raw.as_bytes();
        let valid_edge = |value: u8| value.is_ascii_lowercase() || value.is_ascii_digit();
        if !valid_edge(bytes[0]) || !valid_edge(*bytes.last().expect("non-empty")) {
            return Err(format!(
                "overlay node id '{raw}' must start and end with a lowercase ASCII letter or digit"
            ));
        }
        let mut previous_sep = false;
        for byte in bytes {
            if valid_edge(*byte) {
                previous_sep = false;
                continue;
            }
            if matches!(*byte, b'.' | b'_' | b'-') && !previous_sep {
                previous_sep = true;
                continue;
            }
            return Err(format!(
                "overlay node id '{raw}' must match ^[a-z0-9]+([._-][a-z0-9]+)*$"
            ));
        }
        Ok(Self(raw.to_string()))
    }

    /// Returns the canonical node identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OverlayNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One plugin-owned overlay render graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayGraph {
    pub nodes: Vec<OverlayNode>,
    pub output: OverlayOutput,
}

impl OverlayGraph {
    /// Validates graph shape and returns deterministic execution order.
    pub fn execution_order(&self) -> Result<Vec<OverlayNodeId>, OverlayGraphError> {
        validate_overlay_graph(self)
    }

    /// Validates this graph without returning the execution order.
    pub fn validate(&self) -> Result<(), OverlayGraphError> {
        self.execution_order().map(|_| ())
    }
}

/// One named overlay graph node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayNode {
    pub id: OverlayNodeId,
    pub depends_on: Vec<OverlayNodeId>,
    pub kind: OverlayNodeKind,
}

/// Supported overlay graph node kinds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OverlayNodeKind {
    RenderEntities(RenderEntitiesNode),
    RenderFreeGas(RenderFreeGasNode),
    RenderImage(RenderImageNode),
    Blend(BlendNode),
    Material(MaterialNode),
}

/// Renders world entities selected by content ids or tags.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderEntitiesNode {
    pub selector: OverlaySelectorExpr,
    pub style: OverlayEntityStyle,
}

/// Style applied to entity render subjects selected by a graph node.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OverlayEntityStyle {
    pub tint: Option<Vec4>,
    pub sprite_override: Option<OverlayEntitySpriteOverride>,
}

/// Controls which sprite set is used for selected entities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayEntitySpriteOverride {
    OverlayVariant,
    Silhouette,
    Asset(ContentId),
}

/// Renders core free-gas visualization as one graph input.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderFreeGasNode {
    pub alpha: f32,
}

/// Renders plugin-provided images in cell/grid coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderImageNode {
    pub instances: Vec<OverlayImageInstance>,
}

/// One image instance emitted by `RenderImageNode`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayImageInstance {
    pub image: OverlayImageSource,
    pub placement: OverlayPlacement,
    pub tint: Vec4,
}

/// Image payload used by one `RenderImageNode` instance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OverlayImageSource {
    Asset(ContentId),
    Rgba8 { size_px: UVec2, rgba8: Vec<u8> },
}

/// Placement for one image instance.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum OverlayPlacement {
    CellLocal {
        cell: UVec2,
        anchor: Vec2,
        offset_in_cell: Vec2,
        size_in_cell: Vec2,
        rotation: f32,
    },
    GridLocal {
        position_in_grid: Vec2,
        size_in_grid: Vec2,
        rotation: f32,
        origin: Vec2,
    },
}

/// Blends previously rendered input nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlendNode {
    pub mode: OverlayBlendMode,
}

/// Blend mode used by `BlendNode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayBlendMode {
    AlphaOver,
    Add,
    Multiply,
    Screen,
    TintMix,
    MaskMix,
}

/// Applies one plugin-owned material to input nodes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialNode {
    pub material_id: OverlayMaterialId,
    pub params: Vec<OverlayMaterialParam>,
}

/// One plugin-owned overlay material or shader descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayMaterialDescriptor {
    pub id: OverlayMaterialId,
    pub label: String,
    pub shader_path: String,
}

/// One named material parameter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayMaterialParam {
    pub name: String,
    pub value: OverlayMaterialParamValue,
}

/// Supported material parameter values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OverlayMaterialParamValue {
    Float(f32),
    Vec2(Vec2),
    Vec4(Vec4),
    Image(ContentId),
}

/// Entity selector expression for overlay graph nodes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlaySelectorExpr {
    ContentId(ContentId),
    Tag(ContentTag),
    Not(Box<OverlaySelectorExpr>),
    Any(Vec<OverlaySelectorExpr>),
    All(Vec<OverlaySelectorExpr>),
}

impl OverlaySelectorExpr {
    /// Selects entities with one stable content id.
    pub fn content_id(id: ContentId) -> Self {
        Self::ContentId(id)
    }

    /// Selects entities carrying one content tag.
    pub fn tag(tag: ContentTag) -> Self {
        Self::Tag(tag)
    }

    /// Selects entities matching any child selector.
    pub fn any(items: impl IntoIterator<Item = Self>) -> Self {
        Self::Any(items.into_iter().collect())
    }

    /// Selects entities matching every child selector.
    pub fn all(items: impl IntoIterator<Item = Self>) -> Self {
        Self::All(items.into_iter().collect())
    }

    /// Inverts one child selector without exposing `Box` to plugin authors.
    pub fn not(selector: Self) -> Self {
        Self::Not(Box::new(selector))
    }
}

/// Validation error for an overlay graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayGraphError {
    message: String,
}

impl OverlayGraphError {
    /// Builds one graph validation error.
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the validation message.
    pub fn as_str(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for OverlayGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for OverlayGraphError {}

fn validate_overlay_graph(graph: &OverlayGraph) -> Result<Vec<OverlayNodeId>, OverlayGraphError> {
    if graph.nodes.is_empty() {
        return Err(OverlayGraphError::message(
            "overlay graph must contain at least one node",
        ));
    }

    let mut nodes_by_id = BTreeMap::new();
    for node in &graph.nodes {
        if nodes_by_id.insert(node.id.clone(), node).is_some() {
            return Err(OverlayGraphError::message(format!(
                "duplicate overlay node '{}'",
                node.id
            )));
        }
    }

    if !nodes_by_id.contains_key(&graph.output) {
        return Err(OverlayGraphError::message(format!(
            "overlay graph output '{}' does not exist",
            graph.output
        )));
    }

    for node in &graph.nodes {
        validate_node(node)?;
        for dependency in &node.depends_on {
            if !nodes_by_id.contains_key(dependency) {
                return Err(OverlayGraphError::message(format!(
                    "overlay node '{}' depends on missing node '{}'",
                    node.id, dependency
                )));
            }
        }
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    visit_node(
        &graph.output,
        &nodes_by_id,
        &mut visiting,
        &mut visited,
        &mut order,
    )?;
    Ok(order)
}

fn validate_node(node: &OverlayNode) -> Result<(), OverlayGraphError> {
    match &node.kind {
        OverlayNodeKind::RenderEntities(render) => validate_selector(&render.selector),
        OverlayNodeKind::RenderImage(render) => {
            for instance in &render.instances {
                validate_image_source(&instance.image)?;
            }
            Ok(())
        }
        OverlayNodeKind::Blend(_) if node.depends_on.len() < 2 => {
            Err(OverlayGraphError::message(format!(
                "blend node '{}' must depend on at least two input nodes",
                node.id
            )))
        }
        OverlayNodeKind::Material(_) if node.depends_on.is_empty() => {
            Err(OverlayGraphError::message(format!(
                "material node '{}' must depend on at least one input node",
                node.id
            )))
        }
        _ => Ok(()),
    }
}

fn validate_image_source(image: &OverlayImageSource) -> Result<(), OverlayGraphError> {
    match image {
        OverlayImageSource::Asset(_) => Ok(()),
        OverlayImageSource::Rgba8 { size_px, rgba8 } => {
            if size_px.x == 0 || size_px.y == 0 {
                return Err(OverlayGraphError::message(
                    "overlay rgba8 image must have non-zero size",
                ));
            }
            let Some(expected_len) = (size_px.x as usize)
                .checked_mul(size_px.y as usize)
                .and_then(|pixels| pixels.checked_mul(4))
            else {
                return Err(OverlayGraphError::message(
                    "overlay rgba8 image size overflows addressable memory",
                ));
            };
            if rgba8.len() != expected_len {
                return Err(OverlayGraphError::message(format!(
                    "overlay rgba8 image expects {expected_len} bytes but received {}",
                    rgba8.len()
                )));
            }
            Ok(())
        }
    }
}

fn validate_selector(selector: &OverlaySelectorExpr) -> Result<(), OverlayGraphError> {
    match selector {
        OverlaySelectorExpr::Not(inner) => validate_selector(inner),
        OverlaySelectorExpr::Any(items) | OverlaySelectorExpr::All(items) => {
            if items.is_empty() {
                return Err(OverlayGraphError::message(
                    "selector Any/All must not be empty",
                ));
            }
            for item in items {
                validate_selector(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn visit_node<'a>(
    id: &OverlayNodeId,
    nodes_by_id: &BTreeMap<OverlayNodeId, &'a OverlayNode>,
    visiting: &mut BTreeSet<OverlayNodeId>,
    visited: &mut BTreeSet<OverlayNodeId>,
    order: &mut Vec<OverlayNodeId>,
) -> Result<(), OverlayGraphError> {
    if visited.contains(id) {
        return Ok(());
    }
    if !visiting.insert(id.clone()) {
        return Err(OverlayGraphError::message(format!(
            "overlay graph contains a cycle at node '{id}'"
        )));
    }
    let node = nodes_by_id
        .get(id)
        .ok_or_else(|| OverlayGraphError::message(format!("missing overlay node '{id}'")))?;
    let mut dependencies = node.depends_on.clone();
    dependencies.sort();
    for dependency in dependencies {
        visit_node(&dependency, nodes_by_id, visiting, visited, order)?;
    }
    visiting.remove(id);
    visited.insert(id.clone());
    order.push(id.clone());
    Ok(())
}

#[cfg(test)]
#[path = "overlay_graph_tests.rs"]
mod tests;
