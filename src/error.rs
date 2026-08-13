//! Typed parsing errors and stable machine-readable error codes.

use core::fmt;

/// Stable classification for failures produced by the reference and passage parsers.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// A fallback classification for a parse failure without a more specific kind.
    Unknown,
    /// The input contains no reference text.
    EmptyReference,
    /// The input does not match the requested grammar.
    PatternMismatch,
    /// No known alias matched the book token.
    UnknownBook,
    /// A numeric token was not an integer.
    InvalidNumericToken,
    /// A chapter or verse number was zero.
    NonPositiveNumericToken,
    /// A chapter or verse number exceeded the package sanity limit.
    NumericTokenOutOfRange,
    /// No text remained in a required book token.
    EmptyBookToken,
    /// More than one distinct book matched under a rejecting policy.
    AmbiguousBook,
    /// The requested language has no built-in or custom aliases.
    UnsupportedLanguage,
    /// A same-book range does not move forward.
    SameBookRangeNotAscending,
    /// A cross-book range does not move forward in this crate's book order.
    CrossBookRangeNotAscending,
    /// A required chapter or verse number is absent.
    MissingNumericToken,
}

impl ParseErrorKind {
    /// Return the stable snake-case code used by the Dart package.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::EmptyReference => "empty_reference",
            Self::PatternMismatch => "pattern_mismatch",
            Self::UnknownBook => "unknown_book",
            Self::InvalidNumericToken => "invalid_numeric_token",
            Self::NonPositiveNumericToken => "non_positive_numeric_token",
            Self::NumericTokenOutOfRange => "numeric_token_out_of_range",
            Self::EmptyBookToken => "empty_book_token",
            Self::AmbiguousBook => "ambiguous_book",
            Self::UnsupportedLanguage => "unsupported_language",
            Self::SameBookRangeNotAscending => "same_book_range_not_ascending",
            Self::CrossBookRangeNotAscending => "cross_book_range_not_ascending",
            Self::MissingNumericToken => "missing_numeric_token",
        }
    }
}

/// An owned, structured parse failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    kind: ParseErrorKind,
    details: String,
}

impl ParseError {
    /// Construct a parse error with a stable classification and diagnostic.
    #[must_use]
    pub fn new(kind: ParseErrorKind, details: impl Into<String>) -> Self {
        Self {
            kind,
            details: details.into(),
        }
    }

    /// Return the typed error classification.
    #[must_use]
    pub const fn kind(&self) -> ParseErrorKind {
        self.kind
    }

    /// Return the stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// Return a human-readable diagnostic with input-specific context.
    #[must_use]
    pub fn details(&self) -> &str {
        &self.details
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.details)
    }
}

impl std::error::Error for ParseError {}
