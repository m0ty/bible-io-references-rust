#![forbid(unsafe_code)]
#![warn(clippy::all)]

use std::sync::Arc;

use bible_io_references::{
    Book, Language, MachineIdentifiers, Passage, Reference, ReferenceExtractor, ReferenceFormatter,
    ReferenceParser, VersePassage, VerseRef, auto_language_collisions, localized_books,
};

#[test]
fn release_1_1_surface_is_usable_from_an_external_crate() {
    let reference = Reference::parse_with_language("Juan 3:16", Language::Spanish).unwrap();
    assert_eq!(reference.start().book(), Book::John);

    let passage: Passage = "John 3:16,18-20; Acts 2".parse().unwrap();
    assert_eq!(
        passage.osis_identifier(),
        "John.3.16 John.3.18-John.3.20 Acts.2"
    );
    assert_eq!(
        ReferenceFormatter::for_language(Language::Spanish)
            .format_passage(&passage)
            .to_string(),
        "Juan 3:16,18-20; Hechos 2"
    );

    let found = ReferenceExtractor::new().extract("Read John 3:16 today.");
    assert_eq!(found[0].source_text(), "John 3:16");
}

#[test]
fn public_registry_and_metadata_preserve_alias_provenance() {
    assert_eq!(localized_books(Language::Spanish).unwrap().len(), 83);
    assert!(auto_language_collisions()["jn"].contains(&Book::John));

    let parser = ReferenceParser::builder()
        .alias("Favorite", Book::John)
        .build()
        .unwrap();
    let parsed = parser.parse_detailed("favorite 3:16").unwrap();
    assert_eq!(
        parsed.metadata().book_matches()[0].selected().alias(),
        "Favorite"
    );
}

#[test]
fn mixed_book_selections_and_parsers_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ReferenceParser>();
    assert_send_sync::<Passage>();

    let selections = VersePassage::new([
        VerseRef::new(Book::John, 3, 16).unwrap().into(),
        VerseRef::new(Book::Acts, 2, 1).unwrap().into(),
    ])
    .unwrap();
    assert_eq!(selections.usfm_identifier(), "JHN 3:16,ACT 2:1");

    let parser = Arc::new(ReferenceParser::new());
    let workers = (0..4)
        .map(|_| {
            let parser = Arc::clone(&parser);
            std::thread::spawn(move || parser.parse("John 3:16").unwrap())
        })
        .collect::<Vec<_>>();
    for worker in workers {
        assert_eq!(worker.join().unwrap().start().book(), Book::John);
    }
}

#[cfg(feature = "serde")]
#[test]
fn metadata_serializes_with_dart_field_names() {
    let parsed = ReferenceParser::new().parse_detailed("Juan 3:16").unwrap();
    let value = serde_json::to_value(parsed.metadata()).unwrap();
    assert_eq!(value["normalizedInput"], "Juan 3:16");
    assert_eq!(value["detectedLanguage"], "es");
    assert_eq!(value["bookMatches"][0]["selected"]["alias"], "Juan");
}
