use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
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
    U32Range {
        min: u32,
        max: u32,
    },
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
/// Stores `TextInputField` state.
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
    /// Runs `new_string` logic.
    pub fn new_string(
        initial: impl Into<String>,
        max_len: usize,
        allowed_chars: InputAllowedChars,
    ) -> Self {
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

    /// Runs `new_u32` logic.
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

    /// Runs `new_f32` logic.
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

    /// Runs `parsed_u32` logic.
    pub fn parsed_u32(&self) -> Option<u32> {
        match self.value {
            ParsedInputValue::U32(v) => Some(v),
            _ => None,
        }
    }

    /// Runs `parsed_f32` logic.
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
                    let separator_char_index = next[..separator_byte_index].chars().count();
                    let fraction_len = next
                        .chars()
                        .count()
                        .saturating_sub(separator_char_index + 1);
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
/// Stores `TextInputDisplay` state.
pub struct TextInputDisplay;

#[derive(Component)]
/// Stores `TextInputCaret` state.
pub struct TextInputCaret;

#[derive(Component, Clone, Copy)]
/// Stores `TextInputStyle` state.
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

include!("input_field_systems_block.rs");
include!("input_field_helpers_block.rs");

#[cfg(test)]
mod tests {
    use super::{InputAllowedChars, TextInputField};
    use bevy::text::cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

    #[test]
    fn any_input_allows_unicode_and_space() {
        let mut field = TextInputField::new_string("A", 64, InputAllowedChars::Any);
        field.cursor = field.text.chars().count();
        field.insert_char(' ');
        field.insert_char('Я');
        field.insert_char('ß');
        assert_eq!(field.text, "A Яß");
    }

    #[test]
    fn any_input_blocks_control_chars() {
        let mut field = TextInputField::new_string("", 64, InputAllowedChars::Any);
        field.insert_char('\n');
        field.insert_char('\t');
        assert_eq!(field.text, "");
    }

    #[test]
    fn trailing_space_changes_text_width() {
        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(20.0, 24.0));
        let attrs = Attrs::new();

        buffer.set_text(&mut font_system, "SaveName", attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut font_system, false);
        let without_space_width = buffer.layout_runs().next().map(|run| run.line_w).unwrap_or(0.0);

        buffer.set_text(&mut font_system, "SaveName ", attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut font_system, false);
        let with_space_width = buffer.layout_runs().next().map(|run| run.line_w).unwrap_or(0.0);

        assert!(
            with_space_width > without_space_width,
            "expected trailing space to increase width: without={without_space_width}, with={with_space_width}"
        );
    }
}
