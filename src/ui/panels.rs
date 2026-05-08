use std::collections::HashMap;

use crate::ui::{
    palette,
    scroll_area::{
        spawn_scroll_area_scrollbar_with_style, ScrollAreaScrollbarStyle, ScrollAreaViewport,
    },
};
use bevy::{
    prelude::*,
    ui::{ComputedNode, RelativeCursorPosition, ScrollPosition},
    window::PrimaryWindow,
};

const PANEL_HEADER_HEIGHT: f32 = 34.0;
const PANEL_CONTENT_PADDING_Y: f32 = 10.0;
const PANEL_HEADER_BUTTON_SIZE: f32 = 24.0;
const PANEL_HEADER_BUTTON_BG: Color = palette::PANEL_HEADER_BUTTON_BG;
const PANEL_HEADER_TEXT: Color = palette::TEXT_PRIMARY;

/// Default gap between panels stacked in the same corner.
pub const DEFAULT_PANEL_STACK_GAP: f32 = 12.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
/// Stores `PanelId` state.
pub struct PanelId(&'static str);

impl PanelId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
/// Stores `PanelHeaderActionId` state.
pub struct PanelHeaderActionId(&'static str);

impl PanelHeaderActionId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanelCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Debug)]
/// Stores `PanelControls` state.
pub struct PanelControls {
    pub show_collapse: bool,
    pub show_close: bool,
    pub custom_actions: Vec<PanelHeaderActionId>,
}

impl Default for PanelControls {
    fn default() -> Self {
        Self {
            show_collapse: false,
            show_close: false,
            custom_actions: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PanelScrollPolicy {
    AutoHalfScreen,
    Never,
    FromTotalHeightPx(f32),
}

#[derive(Clone, Debug)]
/// Stores `PanelSpec` state.
pub struct PanelSpec {
    pub id: PanelId,
    pub title: String,
    pub corner: PanelCorner,
    pub width: f32,
    pub margin_x: f32,
    pub margin_y: f32,
    pub stack_gap: f32,
    pub controls: PanelControls,
    pub scroll_policy: PanelScrollPolicy,
    pub background: Color,
    pub header_background: Color,
    pub initial_visible: bool,
    pub initial_collapsed: bool,
}

#[derive(Clone, Debug)]
/// Stores `PanelState` state.
pub struct PanelState {
    pub visible: bool,
    pub collapsed: bool,
    pub opened_at: u64,
    pub scroll_enabled: bool,
}

#[derive(Resource, Default, Clone, Copy, Debug)]
/// Stores `PanelOpenOrder` state.
pub struct PanelOpenOrder {
    next: u64,
}

impl PanelOpenOrder {
    /// Runs `allocate` logic.
    pub fn allocate(&mut self) -> u64 {
        self.next = self.next.saturating_add(1);
        self.next
    }
}

include!("panels_manager_block.rs");
include!("panels_runtime_block.rs");
include!("panels_tests_block.rs");
