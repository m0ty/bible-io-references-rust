//! Reusable, configurable parsing for verse references and ranges.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, OnceLock},
};

use crate::{
    Book, Language, ParseError, ParseErrorKind, Reference, VerseRange, VerseRef,
    language_data::aliases as localized_aliases,
    normalize::{normalize as normalize_syntax, normalize_for_parsing},
    reference::{MAX_CHAPTER_NUMBER, MAX_VERSE_NUMBER},
};

/// Built-in language priority after English during automatic detection.
pub const AUTO_LANGUAGE_PRECEDENCE: &[Language] = &[
    Language::Arabic,
    Language::Chinese,
    Language::French,
    Language::German,
    Language::Hebrew,
    Language::Hindi,
    Language::Indonesian,
    Language::Korean,
    Language::Portuguese,
    Language::Russian,
    Language::Spanish,
    Language::Tagalog,
];

/// Controls how a parser handles aliases that name distinct books.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum AmbiguityPolicy {
    /// Select the first match in configured language priority order.
    #[default]
    PreferLanguagePriority,
    /// Reject an alias when it resolves to more than one distinct book.
    Reject,
}

/// One candidate considered while resolving a book token.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BookCandidate {
    book: Book,
    alias: Arc<str>,
    language: Option<Language>,
    custom: bool,
}

impl BookCandidate {
    /// Return the candidate book.
    #[must_use]
    pub const fn book(&self) -> Book {
        self.book
    }

    /// Return the registered alias that produced this candidate.
    #[must_use]
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// Return the language that contributed the alias.
    ///
    /// A language-neutral custom alias returns `None`.
    #[must_use]
    pub const fn language(&self) -> Option<Language> {
        self.language
    }

    /// Whether this candidate came from parser configuration.
    #[must_use]
    pub const fn is_custom(&self) -> bool {
        self.custom
    }
}

/// Resolution details for one explicit book token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookMatch {
    token: String,
    selected: BookCandidate,
    alternatives: Vec<BookCandidate>,
}

impl BookMatch {
    /// Return the normalized token as it appeared in the parsed input.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Return the selected candidate.
    #[must_use]
    pub const fn selected(&self) -> &BookCandidate {
        &self.selected
    }

    /// Return all candidates not selected by the parser.
    #[must_use]
    pub fn alternatives(&self) -> &[BookCandidate] {
        &self.alternatives
    }

    /// Whether distinct books matched this token.
    #[must_use]
    pub fn is_ambiguous(&self) -> bool {
        self.alternatives
            .iter()
            .any(|candidate| candidate.book != self.selected.book)
    }
}

/// Metadata captured during a successful parse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseMetadata {
    normalized_input: String,
    book_matches: Vec<BookMatch>,
}

impl ParseMetadata {
    pub(crate) fn from_parts(normalized_input: String, book_matches: Vec<BookMatch>) -> Self {
        Self {
            normalized_input,
            book_matches,
        }
    }

    /// Return the parser-ready normalized input.
    #[must_use]
    pub fn normalized_input(&self) -> &str {
        &self.normalized_input
    }

    /// Return book matches in source order.
    #[must_use]
    pub fn book_matches(&self) -> &[BookMatch] {
        &self.book_matches
    }

    /// Return the selected languages, without duplicates, in source order.
    #[must_use]
    pub fn detected_languages(&self) -> Vec<Language> {
        let mut languages = Vec::new();
        for language in self
            .book_matches
            .iter()
            .filter_map(|book_match| book_match.selected.language)
        {
            if !languages.contains(&language) {
                languages.push(language);
            }
        }
        languages
    }

    /// Return a language when every localized match selected the same one.
    #[must_use]
    pub fn detected_language(&self) -> Option<Language> {
        let languages = self.detected_languages();
        match languages.as_slice() {
            [language] => Some(*language),
            _ => None,
        }
    }

    /// Whether any token had candidates for distinct books.
    #[must_use]
    pub fn has_ambiguity(&self) -> bool {
        self.book_matches.iter().any(BookMatch::is_ambiguous)
    }

    /// Iterate over every non-selected book candidate.
    pub fn alternate_matches(&self) -> impl Iterator<Item = &BookCandidate> {
        self.book_matches
            .iter()
            .flat_map(|book_match| book_match.alternatives.iter())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CustomAlias {
    normalized: String,
    alias: Arc<str>,
    book: Book,
}

/// A parsed value paired with detection metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parsed<T> {
    value: T,
    metadata: ParseMetadata,
}

impl<T> Parsed<T> {
    /// Construct a parsed value. Primarily useful to parser adapters.
    #[must_use]
    pub const fn new(value: T, metadata: ParseMetadata) -> Self {
        Self { value, metadata }
    }

    /// Borrow the parsed value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Borrow parse metadata.
    #[must_use]
    pub const fn metadata(&self) -> &ParseMetadata {
        &self.metadata
    }

    /// Consume the wrapper and return the parsed value.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }

    /// Consume the wrapper and return both components.
    #[must_use]
    pub fn into_parts(self) -> (T, ParseMetadata) {
        (self.value, self.metadata)
    }
}

/// Builder for a reusable [`ReferenceParser`].
#[derive(Clone, Debug, Default)]
pub struct ParserBuilder {
    aliases: Vec<(String, Book)>,
    aliases_by_language: Vec<(Language, String, Book)>,
    preferred_languages: Vec<Language>,
    ambiguity_policy: AmbiguityPolicy,
}

impl ParserBuilder {
    /// Register a language-neutral alias that takes precedence over bundled data.
    #[must_use]
    pub fn alias(mut self, alias: impl Into<String>, book: Book) -> Self {
        self.aliases.push((alias.into(), book));
        self
    }

    /// Register an alias that is available only for one language.
    #[must_use]
    pub fn language_alias(
        mut self,
        language: Language,
        alias: impl Into<String>,
        book: Book,
    ) -> Self {
        self.aliases_by_language
            .push((language, alias.into(), book));
        self
    }

    /// Set languages that should rank before the built-in automatic order.
    #[must_use]
    pub fn preferred_languages(mut self, languages: impl IntoIterator<Item = Language>) -> Self {
        self.preferred_languages.clear();
        for language in languages {
            if !self.preferred_languages.contains(&language) {
                self.preferred_languages.push(language);
            }
        }
        self
    }

    /// Set the ambiguity policy.
    #[must_use]
    pub const fn ambiguity_policy(mut self, policy: AmbiguityPolicy) -> Self {
        self.ambiguity_policy = policy;
        self
    }

    /// Validate configuration and construct a reusable parser.
    pub fn build(self) -> Result<ReferenceParser, ParseError> {
        if self.preferred_languages.contains(&Language::Auto) {
            return Err(ParseError::new(
                ParseErrorKind::UnsupportedLanguage,
                "preferred languages must be concrete languages, not auto mode",
            ));
        }
        let mut aliases = Vec::new();
        for (alias, book) in self.aliases {
            let key = normalize_book_term(&alias);
            if key.is_empty() {
                return Err(ParseError::new(
                    ParseErrorKind::EmptyBookToken,
                    "custom aliases must contain a book token",
                ));
            }
            aliases.push(CustomAlias {
                normalized: key,
                alias: Arc::from(alias),
                book,
            });
        }

        let mut aliases_by_language = Vec::new();
        for (language, alias, book) in self.aliases_by_language {
            if language.is_auto() {
                return Err(ParseError::new(
                    ParseErrorKind::UnsupportedLanguage,
                    "language-specific aliases cannot use auto mode",
                ));
            }
            let key = normalize_book_term(&alias);
            if key.is_empty() {
                return Err(ParseError::new(
                    ParseErrorKind::EmptyBookToken,
                    "custom aliases must contain a book token",
                ));
            }
            aliases_by_language.push((
                language,
                CustomAlias {
                    normalized: key,
                    alias: Arc::from(alias),
                    book,
                },
            ));
        }

        Ok(ReferenceParser {
            aliases,
            aliases_by_language,
            preferred_languages: self.preferred_languages,
            ambiguity_policy: self.ambiguity_policy,
        })
    }
}

/// A reusable parser with immutable alias and ambiguity configuration.
#[derive(Clone, Debug, Default)]
pub struct ReferenceParser {
    aliases: Vec<CustomAlias>,
    aliases_by_language: Vec<(Language, CustomAlias)>,
    preferred_languages: Vec<Language>,
    ambiguity_policy: AmbiguityPolicy,
}

impl ReferenceParser {
    /// Construct a parser with bundled aliases and automatic detection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Start configuring a parser.
    #[must_use]
    pub fn builder() -> ParserBuilder {
        ParserBuilder::default()
    }

    /// Return configured preferred languages.
    #[must_use]
    pub fn preferred_languages(&self) -> &[Language] {
        &self.preferred_languages
    }

    /// Iterate over configured language-neutral aliases in registration order.
    pub fn aliases(&self) -> impl Iterator<Item = (&str, Book)> {
        self.aliases
            .iter()
            .map(|alias| (alias.alias.as_ref(), alias.book))
    }

    /// Iterate over configured language-specific aliases in registration
    /// order.
    pub fn aliases_by_language(&self) -> impl Iterator<Item = (Language, &str, Book)> {
        self.aliases_by_language
            .iter()
            .map(|(language, alias)| (*language, alias.alias.as_ref(), alias.book))
    }

    /// Return this parser's ambiguity policy.
    #[must_use]
    pub const fn ambiguity_policy(&self) -> AmbiguityPolicy {
        self.ambiguity_policy
    }

    /// Parse a single verse or an inclusive range using automatic detection.
    pub fn parse(&self, input: &str) -> Result<Reference, ParseError> {
        self.parse_detailed(input).map(Parsed::into_value)
    }

    /// Parse a reference using one explicit language.
    pub fn parse_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Reference, ParseError> {
        self.parse_detailed_with_language(input, language)
            .map(Parsed::into_value)
    }

    /// Parse a reference and retain normalization and language metadata.
    pub fn parse_detailed(&self, input: &str) -> Result<Parsed<Reference>, ParseError> {
        self.parse_detailed_inner(input, Language::Auto)
    }

    /// Parse with an explicit language and retain metadata.
    pub fn parse_detailed_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<Reference>, ParseError> {
        self.parse_detailed_inner(input, language)
    }

    /// Return `None` rather than an error for invalid input.
    #[must_use]
    pub fn try_parse(&self, input: &str) -> Option<Reference> {
        self.parse(input).ok()
    }

    /// Parse with an explicit language, returning `None` for invalid input.
    #[must_use]
    pub fn try_parse_with_language(&self, input: &str, language: Language) -> Option<Reference> {
        self.parse_with_language(input, language).ok()
    }

    /// Resolve a standalone book token using automatic language detection.
    pub fn parse_book(&self, input: &str) -> Result<Book, ParseError> {
        self.parse_book_detailed(input).map(Parsed::into_value)
    }

    /// Resolve a standalone book token using an explicit language.
    pub fn parse_book_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Book, ParseError> {
        self.parse_book_detailed_with_language(input, language)
            .map(Parsed::into_value)
    }

    /// Resolve a standalone book token and retain detection metadata.
    pub fn parse_book_detailed(&self, input: &str) -> Result<Parsed<Book>, ParseError> {
        self.parse_book_detailed_with_language(input, Language::Auto)
    }

    /// Resolve a standalone book token with an explicit language and metadata.
    pub fn parse_book_detailed_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<Book>, ParseError> {
        let normalized = checked_normalized_input(input)?;
        let mut matches = Vec::new();
        let book = self.resolve_book(&normalized, language, &mut matches)?;
        Ok(Parsed::new(
            book,
            ParseMetadata::from_parts(normalized, matches),
        ))
    }

    /// Parse only a single verse.
    pub fn parse_verse(&self, input: &str) -> Result<VerseRef, ParseError> {
        self.parse_verse_detailed(input).map(Parsed::into_value)
    }

    /// Parse only a single verse with an explicit language.
    pub fn parse_verse_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<VerseRef, ParseError> {
        self.parse_verse_detailed_with_language(input, language)
            .map(Parsed::into_value)
    }

    /// Parse only a single verse and retain detection metadata.
    pub fn parse_verse_detailed(&self, input: &str) -> Result<Parsed<VerseRef>, ParseError> {
        self.parse_verse_detailed_inner(input, Language::Auto)
    }

    /// Parse only a single verse with an explicit language and metadata.
    pub fn parse_verse_detailed_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<VerseRef>, ParseError> {
        self.parse_verse_detailed_inner(input, language)
    }

    /// Parse only a single verse, returning `None` for invalid input.
    #[must_use]
    pub fn try_parse_verse(&self, input: &str) -> Option<VerseRef> {
        self.parse_verse(input).ok()
    }

    /// Parse only a single verse with an explicit language, returning `None`
    /// for invalid input.
    #[must_use]
    pub fn try_parse_verse_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Option<VerseRef> {
        self.parse_verse_with_language(input, language).ok()
    }

    /// Parse only a range.
    pub fn parse_range(&self, input: &str) -> Result<VerseRange, ParseError> {
        self.parse_range_detailed(input).map(Parsed::into_value)
    }

    /// Parse only a range with an explicit language.
    pub fn parse_range_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<VerseRange, ParseError> {
        self.parse_range_detailed_with_language(input, language)
            .map(Parsed::into_value)
    }

    /// Parse only a range and retain detection metadata.
    pub fn parse_range_detailed(&self, input: &str) -> Result<Parsed<VerseRange>, ParseError> {
        self.parse_range_detailed_inner(input, Language::Auto)
    }

    /// Parse only a range with an explicit language and metadata.
    pub fn parse_range_detailed_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<VerseRange>, ParseError> {
        self.parse_range_detailed_inner(input, language)
    }

    /// Parse only a range, returning `None` for invalid input.
    #[must_use]
    pub fn try_parse_range(&self, input: &str) -> Option<VerseRange> {
        self.parse_range(input).ok()
    }

    /// Parse only a range with an explicit language, returning `None` for
    /// invalid input.
    #[must_use]
    pub fn try_parse_range_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Option<VerseRange> {
        self.parse_range_with_language(input, language).ok()
    }

    fn parse_detailed_inner(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<Reference>, ParseError> {
        let normalized = checked_normalized_input(input)?;
        if find_range_separator(&normalized).is_some() {
            self.parse_normalized_range(&normalized, language)
                .map(|parsed| Parsed::new(Reference::Range(parsed.value), parsed.metadata))
        } else {
            self.parse_normalized_verse(&normalized, language)
                .map(|parsed| Parsed::new(Reference::Verse(parsed.value), parsed.metadata))
        }
    }

    fn parse_verse_detailed_inner(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<VerseRef>, ParseError> {
        let normalized = checked_normalized_input(input)?;
        if find_range_separator(&normalized).is_some() {
            return Err(ParseError::new(
                ParseErrorKind::PatternMismatch,
                "expected a single verse, found range syntax",
            ));
        }
        self.parse_normalized_verse(&normalized, language)
    }

    fn parse_range_detailed_inner(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<VerseRange>, ParseError> {
        let normalized = checked_normalized_input(input)?;
        if find_range_separator(&normalized).is_none() {
            return Err(ParseError::new(
                ParseErrorKind::PatternMismatch,
                "expected a verse range, found no range separator",
            ));
        }
        self.parse_normalized_range(&normalized, language)
    }

    fn parse_normalized_verse(
        &self,
        normalized: &str,
        language: Language,
    ) -> Result<Parsed<VerseRef>, ParseError> {
        let mut matches = Vec::new();
        let verse = self.parse_full_endpoint(normalized, language, &mut matches)?;
        Ok(Parsed::new(
            verse,
            ParseMetadata {
                normalized_input: normalized.to_owned(),
                book_matches: matches,
            },
        ))
    }

    fn parse_normalized_range(
        &self,
        normalized: &str,
        language: Language,
    ) -> Result<Parsed<VerseRange>, ParseError> {
        let separator = find_range_separator(normalized).ok_or_else(|| {
            ParseError::new(ParseErrorKind::PatternMismatch, "missing range separator")
        })?;
        let start_text = &normalized[..separator];
        let end_text = &normalized[separator + 1..];
        if find_range_separator(end_text).is_some()
            || start_text.trim().is_empty()
            || end_text.trim().is_empty()
        {
            return Err(ParseError::new(
                ParseErrorKind::PatternMismatch,
                "range must contain exactly two non-empty endpoints",
            ));
        }

        let mut matches = Vec::new();
        let start = self.parse_full_endpoint(start_text, language, &mut matches)?;
        let end = self.parse_range_end(end_text, start, language, &mut matches)?;
        let range = VerseRange::new(start, end).map_err(|_| {
            let kind = if start.book() == end.book() {
                ParseErrorKind::SameBookRangeNotAscending
            } else {
                ParseErrorKind::CrossBookRangeNotAscending
            };
            ParseError::new(
                kind,
                format!("range end {end} must come after start {start}"),
            )
        })?;

        Ok(Parsed::new(
            range,
            ParseMetadata {
                normalized_input: normalized.to_owned(),
                book_matches: matches,
            },
        ))
    }

    fn parse_full_endpoint(
        &self,
        input: &str,
        language: Language,
        matches: &mut Vec<BookMatch>,
    ) -> Result<VerseRef, ParseError> {
        let (left, verse_token) = split_coordinate(input)?;
        let (book_token, chapter_token) = split_trailing_number(left, "chapter")?;
        let book = self.resolve_book(book_token, language, matches)?;
        let chapter = parse_number(chapter_token, "chapter", MAX_CHAPTER_NUMBER)?;
        let verse = parse_number(verse_token, "verse", MAX_VERSE_NUMBER)?;
        VerseRef::new(book, chapter, verse).map_err(|error| {
            ParseError::new(ParseErrorKind::NumericTokenOutOfRange, error.to_string())
        })
    }

    fn parse_range_end(
        &self,
        input: &str,
        start: VerseRef,
        language: Language,
        matches: &mut Vec<BookMatch>,
    ) -> Result<VerseRef, ParseError> {
        let trimmed = input.trim();
        if trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
            let verse = parse_number(trimmed, "end verse", MAX_VERSE_NUMBER)?;
            return VerseRef::new(start.book(), start.chapter(), verse).map_err(|error| {
                ParseError::new(ParseErrorKind::NumericTokenOutOfRange, error.to_string())
            });
        }

        let (left, verse_token) = split_coordinate(trimmed)?;
        let (book_token, chapter_token) = split_trailing_number(left, "end chapter")?;
        let book = if book_token.trim().is_empty() {
            start.book()
        } else {
            self.resolve_book(book_token, language, matches)?
        };
        let chapter = parse_number(chapter_token, "end chapter", MAX_CHAPTER_NUMBER)?;
        let verse = parse_number(verse_token, "end verse", MAX_VERSE_NUMBER)?;
        VerseRef::new(book, chapter, verse).map_err(|error| {
            ParseError::new(ParseErrorKind::NumericTokenOutOfRange, error.to_string())
        })
    }

    fn resolve_book(
        &self,
        token: &str,
        language: Language,
        matches: &mut Vec<BookMatch>,
    ) -> Result<Book, ParseError> {
        let display_token = token.trim();
        let key = normalize_book_term(display_token);
        if key.is_empty() {
            return Err(ParseError::new(
                ParseErrorKind::EmptyBookToken,
                "book token is empty after normalization",
            ));
        }

        if !language.is_auto()
            && !language.is_parsing_supported()
            && !self
                .aliases_by_language
                .iter()
                .any(|(alias_language, _)| *alias_language == language)
        {
            return Err(ParseError::new(
                ParseErrorKind::UnsupportedLanguage,
                format!("no aliases are registered for language {}", language.code()),
            ));
        }

        let mut candidates = Vec::new();
        for alias in self.aliases.iter().filter(|alias| alias.normalized == key) {
            candidates.push(BookCandidate {
                book: alias.book,
                alias: Arc::clone(&alias.alias),
                language: None,
                custom: true,
            });
        }

        if language.is_auto() {
            self.collect_auto_candidates(&key, &mut candidates);
        } else {
            self.collect_language_candidates(language, &key, &mut candidates);
        }

        let language_priority = self.language_priority();
        candidates.sort_by_key(|candidate| {
            let custom_rank = u8::from(!candidate.custom);
            let language_rank = candidate.language.map_or(0, |language| {
                language_priority
                    .iter()
                    .position(|candidate| *candidate == language)
                    .map_or(language_priority.len() + 1, |index| index + 1)
            });
            (custom_rank, language_rank, candidate.book)
        });
        deduplicate_candidates(&mut candidates);
        let Some(selected) = candidates.first().cloned() else {
            return Err(ParseError::new(
                ParseErrorKind::UnknownBook,
                if language.is_auto() {
                    format!("book token {display_token:?} did not match a known book")
                } else {
                    format!(
                        "book token {display_token:?} is unknown for language {}",
                        language.code()
                    )
                },
            ));
        };

        let ambiguity_candidates = if candidates.iter().any(|candidate| candidate.custom) {
            candidates
                .iter()
                .filter(|candidate| candidate.custom)
                .collect::<Vec<_>>()
        } else {
            candidates.iter().collect::<Vec<_>>()
        };
        let mut distinct_books = Vec::new();
        for candidate in ambiguity_candidates {
            if !distinct_books.contains(&candidate.book) {
                distinct_books.push(candidate.book);
            }
        }
        if self.ambiguity_policy == AmbiguityPolicy::Reject && distinct_books.len() > 1 {
            let names = distinct_books
                .iter()
                .map(|book| book.full_name())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ParseError::new(
                ParseErrorKind::AmbiguousBook,
                format!("book token {display_token:?} matches {names}"),
            ));
        }

        matches.push(BookMatch {
            token: display_token.to_owned(),
            selected: selected.clone(),
            alternatives: candidates.into_iter().skip(1).collect(),
        });
        Ok(selected.book)
    }

    fn collect_auto_candidates(&self, key: &str, candidates: &mut Vec<BookCandidate>) {
        let mut languages = self.language_priority();
        for language in self
            .aliases_by_language
            .iter()
            .map(|(language, _)| *language)
        {
            if !language.is_auto() && !languages.contains(&language) {
                languages.push(language);
            }
        }

        // Configuration is intentional and therefore ranks ahead of every
        // bundled alias, while language priority still breaks custom ties.
        for &language in &languages {
            for (_, alias) in self
                .aliases_by_language
                .iter()
                .filter(|(alias_language, alias)| {
                    *alias_language == language && alias.normalized == key
                })
            {
                candidates.push(BookCandidate {
                    book: alias.book,
                    alias: Arc::clone(&alias.alias),
                    language: Some(language),
                    custom: true,
                });
            }
        }
        for &language in &languages {
            if let Some(records) = bundled_index().get(key) {
                candidates.extend(
                    records
                        .iter()
                        .filter(|candidate| candidate.language == Some(language))
                        .cloned(),
                );
            }
        }
    }

    fn language_priority(&self) -> Vec<Language> {
        let mut languages = Vec::new();
        for language in self
            .preferred_languages
            .iter()
            .copied()
            .chain([Language::English])
            .chain(AUTO_LANGUAGE_PRECEDENCE.iter().copied())
            .chain(Language::SUPPORTED)
        {
            if !language.is_auto() && !languages.contains(&language) {
                languages.push(language);
            }
        }
        languages
    }

    fn collect_language_candidates(
        &self,
        language: Language,
        key: &str,
        candidates: &mut Vec<BookCandidate>,
    ) {
        for (_, alias) in self
            .aliases_by_language
            .iter()
            .filter(|(alias_language, alias)| {
                *alias_language == language && alias.normalized == key
            })
        {
            candidates.push(BookCandidate {
                book: alias.book,
                alias: Arc::clone(&alias.alias),
                language: Some(language),
                custom: true,
            });
        }

        if let Some(records) = bundled_index().get(key) {
            candidates.extend(
                records
                    .iter()
                    .filter(|candidate| candidate.language == Some(language))
                    .cloned(),
            );
        }
    }
}

fn checked_normalized_input(input: &str) -> Result<String, ParseError> {
    let normalized = normalize_for_parsing(input);
    if normalized.is_empty() {
        return Err(ParseError::new(
            ParseErrorKind::EmptyReference,
            "reference must not be empty",
        ));
    }
    Ok(normalized)
}

fn find_range_separator(input: &str) -> Option<usize> {
    input
        .match_indices('-')
        .find_map(|(index, _)| split_coordinate(&input[..index]).is_ok().then_some(index))
}

fn split_coordinate(input: &str) -> Result<(&str, &str), ParseError> {
    let separator = input
        .char_indices()
        .rev()
        .find(|(_, character)| matches!(character, ':' | '.'))
        .map(|(index, character)| (index, character.len_utf8()))
        .ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::MissingNumericToken,
                "reference is missing a chapter/verse separator",
            )
        })?;
    let left = &input[..separator.0];
    let right = input[separator.0 + separator.1..].trim();
    if right.is_empty() {
        return Err(ParseError::new(
            ParseErrorKind::MissingNumericToken,
            "reference is missing a verse number",
        ));
    }
    if !right.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ParseError::new(
            ParseErrorKind::InvalidNumericToken,
            format!("verse token {right:?} is not an integer"),
        ));
    }
    Ok((left, right))
}

fn split_trailing_number<'a>(
    input: &'a str,
    component: &str,
) -> Result<(&'a str, &'a str), ParseError> {
    let trimmed = input.trim_end();
    let digit_start = trimmed
        .char_indices()
        .rev()
        .take_while(|(_, character)| character.is_ascii_digit())
        .last()
        .map(|(index, _)| index)
        .ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::MissingNumericToken,
                format!("reference is missing a {component} number"),
            )
        })?;
    Ok((&trimmed[..digit_start], &trimmed[digit_start..]))
}

fn parse_number(token: &str, component: &str, maximum: u16) -> Result<u16, ParseError> {
    let value = token.parse::<u32>().map_err(|_| {
        ParseError::new(
            ParseErrorKind::InvalidNumericToken,
            format!("{component} token {token:?} is not an integer"),
        )
    })?;
    if value == 0 {
        return Err(ParseError::new(
            ParseErrorKind::NonPositiveNumericToken,
            format!("{component} must be greater than zero"),
        ));
    }
    if value > u32::from(maximum) {
        return Err(ParseError::new(
            ParseErrorKind::NumericTokenOutOfRange,
            format!("{component} {value} exceeds the sanity limit {maximum}"),
        ));
    }
    Ok(value as u16)
}

fn normalize_book_term(term: &str) -> String {
    normalize_syntax(term)
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|character| !character.is_whitespace() && *character != '.')
        .collect()
}

fn deduplicate_candidates(candidates: &mut Vec<BookCandidate>) {
    let mut seen = HashSet::new();
    candidates
        .retain(|candidate| seen.insert((candidate.book, candidate.language, candidate.custom)));
}

/// Return every bundled automatic-detection alias that can resolve to
/// distinct books.
///
/// Keys use the parser's case-folded, period-free, whitespace-free lookup
/// form. Books retain canonical order, making the result deterministic.
#[must_use]
pub fn auto_language_collisions() -> &'static BTreeMap<String, Vec<Book>> {
    static COLLISIONS: OnceLock<BTreeMap<String, Vec<Book>>> = OnceLock::new();
    COLLISIONS.get_or_init(|| {
        let mut collisions = BTreeMap::new();
        for (alias, candidates) in bundled_index() {
            let mut books = candidates
                .iter()
                .map(BookCandidate::book)
                .collect::<Vec<_>>();
            books.sort_unstable();
            books.dedup();
            if books.len() > 1 {
                collisions.insert(alias.clone(), books);
            }
        }
        collisions
    })
}

fn bundled_index() -> &'static HashMap<String, Vec<BookCandidate>> {
    static INDEX: OnceLock<HashMap<String, Vec<BookCandidate>>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index: HashMap<String, Vec<BookCandidate>> = HashMap::new();
        for &book in Book::ALL {
            for alias in [book.full_name(), book.abbreviation()] {
                register_bundled_alias(&mut index, alias, book, Language::English);
            }
        }
        for &(alias, book) in COMMON_ENGLISH_ALIASES {
            register_bundled_alias(&mut index, alias, book, Language::English);
        }
        for &language in AUTO_LANGUAGE_PRECEDENCE {
            if let Some(books) = localized_aliases(language) {
                for record in books {
                    for alias in record.all_aliases() {
                        register_bundled_alias(&mut index, alias, record.book, language);
                    }
                }
            }
        }
        index
    })
}

fn register_bundled_alias(
    index: &mut HashMap<String, Vec<BookCandidate>>,
    alias: &str,
    book: Book,
    language: Language,
) {
    index
        .entry(normalize_book_term(alias))
        .or_default()
        .push(BookCandidate {
            book,
            alias: Arc::from(alias),
            language: Some(language),
            custom: false,
        });
}

// Common English abbreviations supplement the Dart wire abbreviations. This
// deliberately omits `Jn`, whose normalized form is the package's canonical
// Jonah abbreviation and would make explicit-English parsing unstable.
const COMMON_ENGLISH_ALIASES: &[(&str, Book)] = &[
    ("Gen", Book::Genesis),
    ("Exod", Book::Exodus),
    ("Lev", Book::Leviticus),
    ("Num", Book::Numbers),
    ("Deut", Book::Deuteronomy),
    ("Josh", Book::Joshua),
    ("Judg", Book::Judges),
    ("1 Sam", Book::FirstSamuel),
    ("2 Sam", Book::SecondSamuel),
    ("1 Kings", Book::FirstKings),
    ("2 Kings", Book::SecondKings),
    ("1 Chr", Book::FirstChronicles),
    ("2 Chr", Book::SecondChronicles),
    ("Neh", Book::Nehemiah),
    ("Est", Book::Esther),
    ("Prov", Book::Proverbs),
    ("Eccl", Book::Ecclesiastes),
    ("Song", Book::SongOfSolomon),
    ("Isa", Book::Isaiah),
    ("Jer", Book::Jeremiah),
    ("Lam", Book::Lamentations),
    ("Ezek", Book::Ezekiel),
    ("Dan", Book::Daniel),
    ("Hos", Book::Hosea),
    ("Obad", Book::Obadiah),
    ("Mic", Book::Micah),
    ("Nah", Book::Nahum),
    ("Hab", Book::Habakkuk),
    ("Zeph", Book::Zephaniah),
    ("Zech", Book::Zechariah),
    ("Matt", Book::Matthew),
    ("Mar", Book::Mark),
    ("Luk", Book::Luke),
    ("Joh", Book::John),
    ("Rom", Book::Romans),
    ("1 Cor", Book::FirstCorinthians),
    ("2 Cor", Book::SecondCorinthians),
    ("Gal", Book::Galatians),
    ("Phil", Book::Philippians),
    ("Col", Book::Colossians),
    ("1 Thess", Book::FirstThessalonians),
    ("2 Thess", Book::SecondThessalonians),
    ("1 Tim", Book::FirstTimothy),
    ("2 Tim", Book::SecondTimothy),
    ("Phlm", Book::Philemon),
    ("Heb", Book::Hebrews),
    ("Jas", Book::James),
    ("1 Pet", Book::FirstPeter),
    ("2 Pet", Book::SecondPeter),
    ("1 John", Book::FirstJohn),
    ("2 John", Book::SecondJohn),
    ("3 John", Book::ThirdJohn),
    ("Rev", Book::Revelation),
];

#[cfg(test)]
#[path = "../tests/unit/parser.rs"]
mod tests;
