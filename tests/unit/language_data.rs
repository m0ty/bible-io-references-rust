use super::*;

#[test]
fn exposes_complete_localized_registry_records() {
    let spanish = localized_books(Language::Spanish).unwrap();
    assert_eq!(spanish.len(), Book::ALL.len());

    let john = spanish
        .iter()
        .copied()
        .find(|record| record.book() == Book::John)
        .unwrap();
    assert!(john.names().contains(&"Juan"));
    assert!(john.abbreviations().contains(&"Jn"));
    assert_eq!(localized_books(Language::English), None);
}

#[test]
fn every_localized_pack_contains_every_book_once() {
    for language in Language::SUPPORTED {
        if matches!(language, Language::Auto | Language::English) {
            continue;
        }

        let entries = aliases(language).expect("supported language must have bundled aliases");
        assert_eq!(entries.len(), Book::ALL.len(), "{language}");

        for &book in Book::ALL {
            let mut matching = entries.iter().filter(|entry| entry.book == book);
            let entry = matching.next().expect("every book must be present");
            assert!(matching.next().is_none(), "{language}: {book:?}");
            assert!(entry.all_aliases().next().is_some());
        }
    }
}

#[test]
fn localized_term_counts_match_dart_1_1_contract() {
    const EXPECTED_COUNTS: &[(Language, usize, usize)] = &[
        (Language::Arabic, 226, 314),
        (Language::Chinese, 335, 166),
        (Language::French, 323, 338),
        (Language::German, 147, 343),
        (Language::Hebrew, 230, 286),
        (Language::Hindi, 180, 218),
        (Language::Indonesian, 145, 430),
        (Language::Korean, 111, 416),
        (Language::Portuguese, 180, 335),
        (Language::Russian, 193, 160),
        (Language::Spanish, 191, 443),
        (Language::Tagalog, 83, 247),
    ];

    let mut total_terms = 0;
    for &(language, expected_names, expected_abbreviations) in EXPECTED_COUNTS {
        let entries = localized_books(language).expect("contract language must have data");
        let name_count = entries
            .iter()
            .map(|entry| entry.names().len())
            .sum::<usize>();
        let abbreviation_count = entries
            .iter()
            .map(|entry| entry.abbreviations().len())
            .sum::<usize>();

        assert_eq!(name_count, expected_names, "{language}: long names");
        assert_eq!(
            abbreviation_count, expected_abbreviations,
            "{language}: abbreviations"
        );
        total_terms += name_count + abbreviation_count;
    }

    assert_eq!(total_terms, 6_040);
}

#[test]
fn registry_cardinalities_match_dart_1_1_contract() {
    assert_eq!(Book::ALL.len(), 83);
    assert_eq!(Language::ALL.len(), 19);
    assert_eq!(Language::SUPPORTED.len(), 14);
}

#[test]
fn formats_localized_names_and_falls_back_to_english() {
    assert_eq!(long_name(Book::John, Language::Spanish), "Juan");
    assert_eq!(short_name(Book::John, Language::Spanish), "Jn");
    assert_eq!(short_name(Book::Genesis, Language::Arabic), "تك");
    assert_eq!(long_name(Book::John, Language::Greek), "John");
    assert_eq!(short_name(Book::John, Language::Auto), "jo");
}
