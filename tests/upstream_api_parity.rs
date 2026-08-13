#![forbid(unsafe_code)]
#![warn(clippy::all)]

use bible_io_references::{
    Book, ParseError, ParseErrorKind, VerseRange, VerseRef, verse_range_ref_from_str,
    verse_ref_from_str,
};

#[test]
fn unknown_parse_error_kind_has_the_stable_fallback_code() {
    let error = ParseError::new(ParseErrorKind::Unknown, "unclassified parse failure");

    assert_eq!(ParseErrorKind::Unknown.code(), "unknown");
    assert_eq!(error.code(), "unknown");
}

#[test]
fn verse_range_copy_helpers_revalidate_order() {
    let start = VerseRef::new(Book::John, 3, 16).unwrap();
    let end = VerseRef::new(Book::John, 3, 18).unwrap();
    let range = VerseRange::new(start, end).unwrap();

    let earlier_start = VerseRef::new(Book::John, 3, 15).unwrap();
    assert_eq!(
        range.with_start(earlier_start).unwrap().start(),
        earlier_start
    );

    let later_end = VerseRef::new(Book::John, 4, 1).unwrap();
    assert_eq!(range.with_end(later_end).unwrap().end(), later_end);

    assert!(range.with_start(end).is_err());
    assert!(range.with_end(start).is_err());
}

#[test]
fn named_compatibility_helpers_use_normalized_strict_parsing() {
    let verse = verse_ref_from_str("\u{200f}John \u{0663}\u{ff1a}\u{0661}\u{0666}").unwrap();
    assert_eq!(verse, VerseRef::new(Book::John, 3, 16).unwrap());

    let range = verse_range_ref_from_str("John 3:16\u{2013}18").unwrap();
    assert_eq!(range.start(), verse);
    assert_eq!(range.end(), VerseRef::new(Book::John, 3, 18).unwrap());

    assert!(verse_ref_from_str("John 3:16-18").is_err());
    assert!(verse_range_ref_from_str("John 3:16").is_err());
}
