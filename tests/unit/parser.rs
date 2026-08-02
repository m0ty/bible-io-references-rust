use super::*;

#[test]
fn parses_verse_and_range_shapes() {
    let parser = ReferenceParser::new();
    assert_eq!(
        parser.parse("John 3:16").unwrap(),
        Reference::Verse(VerseRef::new(Book::John, 3, 16).unwrap())
    );
    assert_eq!(
        parser.parse("John 3.16-4.1").unwrap().to_string(),
        "John 3:16-4:1"
    );
    assert_eq!(
        parser.parse("John 3:16-Acts 1:2").unwrap().to_string(),
        "John 3:16-Acts 1:2"
    );
}

#[test]
fn supports_unicode_and_adjacent_coordinates() {
    let parser = ReferenceParser::new();
    assert_eq!(
        parser
            .parse("\u{200f}John\u{0663}\u{ff1a}\u{0661}\u{0666}")
            .unwrap()
            .to_string(),
        "John 3:16"
    );
    assert_eq!(
        parser.parse("1John1:1-2John1:2").unwrap().to_string(),
        "1 John 1:1-2 John 1:2"
    );
}

#[test]
fn handles_language_detection_and_collisions() {
    let parser = ReferenceParser::new();
    let parsed = parser.parse_detailed("Juan 3:16").unwrap();
    assert_eq!(parsed.value().start().book(), Book::John);
    assert_eq!(
        parsed.metadata().detected_language(),
        Some(Language::Spanish)
    );
    assert_eq!(parser.parse("jn 1:1").unwrap().start().book(), Book::Jonah);

    let collision = auto_language_collisions().get("jn").unwrap();
    assert!(collision.contains(&Book::Jonah));
    assert!(collision.contains(&Book::John));
}

#[test]
fn configurable_aliases_and_rejection_are_deterministic() {
    let parser = ReferenceParser::builder()
        .language_alias(Language::Spanish, "shared", Book::John)
        .language_alias(Language::French, "shared", Book::Jonah)
        .preferred_languages([Language::French])
        .build()
        .unwrap();
    assert_eq!(
        parser.parse("shared 1:1").unwrap().start().book(),
        Book::Jonah
    );

    let rejecting = ReferenceParser::builder()
        .language_alias(Language::Spanish, "shared", Book::John)
        .language_alias(Language::French, "shared", Book::Jonah)
        .ambiguity_policy(AmbiguityPolicy::Reject)
        .build()
        .unwrap();
    assert_eq!(
        rejecting.parse("shared 1:1").unwrap_err().kind(),
        ParseErrorKind::AmbiguousBook
    );

    let custom_override = ReferenceParser::builder()
        .language_alias(Language::Greek, "jn", Book::John)
        .ambiguity_policy(AmbiguityPolicy::Reject)
        .build()
        .unwrap();
    let parsed = custom_override.parse_detailed("jn 3:16").unwrap();
    assert_eq!(parsed.value().start().book(), Book::John);
    assert_eq!(parsed.metadata().detected_language(), Some(Language::Greek));

    let global_parser = ReferenceParser::builder()
        .alias("Favorite", Book::John)
        .build()
        .unwrap();
    let global = global_parser.parse_detailed("favorite 1:1").unwrap();
    assert_eq!(global.metadata().detected_language(), None);
    assert_eq!(
        global.metadata().book_matches()[0].selected().alias(),
        "Favorite"
    );
    assert_eq!(
        global_parser.aliases().collect::<Vec<_>>(),
        vec![("Favorite", Book::John)]
    );
    assert_eq!(
        global_parser
            .parse_with_language("favorite 1:1", Language::Greek)
            .unwrap_err()
            .kind(),
        ParseErrorKind::UnsupportedLanguage
    );

    let normalized_collision = ReferenceParser::builder()
        .alias("Shared", Book::Acts)
        .alias("shared.", Book::John)
        .ambiguity_policy(AmbiguityPolicy::Reject)
        .build()
        .unwrap();
    assert_eq!(
        normalized_collision.parse("shared 1:1").unwrap_err().kind(),
        ParseErrorKind::AmbiguousBook
    );

    let unsupported_language_order = ReferenceParser::builder()
        .language_alias(Language::Vietnamese, "same", Book::John)
        .language_alias(Language::Greek, "same", Book::John)
        .build()
        .unwrap();
    let parsed = unsupported_language_order
        .parse_detailed("same 1:1")
        .unwrap();
    assert_eq!(
        parsed.metadata().detected_language(),
        Some(Language::Vietnamese)
    );
    assert_eq!(
        unsupported_language_order
            .aliases_by_language()
            .collect::<Vec<_>>(),
        vec![
            (Language::Vietnamese, "same", Book::John),
            (Language::Greek, "same", Book::John),
        ]
    );

    let priorities = ReferenceParser::builder()
        .preferred_languages([Language::French, Language::Spanish, Language::French])
        .build()
        .unwrap();
    assert_eq!(
        priorities.preferred_languages(),
        &[Language::French, Language::Spanish]
    );
    assert_eq!(
        ReferenceParser::builder()
            .preferred_languages([Language::Auto])
            .build()
            .unwrap_err()
            .kind(),
        ParseErrorKind::UnsupportedLanguage
    );
}

#[test]
fn subtype_helpers_return_metadata_and_options() {
    let parser = ReferenceParser::new();
    let verse = parser.parse_verse_detailed("Juan 3:16").unwrap();
    assert_eq!(verse.value().book(), Book::John);
    assert_eq!(
        verse.metadata().detected_language(),
        Some(Language::Spanish)
    );

    let range = parser
        .parse_range_detailed_with_language("Juan 3:16-17", Language::Spanish)
        .unwrap();
    assert_eq!(range.value().start().book(), Book::John);
    assert_eq!(parser.try_parse_verse("John 3:16").unwrap().verse(), 16);
    assert!(parser.try_parse_verse("John 3:16-17").is_none());
    assert!(parser.try_parse_range("John 3:16-17").is_some());
    assert!(parser.try_parse_range("John 3:16").is_none());
}

#[test]
fn validates_coordinates_and_order() {
    let parser = ReferenceParser::new();
    assert_eq!(
        parser.parse("John 0:1").unwrap_err().kind(),
        ParseErrorKind::NonPositiveNumericToken
    );
    assert_eq!(
        parser.parse("John 1000:1").unwrap_err().kind(),
        ParseErrorKind::NumericTokenOutOfRange
    );
    assert_eq!(
        parser.parse("John 3:17-16").unwrap_err().kind(),
        ParseErrorKind::SameBookRangeNotAscending
    );
    assert_eq!(
        parser.parse("Acts 1:2-John 3:16").unwrap_err().kind(),
        ParseErrorKind::CrossBookRangeNotAscending
    );
}

#[test]
fn every_canonical_book_form_parses() {
    let parser = ReferenceParser::new();
    for &book in Book::ALL {
        for alias in [book.full_name(), book.abbreviation()] {
            let input = format!("{alias} 1:1");
            assert_eq!(
                parser.parse(&input).unwrap().start().book(),
                book,
                "failed to round-trip {input}"
            );
        }
    }
}

#[test]
fn every_bundled_localized_alias_parses_explicitly() {
    let parser = ReferenceParser::new();
    for &language in AUTO_LANGUAGE_PRECEDENCE {
        for record in localized_aliases(language).unwrap() {
            for alias in record.all_aliases() {
                let input = format!("{alias} 1:1");
                assert_eq!(
                    parser
                        .parse_with_language(&input, language)
                        .unwrap()
                        .start()
                        .book(),
                    record.book,
                    "failed to parse {input:?} as {language:?}"
                );
            }
        }
    }
}
