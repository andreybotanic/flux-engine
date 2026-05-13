use super::*;

fn node_id(raw: &str) -> OverlayNodeId {
    OverlayNodeId::parse(raw).expect("valid node id")
}

fn content_id(raw: &str) -> ContentId {
    ContentId::parse(raw).expect("valid content id")
}

fn simple_image_node(id: &str) -> OverlayNode {
    OverlayNode {
        id: node_id(id),
        depends_on: Vec::new(),
        kind: OverlayNodeKind::RenderImage(RenderImageNode {
            instances: vec![OverlayImageInstance {
                image: OverlayImageSource::Asset(content_id("flux.test.image")),
                placement: OverlayPlacement::GridLocal {
                    position_in_grid: Vec2::ZERO,
                    size_in_grid: Vec2::ONE,
                    rotation: 0.0,
                    origin: Vec2::ZERO,
                },
                tint: Vec4::ONE,
            }],
        }),
    }
}

#[test]
fn graph_orders_dependencies_before_output() {
    let graph = OverlayGraph {
        nodes: vec![
            OverlayNode {
                id: node_id("compose"),
                depends_on: vec![node_id("b"), node_id("a")],
                kind: OverlayNodeKind::Blend(BlendNode {
                    mode: OverlayBlendMode::AlphaOver,
                }),
            },
            simple_image_node("b"),
            simple_image_node("a"),
        ],
        output: node_id("compose"),
    };

    let order = graph.execution_order().expect("valid graph");
    assert_eq!(order, vec![node_id("a"), node_id("b"), node_id("compose")]);
}

#[test]
fn render_image_node_allows_empty_instances() {
    let graph = OverlayGraph {
        nodes: vec![
            OverlayNode {
                id: node_id("empty"),
                depends_on: Vec::new(),
                kind: OverlayNodeKind::RenderImage(RenderImageNode {
                    instances: Vec::new(),
                }),
            },
            OverlayNode {
                id: node_id("compose"),
                depends_on: vec![node_id("empty"), node_id("image")],
                kind: OverlayNodeKind::Blend(BlendNode {
                    mode: OverlayBlendMode::AlphaOver,
                }),
            },
            simple_image_node("image"),
        ],
        output: node_id("compose"),
    };

    let order = graph
        .execution_order()
        .expect("graph with empty image layer should be valid");
    assert_eq!(
        order,
        vec![node_id("empty"), node_id("image"), node_id("compose")]
    );
}

#[test]
fn graph_rejects_missing_dependency() {
    let graph = OverlayGraph {
        nodes: vec![OverlayNode {
            id: node_id("compose"),
            depends_on: vec![node_id("missing")],
            kind: OverlayNodeKind::Material(MaterialNode {
                material_id: content_id("flux.test.material"),
                params: Vec::new(),
            }),
        }],
        output: node_id("compose"),
    };

    let error = graph.validate().expect_err("missing dependency must fail");
    assert!(error.as_str().contains("missing"));
}

#[test]
fn graph_rejects_cycles() {
    let graph = OverlayGraph {
        nodes: vec![
            OverlayNode {
                id: node_id("a"),
                depends_on: vec![node_id("b")],
                kind: OverlayNodeKind::Material(MaterialNode {
                    material_id: content_id("flux.test.material"),
                    params: Vec::new(),
                }),
            },
            OverlayNode {
                id: node_id("b"),
                depends_on: vec![node_id("a")],
                kind: OverlayNodeKind::Material(MaterialNode {
                    material_id: content_id("flux.test.material"),
                    params: Vec::new(),
                }),
            },
        ],
        output: node_id("a"),
    };

    let error = graph.validate().expect_err("cycle must fail");
    assert!(error.as_str().contains("cycle"));
}

#[test]
fn selector_rejects_empty_sets() {
    let selector = OverlaySelectorExpr::any(Vec::new());

    let error = validate_selector(&selector).expect_err("empty set must fail");
    assert!(error.as_str().contains("Any/All"));
}

#[test]
fn selector_builders_hide_boxed_not() {
    let tag = ContentTag::parse("flux.test.pipe").expect("tag");
    let selector = Selector::not(Selector::tag(tag.clone()));

    assert_eq!(
        selector,
        OverlaySelectorExpr::Not(Box::new(OverlaySelectorExpr::Tag(tag)))
    );
}

#[test]
fn rgba8_image_source_requires_exact_byte_count() {
    let graph = OverlayGraph {
        nodes: vec![OverlayNode {
            id: node_id("image"),
            depends_on: Vec::new(),
            kind: OverlayNodeKind::RenderImage(RenderImageNode {
                instances: vec![OverlayImageInstance {
                    image: OverlayImageSource::Rgba8 {
                        size_px: UVec2::new(2, 2),
                        rgba8: vec![255; 12],
                    },
                    placement: OverlayPlacement::GridLocal {
                        position_in_grid: Vec2::ZERO,
                        size_in_grid: Vec2::ONE,
                        rotation: 0.0,
                        origin: Vec2::ZERO,
                    },
                    tint: Vec4::ONE,
                }],
            }),
        }],
        output: node_id("image"),
    };

    let error = graph.validate().expect_err("bad rgba8 payload must fail");
    assert!(error.as_str().contains("expects 16 bytes"));
}

#[test]
fn graph_serializes_and_deserializes_via_json() {
    let graph = OverlayGraph {
        nodes: vec![simple_image_node("image")],
        output: node_id("image"),
    };

    let encoded = serde_json::to_string(&graph).expect("serialize graph");
    let decoded: OverlayGraph = serde_json::from_str(&encoded).expect("deserialize graph");
    assert_eq!(decoded, graph);
}
