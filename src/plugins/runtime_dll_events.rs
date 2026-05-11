use bevy::prelude::UVec2;
use libloading::Library;

use crate::{
    plugins::{
        abi::{
            FluxBuildHudForCellEventPayload, FluxBuildPanelEventPayload, FluxEmptyEventPayload,
            FluxKeyEventPayload, FluxMouseCellEventPayload, FluxOnBuildHudForCellFn,
            FluxOnBuildPanelFn, FluxOnKeyPressedFn, FluxOnKeyReleasedFn, FluxOnMouseDownCellFn,
            FluxOnMouseEnterCellFn, FluxOnMouseLeaveCellFn, FluxOnMouseMoveCellFn,
            FluxOnMouseUpCellFn, FluxOnOverlayChangedFn, FluxOnRenderOverlayFn,
            FluxOnSimulationPausedChangedFn, FluxOnSimulationPostCellGasStepFn,
            FluxOnSimulationPreCellGasStepFn, FluxOnStructurePlacedFn, FluxOnStructureRemovedFn,
            FluxOnToolSelectedFn, FluxOnWorldAfterSaveFn, FluxOnWorldBeforeSaveFn,
            FluxOnWorldCreatedFn, FluxOnWorldLoadedFn, FluxOnWorldUnloadedFn,
            FluxOverlayChangedEventPayload, FluxPluginHandle, FluxRenderOverlayEventPayload,
            FluxRuntimeHost, FluxSimulationPausedChangedEvent, FluxStructureEventPayload,
            FluxToolSelectedEventPayload, FluxUtf8Slice,
        },
        api::events::{
            InputModifiers, MouseButton, MouseCellEvent, PluginEvent, PluginRuntimeEvent,
        },
        diagnostics::PluginContractError,
    },
    world::structures::PlacedStructureId,
};

use super::RuntimeHostContext;

/// One typed runtime event handler symbol resolved from a live plugin DLL.
pub(crate) enum RuntimeEventHandler {
    WorldCreated(FluxOnWorldCreatedFn),
    WorldLoaded(FluxOnWorldLoadedFn),
    WorldBeforeSave(FluxOnWorldBeforeSaveFn),
    WorldAfterSave(FluxOnWorldAfterSaveFn),
    WorldUnloaded(FluxOnWorldUnloadedFn),
    SimulationPreCellGasStep(FluxOnSimulationPreCellGasStepFn),
    SimulationPostCellGasStep(FluxOnSimulationPostCellGasStepFn),
    SimulationPausedChanged(FluxOnSimulationPausedChangedFn),
    StructurePlaced(FluxOnStructurePlacedFn),
    StructureRemoved(FluxOnStructureRemovedFn),
    ToolSelected(FluxOnToolSelectedFn),
    MouseDownCell(FluxOnMouseDownCellFn),
    MouseMoveCell(FluxOnMouseMoveCellFn),
    MouseUpCell(FluxOnMouseUpCellFn),
    MouseEnterCell(FluxOnMouseEnterCellFn),
    MouseLeaveCell(FluxOnMouseLeaveCellFn),
    KeyPressed(FluxOnKeyPressedFn),
    KeyReleased(FluxOnKeyReleasedFn),
    OverlayChanged(FluxOnOverlayChangedFn),
    BuildHudForCell(FluxOnBuildHudForCellFn),
    BuildPanel(FluxOnBuildPanelFn),
    RenderOverlay(FluxOnRenderOverlayFn),
}

pub(crate) fn load_event_handler(
    library: &Library,
    event_kind: PluginEvent,
    handler_name: &str,
) -> Result<RuntimeEventHandler, PluginContractError> {
    Ok(match event_kind {
        PluginEvent::WorldCreated => {
            RuntimeEventHandler::WorldCreated(load_handler::<FluxOnWorldCreatedFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::WorldLoaded => {
            RuntimeEventHandler::WorldLoaded(load_handler::<FluxOnWorldLoadedFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::WorldBeforeSave => {
            RuntimeEventHandler::WorldBeforeSave(load_handler::<FluxOnWorldBeforeSaveFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::WorldAfterSave => {
            RuntimeEventHandler::WorldAfterSave(load_handler::<FluxOnWorldAfterSaveFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::WorldUnloaded => {
            RuntimeEventHandler::WorldUnloaded(load_handler::<FluxOnWorldUnloadedFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::SimulationPreCellGasStep => {
            RuntimeEventHandler::SimulationPreCellGasStep(load_handler::<
                FluxOnSimulationPreCellGasStepFn,
            >(
                library, handler_name, event_kind
            )?)
        }
        PluginEvent::SimulationPostCellGasStep => {
            RuntimeEventHandler::SimulationPostCellGasStep(load_handler::<
                FluxOnSimulationPostCellGasStepFn,
            >(
                library, handler_name, event_kind
            )?)
        }
        PluginEvent::SimulationPausedChanged => {
            RuntimeEventHandler::SimulationPausedChanged(load_handler::<
                FluxOnSimulationPausedChangedFn,
            >(
                library, handler_name, event_kind
            )?)
        }
        PluginEvent::StructurePlaced => {
            RuntimeEventHandler::StructurePlaced(load_handler::<FluxOnStructurePlacedFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::StructureRemoved => {
            RuntimeEventHandler::StructureRemoved(load_handler::<FluxOnStructureRemovedFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::ToolSelected => {
            RuntimeEventHandler::ToolSelected(load_handler::<FluxOnToolSelectedFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::MouseDownCell => {
            RuntimeEventHandler::MouseDownCell(load_handler::<FluxOnMouseDownCellFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::MouseMoveCell => {
            RuntimeEventHandler::MouseMoveCell(load_handler::<FluxOnMouseMoveCellFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::MouseUpCell => {
            RuntimeEventHandler::MouseUpCell(load_handler::<FluxOnMouseUpCellFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::MouseEnterCell => {
            RuntimeEventHandler::MouseEnterCell(load_handler::<FluxOnMouseEnterCellFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::MouseLeaveCell => {
            RuntimeEventHandler::MouseLeaveCell(load_handler::<FluxOnMouseLeaveCellFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::KeyPressed => {
            RuntimeEventHandler::KeyPressed(load_handler::<FluxOnKeyPressedFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::KeyReleased => {
            RuntimeEventHandler::KeyReleased(load_handler::<FluxOnKeyReleasedFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::OverlayChanged => {
            RuntimeEventHandler::OverlayChanged(load_handler::<FluxOnOverlayChangedFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::BuildHudForCell => {
            RuntimeEventHandler::BuildHudForCell(load_handler::<FluxOnBuildHudForCellFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::BuildPanel => {
            RuntimeEventHandler::BuildPanel(load_handler::<FluxOnBuildPanelFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
        PluginEvent::RenderOverlay => {
            RuntimeEventHandler::RenderOverlay(load_handler::<FluxOnRenderOverlayFn>(
                library,
                handler_name,
                event_kind,
            )?)
        }
    })
}

pub(super) fn dispatch_to_plugin(
    handle: *mut FluxPluginHandle,
    handler: &RuntimeEventHandler,
    event: &PluginRuntimeEvent,
    context: &mut RuntimeHostContext,
) {
    let mut host = FluxRuntimeHost::new((context as *mut RuntimeHostContext).cast());
    unsafe {
        match (handler, event) {
            (RuntimeEventHandler::WorldCreated(handler), PluginRuntimeEvent::WorldCreated) => {
                let payload = FluxEmptyEventPayload::new();
                let _ = handler(handle, &payload, &mut host);
            }
            (RuntimeEventHandler::WorldLoaded(handler), PluginRuntimeEvent::WorldLoaded) => {
                let payload = FluxEmptyEventPayload::new();
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::WorldBeforeSave(handler),
                PluginRuntimeEvent::WorldBeforeSave,
            ) => {
                let payload = FluxEmptyEventPayload::new();
                let _ = handler(handle, &payload, &mut host);
            }
            (RuntimeEventHandler::WorldAfterSave(handler), PluginRuntimeEvent::WorldAfterSave) => {
                let payload = FluxEmptyEventPayload::new();
                let _ = handler(handle, &payload, &mut host);
            }
            (RuntimeEventHandler::WorldUnloaded(handler), PluginRuntimeEvent::WorldUnloaded) => {
                let payload = FluxEmptyEventPayload::new();
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::SimulationPreCellGasStep(handler),
                PluginRuntimeEvent::SimulationPreCellGasStep,
            ) => {
                let payload = FluxEmptyEventPayload::new();
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::SimulationPostCellGasStep(handler),
                PluginRuntimeEvent::SimulationPostCellGasStep,
            ) => {
                let payload = FluxEmptyEventPayload::new();
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::SimulationPausedChanged(handler),
                PluginRuntimeEvent::SimulationPausedChanged { paused },
            ) => {
                let payload = build_paused_payload(*paused);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::StructurePlaced(handler),
                PluginRuntimeEvent::StructurePlaced(event),
            ) => {
                let payload = build_structure_payload(event.id, event.kind.as_str(), event.cell);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::StructureRemoved(handler),
                PluginRuntimeEvent::StructureRemoved(event),
            ) => {
                let payload = build_structure_payload(event.id, event.kind.as_str(), event.cell);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::ToolSelected(handler),
                PluginRuntimeEvent::ToolSelected { tool_id },
            ) => {
                let payload = build_tool_selected_payload(tool_id.as_ref().map(|id| id.as_str()));
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::MouseDownCell(handler),
                PluginRuntimeEvent::MouseDownCell(mouse),
            ) => {
                let payload = build_mouse_payload(mouse);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::MouseMoveCell(handler),
                PluginRuntimeEvent::MouseMoveCell(mouse),
            ) => {
                let payload = build_mouse_payload(mouse);
                let _ = handler(handle, &payload, &mut host);
            }
            (RuntimeEventHandler::MouseUpCell(handler), PluginRuntimeEvent::MouseUpCell(mouse)) => {
                let payload = build_mouse_payload(mouse);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::MouseEnterCell(handler),
                PluginRuntimeEvent::MouseEnterCell(mouse),
            ) => {
                let payload = build_mouse_payload(mouse);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::MouseLeaveCell(handler),
                PluginRuntimeEvent::MouseLeaveCell(mouse),
            ) => {
                let payload = build_mouse_payload(mouse);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::KeyPressed(handler),
                PluginRuntimeEvent::KeyPressed { key, modifiers },
            ) => {
                let payload = build_key_payload(key, *modifiers);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::KeyReleased(handler),
                PluginRuntimeEvent::KeyReleased { key, modifiers },
            ) => {
                let payload = build_key_payload(key, *modifiers);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::OverlayChanged(handler),
                PluginRuntimeEvent::OverlayChanged { overlay_id },
            ) => {
                let payload =
                    build_overlay_changed_payload(overlay_id.as_ref().map(|id| id.as_str()));
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::BuildHudForCell(handler),
                PluginRuntimeEvent::BuildHudForCell { cell },
            ) => {
                let payload = build_hud_payload(*cell);
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::BuildPanel(handler),
                PluginRuntimeEvent::BuildPanel { panel_id },
            ) => {
                let payload = build_panel_payload(panel_id.as_str());
                let _ = handler(handle, &payload, &mut host);
            }
            (
                RuntimeEventHandler::RenderOverlay(handler),
                PluginRuntimeEvent::RenderOverlay { overlay_id },
            ) => {
                let payload = build_render_overlay_payload(overlay_id.as_str());
                let _ = handler(handle, &payload, &mut host);
            }
            _ => {}
        }
    }
}

fn build_paused_payload(paused: bool) -> FluxSimulationPausedChangedEvent {
    FluxSimulationPausedChangedEvent {
        struct_size: std::mem::size_of::<FluxSimulationPausedChangedEvent>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        paused: paused as u8,
    }
}

fn build_structure_payload(
    id: PlacedStructureId,
    structure_kind: &str,
    cell: UVec2,
) -> FluxStructureEventPayload {
    FluxStructureEventPayload {
        struct_size: std::mem::size_of::<FluxStructureEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        structure_id: id.0,
        structure_kind: FluxUtf8Slice::from_str(structure_kind),
        cell_x: cell.x,
        cell_y: cell.y,
    }
}

fn build_tool_selected_payload(tool_id: Option<&str>) -> FluxToolSelectedEventPayload {
    FluxToolSelectedEventPayload {
        struct_size: std::mem::size_of::<FluxToolSelectedEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        has_tool_id: tool_id.is_some() as u8,
        tool_id: FluxUtf8Slice::from_str(tool_id.unwrap_or("")),
    }
}

fn build_mouse_payload(mouse: &MouseCellEvent) -> FluxMouseCellEventPayload {
    FluxMouseCellEventPayload {
        struct_size: std::mem::size_of::<FluxMouseCellEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        button: mouse.button.map(mouse_button_to_abi).unwrap_or(0),
        has_cell: 1,
        cell_x: mouse.cell.x,
        cell_y: mouse.cell.y,
        world_x: mouse.world_position.x,
        world_y: mouse.world_position.y,
        screen_x: mouse.screen_position.x,
        screen_y: mouse.screen_position.y,
        modifiers: modifiers_to_abi(mouse.modifiers),
        has_active_tool_id: mouse.active_tool_id.is_some() as u8,
        active_tool_id: FluxUtf8Slice::from_str(
            mouse
                .active_tool_id
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or(""),
        ),
        is_over_ui: mouse.is_over_ui as u8,
    }
}

fn build_key_payload(key: &str, modifiers: InputModifiers) -> FluxKeyEventPayload {
    FluxKeyEventPayload {
        struct_size: std::mem::size_of::<FluxKeyEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        key: FluxUtf8Slice::from_str(key),
        modifiers: modifiers_to_abi(modifiers),
    }
}

fn build_overlay_changed_payload(overlay_id: Option<&str>) -> FluxOverlayChangedEventPayload {
    FluxOverlayChangedEventPayload {
        struct_size: std::mem::size_of::<FluxOverlayChangedEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        has_overlay_id: overlay_id.is_some() as u8,
        overlay_id: FluxUtf8Slice::from_str(overlay_id.unwrap_or("")),
    }
}

fn build_hud_payload(cell: UVec2) -> FluxBuildHudForCellEventPayload {
    FluxBuildHudForCellEventPayload {
        struct_size: std::mem::size_of::<FluxBuildHudForCellEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        cell_x: cell.x,
        cell_y: cell.y,
    }
}

fn build_panel_payload(panel_id: &str) -> FluxBuildPanelEventPayload {
    FluxBuildPanelEventPayload {
        struct_size: std::mem::size_of::<FluxBuildPanelEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        panel_id: FluxUtf8Slice::from_str(panel_id),
    }
}

fn build_render_overlay_payload(overlay_id: &str) -> FluxRenderOverlayEventPayload {
    FluxRenderOverlayEventPayload {
        struct_size: std::mem::size_of::<FluxRenderOverlayEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        overlay_id: FluxUtf8Slice::from_str(overlay_id),
    }
}

fn mouse_button_to_abi(button: MouseButton) -> u32 {
    match button {
        MouseButton::Left => 1,
        MouseButton::Right => 2,
        MouseButton::Middle => 3,
        MouseButton::Other(value) => 1000 + value as u32,
    }
}

fn modifiers_to_abi(modifiers: InputModifiers) -> u32 {
    let mut bits = 0u32;
    if modifiers.shift {
        bits |= 1;
    }
    if modifiers.ctrl {
        bits |= 2;
    }
    if modifiers.alt {
        bits |= 4;
    }
    bits
}

fn load_handler<T: Copy>(
    library: &Library,
    handler_name: &str,
    event_kind: PluginEvent,
) -> Result<T, PluginContractError> {
    let export_name = format!("{handler_name}\0");
    unsafe {
        library
            .get::<T>(export_name.as_bytes())
            .map(|symbol| *symbol)
            .map_err(|error| {
                PluginContractError::Dll(format!(
                    "missing handler '{}' for event {:?}: {}",
                    handler_name, event_kind, error
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_hud_payload, build_key_payload, build_mouse_payload, build_render_overlay_payload,
    };
    use crate::plugins::api::events::{InputModifiers, MouseButton, MouseCellEvent};
    use bevy::prelude::*;

    #[test]
    fn key_payload_carries_key_and_modifier_bits() {
        let payload = build_key_payload(
            "Space",
            InputModifiers {
                shift: true,
                ctrl: false,
                alt: true,
            },
        );

        assert_eq!(payload.key.len, 5);
        assert_eq!(payload.modifiers, 5);
    }

    #[test]
    fn mouse_payload_carries_cell_tool_and_button() {
        let payload = build_mouse_payload(&MouseCellEvent {
            button: Some(MouseButton::Right),
            cell: UVec2::new(11, 13),
            world_position: Vec2::new(1.5, 2.5),
            screen_position: Vec2::new(30.0, 40.0),
            modifiers: InputModifiers {
                shift: false,
                ctrl: true,
                alt: false,
            },
            active_tool_id: Some(
                crate::plugins::ContentId::parse("flux.test.tool.paint").expect("tool id"),
            ),
            is_over_ui: true,
        });

        assert_eq!(payload.button, 2);
        assert_eq!(payload.cell_x, 11);
        assert_eq!(payload.cell_y, 13);
        assert_eq!(payload.modifiers, 2);
        assert_eq!(payload.has_active_tool_id, 1);
        assert_eq!(payload.is_over_ui, 1);
    }

    #[test]
    fn build_hud_payload_contains_cell_coordinates() {
        let payload = build_hud_payload(UVec2::new(7, 9));

        assert_eq!(payload.cell_x, 7);
        assert_eq!(payload.cell_y, 9);
    }

    #[test]
    fn render_overlay_payload_contains_overlay_id() {
        let payload = build_render_overlay_payload("flux.test.overlay.temperature");

        assert_eq!(
            payload.overlay_id.len,
            "flux.test.overlay.temperature".len()
        );
    }
}
