use std::ops::Range;

#[derive(Debug)]
pub(crate) struct TextEditState {
    pub(crate) text: String,
    pub(crate) selection: Range<usize>,
    pub(crate) reversed: bool,
    pub(crate) marked_range: Option<Range<usize>>,
}

#[derive(Debug)]
pub(crate) enum InputEdit {
    Commit {
        range_utf16: Option<Range<usize>>,
        text: String,
    },
    Mark {
        replacement_range_utf16: Option<Range<usize>>,
        text: String,
        selected_range_utf16: Option<Range<usize>>,
    },
    Unmark,
}

impl TextEditState {
    pub(crate) fn new(
        text: String,
        selection: Range<usize>,
        reversed: bool,
        marked_range: Option<Range<usize>>,
    ) -> Self {
        let selection = normalize_utf8_range(&text, selection);
        let marked_range = marked_range.map(|range| normalize_utf8_range(&text, range));
        Self {
            text,
            selection,
            reversed,
            marked_range,
        }
    }

    pub(crate) fn apply(&mut self, edit: InputEdit) {
        match edit {
            InputEdit::Commit { range_utf16, text } => {
                let replacement_range = self.replacement_range(range_utf16);
                let replacement = strip_line_breaks(&text);
                let cursor = replacement_range.start + replacement.len();
                self.replace(replacement_range, &replacement);
                self.selection = cursor..cursor;
                self.reversed = false;
                self.marked_range = None;
            }
            InputEdit::Mark {
                replacement_range_utf16,
                text,
                selected_range_utf16,
            } => {
                let replacement_range = self.replacement_range(replacement_range_utf16);
                let (replacement, selected_range) =
                    prepare_marked_text(&text, selected_range_utf16);
                let start = replacement_range.start;
                let end = start + replacement.len();
                self.replace(replacement_range, &replacement);
                self.marked_range = (start < end).then_some(start..end);
                let selected_range =
                    selected_range.unwrap_or_else(|| replacement.len()..replacement.len());
                // GPUI's selected range is relative to the newly marked text. Both
                // endpoints must be offset from the replacement start (not its end).
                self.selection = start + selected_range.start..start + selected_range.end;
                self.reversed = false;
            }
            InputEdit::Unmark => {
                self.marked_range = None;
            }
        }
    }

    fn replacement_range(&self, range_utf16: Option<Range<usize>>) -> Range<usize> {
        let range = range_utf16
            .map(|range| utf16_range_to_utf8(&self.text, range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selection.clone());
        normalize_utf8_range(&self.text, range)
    }

    fn replace(&mut self, range: Range<usize>, replacement: &str) {
        let mut updated =
            String::with_capacity(self.text.len() - (range.end - range.start) + replacement.len());
        updated.push_str(&self.text[..range.start]);
        updated.push_str(replacement);
        updated.push_str(&self.text[range.end..]);
        self.text = updated;
    }
}

/// Convert a UTF-16 range into UTF-8 byte offsets without returning a partial
/// code point. A collapsed offset inside a surrogate pair rounds down; a
/// non-empty range expands outward to include any split code point.
pub(crate) fn utf16_range_to_utf8(text: &str, range: Range<usize>) -> Range<usize> {
    let start_units = range.start.min(range.end);
    let end_units = range.start.max(range.end);
    if start_units == end_units {
        let cursor = utf16_offset_to_utf8(text, start_units, false);
        return cursor..cursor;
    }
    utf16_offset_to_utf8(text, start_units, false)..utf16_offset_to_utf8(text, end_units, true)
}

/// Convert a UTF-8 byte range into UTF-16 code-unit offsets, clamping partial
/// UTF-8 code points outward to valid string boundaries.
pub(crate) fn utf8_range_to_utf16(text: &str, range: Range<usize>) -> Range<usize> {
    let start = range.start.min(range.end);
    let end = range.start.max(range.end);
    if start == end {
        let byte = clamp_utf8_offset(text, start, false);
        let units = text[..byte].encode_utf16().count();
        return units..units;
    }
    let start = clamp_utf8_offset(text, start, false);
    let end = clamp_utf8_offset(text, end, true);
    text[..start].encode_utf16().count()..text[..end].encode_utf16().count()
}

pub(crate) fn clamp_utf8_offset(text: &str, offset: usize, round_up: bool) -> usize {
    let offset = offset.min(text.len());
    if text.is_char_boundary(offset) {
        return offset;
    }
    if round_up {
        (offset + 1..=text.len())
            .find(|&boundary| text.is_char_boundary(boundary))
            .unwrap_or(text.len())
    } else {
        (0..offset)
            .rev()
            .find(|&boundary| text.is_char_boundary(boundary))
            .unwrap_or(0)
    }
}

fn utf16_offset_to_utf8(text: &str, offset: usize, round_up: bool) -> usize {
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if offset == units {
            return byte;
        }
        let next_units = units + ch.len_utf16();
        if offset < next_units {
            return if round_up { byte + ch.len_utf8() } else { byte };
        }
        units = next_units;
    }
    text.len()
}

fn normalize_utf8_range(text: &str, range: Range<usize>) -> Range<usize> {
    let start = range.start.min(range.end);
    let end = range.start.max(range.end);
    if start == end {
        let cursor = clamp_utf8_offset(text, start, false);
        return cursor..cursor;
    }
    clamp_utf8_offset(text, start, false)..clamp_utf8_offset(text, end, true)
}

fn strip_line_breaks(text: &str) -> String {
    text.chars()
        .filter(|ch| !matches!(ch, '\r' | '\n'))
        .collect()
}

fn prepare_marked_text(
    text: &str,
    selected_range_utf16: Option<Range<usize>>,
) -> (String, Option<Range<usize>>) {
    let selected_bytes = selected_range_utf16.map(|range| utf16_range_to_utf8(text, range));
    let normalized = strip_line_breaks(text);
    let selected = selected_bytes.map(|range| {
        let normalized_offset = |offset| {
            let offset = clamp_utf8_offset(text, offset, false);
            text[..offset]
                .chars()
                .filter(|ch| !matches!(ch, '\r' | '\n'))
                .map(char::len_utf8)
                .sum::<usize>()
        };
        normalized_offset(range.start)..normalized_offset(range.end)
    });
    (normalized, selected)
}

#[cfg(test)]
mod tests {
    use super::{InputEdit, TextEditState, utf8_range_to_utf16, utf16_range_to_utf8};

    #[test]
    fn utf16_ranges_never_split_utf8_or_surrogate_boundaries() {
        let text = "a😀é";
        assert_eq!(utf16_range_to_utf8(text, 1..3), 1..5);
        assert_eq!(utf16_range_to_utf8(text, 2..2), 1..1);
        assert_eq!(utf16_range_to_utf8(text, 2..4), 1..7);
        assert_eq!(utf8_range_to_utf16(text, 1..5), 1..3);
        assert_eq!(utf8_range_to_utf16(text, 2..2), 1..1);
    }

    #[test]
    fn composition_updates_replace_marked_text_and_keep_relative_selection_offsets() {
        let mut state = TextEditState::new("x😀z".into(), 1..5, false, None);

        state.apply(InputEdit::Mark {
            replacement_range_utf16: None,
            text: "A😀B".into(),
            selected_range_utf16: Some(3..3),
        });
        assert_eq!(state.text, "xA😀Bz");
        assert_eq!(state.marked_range, Some(1..7));
        assert_eq!(state.selection, 6..6);

        state.apply(InputEdit::Mark {
            replacement_range_utf16: None,
            text: "日本😀".into(),
            selected_range_utf16: Some(2..2),
        });
        assert_eq!(state.text, "x日本😀z");
        assert_eq!(state.marked_range, Some(1..11));
        assert_eq!(state.selection, 7..7);

        state.apply(InputEdit::Commit {
            range_utf16: None,
            text: "日本😀".into(),
        });
        assert_eq!(state.text, "x日本😀z");
        assert_eq!(state.selection, 11..11);
        assert_eq!(state.marked_range, None);
    }

    #[test]
    fn unmark_clears_composition_without_rewriting_text() {
        let mut state = TextEditState::new("aかなb".into(), 1..7, false, Some(1..7));
        state.apply(InputEdit::Unmark);
        assert_eq!(state.text, "aかなb");
        assert_eq!(state.marked_range, None);
    }
}
