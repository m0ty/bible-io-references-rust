//! Unicode normalization tailored to Bible-reference syntax.
//!
//! This module intentionally does not perform general-purpose Unicode
//! normalization or case folding. It only normalizes characters commonly used
//! as reference syntax and removes marks that otherwise make book aliases hard
//! to match.

use std::{fmt, ops::Range};

/// Normalizes compatibility characters used in Bible references.
///
/// Letter case and whitespace runs are preserved. Use
/// [`normalize_for_parsing`] when surrounding and repeated whitespace should be
/// removed as well.
#[must_use]
pub fn normalize(input: &str) -> String {
    let mut output = String::with_capacity(input.len());

    for character in input.chars() {
        if !is_removed(character) {
            output.push(normalize_character(character));
        }
    }

    output
}

/// Normalizes an input while retaining byte spans into the original string.
///
/// The mapping contains one [`SourceSpan`] per byte in the normalized UTF-8
/// string. Every byte of a multi-byte scalar maps to that scalar's complete
/// source span. Removed characters are attached to the preceding output scalar,
/// or to the first output scalar when they appear at the beginning.
#[must_use]
pub fn normalize_detailed(input: &str) -> NormalizedInput {
    let mut normalized = String::with_capacity(input.len());
    let mut source_spans: Vec<SourceSpan> = Vec::with_capacity(input.len());
    let mut pending_removed_start = None;
    let mut last_emission_start = 0;

    for (source_start, character) in input.char_indices() {
        let source_end = source_start + character.len_utf8();

        if is_removed(character) {
            if source_spans.is_empty() {
                pending_removed_start.get_or_insert(source_start);
            } else {
                for span in &mut source_spans[last_emission_start..] {
                    span.end = source_end;
                }
            }
            continue;
        }

        let normalized_character = normalize_character(character);
        let mapped_start = pending_removed_start.take().unwrap_or(source_start);
        last_emission_start = source_spans.len();
        normalized.push(normalized_character);

        let span = SourceSpan::new(mapped_start, source_end);
        for _ in 0..normalized_character.len_utf8() {
            source_spans.push(span);
        }
    }

    NormalizedInput {
        original: input.to_owned(),
        normalized,
        source_spans,
    }
}

/// Trims an input and replaces each non-empty whitespace run with one ASCII
/// space.
///
/// This helper accepts any Unicode whitespace, not just the spacing characters
/// handled by [`normalize`]. It does not otherwise alter the input.
#[must_use]
pub fn collapse_whitespace(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut pending_space = false;

    for character in input.chars() {
        if character.is_whitespace() {
            pending_space = !output.is_empty();
        } else {
            if pending_space {
                output.push(' ');
                pending_space = false;
            }
            output.push(character);
        }
    }

    output
}

/// Applies reference normalization, then trims and collapses whitespace.
#[must_use]
pub fn normalize_for_parsing(input: &str) -> String {
    collapse_whitespace(&normalize(input))
}

/// A normalized input together with a byte mapping back to the original text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedInput {
    original: String,
    normalized: String,
    source_spans: Vec<SourceSpan>,
}

/// A shorter alias for [`NormalizedInput`].
pub type Normalization = NormalizedInput;

impl NormalizedInput {
    /// Returns the input supplied to [`normalize_detailed`].
    #[must_use]
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Returns the syntax-normalized text.
    #[must_use]
    pub fn normalized(&self) -> &str {
        &self.normalized
    }

    /// Returns whether normalization changed the input text.
    #[must_use]
    pub fn changed(&self) -> bool {
        self.normalized != self.original
    }

    /// Returns the normalized UTF-8 length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.normalized.len()
    }

    /// Returns whether the normalized text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.normalized.is_empty()
    }

    /// Returns one original source span per normalized UTF-8 byte.
    #[must_use]
    pub fn source_spans(&self) -> &[SourceSpan] {
        &self.source_spans
    }

    /// Maps a normalized byte range to the corresponding original byte span.
    ///
    /// Returns `None` if the range is reversed or outside the normalized text.
    /// Non-empty ranges that select part of a multi-byte scalar expand to that
    /// scalar's complete original span.
    #[must_use]
    pub fn map_span(&self, range: Range<usize>) -> Option<SourceSpan> {
        let start = range.start;
        let end = range.end;
        if start > end || end > self.normalized.len() {
            return None;
        }

        if start < end {
            return Some(SourceSpan::new(
                self.source_spans[start].start,
                self.source_spans[end - 1].end,
            ));
        }

        if self.source_spans.is_empty() {
            // Preserve the source extent when the entire input was removed.
            return Some(SourceSpan::new(0, self.original.len()));
        }
        if start == 0 {
            let offset = self.source_spans[0].start;
            return Some(SourceSpan::new(offset, offset));
        }
        if start == self.source_spans.len() {
            let offset = self.source_spans[self.source_spans.len() - 1].end;
            return Some(SourceSpan::new(offset, offset));
        }

        let offset = self.source_spans[start].start;
        Some(SourceSpan::new(offset, offset))
    }

    /// Maps normalized byte offsets `[start, end)` to an original byte span.
    #[must_use]
    pub fn map_normalized_span(&self, start: usize, end: usize) -> Option<SourceSpan> {
        self.map_span(start..end)
    }

    /// Returns the original text represented by a normalized byte range.
    #[must_use]
    pub fn original_text_for(&self, range: Range<usize>) -> Option<&str> {
        let span = self.map_span(range)?;
        self.original.get(span.as_range())
    }

    /// Consumes the mapping and returns the normalized string.
    #[must_use]
    pub fn into_normalized(self) -> String {
        self.normalized
    }
}

impl AsRef<str> for NormalizedInput {
    fn as_ref(&self) -> &str {
        self.normalized()
    }
}

impl fmt::Display for NormalizedInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.normalized())
    }
}

/// A half-open byte span into an original UTF-8 string.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceSpan {
    /// Inclusive byte offset.
    start: usize,
    /// Exclusive byte offset.
    end: usize,
}

impl SourceSpan {
    /// Creates a source span.
    ///
    /// # Panics
    ///
    /// Panics when `end` is less than `start`.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        assert!(end >= start, "span end must not precede its start");
        Self { start, end }
    }

    /// Returns the inclusive byte offset.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Returns the exclusive byte offset.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// Returns the span length in bytes.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// Returns whether this span is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns this span as a standard byte range.
    #[must_use]
    pub const fn as_range(self) -> Range<usize> {
        self.start..self.end
    }

    /// Extracts this span from `text` when it lies on UTF-8 boundaries.
    #[must_use]
    pub fn text_from(self, text: &str) -> Option<&str> {
        text.get(self.as_range())
    }
}

impl From<SourceSpan> for Range<usize> {
    fn from(span: SourceSpan) -> Self {
        span.as_range()
    }
}

impl fmt::Display for SourceSpan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}..{}", self.start, self.end)
    }
}

fn normalize_character(character: char) -> char {
    let value = character as u32;

    // Fullwidth ASCII forms.
    if (0xff01..=0xff5e).contains(&value) {
        return char::from_u32(value - 0xfee0).expect("fullwidth ASCII maps to a scalar");
    }

    // Arabic-Indic and Eastern Arabic/Persian digits.
    if (0x0660..=0x0669).contains(&value) {
        return char::from_u32(0x30 + value - 0x0660).expect("digit maps to ASCII");
    }
    if (0x06f0..=0x06f9).contains(&value) {
        return char::from_u32(0x30 + value - 0x06f0).expect("digit maps to ASCII");
    }

    if is_comma(value) {
        ','
    } else if is_semicolon(value) {
        ';'
    } else if is_colon(value) {
        ':'
    } else if is_dot(value) {
        '.'
    } else if is_dash(value) {
        '-'
    } else if is_space(value) {
        ' '
    } else {
        character
    }
}

fn is_comma(value: u32) -> bool {
    matches!(
        value,
        0x060c | 0x3001 | 0xfe10 | 0xfe11 | 0xfe50 | 0xfe51 | 0xff64
    )
}

fn is_semicolon(value: u32) -> bool {
    matches!(value, 0x061b | 0xfe14 | 0xfe54)
}

fn is_colon(value: u32) -> bool {
    matches!(value, 0x2236 | 0xfe13 | 0xfe55)
}

fn is_dot(value: u32) -> bool {
    matches!(value, 0x066b | 0x06d4 | 0x2024 | 0x3002 | 0xfe52 | 0xff61)
}

fn is_dash(value: u32) -> bool {
    value == 0x05be
        || (0x2010..=0x2015).contains(&value)
        || matches!(value, 0x2212 | 0x2e3a | 0x2e3b | 0xfe58 | 0xfe63)
}

fn is_space(value: u32) -> bool {
    matches!(value, 0x00a0 | 0x1680 | 0x202f | 0x205f | 0x3000)
        || (0x2000..=0x200a).contains(&value)
}

fn is_removed(character: char) -> bool {
    let value = character as u32;
    is_bidi_control(value)
        || is_invisible_format(value)
        || is_variation_selector(value)
        || is_combining_diacritic(value)
        || is_hebrew_mark(value)
        || is_arabic_mark(value)
}

fn is_bidi_control(value: u32) -> bool {
    matches!(value, 0x061c | 0x200e | 0x200f)
        || (0x202a..=0x202e).contains(&value)
        || (0x2066..=0x206f).contains(&value)
}

fn is_invisible_format(value: u32) -> bool {
    matches!(value, 0x00ad | 0x180e | 0x2060 | 0xfeff) || (0x200b..=0x200d).contains(&value)
}

fn is_variation_selector(value: u32) -> bool {
    (0xfe00..=0xfe0f).contains(&value) || (0xe0100..=0xe01ef).contains(&value)
}

fn is_combining_diacritic(value: u32) -> bool {
    (0x0300..=0x036f).contains(&value)
        || (0x1dc0..=0x1dff).contains(&value)
        || (0x20d0..=0x20ff).contains(&value)
        || (0xfe20..=0xfe2f).contains(&value)
}

fn is_hebrew_mark(value: u32) -> bool {
    (0x0591..=0x05bd).contains(&value)
        || value == 0x05bf
        || (0x05c1..=0x05c2).contains(&value)
        || (0x05c4..=0x05c5).contains(&value)
        || value == 0x05c7
}

fn is_arabic_mark(value: u32) -> bool {
    (0x0610..=0x061a).contains(&value)
        || (0x064b..=0x065f).contains(&value)
        || value == 0x0670
        || (0x06d6..=0x06dc).contains(&value)
        || (0x06df..=0x06e4).contains(&value)
        || (0x06e7..=0x06e8).contains(&value)
        || (0x06ea..=0x06ed).contains(&value)
        || (0x08d3..=0x08e1).contains(&value)
        || (0x08e3..=0x08ff).contains(&value)
}

#[cfg(test)]
#[path = "../tests/unit/normalize.rs"]
mod tests;
