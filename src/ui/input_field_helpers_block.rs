fn cursor_index_from_click_position_exact(
    text: &str,
    click_x: f32,
    click_y: f32,
    text_layout: Option<&bevy::text::ComputedTextBlock>,
) -> usize {
    let Some(text_layout) = text_layout else {
        return if click_x <= 0.0 { 0 } else { text.chars().count() };
    };

    let cursor = text_layout
        .buffer()
        .hit(click_x.max(0.0), click_y.max(0.0));
    let Some(cursor) = cursor else {
        return if click_x <= 0.0 { 0 } else { text.chars().count() };
    };

    char_index_at_byte(text, cursor.index)
}

fn caret_x_from_cursor_exact(
    text: &str,
    cursor_char_index: usize,
    text_layout: Option<&bevy::text::ComputedTextBlock>,
) -> f32 {
    let Some(text_layout) = text_layout else {
        return 0.0;
    };

    let target_byte = byte_index_at_char(text, cursor_char_index);
    let mut fallback_line_width = 0.0f32;

    for run in text_layout.buffer().layout_runs() {
        fallback_line_width = fallback_line_width.max(run.line_w);

        if run.glyphs.is_empty() {
            if target_byte == 0 {
                return 0.0;
            }
            if target_byte >= run.text.len() {
                return run.line_w.max(0.0);
            }
            continue;
        }

        for glyph in run.glyphs.iter() {
            let left = glyph.x.min(glyph.x + glyph.w);
            let right = glyph.x.max(glyph.x + glyph.w);
            if target_byte == glyph.start {
                return left.max(0.0);
            }
            if target_byte == glyph.end {
                return right.max(0.0);
            }
            if target_byte > glyph.start && target_byte < glyph.end {
                let span = (glyph.end - glyph.start).max(1);
                let local = target_byte - glyph.start;
                let t = local as f32 / span as f32;
                return (left + (right - left) * t).max(0.0);
            }
        }

        if let Some(last_glyph) = run.glyphs.last() {
            if target_byte >= last_glyph.end {
                let last_right = last_glyph.x.max(last_glyph.x + last_glyph.w);
                return run.line_w.max(last_right).max(0.0);
            }
        }
    }

    fallback_line_width.max(0.0)
}

fn char_index_at_byte(text: &str, byte_index: usize) -> usize {
    if text.is_empty() {
        return 0;
    }

    let clamped = byte_index.min(text.len());
    let mut boundary = clamped;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text[..boundary].chars().count()
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
