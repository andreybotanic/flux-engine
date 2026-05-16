use super::*;

#[test]
fn default_plugin_content_ids_roundtrip_legacy_enums() {
    for material in [
        crate::plugins::default_plugin::boundary_cell_material(),
        crate::plugins::default_plugin::brick_cell_material(),
        crate::plugins::default_plugin::metal_cell_material(),
    ] {
        let id = cell_material_content_id(material);
        assert_eq!(cell_material_from_content_id(&id), Some(material));
    }

    for kind in [
        crate::plugins::default_plugin::pipe_structure_kind(),
        crate::plugins::default_plugin::vent_structure_kind(),
        crate::plugins::default_plugin::gas_source_structure_kind(),
        crate::plugins::default_plugin::gas_sink_structure_kind(),
        crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
    ] {
        let id = structure_kind_content_id(kind);
        assert_eq!(structure_kind_from_content_id(&id), Some(kind));
    }

    for mode in [OverlayMode::Main, OverlayMode::Gas, pipes_overlay_mode()] {
        let id = overlay_mode_content_id(mode);
        assert_eq!(overlay_mode_from_content_id(&id), Some(mode));
    }
}

#[test]
fn default_plugin_registry_contains_all_builtin_content() {
    let registry = default_content_registry();
    assert!(registry
        .provider_plugins()
        .contains(&PluginId::default_plugin()));
    assert_eq!(registry.cells().len(), 3);
    assert_eq!(registry.structures().len(), 5);
    assert_eq!(registry.overlays().len(), 3);
    assert_eq!(registry.substances().len(), 3);
    assert!(registry
        .substances()
        .contains_key(&SubstanceId::parse(SUBSTANCE_H2_ID).expect("h2 id")));
    assert!(registry
        .world_cell_hud()
        .expect("world hud")
        .block
        .substance_containers
        .iter()
        .any(|container| container.backing == ContainerBacking::WorldCell));
}

#[test]
fn default_plugin_bridge_keeps_legacy_shape_and_rotations() {
    let descriptor = structure_content_descriptor(
        crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
    );
    assert_eq!(
        descriptor.allowed_rotations,
        vec![StructureRotation::Deg0, StructureRotation::Deg90]
    );
    assert_eq!(
        descriptor
            .layer_descriptor(StructureRotation::Deg0)
            .size_in_cells(),
        UVec2::new(3, 1)
    );
    assert_eq!(
        descriptor
            .layer_descriptor(StructureRotation::Deg90)
            .size_in_cells(),
        UVec2::new(1, 3)
    );
}

#[test]
fn default_plugin_hud_order_matches_legacy_blocks() {
    let structures = default_structure_descriptors();
    let orders = structures
        .iter()
        .map(|descriptor| (descriptor.kind, descriptor.hud.sort_order))
        .collect::<Vec<_>>();
    assert!(orders.contains(&(crate::plugins::default_plugin::pipe_structure_kind(), 10)));
    assert!(orders.contains(&(
        crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
        20
    )));
    assert!(orders.contains(&(crate::plugins::default_plugin::vent_structure_kind(), 30)));
    assert!(orders.contains(&(
        crate::plugins::default_plugin::gas_source_structure_kind(),
        40
    )));
    assert!(orders.contains(&(
        crate::plugins::default_plugin::gas_sink_structure_kind(),
        50
    )));
}

#[test]
fn default_content_tags_support_overlay_selectors() {
    let registry = default_content_registry();
    let pipe_id = structure_kind_content_id(crate::plugins::default_plugin::pipe_structure_kind());
    let brick_id = cell_material_content_id(crate::plugins::default_plugin::brick_cell_material());
    let pipe_tag =
        flux_plugin_sdk::ContentTag::parse("flux.default.tag.pipe-network").expect("pipe tag");
    let solid_tag =
        flux_plugin_sdk::ContentTag::parse("flux.default.tag.solid").expect("solid tag");

    assert!(registry
        .structure_matches_selector(&pipe_id, &flux_plugin_sdk::Selector::tag(pipe_tag.clone()),));
    assert!(registry.cell_matches_selector(&brick_id, &flux_plugin_sdk::Selector::tag(solid_tag),));
    assert!(!registry.structure_matches_selector(
        &pipe_id,
        &flux_plugin_sdk::Selector::not(flux_plugin_sdk::Selector::tag(pipe_tag)),
    ));
}

#[test]
fn default_plugin_publishes_core_role_tags_for_cells_and_structures() {
    let registry = default_content_registry();
    let boundary = registry
        .cell_by_tag(crate::plugins::content::CORE_TAG_CELL_BOUNDARY)
        .expect("boundary cell must expose core boundary tag");
    assert_eq!(boundary.material, boundary_cell_material());

    let pipe = registry
        .structure_by_tag(crate::plugins::content::CORE_TAG_STRUCTURE_PIPE)
        .expect("pipe structure must expose core pipe tag");
    assert_eq!(pipe.kind, pipe_structure_kind());

    let vent = registry
        .structure_by_tag(crate::plugins::content::CORE_TAG_STRUCTURE_VENT)
        .expect("vent structure must expose core vent tag");
    assert_eq!(vent.kind, vent_structure_kind());

    let bridge = registry
        .structure_by_tag(crate::plugins::content::CORE_TAG_STRUCTURE_PIPE_BRIDGE)
        .expect("bridge structure must expose core bridge tag");
    assert_eq!(bridge.kind, gas_pipe_bridge_structure_kind());

    let source = registry
        .structure_by_tag(crate::plugins::content::CORE_TAG_STRUCTURE_GAS_SOURCE)
        .expect("gas source structure must expose core source tag");
    assert_eq!(source.kind, gas_source_structure_kind());

    let sink = registry
        .structure_by_tag(crate::plugins::content::CORE_TAG_STRUCTURE_GAS_SINK)
        .expect("gas sink structure must expose core sink tag");
    assert_eq!(sink.kind, gas_sink_structure_kind());
}
