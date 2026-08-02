use super::*;

#[test]
fn non_throwing_helpers_preserve_strict_reference_shapes() {
    assert_eq!(
        VerseRef::try_parse("John 3:16"),
        VerseRef::new(Book::John, 3, 16).ok()
    );
    assert_eq!(VerseRef::try_parse("John 3:16-17"), None);

    assert_eq!(
        VerseRange::try_parse("John 3:16-17").map(|range| range.to_string()),
        Some("John 3:16-17".to_owned())
    );
    assert_eq!(VerseRange::try_parse("John 3:16"), None);

    assert_eq!(
        Reference::try_parse_with_language("Juan 3:16", Language::Spanish)
            .map(|reference| reference.start().book()),
        Some(Book::John)
    );
    assert_eq!(Reference::try_parse("not a reference"), None);
}
