//! Parser-driven extraction and replacement of passages embedded in prose.

use core::{fmt, ops::Range};
use std::{collections::BTreeMap, fmt::Write as _};

use crate::{
    Book, Language, ParseMetadata,
    normalize::{NormalizedInput, SourceSpan, normalize_detailed},
    passage::{Passage, PassageParser},
};

/// Default normalized-byte search distance before a numeric anchor.
pub const DEFAULT_MAX_LOOKBEHIND: usize = 96;

/// Default normalized-byte search distance after a numeric anchor.
pub const DEFAULT_MAX_LOOKAHEAD: usize = 256;

/// Smallest accepted lookbehind or lookahead window.
pub const MIN_LOOKAROUND: usize = 1;

/// Largest accepted lookbehind or lookahead window.
pub const MAX_LOOKAROUND: usize = 4096;

/// A successfully parsed passage and its exact byte location in source text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PassageMatch {
    passage: Passage,
    start: usize,
    end: usize,
    source_text: String,
    metadata: ParseMetadata,
}

impl PassageMatch {
    /// Return the parsed passage.
    #[must_use]
    pub const fn passage(&self) -> &Passage {
        &self.passage
    }

    /// Return the inclusive UTF-8 byte offset in the original source.
    #[must_use]
    pub const fn start(&self) -> usize {
        self.start
    }

    /// Return the exclusive UTF-8 byte offset in the original source.
    #[must_use]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// Alias for [`Self::start`] emphasizing that the offset is measured in bytes.
    #[must_use]
    pub const fn start_offset(&self) -> usize {
        self.start
    }

    /// Alias for [`Self::end`] emphasizing that the offset is measured in bytes.
    #[must_use]
    pub const fn end_offset(&self) -> usize {
        self.end
    }

    /// Return the half-open UTF-8 byte range in the original source.
    #[must_use]
    pub const fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    /// Alias for [`Self::range`].
    #[must_use]
    pub const fn byte_range(&self) -> Range<usize> {
        self.range()
    }

    /// Return the matched length in UTF-8 bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// Return whether the source range is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Return the exact substring copied from the original source.
    #[must_use]
    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    /// Alias for [`Self::source_text`].
    #[must_use]
    pub fn exact_source(&self) -> &str {
        self.source_text()
    }

    /// Return metadata produced by the passage parser.
    #[must_use]
    pub const fn metadata(&self) -> &ParseMetadata {
        &self.metadata
    }

    /// Consume the match and return its parsed passage.
    #[must_use]
    pub fn into_passage(self) -> Passage {
        self.passage
    }
}

/// Identifies an invalid extractor window setting.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExtractorWindow {
    /// The search distance before an anchor.
    Lookbehind,
    /// The search distance after an anchor.
    Lookahead,
}

impl fmt::Display for ExtractorWindow {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Lookbehind => "max lookbehind",
            Self::Lookahead => "max lookahead",
        })
    }
}

/// Error returned for an extractor window outside `1..=4096`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExtractorConfigError {
    window: ExtractorWindow,
    value: usize,
}

impl ExtractorConfigError {
    /// Return the invalid setting.
    #[must_use]
    pub const fn window(self) -> ExtractorWindow {
        self.window
    }

    /// Return the rejected value.
    #[must_use]
    pub const fn value(self) -> usize {
        self.value
    }
}

impl fmt::Display for ExtractorConfigError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            output,
            "{} must be between {MIN_LOOKAROUND} and {MAX_LOOKAROUND} bytes (got {})",
            self.window, self.value
        )
    }
}

impl std::error::Error for ExtractorConfigError {}

/// Builder for a configurable [`ReferenceExtractor`].
#[derive(Clone, Debug)]
pub struct ReferenceExtractorBuilder {
    parser: PassageParser,
    include_bare_books: bool,
    max_lookbehind: usize,
    max_lookahead: usize,
}

impl ReferenceExtractorBuilder {
    /// Replace the parser used to validate candidate spans.
    #[must_use]
    pub fn parser(mut self, parser: PassageParser) -> Self {
        self.parser = parser;
        self
    }

    /// Configure whether whole-book passages are extracted.
    #[must_use]
    pub const fn include_bare_books(mut self, include: bool) -> Self {
        self.include_bare_books = include;
        self
    }

    /// Set the normalized-byte search distance before numeric anchors.
    ///
    /// The value is checked by [`Self::build`].
    #[must_use]
    pub const fn max_lookbehind(mut self, value: usize) -> Self {
        self.max_lookbehind = value;
        self
    }

    /// Set the normalized-byte search distance after numeric anchors.
    ///
    /// The value is checked by [`Self::build`].
    #[must_use]
    pub const fn max_lookahead(mut self, value: usize) -> Self {
        self.max_lookahead = value;
        self
    }

    /// Validate the settings and construct an extractor.
    pub fn build(self) -> Result<ReferenceExtractor, ExtractorConfigError> {
        ReferenceExtractor::with_options(
            self.parser,
            self.include_bare_books,
            self.max_lookbehind,
            self.max_lookahead,
        )
    }
}

impl Default for ReferenceExtractorBuilder {
    fn default() -> Self {
        Self {
            parser: PassageParser::new(),
            include_bare_books: false,
            max_lookbehind: DEFAULT_MAX_LOOKBEHIND,
            max_lookahead: DEFAULT_MAX_LOOKAHEAD,
        }
    }
}

/// Finds parseable Bible passages embedded in arbitrary prose.
///
/// Results are returned from left to right, never overlap, and prefer the
/// longest successful parse beginning at a given byte offset.
#[derive(Clone, Debug)]
pub struct ReferenceExtractor {
    parser: PassageParser,
    include_bare_books: bool,
    max_lookbehind: usize,
    max_lookahead: usize,
}

impl ReferenceExtractor {
    /// Construct an extractor with a default [`PassageParser`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Start configuring an extractor.
    #[must_use]
    pub fn builder() -> ReferenceExtractorBuilder {
        ReferenceExtractorBuilder::default()
    }

    /// Construct an extractor around an existing parser.
    #[must_use]
    pub fn from_parser(parser: PassageParser) -> Self {
        Self {
            parser,
            ..Self::default()
        }
    }

    /// Construct an extractor from validated configuration values.
    pub fn with_options(
        parser: PassageParser,
        include_bare_books: bool,
        max_lookbehind: usize,
        max_lookahead: usize,
    ) -> Result<Self, ExtractorConfigError> {
        validate_window(ExtractorWindow::Lookbehind, max_lookbehind)?;
        validate_window(ExtractorWindow::Lookahead, max_lookahead)?;
        Ok(Self {
            parser,
            include_bare_books,
            max_lookbehind,
            max_lookahead,
        })
    }

    /// Return the parser used to validate candidate spans.
    #[must_use]
    pub const fn parser(&self) -> &PassageParser {
        &self.parser
    }

    /// Return whether whole-book passages are included.
    #[must_use]
    pub const fn include_bare_books(&self) -> bool {
        self.include_bare_books
    }

    /// Return the normalized-byte search distance before numeric anchors.
    #[must_use]
    pub const fn max_lookbehind(&self) -> usize {
        self.max_lookbehind
    }

    /// Return the normalized-byte search distance after numeric anchors.
    #[must_use]
    pub const fn max_lookahead(&self) -> usize {
        self.max_lookahead
    }

    /// Return a copy configured to include or exclude whole-book passages.
    #[must_use]
    pub fn with_include_bare_books(mut self, include: bool) -> Self {
        self.include_bare_books = include;
        self
    }

    /// Return a copy with a validated lookbehind window.
    pub fn with_max_lookbehind(mut self, value: usize) -> Result<Self, ExtractorConfigError> {
        validate_window(ExtractorWindow::Lookbehind, value)?;
        self.max_lookbehind = value;
        Ok(self)
    }

    /// Return a copy with a validated lookahead window.
    pub fn with_max_lookahead(mut self, value: usize) -> Result<Self, ExtractorConfigError> {
        validate_window(ExtractorWindow::Lookahead, value)?;
        self.max_lookahead = value;
        Ok(self)
    }

    /// Find deterministic, non-overlapping passages in `source`.
    #[must_use]
    pub fn extract(&self, source: &str) -> Vec<PassageMatch> {
        self.extract_inner(source, None)
    }

    /// Find passages while resolving book names in one explicit language.
    #[must_use]
    pub fn extract_with_language(&self, source: &str, language: Language) -> Vec<PassageMatch> {
        self.extract_inner(source, Some(language))
    }

    /// Alias for [`Self::extract`].
    #[must_use]
    pub fn find_all(&self, source: &str) -> Vec<PassageMatch> {
        self.extract(source)
    }

    /// Language-specific alias for [`Self::extract_with_language`].
    #[must_use]
    pub fn find_all_with_language(&self, source: &str, language: Language) -> Vec<PassageMatch> {
        self.extract_with_language(source, language)
    }

    /// Replace all matches in one pass without scanning replacement text.
    #[must_use]
    pub fn replace_matches<F, R>(&self, source: &str, replacement: F) -> String
    where
        F: FnMut(&PassageMatch) -> R,
        R: fmt::Display,
    {
        self.replace_matches_inner(source, None, replacement)
    }

    /// Replace language-specific matches in one pass.
    #[must_use]
    pub fn replace_matches_with_language<F, R>(
        &self,
        source: &str,
        language: Language,
        replacement: F,
    ) -> String
    where
        F: FnMut(&PassageMatch) -> R,
        R: fmt::Display,
    {
        self.replace_matches_inner(source, Some(language), replacement)
    }

    /// Linkify matches with a caller-provided renderer.
    #[must_use]
    pub fn linkify<F, R>(&self, source: &str, link_builder: F) -> String
    where
        F: FnMut(&PassageMatch) -> R,
        R: fmt::Display,
    {
        self.replace_matches(source, link_builder)
    }

    /// Linkify matches resolved with one explicit language.
    #[must_use]
    pub fn linkify_with_language<F, R>(
        &self,
        source: &str,
        language: Language,
        link_builder: F,
    ) -> String
    where
        F: FnMut(&PassageMatch) -> R,
        R: fmt::Display,
    {
        self.replace_matches_with_language(source, language, link_builder)
    }

    /// Linkify matches as Markdown using each exact source slice as its label.
    #[must_use]
    pub fn linkify_markdown<F, U>(&self, source: &str, mut uri_builder: F) -> String
    where
        F: FnMut(&PassageMatch) -> U,
        U: fmt::Display,
    {
        self.replace_matches(source, |passage_match| {
            markdown_link(passage_match.source_text(), &uri_builder(passage_match))
        })
    }

    /// Linkify explicitly localized matches as Markdown using exact source labels.
    #[must_use]
    pub fn linkify_markdown_with_language<F, U>(
        &self,
        source: &str,
        language: Language,
        mut uri_builder: F,
    ) -> String
    where
        F: FnMut(&PassageMatch) -> U,
        U: fmt::Display,
    {
        self.replace_matches_with_language(source, language, |passage_match| {
            markdown_link(passage_match.source_text(), &uri_builder(passage_match))
        })
    }

    /// Linkify matches as Markdown with caller-provided labels and destinations.
    #[must_use]
    pub fn linkify_markdown_with_label<F, U, L>(
        &self,
        source: &str,
        mut uri_builder: F,
        mut label_builder: L,
    ) -> String
    where
        F: FnMut(&PassageMatch) -> U,
        U: fmt::Display,
        L: FnMut(&PassageMatch) -> String,
    {
        self.replace_matches(source, |passage_match| {
            markdown_link(&label_builder(passage_match), &uri_builder(passage_match))
        })
    }

    /// Linkify explicitly localized matches with custom Markdown labels.
    #[must_use]
    pub fn linkify_markdown_with_label_and_language<F, U, L>(
        &self,
        source: &str,
        language: Language,
        mut uri_builder: F,
        mut label_builder: L,
    ) -> String
    where
        F: FnMut(&PassageMatch) -> U,
        U: fmt::Display,
        L: FnMut(&PassageMatch) -> String,
    {
        self.replace_matches_with_language(source, language, |passage_match| {
            markdown_link(&label_builder(passage_match), &uri_builder(passage_match))
        })
    }

    /// Alias for [`Self::linkify_markdown`].
    #[must_use]
    pub fn markdown_linkify<F, U>(&self, source: &str, uri_builder: F) -> String
    where
        F: FnMut(&PassageMatch) -> U,
        U: fmt::Display,
    {
        self.linkify_markdown(source, uri_builder)
    }

    /// Language-specific alias for [`Self::linkify_markdown_with_language`].
    #[must_use]
    pub fn markdown_linkify_with_language<F, U>(
        &self,
        source: &str,
        language: Language,
        uri_builder: F,
    ) -> String
    where
        F: FnMut(&PassageMatch) -> U,
        U: fmt::Display,
    {
        self.linkify_markdown_with_language(source, language, uri_builder)
    }

    fn extract_inner(&self, source: &str, language: Option<Language>) -> Vec<PassageMatch> {
        if source.is_empty() {
            return Vec::new();
        }

        let normalization = normalize_detailed(source);
        let search_text = normalization.normalized();
        let mut candidates = BTreeMap::new();

        for (anchor_start, anchor_end) in numeric_anchors(search_text) {
            let minimum_start = anchor_start.saturating_sub(self.max_lookbehind);
            let maximum_end = anchor_end
                .saturating_add(self.max_lookahead)
                .min(search_text.len());
            self.search_window(
                search_text,
                &normalization,
                minimum_start,
                anchor_start,
                anchor_end,
                maximum_end,
                language,
                &mut candidates,
            );
        }

        if self.include_bare_books {
            self.search_bare_passages(search_text, &normalization, language, &mut candidates);
        }

        let mut ordered = candidates.into_values().collect::<Vec<_>>();
        ordered.sort_unstable_by(|left, right| {
            left.start
                .cmp(&right.start)
                .then_with(|| right.end.cmp(&left.end))
        });

        let mut selected = Vec::new();
        let mut consumed_through = 0;
        for candidate in ordered {
            if candidate.start < consumed_through {
                continue;
            }
            consumed_through = candidate.end;
            selected.push(candidate);
        }
        selected
    }

    #[allow(clippy::too_many_arguments)]
    fn search_window(
        &self,
        search_text: &str,
        normalization: &NormalizedInput,
        minimum_start: usize,
        required_start_before: usize,
        required_end_after: usize,
        maximum_end: usize,
        language: Option<Language>,
        output: &mut BTreeMap<(usize, usize), PassageMatch>,
    ) {
        let bounded_start = next_char_boundary(search_text, minimum_start);
        let starts = if bounded_start < required_start_before {
            search_text[bounded_start..required_start_before]
                .char_indices()
                .map(|(index, _)| bounded_start + index)
                .filter(|&index| can_start_at(search_text, index))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let bounded_end = previous_char_boundary(search_text, maximum_end);
        let mut ends = Vec::new();
        if required_end_after <= bounded_end
            && search_text[..required_end_after]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_ascii_digit())
        {
            ends.push(required_end_after);
        }
        if required_end_after < bounded_end {
            ends.extend(
                search_text[required_end_after..bounded_end]
                    .char_indices()
                    .filter_map(|(index, character)| {
                        let end = required_end_after + index + character.len_utf8();
                        character.is_ascii_digit().then_some(end)
                    }),
            );
        }
        ends.reverse();

        for start in starts {
            for &end in &ends {
                if end <= start || !has_safe_outer_boundaries(search_text, start, end) {
                    continue;
                }
                if self.try_candidate(search_text, start, end, normalization, language, output) {
                    break;
                }
            }
        }
    }

    fn search_bare_passages(
        &self,
        search_text: &str,
        normalization: &NormalizedInput,
        language: Option<Language>,
        output: &mut BTreeMap<(usize, usize), PassageMatch>,
    ) {
        for (start, _) in search_text.char_indices() {
            if !can_start_at(search_text, start) {
                continue;
            }
            let maximum_end = start
                .saturating_add(self.max_lookahead)
                .min(search_text.len());
            let maximum_end = previous_char_boundary(search_text, maximum_end);
            let mut ends = search_text[start..maximum_end]
                .char_indices()
                .filter_map(|(index, character)| {
                    let end = start + index + character.len_utf8();
                    can_end_at(search_text, end).then_some(end)
                })
                .collect::<Vec<_>>();
            ends.reverse();

            for end in ends {
                if !has_safe_outer_boundaries(search_text, start, end) {
                    continue;
                }
                self.try_candidate(search_text, start, end, normalization, language, output);
            }
        }
    }

    fn try_candidate(
        &self,
        search_text: &str,
        start: usize,
        end: usize,
        normalization: &NormalizedInput,
        language: Option<Language>,
        output: &mut BTreeMap<(usize, usize), PassageMatch>,
    ) -> bool {
        let Some(normalized_candidate) = search_text.get(start..end) else {
            return false;
        };
        let Some(parsed) = self.parse_candidate(normalized_candidate, language) else {
            return false;
        };
        let (mut passage, mut metadata) = parsed.into_parts();
        if !self.include_bare_books
            && (contains_bare_book(&passage) || !contains_reference_number(normalized_candidate))
        {
            return false;
        }

        let Some(mapped_span) = normalization.map_normalized_span(start, end) else {
            return false;
        };
        let original_span = trim_bidi_controls(normalization.original(), mapped_span);
        let key = (original_span.start(), original_span.end());
        if output.contains_key(&key) {
            return true;
        }
        let Some(source_text) = normalization.original().get(original_span.as_range()) else {
            return false;
        };

        if let Some(original) = self.parse_candidate(source_text, language) {
            (passage, metadata) = original.into_parts();
        }
        if looks_like_common_word(&metadata, language) {
            return false;
        }

        output.insert(
            key,
            PassageMatch {
                passage,
                start: original_span.start(),
                end: original_span.end(),
                source_text: source_text.to_owned(),
                metadata,
            },
        );
        true
    }

    fn parse_candidate(
        &self,
        candidate: &str,
        language: Option<Language>,
    ) -> Option<crate::Parsed<Passage>> {
        match language {
            Some(language) => self
                .parser
                .parse_detailed_with_language(candidate, language)
                .ok(),
            None => self.parser.parse_detailed(candidate).ok(),
        }
    }

    fn replace_matches_inner<F, R>(
        &self,
        source: &str,
        language: Option<Language>,
        mut replacement: F,
    ) -> String
    where
        F: FnMut(&PassageMatch) -> R,
        R: fmt::Display,
    {
        let matches = self.extract_inner(source, language);
        if matches.is_empty() {
            return source.to_owned();
        }

        let mut output = String::with_capacity(source.len());
        let mut copied_through = 0;
        for passage_match in &matches {
            output.push_str(&source[copied_through..passage_match.start]);
            write!(output, "{}", replacement(passage_match))
                .expect("writing into a String cannot fail");
            copied_through = passage_match.end;
        }
        output.push_str(&source[copied_through..]);
        output
    }
}

impl Default for ReferenceExtractor {
    fn default() -> Self {
        Self {
            parser: PassageParser::new(),
            include_bare_books: false,
            max_lookbehind: DEFAULT_MAX_LOOKBEHIND,
            max_lookahead: DEFAULT_MAX_LOOKAHEAD,
        }
    }
}

fn validate_window(window: ExtractorWindow, value: usize) -> Result<(), ExtractorConfigError> {
    if (MIN_LOOKAROUND..=MAX_LOOKAROUND).contains(&value) {
        Ok(())
    } else {
        Err(ExtractorConfigError { window, value })
    }
}

fn numeric_anchors(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut anchors = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        anchors.push((start, index));
    }
    anchors
}

fn next_char_boundary(source: &str, mut index: usize) -> usize {
    index = index.min(source.len());
    while index < source.len() && !source.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn previous_char_boundary(source: &str, mut index: usize) -> usize {
    index = index.min(source.len());
    while index > 0 && !source.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn contains_reference_number(value: &str) -> bool {
    value.bytes().any(|byte| byte.is_ascii_digit())
}

fn contains_bare_book(passage: &Passage) -> bool {
    match passage {
        Passage::Book(_) => true,
        Passage::Chapter(_) | Passage::Verses(_) => false,
        Passage::Sequence(sequence) => sequence.passages().iter().any(contains_bare_book),
    }
}

fn can_start_at(source: &str, index: usize) -> bool {
    if index >= source.len() || !source.is_char_boundary(index) {
        return false;
    }
    let current = source[index..]
        .chars()
        .next()
        .expect("a valid non-terminal boundary has a character");
    if is_edge_delimiter(current) {
        return false;
    }
    if index == 0 {
        return true;
    }
    let previous = source[..index]
        .chars()
        .next_back()
        .expect("a positive character boundary has a previous character");
    !forms_word_continuation(previous, current)
}

fn can_end_at(source: &str, index: usize) -> bool {
    if index == 0 || index > source.len() || !source.is_char_boundary(index) {
        return false;
    }
    let previous = source[..index]
        .chars()
        .next_back()
        .expect("a positive character boundary has a previous character");
    !is_edge_delimiter(previous)
}

fn has_safe_outer_boundaries(source: &str, start: usize, end: usize) -> bool {
    if start > 0 {
        let previous = source[..start]
            .chars()
            .next_back()
            .expect("a positive character boundary has a previous character");
        let current = source[start..]
            .chars()
            .next()
            .expect("a candidate start precedes its end");
        if forms_word_continuation(previous, current) {
            return false;
        }
    }
    if end < source.len() {
        let remaining = source[end..].trim_start();
        if let Some(punctuation) = remaining.chars().next() {
            if matches!(punctuation, ':' | '.' | '-' | ',')
                && remaining[punctuation.len_utf8()..]
                    .trim_start()
                    .starts_with(|character: char| character.is_ascii_digit())
            {
                return false;
            }
        }
        let previous = source[..end]
            .chars()
            .next_back()
            .expect("a candidate end follows its start");
        let current = source[end..]
            .chars()
            .next()
            .expect("a non-terminal boundary has a following character");
        if forms_word_continuation(previous, current) {
            return false;
        }
    }
    true
}

fn trim_bidi_controls(source: &str, span: SourceSpan) -> SourceSpan {
    let mut start = span.start();
    let mut end = span.end();
    while start < end {
        let Some(character) = source[start..end].chars().next() else {
            break;
        };
        if !is_bidi_control(character) {
            break;
        }
        start += character.len_utf8();
    }
    while end > start {
        let Some(character) = source[start..end].chars().next_back() else {
            break;
        };
        if !is_bidi_control(character) {
            break;
        }
        end -= character.len_utf8();
    }
    SourceSpan::new(start, end)
}

fn looks_like_common_word(metadata: &ParseMetadata, language: Option<Language>) -> bool {
    let automatic = language.is_none_or(Language::is_auto);
    for book_match in metadata.book_matches() {
        let book = book_match.selected().book();
        let token = book_match.token().trim_start();
        let compact_token = token
            .chars()
            .filter(|character| !matches!(character, ' ' | '.'))
            .flat_map(char::to_lowercase)
            .collect::<String>();
        if automatic && matches!(compact_token.as_str(), "am" | "at" | "is" | "so") {
            return true;
        }

        let mut first_ascii_letter_is_lowercase = false;
        let mut ascii_letter_count = 0;
        let mut is_simple_ascii_alias = true;
        for character in token.chars() {
            if character.is_ascii_uppercase() {
                ascii_letter_count += 1;
            } else if character.is_ascii_lowercase() {
                first_ascii_letter_is_lowercase |= ascii_letter_count == 0;
                ascii_letter_count += 1;
            } else if character != ' ' && character != '.' && !character.is_ascii_digit() {
                is_simple_ascii_alias = false;
            }
        }
        if !first_ascii_letter_is_lowercase {
            continue;
        }
        if matches!(book, Book::Mark | Book::Job) {
            return true;
        }
        if automatic && is_simple_ascii_alias && ascii_letter_count <= 3 {
            return true;
        }
    }
    false
}

fn forms_word_continuation(left: char, right: char) -> bool {
    if is_east_asian(left) || is_east_asian(right) {
        return false;
    }
    (left.is_alphanumeric() || left == '_') && (right.is_alphanumeric() || right == '_')
}

fn is_east_asian(character: char) -> bool {
    matches!(
        character as u32,
        0x1100..=0x11ff
            | 0x2e80..=0x2fff
            | 0x3040..=0x30ff
            | 0x3130..=0x318f
            | 0x3400..=0x4dbf
            | 0x4e00..=0x9fff
            | 0xac00..=0xd7af
            | 0xf900..=0xfaff
            | 0x20000..=0x2fa1f
    )
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character as u32,
        0x061c | 0x200e | 0x200f | 0x202a..=0x202e | 0x2066..=0x206f
    )
}

fn is_edge_delimiter(character: char) -> bool {
    character <= '\u{20}'
        || character == '\u{7f}'
        || is_bidi_control(character)
        || matches!(
            character,
            '.' | ','
                | '!'
                | '?'
                | ';'
                | ':'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | '"'
                | '\''
        )
}

fn markdown_link(label: &str, destination: &impl fmt::Display) -> String {
    let label = escape_markdown_label(label);
    let destination = escape_markdown_destination(&destination.to_string());
    format!("[{label}]({destination})")
}

fn escape_markdown_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_markdown_destination(value: &str) -> String {
    value.replace('(', "%28").replace(')', "%29")
}

#[cfg(test)]
#[path = "../tests/unit/extractor.rs"]
mod tests;
