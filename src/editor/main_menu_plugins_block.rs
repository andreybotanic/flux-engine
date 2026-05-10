use crate::plugins::{
    rebuild_plugin_registry_from_enabled_set, reload_plugin_registry, ContentRegistry,
    EnabledPluginSet, LoadedPluginRegistry, PluginBootstrapConfig, PluginRegistryEntry,
    PluginRegistryState, PluginReloadError, PluginReloadRequest, PluginRuntimeStatus,
    PluginSourceKind, PluginSourceRegistry,
};
use crate::ui::toggle_switch::{
    spawn_toggle_switch, spawn_toggle_switch_button, ToggleSwitchConfig, ToggleSwitchRoot,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PluginToggleControl {
    Actionable { next_enabled: bool },
    Locked,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PluginTogglePresentation {
    Switch { interactive: bool },
    StateLabel,
}

fn root_plugins_button_visible(mode: MainMenuMode) -> bool {
    matches!(mode, MainMenuMode::Main | MainMenuMode::InGame)
}

fn plugin_toggles_allowed(mode: MainMenuMode, has_world: bool) -> bool {
    mode == MainMenuMode::Main && !has_world
}

fn plugin_reload_allowed(mode: MainMenuMode, has_world: bool) -> bool {
    mode == MainMenuMode::Main && !has_world
}

fn plugin_toggle_control(
    entry: &PluginRegistryEntry,
    enabled_set: &EnabledPluginSet,
    mode: MainMenuMode,
    has_world: bool,
) -> PluginToggleControl {
    let Some(plugin_id) = entry.plugin_id.as_ref() else {
        return PluginToggleControl::Disabled;
    };
    if entry.locked || *plugin_id == PluginId::default_plugin() {
        return PluginToggleControl::Locked;
    }
    if !plugin_toggles_allowed(mode, has_world) {
        return PluginToggleControl::Disabled;
    }

    let currently_enabled = enabled_set.is_enabled(plugin_id);
    match entry.status {
        PluginRuntimeStatus::Enabled | PluginRuntimeStatus::Disabled => {
            PluginToggleControl::Actionable {
                next_enabled: !currently_enabled,
            }
        }
        PluginRuntimeStatus::Missing | PluginRuntimeStatus::Error if currently_enabled => {
            PluginToggleControl::Actionable {
                next_enabled: false,
            }
        }
        PluginRuntimeStatus::Missing | PluginRuntimeStatus::Error => PluginToggleControl::Disabled,
    }
}

fn plugin_toggle_label(
    entry: &PluginRegistryEntry,
    enabled_set: &EnabledPluginSet,
    mode: MainMenuMode,
    has_world: bool,
) -> String {
    let currently_enabled = entry
        .plugin_id
        .as_ref()
        .map(|plugin_id| enabled_set.is_enabled(plugin_id))
        .unwrap_or(false);
    match plugin_toggle_control(entry, enabled_set, mode, has_world) {
        PluginToggleControl::Actionable { next_enabled } => {
            if next_enabled {
                "Off".to_string()
            } else if matches!(
                entry.status,
                PluginRuntimeStatus::Error | PluginRuntimeStatus::Missing
            ) {
                "On / Disable".to_string()
            } else {
                "On".to_string()
            }
        }
        PluginToggleControl::Locked => {
            if currently_enabled {
                "On / Locked".to_string()
            } else {
                "Off / Locked".to_string()
            }
        }
        PluginToggleControl::Disabled => {
            if currently_enabled {
                if matches!(
                    entry.status,
                    PluginRuntimeStatus::Error | PluginRuntimeStatus::Missing
                ) {
                    "On / Invalid".to_string()
                } else {
                    "On".to_string()
                }
            } else if matches!(
                entry.status,
                PluginRuntimeStatus::Error | PluginRuntimeStatus::Missing
            ) {
                "Off / Invalid".to_string()
            } else {
                "Off".to_string()
            }
        }
    }
}

fn plugin_toggle_presentation(
    entry: &PluginRegistryEntry,
    enabled_set: &EnabledPluginSet,
    mode: MainMenuMode,
    has_world: bool,
) -> PluginTogglePresentation {
    if mode == MainMenuMode::InGame {
        return PluginTogglePresentation::StateLabel;
    }
    match plugin_toggle_control(entry, enabled_set, mode, has_world) {
        PluginToggleControl::Actionable { .. } => {
            PluginTogglePresentation::Switch { interactive: true }
        }
        PluginToggleControl::Locked | PluginToggleControl::Disabled => {
            PluginTogglePresentation::Switch { interactive: false }
        }
    }
}

fn return_main_menu_to_root(menu_ui: &mut MainMenuUiState) {
    menu_ui.screen = MainMenuScreen::Root;
    menu_ui.confirm_state = None;
    menu_ui.confirm_text.clear();
    menu_ui.post_save_action = None;
    menu_ui.status_text.clear();
}

fn open_plugins_screen(menu_ui: &mut MainMenuUiState) {
    menu_ui.screen = MainMenuScreen::Plugins;
    menu_ui.confirm_state = None;
    menu_ui.confirm_text.clear();
    menu_ui.post_save_action = None;
    menu_ui.needs_plugin_list_refresh = true;
}

fn handle_plugin_toggle(
    plugin_id: PluginId,
    menu_ui: &mut MainMenuUiState,
    world_load_state: &WorldLoadState,
    config: &PluginBootstrapConfig,
    source_registry: &mut PluginSourceRegistry,
    loaded_registry: &mut LoadedPluginRegistry,
    enabled_set: &mut EnabledPluginSet,
    content_registry: &mut ContentRegistry,
    gas_registry: &mut GasRegistry,
    gas: &mut GasField,
    pipe_gas: &mut crate::plugins::default_plugin::pipe_runtime::PipeGasField,
    pipe_flux: &mut crate::plugins::default_plugin::pipe_runtime::PipeFluxField,
    gpu_state: &mut crate::simulation::GpuRuntimeState,
    select_fields: &mut SelectFieldState,
    gas_settings: &mut GasToolSettings,
    source_settings: &mut SourceStructureToolSettings,
    registry_state: &mut PluginRegistryState,
) {
    let old_registry_state = registry_state.clone();
    let Some(entry) = registry_state
        .entries
        .iter()
        .find(|entry| entry.plugin_id.as_ref() == Some(&plugin_id))
        .cloned()
    else {
        menu_ui.status_text = format!("Plugin '{}' is not in the registry.", plugin_id);
        return;
    };

    let PluginToggleControl::Actionable { next_enabled } =
        plugin_toggle_control(&entry, enabled_set, menu_ui.mode, world_load_state.has_world)
    else {
        menu_ui.status_text = format!("Plugin '{}' cannot be toggled now.", plugin_id);
        return;
    };

    let mut next_enabled_set = enabled_set.clone();
    next_enabled_set.set_enabled(&plugin_id, next_enabled);
    if let Err(error) = next_enabled_set.save_to_path(&config.state_file_path) {
        menu_ui.status_text = format!("Plugin toggle failed: {}", error);
        return;
    }

    let output = rebuild_plugin_registry_from_enabled_set(config, next_enabled_set);
    let next_gas_registry =
        match crate::config::GameConfig::load_gas_registry_from_default_location(
            &output.content_registry,
        ) {
            Ok(registry) => registry,
            Err(error) => {
                menu_ui.status_text = format!("Plugin toggle failed: {}", error);
                return;
            }
        };
    let list_requires_rebuild =
        plugin_list_requires_rebuild(&old_registry_state, &output.registry_state);
    output.registry_state.log_to_stderr();

    *source_registry = output.source_registry;
    *loaded_registry = output.loaded_registry;
    *enabled_set = output.enabled_set;
    *content_registry = output.content_registry;
    *gas_registry = next_gas_registry;
    *gas = GasField::from_registry(gas_registry);
    *pipe_gas = crate::plugins::default_plugin::pipe_runtime::PipeGasField::from_registry(gas_registry);
    pipe_flux.clear_all();
    gpu_state.reset_solver();
    refresh_gas_select_options(select_fields, gas_registry, gas_settings, source_settings);
    *registry_state = output.registry_state;
    menu_ui.status_text.clear();
    menu_ui.needs_plugin_list_refresh = list_requires_rebuild;
}

fn handle_plugin_reload(
    menu_ui: &mut MainMenuUiState,
    world_load_state: &WorldLoadState,
    config: &PluginBootstrapConfig,
    source_registry: &mut PluginSourceRegistry,
    loaded_registry: &mut LoadedPluginRegistry,
    enabled_set: &mut EnabledPluginSet,
    content_registry: &mut ContentRegistry,
    gas_registry: &mut GasRegistry,
    gas: &mut GasField,
    pipe_gas: &mut crate::plugins::default_plugin::pipe_runtime::PipeGasField,
    pipe_flux: &mut crate::plugins::default_plugin::pipe_runtime::PipeFluxField,
    gpu_state: &mut crate::simulation::GpuRuntimeState,
    select_fields: &mut SelectFieldState,
    gas_settings: &mut GasToolSettings,
    source_settings: &mut SourceStructureToolSettings,
    registry_state: &mut PluginRegistryState,
) {
    if !plugin_reload_allowed(menu_ui.mode, world_load_state.has_world) {
        menu_ui.status_text = PluginReloadError::WorldLoaded.to_string();
        return;
    }

    match reload_plugin_registry(
        &PluginReloadRequest::manual(),
        config,
        enabled_set,
        world_load_state.has_world,
        source_registry,
        registry_state,
    ) {
        Ok(report) => {
            report.output.registry_state.log_to_stderr();
            *source_registry = report.output.source_registry;
            *loaded_registry = report.output.loaded_registry;
            *enabled_set = report.output.enabled_set;
            *content_registry = report.output.content_registry;
            *gas_registry = report.gas_registry;
            *gas = GasField::from_registry(gas_registry);
            *pipe_gas =
                crate::plugins::default_plugin::pipe_runtime::PipeGasField::from_registry(
                    gas_registry,
                );
            pipe_flux.clear_all();
            gpu_state.reset_solver();
            refresh_gas_select_options(select_fields, gas_registry, gas_settings, source_settings);
            *registry_state = report.output.registry_state;
            menu_ui.status_text = report.message;
            menu_ui.needs_plugin_list_refresh = true;
        }
        Err(error) => {
            menu_ui.status_text = match error {
                PluginReloadError::WorldLoaded => error.to_string(),
                PluginReloadError::Registry(message) => format!("Plugin reload failed: {}", message),
                PluginReloadError::Config(message) => format!("Plugin reload failed: {}", message),
            };
        }
    }
}

fn refresh_gas_select_options(
    select_fields: &mut SelectFieldState,
    gas_registry: &GasRegistry,
    gas_settings: &mut GasToolSettings,
    source_settings: &mut SourceStructureToolSettings,
) {
    let options = gas_select_options(gas_registry);
    select_fields.set_options_preserving_selection(GAS_SELECT_ADD_ID, options.clone());
    select_fields.set_options_preserving_selection(GAS_SELECT_SOURCE_ID, options);
    if gas_registry.count() == 0 {
        gas_settings.gas_index = 0;
        source_settings.gas_index = 0;
        return;
    }
    gas_settings.gas_index = gas_settings.gas_index.min(gas_registry.count() - 1);
    source_settings.gas_index = source_settings.gas_index.min(gas_registry.count() - 1);
}

fn gas_select_options(gas_registry: &GasRegistry) -> Vec<String> {
    gas_registry
        .all()
        .iter()
        .map(|gas| gas.id.to_uppercase())
        .collect()
}

fn plugin_list_requires_rebuild(
    old_state: &PluginRegistryState,
    new_state: &PluginRegistryState,
) -> bool {
    plugin_registry_entry_keys(old_state) != plugin_registry_entry_keys(new_state)
}

fn plugin_registry_entry_keys(registry_state: &PluginRegistryState) -> BTreeSet<String> {
    registry_state
        .entries
        .iter()
        .map(plugin_registry_entry_key)
        .collect()
}

fn plugin_registry_entry_key(entry: &PluginRegistryEntry) -> String {
    entry
        .plugin_id
        .as_ref()
        .map(|plugin_id| format!("id:{}", plugin_id.as_str()))
        .unwrap_or_else(|| format!("source:{}", entry.source_name))
}

fn rebuild_main_menu_plugin_list(
    commands: &mut Commands,
    content_entity: Entity,
    registry_state: &PluginRegistryState,
    enabled_set: &EnabledPluginSet,
    mode: MainMenuMode,
    has_world: bool,
    list_item_entities: &mut Vec<Entity>,
) {
    let mut created = Vec::new();
    let entries = registry_state.entries.clone();
    commands.entity(content_entity).with_children(|parent| {
        if entries.is_empty() {
            let row = parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(120.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(crate::ui::palette::TRANSPARENT),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new("No plugins found."),
                        TextFont::from_font_size(14.0),
                        TextColor(crate::ui::palette::TEXT_MUTED),
                        TextLayout::new_with_justify(JustifyText::Center),
                    ));
                })
                .id();
            created.push(row);
            return;
        }

        for entry in entries {
            let row = spawn_plugin_row(parent, &entry, enabled_set, mode, has_world);
            created.push(row);
        }
    });

    *list_item_entities = created;
}

fn spawn_plugin_row(
    parent: &mut ChildSpawnerCommands,
    entry: &PluginRegistryEntry,
    enabled_set: &EnabledPluginSet,
    mode: MainMenuMode,
    has_world: bool,
) -> Entity {
    let plugin_id_label = entry
        .plugin_id
        .as_ref()
        .map(|plugin_id| plugin_id.as_str().to_string())
        .unwrap_or_else(|| "unknown plugin id".to_string());
    let toggle_label = plugin_toggle_label(entry, enabled_set, mode, has_world);

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(126.0),
                padding: UiRect::all(Val::Px(12.0)),
                display: Display::Flex,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Stretch,
                column_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(MENU_MODAL_CARD_BG),
        ))
        .with_children(|row| {
            row.spawn(Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                flex_grow: 1.0,
                row_gap: Val::Px(5.0),
                ..default()
            })
            .with_children(|left| {
                left.spawn((
                    Text::new(entry.display_name.clone()),
                    TextFont::from_font_size(18.0),
                    TextColor(crate::ui::palette::TEXT_PRIMARY),
                ));
                left.spawn((
                    Text::new(plugin_id_label),
                    TextFont::from_font_size(13.0),
                    TextColor(crate::ui::palette::TEXT_SECONDARY),
                ));
                left.spawn((
                    Text::new(plugin_status_text(entry)),
                    TextFont::from_font_size(12.0),
                    TextColor(crate::ui::palette::TEXT_MUTED),
                    MainMenuPluginStatusText {
                        plugin_id: entry.plugin_id.clone(),
                        source_name: entry.source_name.clone(),
                    },
                ));
                if let Some(error) = entry.error_message.as_deref() {
                    left.spawn((
                        Text::new(format!("Error: {}", short_plugin_error(error))),
                        TextFont::from_font_size(12.0),
                        TextColor(crate::ui::palette::DANGER_TEXT),
                    ));
                }
            });

            spawn_plugin_toggle(row, entry, enabled_set, mode, has_world, &toggle_label);
        })
        .id()
}

fn spawn_plugin_toggle(
    parent: &mut ChildSpawnerCommands,
    entry: &PluginRegistryEntry,
    enabled_set: &EnabledPluginSet,
    mode: MainMenuMode,
    has_world: bool,
    label: &str,
) {
    let currently_enabled = entry
        .plugin_id
        .as_ref()
        .map(|plugin_id| enabled_set.is_enabled(plugin_id))
        .unwrap_or(false);
    match plugin_toggle_presentation(entry, enabled_set, mode, has_world) {
        PluginTogglePresentation::StateLabel => spawn_plugin_state_label(parent, label),
        PluginTogglePresentation::Switch { interactive: true } => {
            let Some(plugin_id) = entry.plugin_id.clone() else {
                let config = ToggleSwitchConfig::new(currently_enabled, false, label);
                spawn_toggle_switch(parent, config);
                return;
            };
            let config = ToggleSwitchConfig::new(currently_enabled, true, label);
            spawn_toggle_switch_button(
                parent,
                config,
                (
                    MainMenuButtonPalette {
                        idle: crate::ui::palette::TRANSPARENT,
                        hover: crate::ui::palette::TRANSPARENT,
                    },
                    MainMenuActionButton(MainMenuButtonAction::TogglePlugin(plugin_id.clone())),
                    MainMenuPluginToggle { plugin_id },
                ),
            );
        }
        PluginTogglePresentation::Switch { interactive: false } => {
            let config = ToggleSwitchConfig::new(currently_enabled, false, label);
            spawn_toggle_switch(parent, config);
        }
    }
}

fn spawn_plugin_state_label(parent: &mut ChildSpawnerCommands, label: &str) {
    parent
        .spawn((
            Node {
                width: Val::Px(154.0),
                height: Val::Px(38.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                align_self: AlignSelf::Center,
                ..default()
            },
            BackgroundColor(crate::ui::palette::TRANSPARENT),
        ))
        .with_children(|label_root| {
            label_root.spawn((
                Text::new(label.to_string()),
                TextFont::from_font_size(13.0),
                TextColor(crate::ui::palette::TEXT_MUTED),
            ));
        });
}

fn plugin_status_text(entry: &PluginRegistryEntry) -> String {
    let version_label = entry
        .version
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_string());
    let source_label = entry
        .source_kind
        .map(source_kind_label)
        .unwrap_or("unknown");
    let content_label = if entry.content {
        "content"
    } else {
        "non-content"
    };
    let status_label = entry.status.as_str();
    format!(
        "Version: {version_label} | Source: {source_label} | Status: {status_label} | {content_label}"
    )
}

fn sync_main_menu_plugin_rows(
    menu_ui: Res<MainMenuUiState>,
    registry_state: Res<PluginRegistryState>,
    enabled_set: Res<EnabledPluginSet>,
    world_load_state: Res<WorldLoadState>,
    mut status_texts: Query<(&MainMenuPluginStatusText, &mut Text)>,
    mut toggles: Query<(&MainMenuPluginToggle, &mut ToggleSwitchRoot)>,
) {
    if menu_ui.screen != MainMenuScreen::Plugins {
        return;
    }

    for (marker, mut text) in &mut status_texts {
        if let Some(entry) = find_plugin_entry(
            &registry_state,
            marker.plugin_id.as_ref(),
            &marker.source_name,
        ) {
            text.0 = plugin_status_text(entry);
        }
    }

    for (marker, mut root) in &mut toggles {
        let Some(entry) = find_plugin_entry(&registry_state, Some(&marker.plugin_id), "") else {
            continue;
        };
        let interactive = matches!(
            plugin_toggle_control(
                entry,
                &enabled_set,
                menu_ui.mode,
                world_load_state.has_world
            ),
            PluginToggleControl::Actionable { .. }
        );
        let on = enabled_set.is_enabled(&marker.plugin_id);
        let label = plugin_toggle_label(entry, &enabled_set, menu_ui.mode, world_load_state.has_world);
        if root.on != on || root.interactive != interactive || root.label != label {
            root.on = on;
            root.interactive = interactive;
            root.label = label;
        }
    }
}

fn find_plugin_entry<'a>(
    registry_state: &'a PluginRegistryState,
    plugin_id: Option<&PluginId>,
    source_name: &str,
) -> Option<&'a PluginRegistryEntry> {
    registry_state.entries.iter().find(|entry| match plugin_id {
        Some(plugin_id) => entry.plugin_id.as_ref() == Some(plugin_id),
        None => entry.source_name == source_name,
    })
}

fn source_kind_label(source_kind: PluginSourceKind) -> &'static str {
    match source_kind {
        PluginSourceKind::Builtin => "builtin",
        PluginSourceKind::Packaged => "packaged",
        PluginSourceKind::Dev => "dev",
    }
}

fn short_plugin_error(error: &str) -> String {
    let mut shortened = error.chars().take(120).collect::<String>();
    if error.chars().count() > 120 {
        shortened.push_str("...");
    }
    shortened
}

#[cfg(test)]
mod main_menu_plugins_tests {
    use super::{
        plugin_list_requires_rebuild, plugin_registry_entry_keys, plugin_toggle_control,
        plugin_toggle_label, plugin_toggle_presentation, plugin_reload_allowed,
        root_plugins_button_visible, return_main_menu_to_root, PluginToggleControl,
        PluginTogglePresentation,
    };
    use crate::{
        plugins::{
            EnabledPluginSet, PluginId, PluginRegistryEntry, PluginRegistryState, PluginRuntimeStatus,
            PluginSourceKind,
        },
        save::{MainMenuMode, MainMenuScreen, MainMenuUiState},
    };

    fn plugin_entry(id: &str, status: PluginRuntimeStatus, locked: bool) -> PluginRegistryEntry {
        let plugin_id = PluginId::parse(id).expect("valid plugin id");
        PluginRegistryEntry {
            plugin_id: Some(plugin_id.clone()),
            display_name: id.to_string(),
            version: None,
            source_kind: Some(if locked {
                PluginSourceKind::Builtin
            } else {
                PluginSourceKind::Packaged
            }),
            status,
            locked,
            content: true,
            source_name: id.to_string(),
            source_path: None,
            error_message: None,
        }
    }

    #[test]
    fn main_menu_plugins_button_is_available_from_root_modes() {
        assert!(root_plugins_button_visible(MainMenuMode::Main));
        assert!(root_plugins_button_visible(MainMenuMode::InGame));
        assert!(!root_plugins_button_visible(MainMenuMode::Hidden));
    }

    #[test]
    fn main_menu_plugins_default_plugin_is_locked() {
        let mut enabled = EnabledPluginSet::default();
        enabled.enforce_default_plugin();
        let entry = plugin_entry("flux.default", PluginRuntimeStatus::Enabled, true);
        assert_eq!(
            plugin_toggle_control(&entry, &enabled, MainMenuMode::Main, false),
            PluginToggleControl::Locked
        );
        assert_eq!(
            plugin_toggle_label(&entry, &enabled, MainMenuMode::Main, false),
            "On / Locked"
        );
    }

    #[test]
    fn main_menu_plugins_toggle_is_disabled_with_loaded_world() {
        let enabled = EnabledPluginSet::default();
        let entry = plugin_entry("sample.plugin", PluginRuntimeStatus::Disabled, false);
        assert_eq!(
            plugin_toggle_control(&entry, &enabled, MainMenuMode::Main, true),
            PluginToggleControl::Disabled
        );
    }

    #[test]
    fn main_menu_plugins_read_only_label_preserves_enabled_state() {
        let plugin_id = PluginId::parse("sample.plugin").expect("valid plugin id");
        let mut enabled = EnabledPluginSet::default();
        enabled.set_enabled(&plugin_id, true);
        let entry = plugin_entry("sample.plugin", PluginRuntimeStatus::Enabled, false);

        assert_eq!(
            plugin_toggle_control(&entry, &enabled, MainMenuMode::InGame, true),
            PluginToggleControl::Disabled
        );
        assert_eq!(
            plugin_toggle_label(&entry, &enabled, MainMenuMode::InGame, true),
            "On"
        );
    }

    #[test]
    fn main_menu_plugins_game_menu_uses_state_label_instead_of_switch() {
        let plugin_id = PluginId::parse("sample.plugin").expect("valid plugin id");
        let mut enabled = EnabledPluginSet::default();
        enabled.set_enabled(&plugin_id, true);
        let entry = plugin_entry("sample.plugin", PluginRuntimeStatus::Enabled, false);

        assert_eq!(
            plugin_toggle_presentation(&entry, &enabled, MainMenuMode::InGame, true),
            PluginTogglePresentation::StateLabel
        );
    }

    #[test]
    fn main_menu_plugins_main_menu_uses_interactive_switch_before_world_load() {
        let enabled = EnabledPluginSet::default();
        let entry = plugin_entry("sample.plugin", PluginRuntimeStatus::Disabled, false);

        assert_eq!(
            plugin_toggle_presentation(&entry, &enabled, MainMenuMode::Main, false),
            PluginTogglePresentation::Switch { interactive: true }
        );
    }

    #[test]
    fn main_menu_plugins_reload_is_allowed_only_before_world_load() {
        assert!(plugin_reload_allowed(MainMenuMode::Main, false));
        assert!(!plugin_reload_allowed(MainMenuMode::Main, true));
        assert!(!plugin_reload_allowed(MainMenuMode::InGame, true));
        assert!(!plugin_reload_allowed(MainMenuMode::Hidden, false));
    }

    #[test]
    fn main_menu_plugins_read_only_label_preserves_disabled_state() {
        let enabled = EnabledPluginSet::default();
        let entry = plugin_entry("sample.plugin", PluginRuntimeStatus::Disabled, false);

        assert_eq!(
            plugin_toggle_control(&entry, &enabled, MainMenuMode::InGame, true),
            PluginToggleControl::Disabled
        );
        assert_eq!(
            plugin_toggle_label(&entry, &enabled, MainMenuMode::InGame, true),
            "Off"
        );
    }

    #[test]
    fn main_menu_plugins_toggle_changes_enabled_set_before_world_load() {
        let plugin_id = PluginId::parse("sample.plugin").expect("valid plugin id");
        let enabled = EnabledPluginSet::default();
        let entry = plugin_entry("sample.plugin", PluginRuntimeStatus::Disabled, false);

        let PluginToggleControl::Actionable { next_enabled } =
            plugin_toggle_control(&entry, &enabled, MainMenuMode::Main, false)
        else {
            panic!("valid disabled plugin should be toggleable");
        };
        let mut next_enabled_set = enabled.clone();
        next_enabled_set.set_enabled(&plugin_id, next_enabled);
        assert!(next_enabled_set.is_enabled(&plugin_id));
    }

    #[test]
    fn main_menu_plugins_invalid_plugin_cannot_be_enabled() {
        let enabled = EnabledPluginSet::default();
        let entry = plugin_entry("sample.plugin", PluginRuntimeStatus::Error, false);
        assert_eq!(
            plugin_toggle_control(&entry, &enabled, MainMenuMode::Main, false),
            PluginToggleControl::Disabled
        );
    }

    #[test]
    fn main_menu_plugins_enabled_missing_plugin_can_be_disabled() {
        let plugin_id = PluginId::parse("missing.plugin").expect("valid plugin id");
        let mut enabled = EnabledPluginSet::default();
        enabled.set_enabled(&plugin_id, true);
        let entry = plugin_entry("missing.plugin", PluginRuntimeStatus::Missing, false);
        assert_eq!(
            plugin_toggle_control(&entry, &enabled, MainMenuMode::Main, false),
            PluginToggleControl::Actionable {
                next_enabled: false
            }
        );
    }

    #[test]
    fn main_menu_plugins_back_returns_to_root() {
        let mut ui = MainMenuUiState::default();
        ui.screen = MainMenuScreen::Plugins;
        ui.status_text = "Temporary status".to_string();
        return_main_menu_to_root(&mut ui);
        assert_eq!(ui.screen, MainMenuScreen::Root);
        assert!(ui.status_text.is_empty());
    }

    #[test]
    fn main_menu_plugins_normal_toggle_does_not_require_list_rebuild() {
        let enabled = plugin_entry("sample.plugin", PluginRuntimeStatus::Enabled, false);
        let disabled = plugin_entry("sample.plugin", PluginRuntimeStatus::Disabled, false);
        let old_state = PluginRegistryState {
            entries: vec![enabled],
            warnings: Vec::new(),
        };
        let new_state = PluginRegistryState {
            entries: vec![disabled],
            warnings: Vec::new(),
        };

        assert_eq!(
            plugin_registry_entry_keys(&old_state),
            plugin_registry_entry_keys(&new_state)
        );
        assert!(!plugin_list_requires_rebuild(&old_state, &new_state));
    }
}
