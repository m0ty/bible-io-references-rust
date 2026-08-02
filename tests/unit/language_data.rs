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
fn formats_localized_names_and_falls_back_to_english() {
    assert_eq!(long_name(Book::John, Language::Spanish), "Juan");
    assert_eq!(short_name(Book::John, Language::Spanish), "Jn");
    assert_eq!(short_name(Book::Genesis, Language::Arabic), "تك");
    assert_eq!(long_name(Book::John, Language::Greek), "John");
    assert_eq!(short_name(Book::John, Language::Auto), "jo");
}
