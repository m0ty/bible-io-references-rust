//! Strict OSIS and USFM machine identifiers.
//!
//! These parsers intentionally accept only the case-sensitive machine forms
//! produced by [`MachineIdentifiers`]. They do not normalize whitespace,
//! punctuation, or book-code casing.

use core::fmt;

use crate::{
    Book, BookPassage, ChapterPassage, Coordinate, CoordinateError, Passage, PassageSequence,
    Reference, VersePassage, VerseRange, VerseRef,
};

/// A machine-readable Bible reference vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IdentifierFormat {
    /// Open Scripture Information Standard identifiers.
    Osis,
    /// Unified Standard Format Marker identifiers.
    Usfm,
}

impl IdentifierFormat {
    /// Return the conventional uppercase format name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Osis => "OSIS",
            Self::Usfm => "USFM",
        }
    }
}

impl fmt::Display for IdentifierFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Stable classification for a machine-identifier parse failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum IdentifierErrorKind {
    /// The input does not have one of the supported exact shapes.
    InvalidSyntax,
    /// The case-sensitive book code is not supported.
    UnknownBook,
    /// A chapter is outside the inclusive `1..=999` sanity range.
    InvalidChapter,
    /// A verse is outside the inclusive `1..=999` sanity range.
    InvalidVerse,
    /// A range is equal or descending in canonical book order.
    RangeNotAscending,
}

impl IdentifierErrorKind {
    /// Return a stable snake-case error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidSyntax => "invalid_identifier_syntax",
            Self::UnknownBook => "unknown_identifier_book",
            Self::InvalidChapter => "invalid_identifier_chapter",
            Self::InvalidVerse => "invalid_identifier_verse",
            Self::RangeNotAscending => "identifier_range_not_ascending",
        }
    }
}

/// A typed failure to parse an OSIS or USFM identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentifierError {
    format: IdentifierFormat,
    kind: IdentifierErrorKind,
    input: String,
    details: String,
}

impl IdentifierError {
    fn new(
        format: IdentifierFormat,
        kind: IdentifierErrorKind,
        input: &str,
        details: impl Into<String>,
    ) -> Self {
        Self {
            format,
            kind,
            input: input.to_owned(),
            details: details.into(),
        }
    }

    fn invalid_syntax(format: IdentifierFormat, input: &str) -> Self {
        let expected = match format {
            IdentifierFormat::Osis => {
                "expected BOOK.CHAPTER.VERSE or \
                 BOOK.CHAPTER.VERSE-BOOK.CHAPTER.VERSE"
            }
            IdentifierFormat::Usfm => {
                "expected BOOK CHAPTER:VERSE, BOOK CHAPTER:VERSE-VERSE, \
                 BOOK CHAPTER:VERSE-CHAPTER:VERSE, or \
                 BOOK-BOOK CHAPTER:VERSE-CHAPTER:VERSE"
            }
        };
        Self::new(format, IdentifierErrorKind::InvalidSyntax, input, expected)
    }

    fn unknown_book(format: IdentifierFormat, input: &str, book: &str) -> Self {
        Self::new(
            format,
            IdentifierErrorKind::UnknownBook,
            input,
            format!("unknown or non-canonical {format} book identifier {book:?}"),
        )
    }

    fn invalid_coordinate(
        format: IdentifierFormat,
        input: &str,
        coordinate: Coordinate,
        value: &str,
    ) -> Self {
        let kind = match coordinate {
            Coordinate::Chapter => IdentifierErrorKind::InvalidChapter,
            Coordinate::Verse => IdentifierErrorKind::InvalidVerse,
        };
        Self::new(
            format,
            kind,
            input,
            format!("{coordinate} must be between 1 and 999 (got {value})"),
        )
    }

    fn range_not_ascending(
        format: IdentifierFormat,
        input: &str,
        start: VerseRef,
        end: VerseRef,
    ) -> Self {
        Self::new(
            format,
            IdentifierErrorKind::RangeNotAscending,
            input,
            format!("range end {end} must come after start {start}"),
        )
    }

    /// Return the identifier vocabulary being parsed.
    #[must_use]
    pub const fn format(&self) -> IdentifierFormat {
        self.format
    }

    /// Return the stable failure classification.
    #[must_use]
    pub const fn kind(&self) -> IdentifierErrorKind {
        self.kind
    }

    /// Return the stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// Return the complete rejected identifier.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Return the input-specific diagnostic.
    #[must_use]
    pub fn details(&self) -> &str {
        &self.details
    }
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {} identifier {:?}: {}",
            self.code(),
            self.format,
            self.input,
            self.details
        )
    }
}

impl std::error::Error for IdentifierError {}

/// Canonical OSIS and USFM serialization for Bible reference values.
pub trait MachineIdentifiers {
    /// Encode this value as a case-sensitive OSIS identifier.
    #[must_use]
    fn osis_identifier(&self) -> String;

    /// Encode this value as a case-sensitive USFM identifier.
    #[must_use]
    fn usfm_identifier(&self) -> String;
}

impl MachineIdentifiers for Book {
    fn osis_identifier(&self) -> String {
        self.osis().to_owned()
    }

    fn usfm_identifier(&self) -> String {
        self.usfm().to_owned()
    }
}

impl MachineIdentifiers for VerseRef {
    fn osis_identifier(&self) -> String {
        format!("{}.{}.{}", self.book().osis(), self.chapter(), self.verse())
    }

    fn usfm_identifier(&self) -> String {
        format!("{} {}:{}", self.book().usfm(), self.chapter(), self.verse())
    }
}

impl MachineIdentifiers for VerseRange {
    fn osis_identifier(&self) -> String {
        format!(
            "{}-{}",
            self.start().osis_identifier(),
            self.end().osis_identifier()
        )
    }

    fn usfm_identifier(&self) -> String {
        let start = self.start();
        let end = self.end();
        if start.book() != end.book() {
            return format!(
                "{}-{} {}:{}-{}:{}",
                start.book().usfm(),
                end.book().usfm(),
                start.chapter(),
                start.verse(),
                end.chapter(),
                end.verse()
            );
        }
        if start.chapter() == end.chapter() {
            return format!(
                "{} {}:{}-{}",
                start.book().usfm(),
                start.chapter(),
                start.verse(),
                end.verse()
            );
        }
        format!(
            "{} {}:{}-{}:{}",
            start.book().usfm(),
            start.chapter(),
            start.verse(),
            end.chapter(),
            end.verse()
        )
    }
}

impl MachineIdentifiers for Reference {
    fn osis_identifier(&self) -> String {
        match self {
            Self::Verse(verse) => verse.osis_identifier(),
            Self::Range(range) => range.osis_identifier(),
        }
    }

    fn usfm_identifier(&self) -> String {
        match self {
            Self::Verse(verse) => verse.usfm_identifier(),
            Self::Range(range) => range.usfm_identifier(),
        }
    }
}

impl MachineIdentifiers for BookPassage {
    fn osis_identifier(&self) -> String {
        self.book().osis_identifier()
    }

    fn usfm_identifier(&self) -> String {
        self.book().usfm_identifier()
    }
}

impl MachineIdentifiers for ChapterPassage {
    fn osis_identifier(&self) -> String {
        let book = self.book().osis();
        match self.end_chapter() {
            Some(end) => format!("{book}.{}-{book}.{end}", self.start_chapter()),
            None => format!("{book}.{}", self.start_chapter()),
        }
    }

    fn usfm_identifier(&self) -> String {
        let book = self.book().usfm();
        match self.end_chapter() {
            Some(end) => format!("{book} {}-{end}", self.start_chapter()),
            None => format!("{book} {}", self.start_chapter()),
        }
    }
}

impl MachineIdentifiers for VersePassage {
    fn osis_identifier(&self) -> String {
        self.selections()
            .iter()
            .map(MachineIdentifiers::osis_identifier)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn usfm_identifier(&self) -> String {
        let first = self
            .selections()
            .first()
            .expect("VersePassage guarantees at least one selection");
        let anchor = first.start();
        let mut identifier = first.usfm_identifier();
        for selection in &self.selections()[1..] {
            identifier.push(',');
            identifier.push_str(&compact_usfm_selection(*selection, anchor));
        }
        identifier
    }
}

impl MachineIdentifiers for PassageSequence {
    fn osis_identifier(&self) -> String {
        self.passages()
            .iter()
            .map(MachineIdentifiers::osis_identifier)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn usfm_identifier(&self) -> String {
        self.passages()
            .iter()
            .map(MachineIdentifiers::usfm_identifier)
            .collect::<Vec<_>>()
            .join("; ")
    }
}

impl MachineIdentifiers for Passage {
    fn osis_identifier(&self) -> String {
        match self {
            Self::Book(passage) => passage.osis_identifier(),
            Self::Chapter(passage) => passage.osis_identifier(),
            Self::Verses(passage) => passage.osis_identifier(),
            Self::Sequence(passage) => passage.osis_identifier(),
        }
    }

    fn usfm_identifier(&self) -> String {
        match self {
            Self::Book(passage) => passage.usfm_identifier(),
            Self::Chapter(passage) => passage.usfm_identifier(),
            Self::Verses(passage) => passage.usfm_identifier(),
            Self::Sequence(passage) => passage.usfm_identifier(),
        }
    }
}

fn compact_usfm_selection(selection: Reference, anchor: VerseRef) -> String {
    match selection {
        Reference::Verse(verse)
            if verse.book() == anchor.book() && verse.chapter() == anchor.chapter() =>
        {
            verse.verse().to_string()
        }
        Reference::Range(range)
            if range.start().book() == anchor.book()
                && range.end().book() == anchor.book()
                && range.start().chapter() == anchor.chapter()
                && range.end().chapter() == anchor.chapter() =>
        {
            format!("{}-{}", range.start().verse(), range.end().verse())
        }
        _ => selection.usfm_identifier(),
    }
}

/// Resolve an exact, case-sensitive OSIS book identifier.
pub fn book_from_osis_identifier(identifier: &str) -> Result<Book, IdentifierError> {
    Book::from_osis(identifier).ok_or_else(|| {
        IdentifierError::unknown_book(IdentifierFormat::Osis, identifier, identifier)
    })
}

/// Resolve an exact, case-sensitive USFM book identifier.
pub fn book_from_usfm_identifier(identifier: &str) -> Result<Book, IdentifierError> {
    Book::from_usfm(identifier).ok_or_else(|| {
        IdentifierError::unknown_book(IdentifierFormat::Usfm, identifier, identifier)
    })
}

/// Parse a verse or full-endpoint OSIS range identifier.
///
/// Supported shapes are `John.3.16` and
/// `2Cor.6.14-2Cor.7.1`.
pub fn reference_from_osis_identifier(identifier: &str) -> Result<Reference, IdentifierError> {
    let mut endpoint_tokens = identifier.split('-');
    let start_token = endpoint_tokens
        .next()
        .expect("split always produces one token");
    let end_token = endpoint_tokens.next();
    if endpoint_tokens.next().is_some() {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Osis,
            identifier,
        ));
    }
    if !is_valid_osis_endpoint(start_token)
        || end_token.is_some_and(|endpoint| !is_valid_osis_endpoint(endpoint))
    {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Osis,
            identifier,
        ));
    }

    let start = parse_osis_endpoint(start_token, identifier)?;
    let Some(end_token) = end_token else {
        return Ok(Reference::Verse(start));
    };
    let end = parse_osis_endpoint(end_token, identifier)?;
    checked_range(start, end, IdentifierFormat::Osis, identifier)
}

/// Parse a standard USFM verse or same-book range identifier.
///
/// In addition to `JHN 3:16`, `JHN 3:16-17`, and `JHN 3:16-4:1`, this
/// accepts the package's reversible cross-book extension
/// `JHN-ACT 21:25-1:2`.
pub fn reference_from_usfm_identifier(identifier: &str) -> Result<Reference, IdentifierError> {
    let Some((book_token, coordinate_token)) = identifier.split_once(' ') else {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Usfm,
            identifier,
        ));
    };
    if coordinate_token.contains(' ') {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Usfm,
            identifier,
        ));
    }

    if book_token.contains('-') {
        return parse_cross_book_usfm(book_token, coordinate_token, identifier);
    }
    parse_same_book_usfm(book_token, coordinate_token, identifier)
}

fn parse_osis_endpoint(endpoint: &str, source: &str) -> Result<VerseRef, IdentifierError> {
    let mut components = endpoint.split('.');
    let book_token = components.next().expect("split always produces one token");
    let chapter_token = components.next();
    let verse_token = components.next();
    if components.next().is_some()
        || !is_osis_book_token(book_token)
        || chapter_token.is_none_or(|token| !is_ascii_number(token))
        || verse_token.is_none_or(|token| !is_ascii_number(token))
    {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Osis,
            source,
        ));
    }

    let book = lookup_book(book_token, IdentifierFormat::Osis, source)?;
    build_verse(
        book,
        chapter_token.expect("validated chapter token is present"),
        verse_token.expect("validated verse token is present"),
        IdentifierFormat::Osis,
        source,
    )
}

fn is_valid_osis_endpoint(endpoint: &str) -> bool {
    let mut components = endpoint.split('.');
    let book = components.next().expect("split always produces one token");
    let chapter = components.next();
    let verse = components.next();
    components.next().is_none()
        && is_osis_book_token(book)
        && chapter.is_some_and(is_ascii_number)
        && verse.is_some_and(is_ascii_number)
}

fn parse_same_book_usfm(
    book_token: &str,
    coordinate_token: &str,
    source: &str,
) -> Result<Reference, IdentifierError> {
    if !is_usfm_book_token(book_token) {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Usfm,
            source,
        ));
    }
    let mut range_tokens = coordinate_token.split('-');
    let start_token = range_tokens
        .next()
        .expect("split always produces one token");
    let end_token = range_tokens.next();
    if range_tokens.next().is_some() {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Usfm,
            source,
        ));
    }

    let (start_chapter, start_verse) = parse_full_usfm_coordinate(start_token, source)?;
    let end_coordinate = match end_token {
        Some(token) if token.contains(':') => Some(parse_full_usfm_coordinate(token, source)?),
        Some(token) if is_ascii_number(token) => Some((start_chapter, token)),
        Some(_) => {
            return Err(IdentifierError::invalid_syntax(
                IdentifierFormat::Usfm,
                source,
            ));
        }
        None => None,
    };
    let book = lookup_book(book_token, IdentifierFormat::Usfm, source)?;
    let start = build_verse(
        book,
        start_chapter,
        start_verse,
        IdentifierFormat::Usfm,
        source,
    )?;
    let Some((end_chapter, end_verse)) = end_coordinate else {
        return Ok(Reference::Verse(start));
    };
    let end = build_verse(book, end_chapter, end_verse, IdentifierFormat::Usfm, source)?;
    checked_range(start, end, IdentifierFormat::Usfm, source)
}

fn parse_cross_book_usfm(
    book_token: &str,
    coordinate_token: &str,
    source: &str,
) -> Result<Reference, IdentifierError> {
    let mut books = book_token.split('-');
    let start_book_token = books.next().expect("split always produces one token");
    let end_book_token = books.next();
    if books.next().is_some()
        || !is_usfm_book_token(start_book_token)
        || end_book_token.is_none_or(|token| !is_usfm_book_token(token))
    {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Usfm,
            source,
        ));
    }

    let mut coordinates = coordinate_token.split('-');
    let start_coordinate = coordinates.next().expect("split always produces one token");
    let end_coordinate = coordinates.next();
    if coordinates.next().is_some() || end_coordinate.is_none() {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Usfm,
            source,
        ));
    }

    let (start_chapter, start_verse) = parse_full_usfm_coordinate(start_coordinate, source)?;
    let (end_chapter, end_verse) = parse_full_usfm_coordinate(
        end_coordinate.expect("validated end coordinate is present"),
        source,
    )?;
    let end_book_token = end_book_token.expect("validated end book token is present");
    let start_book = lookup_book(start_book_token, IdentifierFormat::Usfm, source)?;
    let end_book = lookup_book(end_book_token, IdentifierFormat::Usfm, source)?;
    let start = build_verse(
        start_book,
        start_chapter,
        start_verse,
        IdentifierFormat::Usfm,
        source,
    )?;
    let end = build_verse(
        end_book,
        end_chapter,
        end_verse,
        IdentifierFormat::Usfm,
        source,
    )?;
    checked_range(start, end, IdentifierFormat::Usfm, source)
}

fn parse_full_usfm_coordinate<'a>(
    coordinate: &'a str,
    source: &str,
) -> Result<(&'a str, &'a str), IdentifierError> {
    let mut components = coordinate.split(':');
    let chapter = components.next().expect("split always produces one token");
    let verse = components.next();
    if components.next().is_some()
        || !is_ascii_number(chapter)
        || verse.is_none_or(|token| !is_ascii_number(token))
    {
        return Err(IdentifierError::invalid_syntax(
            IdentifierFormat::Usfm,
            source,
        ));
    }
    Ok((
        chapter,
        verse.expect("validated verse coordinate is present"),
    ))
}

fn build_verse(
    book: Book,
    chapter_token: &str,
    verse_token: &str,
    format: IdentifierFormat,
    source: &str,
) -> Result<VerseRef, IdentifierError> {
    let chapter = parse_coordinate(chapter_token, Coordinate::Chapter, format, source)?;
    let verse = parse_coordinate(verse_token, Coordinate::Verse, format, source)?;
    VerseRef::new(book, chapter, verse).map_err(|error| coordinate_error(error, format, source))
}

fn parse_coordinate(
    token: &str,
    coordinate: Coordinate,
    format: IdentifierFormat,
    source: &str,
) -> Result<u16, IdentifierError> {
    token
        .parse::<u16>()
        .map_err(|_| IdentifierError::invalid_coordinate(format, source, coordinate, token))
}

fn coordinate_error(
    error: CoordinateError,
    format: IdentifierFormat,
    source: &str,
) -> IdentifierError {
    IdentifierError::invalid_coordinate(
        format,
        source,
        error.coordinate(),
        &error.value().to_string(),
    )
}

fn checked_range(
    start: VerseRef,
    end: VerseRef,
    format: IdentifierFormat,
    source: &str,
) -> Result<Reference, IdentifierError> {
    VerseRange::new(start, end)
        .map(Reference::Range)
        .map_err(|_| IdentifierError::range_not_ascending(format, source, start, end))
}

fn lookup_book(
    token: &str,
    format: IdentifierFormat,
    source: &str,
) -> Result<Book, IdentifierError> {
    let book = match format {
        IdentifierFormat::Osis => Book::from_osis(token),
        IdentifierFormat::Usfm => Book::from_usfm(token),
    };
    book.ok_or_else(|| IdentifierError::unknown_book(format, source, token))
}

fn is_ascii_number(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_osis_book_token(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn is_usfm_book_token(token: &str) -> bool {
    token.len() == 3
        && token
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || matches!(byte, b'1'..=b'4'))
}

#[cfg(test)]
#[path = "../tests/unit/identifiers.rs"]
mod tests;
