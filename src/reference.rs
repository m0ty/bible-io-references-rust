//! Checked verse, range, and reference value types.

use core::{fmt, str::FromStr};

use crate::{Book, Language, ParseError, ReferenceParser};

/// Broad upper limit for chapter coordinates.
pub const MAX_CHAPTER_NUMBER: u16 = 999;

/// Broad upper limit for verse coordinates.
pub const MAX_VERSE_NUMBER: u16 = 999;

/// Identifies one coordinate in a verse reference.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Coordinate {
    /// Chapter coordinate.
    Chapter,
    /// Verse coordinate.
    Verse,
}

impl fmt::Display for Coordinate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Chapter => "chapter",
            Self::Verse => "verse",
        })
    }
}

/// Error returned when constructing a verse with an invalid coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoordinateError {
    coordinate: Coordinate,
    value: u16,
    maximum: u16,
}

impl CoordinateError {
    /// Return which coordinate failed validation.
    #[must_use]
    pub const fn coordinate(self) -> Coordinate {
        self.coordinate
    }

    /// Return the rejected value.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.value
    }

    /// Return the inclusive sanity limit.
    #[must_use]
    pub const fn maximum(self) -> u16 {
        self.maximum
    }
}

impl fmt::Display for CoordinateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} must be between 1 and {} (got {})",
            self.coordinate, self.maximum, self.value
        )
    }
}

impl std::error::Error for CoordinateError {}

/// A single Bible verse coordinate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VerseRef {
    book: Book,
    chapter: u16,
    verse: u16,
}

impl VerseRef {
    /// Construct a validated verse coordinate.
    pub const fn new(book: Book, chapter: u16, verse: u16) -> Result<Self, CoordinateError> {
        if chapter == 0 || chapter > MAX_CHAPTER_NUMBER {
            return Err(CoordinateError {
                coordinate: Coordinate::Chapter,
                value: chapter,
                maximum: MAX_CHAPTER_NUMBER,
            });
        }
        if verse == 0 || verse > MAX_VERSE_NUMBER {
            return Err(CoordinateError {
                coordinate: Coordinate::Verse,
                value: verse,
                maximum: MAX_VERSE_NUMBER,
            });
        }
        Ok(Self {
            book,
            chapter,
            verse,
        })
    }

    /// Parse a verse with an optional explicit book-name language.
    pub fn parse_with_language(input: &str, language: Language) -> Result<Self, ParseError> {
        ReferenceParser::new().parse_verse_with_language(input, language)
    }

    /// Parse a verse, returning `None` instead of an error for invalid input.
    #[must_use]
    pub fn try_parse(input: &str) -> Option<Self> {
        input.parse().ok()
    }

    /// Parse a verse with an explicit language, returning `None` for invalid
    /// input.
    #[must_use]
    pub fn try_parse_with_language(input: &str, language: Language) -> Option<Self> {
        Self::parse_with_language(input, language).ok()
    }

    /// Return the book.
    #[must_use]
    pub const fn book(self) -> Book {
        self.book
    }

    /// Return the one-based chapter.
    #[must_use]
    pub const fn chapter(self) -> u16 {
        self.chapter
    }

    /// Return the one-based verse.
    #[must_use]
    pub const fn verse(self) -> u16 {
        self.verse
    }

    /// Return a copy with a different book.
    #[must_use]
    pub const fn with_book(self, book: Book) -> Self {
        Self { book, ..self }
    }

    /// Return a validated copy with a different chapter.
    pub const fn with_chapter(self, chapter: u16) -> Result<Self, CoordinateError> {
        Self::new(self.book, chapter, self.verse)
    }

    /// Return a validated copy with a different verse.
    pub const fn with_verse(self, verse: u16) -> Result<Self, CoordinateError> {
        Self::new(self.book, self.chapter, verse)
    }
}

impl fmt::Display for VerseRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}:{}",
            self.book.full_name(),
            self.chapter,
            self.verse
        )
    }
}

impl FromStr for VerseRef {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        ReferenceParser::new().parse_verse(input)
    }
}

/// Error returned when an inclusive range does not have ascending endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RangeOrderError {
    start: VerseRef,
    end: VerseRef,
}

impl RangeOrderError {
    /// Return the proposed inclusive start.
    #[must_use]
    pub const fn start(self) -> VerseRef {
        self.start
    }

    /// Return the proposed inclusive end.
    #[must_use]
    pub const fn end(self) -> VerseRef {
        self.end
    }
}

impl fmt::Display for RangeOrderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "range end {} must come after start {}",
            self.end, self.start
        )
    }
}

impl std::error::Error for RangeOrderError {}

/// A strictly ascending, inclusive range of Bible verses.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VerseRange {
    start: VerseRef,
    end: VerseRef,
}

impl VerseRange {
    /// Construct a validated inclusive range.
    pub const fn new(start: VerseRef, end: VerseRef) -> Result<Self, RangeOrderError> {
        // A const-friendly spelling of `start >= end`.
        let ascending = (start.book as u8) < (end.book as u8)
            || ((start.book as u8) == (end.book as u8)
                && (start.chapter < end.chapter
                    || (start.chapter == end.chapter && start.verse < end.verse)));
        if !ascending {
            return Err(RangeOrderError { start, end });
        }
        Ok(Self { start, end })
    }

    /// Parse a range with an optional explicit book-name language.
    pub fn parse_with_language(input: &str, language: Language) -> Result<Self, ParseError> {
        ReferenceParser::new().parse_range_with_language(input, language)
    }

    /// Parse a range, returning `None` instead of an error for invalid input.
    #[must_use]
    pub fn try_parse(input: &str) -> Option<Self> {
        input.parse().ok()
    }

    /// Parse a range with an explicit language, returning `None` for invalid
    /// input.
    #[must_use]
    pub fn try_parse_with_language(input: &str, language: Language) -> Option<Self> {
        Self::parse_with_language(input, language).ok()
    }

    /// Return the inclusive start.
    #[must_use]
    pub const fn start(self) -> VerseRef {
        self.start
    }

    /// Return the inclusive end.
    #[must_use]
    pub const fn end(self) -> VerseRef {
        self.end
    }
}

impl fmt::Display for VerseRange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let start = self.start;
        let end = self.end;
        if start.book == end.book {
            if start.chapter == end.chapter {
                return write!(
                    formatter,
                    "{} {}:{}-{}",
                    start.book.full_name(),
                    start.chapter,
                    start.verse,
                    end.verse
                );
            }
            return write!(
                formatter,
                "{} {}:{}-{}:{}",
                start.book.full_name(),
                start.chapter,
                start.verse,
                end.chapter,
                end.verse
            );
        }
        write!(formatter, "{start}-{end}")
    }
}

impl FromStr for VerseRange {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        ReferenceParser::new().parse_range(input)
    }
}

/// A single verse or a contiguous inclusive verse range.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Reference {
    /// A single verse.
    Verse(VerseRef),
    /// A contiguous inclusive range.
    Range(VerseRange),
}

impl Reference {
    /// Parse a reference with an explicit book-name language.
    pub fn parse_with_language(input: &str, language: Language) -> Result<Self, ParseError> {
        ReferenceParser::new().parse_with_language(input, language)
    }

    /// Parse a reference, returning `None` instead of an error for invalid
    /// input.
    #[must_use]
    pub fn try_parse(input: &str) -> Option<Self> {
        input.parse().ok()
    }

    /// Parse a reference with an explicit language, returning `None` for
    /// invalid input.
    #[must_use]
    pub fn try_parse_with_language(input: &str, language: Language) -> Option<Self> {
        Self::parse_with_language(input, language).ok()
    }

    /// Return the first verse represented by this value.
    #[must_use]
    pub const fn start(self) -> VerseRef {
        match self {
            Self::Verse(verse) => verse,
            Self::Range(range) => range.start,
        }
    }

    /// Return the last verse represented by this value.
    #[must_use]
    pub const fn end(self) -> VerseRef {
        match self {
            Self::Verse(verse) => verse,
            Self::Range(range) => range.end,
        }
    }

    /// Return the inner verse when this is a single-verse reference.
    #[must_use]
    pub const fn as_verse(self) -> Option<VerseRef> {
        match self {
            Self::Verse(verse) => Some(verse),
            Self::Range(_) => None,
        }
    }

    /// Return the inner range when this is a range reference.
    #[must_use]
    pub const fn as_range(self) -> Option<VerseRange> {
        match self {
            Self::Verse(_) => None,
            Self::Range(range) => Some(range),
        }
    }
}

impl From<VerseRef> for Reference {
    fn from(value: VerseRef) -> Self {
        Self::Verse(value)
    }
}

impl From<VerseRange> for Reference {
    fn from(value: VerseRange) -> Self {
        Self::Range(value)
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Verse(verse) => verse.fmt(formatter),
            Self::Range(range) => range.fmt(formatter),
        }
    }
}

impl FromStr for Reference {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        ReferenceParser::new().parse(input)
    }
}

#[cfg(test)]
#[path = "../tests/unit/reference.rs"]
mod tests;
