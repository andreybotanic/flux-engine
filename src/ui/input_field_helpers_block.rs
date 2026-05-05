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
