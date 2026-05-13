use std::path::Path;

use flux_plugin_sdk::{
    __private::{BuiltinPluginRuntime, DispatchState},
    BuildHudForCellEvent, EntityEvent, KeyEvent, MouseCellEvent, OverlayChangedEvent, PluginError,
    PluginEvent, PluginPaths, RenderOverlayEvent, SimulationPausedChangedEvent,
    SimulationPostCellGasStepEvent, SimulationPreCellGasStepEvent, ToolSelectedEvent,
    WorldAfterSaveEvent, WorldBeforeSaveEvent, WorldCreatedEvent, WorldLoadedEvent,
    WorldUnloadedEvent,
};

use crate::plugins::{
    api::{
        runtime::{RuntimeOverlayDescriptor, SaveChunkDescriptor},
        ui_api::ToolDescriptor,
    },
    default_plugin::{
        asset_root, config_root,
        runtime_sdk::{with_runtime_context, FluxDefaultRuntimeSdkPlugin},
    },
    runtime_dll::RuntimeHostContext,
    runtime_host_binding::sdk_runtime_host_binding,
    PluginId, PluginRuntimeEvent, PluginRuntimeRegistration, PluginSubscriptionRegistration,
    SubstanceDefinition, SubstanceFlags,
};

/// Common runtime endpoint abstraction shared by built-in and DLL plugins.
pub trait RuntimePluginEndpoint: Send + Sync {
    /// Returns the stable plugin id of this runtime endpoint.
    fn plugin_id(&self) -> &PluginId;

    /// Returns the deterministic dispatch group order.
    fn group_order(&self) -> u8;

    /// Dispatches one runtime event into the endpoint.
    fn dispatch(
        &mut self,
        event: &PluginRuntimeEvent,
        context: &mut RuntimeHostContext,
    ) -> Result<(), String>;
}

/// Built-in runtime endpoint backed by the shared SDK contract.
pub struct BuiltinRuntimePluginEndpoint<P: flux_plugin_sdk::Plugin> {
    plugin_id: PluginId,
    runtime: BuiltinPluginRuntime<P>,
}

impl<P: flux_plugin_sdk::Plugin + Send + Sync> RuntimePluginEndpoint
    for BuiltinRuntimePluginEndpoint<P>
{
    fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    fn group_order(&self) -> u8 {
        0
    }

    fn dispatch(
        &mut self,
        event: &PluginRuntimeEvent,
        context: &mut RuntimeHostContext,
    ) -> Result<(), String> {
        let context_ptr = context as *mut RuntimeHostContext;
        with_runtime_context(context_ptr, || {
            let context = unsafe { &mut *context_ptr };
            dispatch_builtin_sdk_event(&mut self.runtime, event, context)
        })
        .map_err(|error| error.to_string())
    }
}

/// Builds the locked `flux.default` runtime registration through the shared SDK contract.
pub fn default_builtin_runtime_registration() -> Result<PluginRuntimeRegistration, String> {
    let runtime = build_default_builtin_runtime()?;
    Ok(convert_memory_registration(
        runtime.plugin_id().clone(),
        runtime.registration(),
    ))
}

/// Creates the live built-in runtime endpoint for `flux.default`.
pub fn default_builtin_runtime_endpoint() -> Result<Box<dyn RuntimePluginEndpoint>, String> {
    let runtime = build_default_builtin_runtime()?;
    Ok(Box::new(BuiltinRuntimePluginEndpoint::<
        FluxDefaultRuntimeSdkPlugin,
    > {
        plugin_id: engine_plugin_id(runtime.plugin_id()),
        runtime,
    }))
}

fn build_default_builtin_runtime(
) -> Result<BuiltinPluginRuntime<FluxDefaultRuntimeSdkPlugin>, String> {
    let plugin_id = sdk_plugin_id(&PluginId::default_plugin())?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut runtime = BuiltinPluginRuntime::<FluxDefaultRuntimeSdkPlugin>::create(
        plugin_id.clone(),
        env!("CARGO_PKG_VERSION").to_string(),
        flux_plugin_sdk::PluginApiVersion(crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE),
        PluginPaths {
            plugin_root: repo_root.to_path_buf(),
            config_root: config_root(repo_root),
            assets_root: asset_root(repo_root),
        },
    )
    .map_err(|error| error.to_string())?;
    runtime.register().map_err(|error| error.to_string())?;
    Ok(runtime)
}

fn convert_memory_registration(
    plugin_id: flux_plugin_sdk::PluginId,
    registration: &flux_plugin_sdk::__private::MemoryRegistration,
) -> PluginRuntimeRegistration {
    PluginRuntimeRegistration {
        gas_substances: registration
            .substances
            .iter()
            .map(|substance| SubstanceDefinition {
                id: engine_substance_id(&substance.id),
                plugin_id: engine_plugin_id(&plugin_id),
                label: substance.label.clone(),
                molecular_mass: substance.molecular_mass,
                color: substance.color,
                aliases: vec![substance.alias.clone()],
                flags: SubstanceFlags::gas(),
            })
            .collect(),
        entities: registration.entities.clone(),
        subscriptions: registration
            .subscriptions
            .iter()
            .copied()
            .map(|event_kind| PluginSubscriptionRegistration {
                event_kind: engine_event_kind(event_kind),
            })
            .collect(),
        tools: registration
            .tools
            .iter()
            .cloned()
            .map(|tool| ToolDescriptor {
                id: engine_content_id(&tool.id),
                label: tool.label,
                icon_path: tool.icon_path,
                silhouette_path: tool.silhouette_path,
            })
            .collect(),
        overlays: registration
            .overlays
            .iter()
            .cloned()
            .map(|overlay| RuntimeOverlayDescriptor {
                id: engine_content_id(&overlay.id),
                plugin_id: engine_plugin_id(&plugin_id),
                label: overlay.label,
                hotkey: overlay.hotkey,
                render_policy: engine_overlay_render_policy(overlay.render_policy),
                graph: overlay.graph,
            })
            .collect(),
        overlay_materials: registration.overlay_materials.clone(),
        save_chunks: registration
            .save_chunks
            .iter()
            .cloned()
            .map(|chunk| SaveChunkDescriptor {
                id: engine_content_id(&chunk.id),
                plugin_id: engine_plugin_id(&plugin_id),
                version: chunk.version,
            })
            .collect(),
    }
}

fn dispatch_builtin_sdk_event<P: flux_plugin_sdk::Plugin>(
    runtime: &mut BuiltinPluginRuntime<P>,
    event: &PluginRuntimeEvent,
    context: &mut RuntimeHostContext,
) -> Result<(), PluginError> {
    let binding = sdk_runtime_host_binding(context);
    match event {
        PluginRuntimeEvent::WorldCreated => {
            let payload = WorldCreatedEvent;
            runtime.dispatch(
                PluginEvent::WorldCreated,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::WorldLoaded => {
            let payload = WorldLoadedEvent;
            runtime.dispatch(
                PluginEvent::WorldLoaded,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::WorldBeforeSave => {
            let payload = WorldBeforeSaveEvent;
            runtime.dispatch(
                PluginEvent::WorldBeforeSave,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::WorldAfterSave => {
            let payload = WorldAfterSaveEvent;
            runtime.dispatch(
                PluginEvent::WorldAfterSave,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::WorldUnloaded => {
            let payload = WorldUnloadedEvent;
            runtime.dispatch(
                PluginEvent::WorldUnloaded,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::SimulationPreCellGasStep => {
            let payload = SimulationPreCellGasStepEvent;
            runtime.dispatch(
                PluginEvent::SimulationPreCellGasStep,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::SimulationPostCellGasStep => {
            let payload = SimulationPostCellGasStepEvent;
            runtime.dispatch(
                PluginEvent::SimulationPostCellGasStep,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::SimulationPausedChanged { paused } => {
            let payload = SimulationPausedChangedEvent { paused: *paused };
            runtime.dispatch(
                PluginEvent::SimulationPausedChanged,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::StructurePlaced(event) => {
            let payload = EntityEvent {
                id: flux_plugin_sdk::EntityInstanceId(event.id.0),
                kind: flux_plugin_sdk::EntityKindId::parse(event.kind.as_str())
                    .map_err(PluginError::message)?,
                cell: event.cell,
            };
            runtime.dispatch(
                PluginEvent::EntityPlaced,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::StructureRemoved(event) => {
            let payload = EntityEvent {
                id: flux_plugin_sdk::EntityInstanceId(event.id.0),
                kind: flux_plugin_sdk::EntityKindId::parse(event.kind.as_str())
                    .map_err(PluginError::message)?,
                cell: event.cell,
            };
            runtime.dispatch(
                PluginEvent::EntityRemoved,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::ToolSelected { tool_id } => {
            let payload = ToolSelectedEvent {
                tool_id: tool_id
                    .as_ref()
                    .map(|id| flux_plugin_sdk::ContentId::parse(id.as_str()))
                    .transpose()
                    .map_err(PluginError::message)?,
            };
            runtime.dispatch(
                PluginEvent::ToolSelected,
                &payload,
                binding,
                DispatchState {
                    active_tool_id: payload.tool_id.clone(),
                    ..DispatchState::default()
                },
            )
        }
        PluginRuntimeEvent::MouseDownCell(mouse)
        | PluginRuntimeEvent::MouseMoveCell(mouse)
        | PluginRuntimeEvent::MouseUpCell(mouse)
        | PluginRuntimeEvent::MouseEnterCell(mouse)
        | PluginRuntimeEvent::MouseLeaveCell(mouse) => {
            let payload = convert_mouse_event(mouse)?;
            let event_kind = sdk_event_kind(event.kind());
            runtime.dispatch(
                event_kind,
                &payload,
                binding,
                dispatch_state_from_mouse(&payload),
            )
        }
        PluginRuntimeEvent::KeyPressed { key, modifiers }
        | PluginRuntimeEvent::KeyReleased { key, modifiers } => {
            let payload = KeyEvent {
                key: key.clone(),
                modifiers: convert_modifiers(*modifiers),
            };
            runtime.dispatch(
                sdk_event_kind(event.kind()),
                &payload,
                binding,
                DispatchState {
                    modifiers: payload.modifiers,
                    ..DispatchState::default()
                },
            )
        }
        PluginRuntimeEvent::OverlayChanged { overlay_id } => {
            let converted = overlay_id
                .as_ref()
                .map(|id| flux_plugin_sdk::ContentId::parse(id.as_str()))
                .transpose()
                .map_err(PluginError::message)?;
            let payload = OverlayChangedEvent {
                overlay_id: converted.clone(),
            };
            runtime.dispatch(
                PluginEvent::OverlayChanged,
                &payload,
                binding,
                DispatchState {
                    active_overlay: converted.clone(),
                    requested_overlay: converted,
                    ..DispatchState::default()
                },
            )
        }
        PluginRuntimeEvent::BuildHudForCell { cell } => {
            let payload = BuildHudForCellEvent { cell: *cell };
            runtime.dispatch(
                PluginEvent::BuildHudForCell,
                &payload,
                binding,
                DispatchState::default(),
            )
        }
        PluginRuntimeEvent::RenderOverlay { overlay_id } => {
            let converted = flux_plugin_sdk::ContentId::parse(overlay_id.as_str())
                .map_err(PluginError::message)?;
            let payload = RenderOverlayEvent {
                overlay_id: converted.clone(),
            };
            runtime.dispatch(
                PluginEvent::RenderOverlay,
                &payload,
                binding,
                DispatchState {
                    active_overlay: Some(converted.clone()),
                    requested_overlay: Some(converted),
                    ..DispatchState::default()
                },
            )
        }
    }
}

fn dispatch_state_from_mouse(event: &MouseCellEvent) -> DispatchState {
    DispatchState {
        modifiers: event.modifiers,
        active_tool_id: event.active_tool_id.clone(),
        cursor_world: Some(event.world_position),
        cursor_screen: Some(event.screen_position),
        is_pointer_over_ui: event.is_over_ui,
        requested_overlay: None,
        active_overlay: None,
    }
}

fn convert_mouse_event(
    event: &crate::plugins::MouseCellEvent,
) -> Result<MouseCellEvent, PluginError> {
    Ok(MouseCellEvent {
        button: event.button.map(|button| match button {
            crate::plugins::MouseButton::Left => flux_plugin_sdk::MouseButton::Left,
            crate::plugins::MouseButton::Right => flux_plugin_sdk::MouseButton::Right,
            crate::plugins::MouseButton::Middle => flux_plugin_sdk::MouseButton::Middle,
            crate::plugins::MouseButton::Other(value) => flux_plugin_sdk::MouseButton::Other(value),
        }),
        cell: event.cell,
        world_position: event.world_position,
        screen_position: event.screen_position,
        modifiers: convert_modifiers(event.modifiers),
        active_tool_id: event
            .active_tool_id
            .as_ref()
            .map(|id| flux_plugin_sdk::ContentId::parse(id.as_str()))
            .transpose()
            .map_err(PluginError::message)?,
        is_over_ui: event.is_over_ui,
    })
}

fn convert_modifiers(modifiers: crate::plugins::InputModifiers) -> flux_plugin_sdk::InputModifiers {
    flux_plugin_sdk::InputModifiers {
        shift: modifiers.shift,
        ctrl: modifiers.ctrl,
        alt: modifiers.alt,
    }
}

fn sdk_plugin_id(plugin_id: &PluginId) -> Result<flux_plugin_sdk::PluginId, String> {
    flux_plugin_sdk::PluginId::parse(plugin_id.as_str())
}

fn engine_plugin_id(plugin_id: &flux_plugin_sdk::PluginId) -> PluginId {
    PluginId::parse(plugin_id.as_str()).expect("sdk plugin id must stay valid")
}

fn engine_content_id(content_id: &flux_plugin_sdk::ContentId) -> crate::plugins::ContentId {
    crate::plugins::ContentId::parse(content_id.as_str()).expect("sdk content id must stay valid")
}

fn engine_substance_id(substance_id: &flux_plugin_sdk::SubstanceId) -> crate::plugins::SubstanceId {
    crate::plugins::SubstanceId::parse(substance_id.as_str())
        .expect("sdk substance id must stay valid")
}

fn engine_event_kind(event: flux_plugin_sdk::PluginEvent) -> crate::plugins::PluginEvent {
    match event {
        flux_plugin_sdk::PluginEvent::WorldCreated => crate::plugins::PluginEvent::WorldCreated,
        flux_plugin_sdk::PluginEvent::WorldLoaded => crate::plugins::PluginEvent::WorldLoaded,
        flux_plugin_sdk::PluginEvent::WorldBeforeSave => {
            crate::plugins::PluginEvent::WorldBeforeSave
        }
        flux_plugin_sdk::PluginEvent::WorldAfterSave => crate::plugins::PluginEvent::WorldAfterSave,
        flux_plugin_sdk::PluginEvent::WorldUnloaded => crate::plugins::PluginEvent::WorldUnloaded,
        flux_plugin_sdk::PluginEvent::SimulationPreCellGasStep => {
            crate::plugins::PluginEvent::SimulationPreCellGasStep
        }
        flux_plugin_sdk::PluginEvent::SimulationPostCellGasStep => {
            crate::plugins::PluginEvent::SimulationPostCellGasStep
        }
        flux_plugin_sdk::PluginEvent::SimulationPausedChanged => {
            crate::plugins::PluginEvent::SimulationPausedChanged
        }
        flux_plugin_sdk::PluginEvent::EntityPlaced => crate::plugins::PluginEvent::StructurePlaced,
        flux_plugin_sdk::PluginEvent::EntityRemoved => {
            crate::plugins::PluginEvent::StructureRemoved
        }
        flux_plugin_sdk::PluginEvent::ToolSelected => crate::plugins::PluginEvent::ToolSelected,
        flux_plugin_sdk::PluginEvent::MouseDownCell => crate::plugins::PluginEvent::MouseDownCell,
        flux_plugin_sdk::PluginEvent::MouseMoveCell => crate::plugins::PluginEvent::MouseMoveCell,
        flux_plugin_sdk::PluginEvent::MouseUpCell => crate::plugins::PluginEvent::MouseUpCell,
        flux_plugin_sdk::PluginEvent::MouseEnterCell => crate::plugins::PluginEvent::MouseEnterCell,
        flux_plugin_sdk::PluginEvent::MouseLeaveCell => crate::plugins::PluginEvent::MouseLeaveCell,
        flux_plugin_sdk::PluginEvent::KeyPressed => crate::plugins::PluginEvent::KeyPressed,
        flux_plugin_sdk::PluginEvent::KeyReleased => crate::plugins::PluginEvent::KeyReleased,
        flux_plugin_sdk::PluginEvent::OverlayChanged => crate::plugins::PluginEvent::OverlayChanged,
        flux_plugin_sdk::PluginEvent::BuildHudForCell => {
            crate::plugins::PluginEvent::BuildHudForCell
        }
        flux_plugin_sdk::PluginEvent::RenderOverlay => crate::plugins::PluginEvent::RenderOverlay,
    }
}

fn sdk_event_kind(event: crate::plugins::PluginEvent) -> flux_plugin_sdk::PluginEvent {
    match event {
        crate::plugins::PluginEvent::WorldCreated => flux_plugin_sdk::PluginEvent::WorldCreated,
        crate::plugins::PluginEvent::WorldLoaded => flux_plugin_sdk::PluginEvent::WorldLoaded,
        crate::plugins::PluginEvent::WorldBeforeSave => {
            flux_plugin_sdk::PluginEvent::WorldBeforeSave
        }
        crate::plugins::PluginEvent::WorldAfterSave => flux_plugin_sdk::PluginEvent::WorldAfterSave,
        crate::plugins::PluginEvent::WorldUnloaded => flux_plugin_sdk::PluginEvent::WorldUnloaded,
        crate::plugins::PluginEvent::SimulationPreCellGasStep => {
            flux_plugin_sdk::PluginEvent::SimulationPreCellGasStep
        }
        crate::plugins::PluginEvent::SimulationPostCellGasStep => {
            flux_plugin_sdk::PluginEvent::SimulationPostCellGasStep
        }
        crate::plugins::PluginEvent::SimulationPausedChanged => {
            flux_plugin_sdk::PluginEvent::SimulationPausedChanged
        }
        crate::plugins::PluginEvent::StructurePlaced => flux_plugin_sdk::PluginEvent::EntityPlaced,
        crate::plugins::PluginEvent::StructureRemoved => {
            flux_plugin_sdk::PluginEvent::EntityRemoved
        }
        crate::plugins::PluginEvent::ToolSelected => flux_plugin_sdk::PluginEvent::ToolSelected,
        crate::plugins::PluginEvent::MouseDownCell => flux_plugin_sdk::PluginEvent::MouseDownCell,
        crate::plugins::PluginEvent::MouseMoveCell => flux_plugin_sdk::PluginEvent::MouseMoveCell,
        crate::plugins::PluginEvent::MouseUpCell => flux_plugin_sdk::PluginEvent::MouseUpCell,
        crate::plugins::PluginEvent::MouseEnterCell => flux_plugin_sdk::PluginEvent::MouseEnterCell,
        crate::plugins::PluginEvent::MouseLeaveCell => flux_plugin_sdk::PluginEvent::MouseLeaveCell,
        crate::plugins::PluginEvent::KeyPressed => flux_plugin_sdk::PluginEvent::KeyPressed,
        crate::plugins::PluginEvent::KeyReleased => flux_plugin_sdk::PluginEvent::KeyReleased,
        crate::plugins::PluginEvent::OverlayChanged => flux_plugin_sdk::PluginEvent::OverlayChanged,
        crate::plugins::PluginEvent::BuildHudForCell => {
            flux_plugin_sdk::PluginEvent::BuildHudForCell
        }
        crate::plugins::PluginEvent::RenderOverlay => flux_plugin_sdk::PluginEvent::RenderOverlay,
    }
}

fn engine_overlay_render_policy(
    policy: flux_plugin_sdk::OverlayRenderPolicy,
) -> crate::plugins::OverlayRenderPolicy {
    match policy {
        flux_plugin_sdk::OverlayRenderPolicy::CoreDefault => {
            crate::plugins::OverlayRenderPolicy::CoreDefault
        }
        flux_plugin_sdk::OverlayRenderPolicy::PluginControlled => {
            crate::plugins::OverlayRenderPolicy::PluginControlled
        }
    }
}
