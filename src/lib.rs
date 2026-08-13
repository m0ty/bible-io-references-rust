#![forbid(unsafe_code)]
#![warn(clippy::all, missing_docs)]
#![doc = include_str!("../README.md")]

pub mod book;
pub mod error;
pub mod extractor;
pub mod formatter;
pub mod identifiers;
pub mod language;
mod language_data;
pub mod normalize;
pub mod parser;
pub mod passage;
pub mod reference;
#[cfg(feature = "serde")]
mod serde_impl;

pub use book::{Book, ParseBookError};
pub use error::{ParseError, ParseErrorKind};
pub use extractor::{
    DEFAULT_MAX_LOOKAHEAD, DEFAULT_MAX_LOOKBEHIND, ExtractorConfigError, ExtractorWindow,
    PassageMatch, ReferenceExtractor, ReferenceExtractorBuilder,
};
pub use formatter::{
    BookNameStyle, FormattedBook, FormattedPassage, FormattedReference, ReferenceFormatter,
};
pub use identifiers::{
    IdentifierError, IdentifierErrorKind, IdentifierFormat, MachineIdentifiers,
    book_from_osis_identifier, book_from_usfm_identifier, reference_from_osis_identifier,
    reference_from_usfm_identifier,
};
pub use language::{Language, ParseLanguageError};
pub use language_data::{LocalizedBook, localized_books, localized_name, long_name, short_name};
pub use parser::{
    AUTO_LANGUAGE_PRECEDENCE, AmbiguityPolicy, BookCandidate, BookMatch, ParseMetadata, Parsed,
    ParserBuilder, ReferenceParser, auto_language_collisions,
};
pub use passage::{
    BookPassage, ChapterPassage, DEFAULT_SINGLE_CHAPTER_BOOKS, Passage, PassageBuildError,
    PassageParser, PassageSequence, VersePassage,
};
pub use reference::{
    Coordinate, CoordinateError, MAX_CHAPTER_NUMBER, MAX_VERSE_NUMBER, RangeOrderError, Reference,
    VerseRange, VerseRef,
};

/// Compatibility name for callers familiar with the Dart package.
pub type VerseRangeRef = VerseRange;

/// Parse a verse reference using the crate's normalized parser.
///
/// This compatibility helper mirrors the Dart package's named parsing API.
pub fn verse_ref_from_str(input: &str) -> Result<VerseRef, ParseError> {
    input.parse()
}

/// Parse an inclusive verse range using the crate's normalized parser.
///
/// This compatibility helper mirrors the Dart package's named parsing API.
pub fn verse_range_ref_from_str(input: &str) -> Result<VerseRangeRef, ParseError> {
    input.parse()
}

/// Parse a verse or inclusive verse range using automatic language detection.
pub fn parse_reference(input: &str) -> Result<Reference, ParseError> {
    input.parse()
}
