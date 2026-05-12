use bevy_math::Vec2;
use flux_plugin_abi::{
    FluxBuildPanelEventPayload, FluxKeyEventPayload, FluxMouseCellEventPayload,
    FluxOverlayChangedEventPayload, FluxRenderOverlayEventPayload,
    FluxToolSelectedEventPayload,
};

use crate::{scope, ContentId, InputModifiers, PluginEvent};

pub(crate) struct DispatchStateBuilder;

impl DispatchStateBuilder {
    pub(crate) fn build(
        event_kind: PluginEvent,
        payload: *const u8,
        payload_len: usize,
    ) -> scope::DispatchState {
        let mut state = scope::DispatchState::default();
        unsafe {
            match event_kind {
                PluginEvent::MouseDownCell
                | PluginEvent::MouseMoveCell
                | PluginEvent::MouseUpCell
                | PluginEvent::MouseEnterCell
                | PluginEvent::MouseLeaveCell => {
                    if let Some(payload) = payload_ref::<FluxMouseCellEventPayload>(payload, payload_len) {
                        state.modifiers = decode_modifiers(payload.modifiers);
                        state.cursor_world = Some(Vec2::new(payload.world_x, payload.world_y));
                        state.cursor_screen = Some(Vec2::new(payload.screen_x, payload.screen_y));
                        state.active_tool_id = if payload.has_active_tool_id == 0 {
                            None
                        } else {
                            ContentId::parse(&read_utf8(payload.active_tool_id)).ok()
                        };
                        state.is_pointer_over_ui = payload.is_over_ui != 0;
                    }
                }
                PluginEvent::KeyPressed | PluginEvent::KeyReleased => {
                    if let Some(payload) = payload_ref::<FluxKeyEventPayload>(payload, payload_len) {
                        state.modifiers = decode_modifiers(payload.modifiers);
                    }
                }
                PluginEvent::ToolSelected => {
                    if let Some(payload) = payload_ref::<FluxToolSelectedEventPayload>(payload, payload_len) {
                        state.active_tool_id = if payload.has_tool_id == 0 {
                            None
                        } else {
                            ContentId::parse(&read_utf8(payload.tool_id)).ok()
                        };
                    }
                }
                PluginEvent::BuildPanel => {
                    if let Some(payload) = payload_ref::<FluxBuildPanelEventPayload>(payload, payload_len) {
                        state.requested_panel = ContentId::parse(&read_utf8(payload.panel_id)).ok();
                    }
                }
                PluginEvent::OverlayChanged => {
                    if let Some(payload) = payload_ref::<FluxOverlayChangedEventPayload>(payload, payload_len) {
                        let overlay_id = if payload.has_overlay_id == 0 {
                            None
                        } else {
                            ContentId::parse(&read_utf8(payload.overlay_id)).ok()
                        };
                        state.active_overlay = overlay_id.clone();
                        state.requested_overlay = overlay_id;
                    }
                }
                PluginEvent::RenderOverlay => {
                    if let Some(payload) = payload_ref::<FluxRenderOverlayEventPayload>(payload, payload_len) {
                        state.requested_overlay = ContentId::parse(&read_utf8(payload.overlay_id)).ok();
                        state.active_overlay = state.requested_overlay.clone();
                    }
                }
                _ => {}
            }
        }
        state
    }
}

unsafe fn payload_ref<T>(payload: *const u8, payload_len: usize) -> Option<&'static T> {
    (!payload.is_null() && payload_len >= std::mem::size_of::<T>()).then_some(&*payload.cast::<T>())
}

fn read_utf8(slice: flux_plugin_abi::FluxUtf8Slice) -> String {
    if slice.len == 0 || slice.ptr.is_null() {
        return String::new();
    }
    let bytes = unsafe { std::slice::from_raw_parts(slice.ptr, slice.len) };
    std::str::from_utf8(bytes).unwrap_or_default().to_string()
}

fn decode_modifiers(flags: u32) -> InputModifiers {
    InputModifiers {
        shift: flags & 0b001 != 0,
        ctrl: flags & 0b010 != 0,
        alt: flags & 0b100 != 0,
    }
}
