use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
    text::TextLayoutInfo,
    ui::{ComputedNode, RelativeCursorPosition},
};

const CARET_WIDTH_PX: f32 = 1.0;
const CARET_HEIGHT_FACTOR: f32 = 0.70;
const CARET_MIN_HEIGHT_PX: f32 = 12.0;
const CARET_BLINK_SECONDS: f32 = 0.55;
const CARET_SIDE_PADDING_PX: f32 = 1.0;

#[derive(Clone, Debug)]
pub enum InputAllowedChars {
    Digits,
    AsciiAlphanumeric,
    AsciiVisible,
    Any,
    Custom(String),
}

impl InputAllowedChars {
    fn allows(&self, ch: char) -> bool {
        match self {
            Self::Digits => ch.is_ascii_digit(),
            Self::AsciiAlphanumeric => ch.is_ascii_alphanumeric(),
            Self::AsciiVisible => ch.is_ascii_graphic() || ch == ' ',
            Self::Any => !ch.is_control(),
            Self::Custom(chars) => chars.contains(ch),
        }
    }
}

#[derive(Clone, Debug)]
pub enum InputParser {
    String,
    U32Range { min: u32, max: u32 },
    F32Range {
        min: f32,
        max: f32,
        max_fraction_digits: usize,
    },
}

#[derive(Clone, Debug)]
pub enum ParsedInputValue {
    String(String),
    U32(u32),
    F32(f32),
}

#[derive(Component, Clone, Debug)]
pub struct TextInputField {
    pub text: String,
    pub cursor: usize,
    pub focused: bool,
    pub max_len: usize,
    pub allowed_chars: InputAllowedChars,
    pub parser: InputParser,
    pub value: ParsedInputValue,
}

impl TextInputField {
    pub fn new_string(initial: impl Into<String>, max_len: usize, allowed_chars: InputAllowedChars) -> Self {
        let mut text = initial.into();
        if text.chars().count() > max_len {
            text = text.chars().take(max_len).collect();
        }
        let cursor = text.chars().count();

        Self {
            value: ParsedInputValue::String(text.clone()),
            text,
            cursor,
            focused: false,
            max_len,
            allowed_chars,
            parser: InputParser::String,
        }
    }

    pub fn new_u32(initial: u32, min: u32, max: u32, max_len: usize) -> Self {
        let value = initial.clamp(min, max);
        let text = value.to_string();

        Self {
            value: ParsedInputValue::U32(value),
            cursor: text.chars().count(),
            focused: false,
            text,
            max_len,
            allowed_chars: InputAllowedChars::Digits,
            parser: InputParser::U32Range { min, max },
        }
    }

    pub fn new_f32(
        initial: f32,
        min: f32,
        max: f32,
        max_len: usize,
        max_fraction_digits: usize,
    ) -> Self {
        let value = initial.clamp(min, max);
        let mut text = format!("{value:.3}");
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        if text.is_empty() {
            text = "0".to_string();
        }

        Self {
            value: ParsedInputValue::F32(value),
            cursor: text.chars().count(),
            focused: false,
            text,
            max_len,
            allowed_chars: InputAllowedChars::Custom("0123456789.,".to_string()),
            parser: InputParser::F32Range {
                min,
                max,
                max_fraction_digits,
            },
        }
    }

    pub fn parsed_u32(&self) -> Option<u32> {
        match self.value {
            ParsedInputValue::U32(v) => Some(v),
            _ => None,
        }
    }

    pub fn parsed_f32(&self) -> Option<f32> {
        match self.value {
            ParsedInputValue::F32(v) => Some(v),
            _ => None,
        }
    }

    fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    fn clamp_cursor(&mut self) {
        self.cursor = self.cursor.min(self.char_count());
    }

    fn byte_index_at(&self, char_index: usize) -> usize {
        if char_index == 0 {
            return 0;
        }

        let mut count = 0usize;
        for (idx, _) in self.text.char_indices() {
            if count == char_index {
                return idx;
            }
            count += 1;
        }
        self.text.len()
    }

    fn insert_char(&mut self, ch: char) {
        if !self.allowed_chars.allows(ch) {
            return;
        }
        if !self.can_insert_char_for_parser(ch) {
            return;
        }
        if self.char_count() >= self.max_len {
            return;
        }

        let byte_index = self.byte_index_at(self.cursor);
        self.text.insert(byte_index, ch);
        self.cursor += 1;
        self.update_parsed_value();
    }

    fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.char_count());
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let start = self.byte_index_at(self.cursor - 1);
        let end = self.byte_index_at(self.cursor);
        self.text.replace_range(start..end, "");
        self.cursor -= 1;
        self.update_parsed_value();
    }

    fn delete(&mut self) {
        if self.cursor >= self.char_count() {
            return;
        }

        let start = self.byte_index_at(self.cursor);
        let end = self.byte_index_at(self.cursor + 1);
        self.text.replace_range(start..end, "");
        self.update_parsed_value();
    }

    fn update_parsed_value(&mut self) {
        match self.parser {
            InputParser::String => {
                self.value = ParsedInputValue::String(self.text.clone());
            }
            InputParser::U32Range { min, max } => {
                if let Ok(parsed) = self.text.parse::<u32>() {
                    self.value = ParsedInputValue::U32(parsed.clamp(min, max));
                }
            }
            InputParser::F32Range { min, max, .. } => {
                if let Some(parsed) = parse_decimal_input(&self.text) {
                    self.value = ParsedInputValue::F32(parsed.clamp(min, max));
                }
            }
        }
    }

    fn can_insert_char_for_parser(&self, ch: char) -> bool {
        match self.parser {
            InputParser::F32Range {
                max_fraction_digits,
                ..
            } => {
                let mut next = self.text.clone();
                let byte_index = self.byte_index_at(self.cursor);
                next.insert(byte_index, ch);

                let separator_count = next
                    .chars()
                    .filter(|candidate| *candidate == '.' || *candidate == ',')
                    .count();
                if separator_count > 1 {
                    return false;
                }

                if let Some(separator_byte_index) = next.find(['.', ',']) {
                    let separator_char_index =
                        next[..separator_byte_index].chars().count();
                    let fraction_len =
                        next.chars().count().saturating_sub(separator_char_index + 1);
                    if fraction_len > max_fraction_digits {
                        return false;
                    }
                }

                true
            }
            _ => true,
        }
    }
}

fn parse_decimal_input(text: &str) -> Option<f32> {
    if text.is_empty() {
        return None;
    }

    let mut normalized = text.replace(',', ".");
    if normalized == "." {
        return None;
    }
    if normalized.starts_with('.') {
        normalized = format!("0{normalized}");
    }

    normalized.parse::<f32>().ok()
}

#[derive(Component)]
pub struct TextInputDisplay;

#[derive(Component)]
pub struct TextInputCaret;

#[derive(Component, Clone, Copy)]
pub struct TextInputStyle {
    pub idle_bg: Color,
    pub focused_bg: Color,
}

#[derive(Resource)]
struct TextInputCaretBlink {
    timer: Timer,
    visible: bool,
}

impl Default for TextInputCaretBlink {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(CARET_BLINK_SECONDS, TimerMode::Repeating),
            visible: true,
        }
    }
}

pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextInputCaretBlink>();
        app.add_systems(
            Update,
            (
                ensure_text_input_caret,
                focus_text_input_on_click,
                handle_text_input_keyboard,
                sync_text_input_display,
                sync_text_input_style,
                sync_text_input_caret,
            )
                .chain(),
        );
    }
}

fn ensure_text_input_caret(
    mut commands: Commands,
    inputs: Query<Entity, (Added<TextInputField>, With<Button>)>,
) {
    for input_entity in &inputs {
        commands.entity(input_entity).with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(CARET_WIDTH_PX),
                    height: Val::Px(CARET_MIN_HEIGHT_PX),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                Visibility::Hidden,
                TextInputCaret,
            ));
        });
    }
}

fn focus_text_input_on_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut query_set: ParamSet<(
        Query<(Entity, &Interaction, &RelativeCursorPosition), (Changed<Interaction>, With<TextInputField>)>,
        Query<(Entity, &mut TextInputField, &ComputedNode)>,
    )>,
    display_layout_query: Query<(&TextLayoutInfo, &ChildOf), With<TextInputDisplay>>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let mut display_layout_by_input = std::collections::HashMap::<Entity, &TextLayoutInfo>::new();
    for (layout_info, child_of) in &display_layout_query {
        display_layout_by_input.insert(child_of.parent(), layout_info);
    }

    let mut clicked: Option<(Entity, Option<Vec2>)> = None;

    for (entity, interaction, relative_cursor) in &mut query_set.p0() {
        if *interaction == Interaction::Pressed {
            clicked = Some((entity, relative_cursor.normalized));
            break;
        }
    }

    for (entity, mut field, input_node) in &mut query_set.p1() {
        if let Some((clicked_entity, normalized)) = clicked {
            if entity == clicked_entity {
                field.focused = true;
                let input_width = input_node.size().x.max(1.0);
                let content_left = input_node.content_inset().left;
                let relative_x = normalized.map(|p| p.x.clamp(0.0, 1.0) * input_width);

                if let Some(layout_info) = display_layout_by_input.get(&entity) {
                    let click_x = relative_x.unwrap_or(input_width) - content_left;
                    field.cursor = cursor_index_from_click_x(&field.text, field.char_count(), layout_info, click_x);
                } else {
                    let len = field.char_count();
                    let cursor_pos = normalized.map(|p| p.x.clamp(0.0, 1.0)).unwrap_or(1.0);
                    let next_cursor = (cursor_pos * len as f32).round() as usize;
                    field.cursor = next_cursor.min(len);
                }

                field.clamp_cursor();
            } else {
                field.focused = false;
            }
        } else {
            field.focused = false;
        }
    }
}

fn handle_text_input_keyboard(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut inputs: Query<&mut TextInputField>,
) {
    let mut target: Option<Mut<TextInputField>> = None;
    for field in &mut inputs {
        if field.focused {
            target = Some(field);
            break;
        }
    }

    let Some(mut field) = target else {
        return;
    };

    field.clamp_cursor();

    for event in keyboard_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match &event.logical_key {
            Key::ArrowLeft => field.move_left(),
            Key::ArrowRight => field.move_right(),
            Key::Home => field.cursor = 0,
            Key::End => field.cursor = field.char_count(),
            Key::Backspace => field.backspace(),
            Key::Delete => field.delete(),
            Key::Enter | Key::Escape => {
                field.focused = false;
            }
            Key::Character(chars) => {
                for ch in chars.chars() {
                    field.insert_char(ch);
                }
            }
            _ => {}
        }
    }
}

fn sync_text_input_display(
    parent_query: Query<&ChildOf>,
    inputs: Query<&TextInputField>,
    mut text_query: Query<(Entity, &mut Text), With<TextInputDisplay>>,
) {
    for (text_entity, mut text) in &mut text_query {
        let Ok(parent) = parent_query.get(text_entity) else {
            continue;
        };

        let Ok(field) = inputs.get(parent.parent()) else {
            continue;
        };

        text.0 = field.text.clone();
    }
}

fn sync_text_input_style(
    inputs: Query<(&TextInputField, &TextInputStyle)>,
    mut bg_query: Query<(&mut BackgroundColor, Entity), With<Button>>,
) {
    for (mut bg, entity) in &mut bg_query {
        let Ok((field, style)) = inputs.get(entity) else {
            continue;
        };

        bg.0 = if field.focused {
            style.focused_bg
        } else {
            style.idle_bg
        };
    }
}

fn sync_text_input_caret(
    time: Res<Time>,
    mut blink: ResMut<TextInputCaretBlink>,
    input_query: Query<(&TextInputField, &ComputedNode)>,
    display_query: Query<(&TextLayoutInfo, &ChildOf), With<TextInputDisplay>>,
    mut caret_query: Query<(&mut Node, &mut Visibility, &ChildOf, &mut BackgroundColor), With<TextInputCaret>>,
) {
    blink.timer.tick(time.delta());
    if blink.timer.just_finished() {
        blink.visible = !blink.visible;
    }

    let mut display_layout_by_input = std::collections::HashMap::<Entity, &TextLayoutInfo>::new();
    for (layout_info, child_of) in &display_query {
        display_layout_by_input.insert(child_of.parent(), layout_info);
    }

    for (mut caret_node, mut caret_visibility, child_of, mut caret_color) in &mut caret_query {
        let input_entity = child_of.parent();
        let Ok((field, input_node)) = input_query.get(input_entity) else {
            *caret_visibility = Visibility::Hidden;
            continue;
        };

        if !field.focused || !blink.visible {
            *caret_visibility = Visibility::Hidden;
            continue;
        }

        let content_inset = input_node.content_inset();
        let text_origin_x = content_inset.left;
        let text_origin_y = content_inset.top;
        let content_height = (input_node.size().y - content_inset.top - content_inset.bottom).max(0.0);
        let caret_height = (content_height * CARET_HEIGHT_FACTOR).max(CARET_MIN_HEIGHT_PX);
        let caret_top = text_origin_y + ((content_height - caret_height) * 0.5).max(0.0);

        let caret_x = if let Some(layout) = display_layout_by_input.get(&input_entity) {
            text_origin_x + caret_x_from_cursor(&field.text, field.cursor, layout)
        } else {
            text_origin_x
        };

        caret_node.left = Val::Px((caret_x - (CARET_WIDTH_PX * 0.5)).max(text_origin_x - CARET_SIDE_PADDING_PX));
        caret_node.top = Val::Px(caret_top);
        caret_node.width = Val::Px(CARET_WIDTH_PX);
        caret_node.height = Val::Px(caret_height);
        caret_color.0 = Color::WHITE;
        *caret_visibility = Visibility::Visible;
    }
}

fn cursor_index_from_click_x(
    text: &str,
    char_count: usize,
    layout_info: &TextLayoutInfo,
    click_x: f32,
) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut boundaries = collect_cursor_boundaries(text, layout_info);
    if boundaries.is_empty() {
        return ((click_x / layout_info.size.x.max(1.0)) * char_count as f32).round() as usize;
    }

    boundaries.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let mut best = boundaries[0];
    let mut best_dist = (click_x - best.0).abs();
    for boundary in boundaries.into_iter().skip(1) {
        let dist = (click_x - boundary.0).abs();
        if dist < best_dist {
            best = boundary;
            best_dist = dist;
        }
    }

    best.1.min(char_count)
}

fn caret_x_from_cursor(text: &str, cursor_char_index: usize, layout_info: &TextLayoutInfo) -> f32 {
    if text.is_empty() {
        return 0.0;
    }

    let cursor_byte_index = byte_index_at_char(text, cursor_char_index);

    let mut glyphs = layout_info.glyphs.iter().collect::<Vec<_>>();
    if glyphs.is_empty() {
        return 0.0;
    }

    glyphs.sort_by(|a, b| a.byte_index.cmp(&b.byte_index));

    if let Some(next_glyph) = glyphs.iter().find(|g| g.byte_index >= cursor_byte_index) {
        return next_glyph.position.x - (next_glyph.size.x * 0.5);
    }

    let last = glyphs[glyphs.len() - 1];
    last.position.x + (last.size.x * 0.5)
}

fn collect_cursor_boundaries(text: &str, layout_info: &TextLayoutInfo) -> Vec<(f32, usize)> {
    let mut boundaries = Vec::new();
    boundaries.push((0.0, 0));

    let mut glyphs = layout_info.glyphs.iter().collect::<Vec<_>>();
    glyphs.sort_by(|a, b| a.byte_index.cmp(&b.byte_index));

    for glyph in glyphs {
        let left = glyph.position.x - (glyph.size.x * 0.5);
        let right = glyph.position.x + (glyph.size.x * 0.5);
        let char_index = char_index_at_byte(text, glyph.byte_index);
        let next_char_index = char_index_at_byte(text, glyph.byte_index + glyph.byte_length);

        boundaries.push((left, char_index));
        boundaries.push((right, next_char_index));
    }

    boundaries
}

fn byte_index_at_char(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    let mut count = 0usize;
    for (idx, _) in text.char_indices() {
        if count == char_index {
            return idx;
        }
        count += 1;
    }

    text.len()
}

fn char_index_at_byte(text: &str, byte_index: usize) -> usize {
    let clamped = byte_index.min(text.len());
    text[..clamped].chars().count()
}
