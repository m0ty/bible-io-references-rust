use super::*;
use crate::{AmbiguityPolicy, ParserBuilder};

fn verse(book: Book, chapter: u16, number: u16) -> Reference {
    Reference::Verse(VerseRef::new(book, chapter, number).unwrap())
}

fn range(
    book: Book,
    start_chapter: u16,
    start_verse: u16,
    end_chapter: u16,
    end_verse: u16,
) -> Reference {
    Reference::Range(
        VerseRange::new(
            VerseRef::new(book, start_chapter, start_verse).unwrap(),
            VerseRef::new(book, end_chapter, end_verse).unwrap(),
        )
        .unwrap(),
    )
}

#[test]
fn constructors_preserve_value_invariants() {
    assert_eq!(
        ChapterPassage::single(Book::John, 0),
        Err(PassageBuildError::InvalidChapter { value: 0 })
    );
    assert_eq!(
        ChapterPassage::range(Book::John, 4, 3),
        Err(PassageBuildError::ChapterRangeNotAscending { start: 4, end: 3 })
    );
    assert_eq!(
        VersePassage::new([]),
        Err(PassageBuildError::EmptyVersePassage)
    );
    assert_eq!(
        PassageSequence::new([]),
        Err(PassageBuildError::EmptyPassageSequence)
    );
}

#[test]
fn verse_passages_support_context_changing_selections() {
    let passage = VersePassage::new([
        verse(Book::John, 3, 16),
        verse(Book::Acts, 2, 1),
        Reference::Range(
            VerseRange::new(
                VerseRef::new(Book::Romans, 8, 1).unwrap(),
                VerseRef::new(Book::FirstCorinthians, 1, 2).unwrap(),
            )
            .unwrap(),
        ),
    ])
    .unwrap();

    assert_eq!(
        passage.to_string(),
        "John 3:16,Acts 2:1,Romans 8:1-1 Corinthians 1:2"
    );
}

#[test]
fn parses_books_chapters_and_chapter_ranges() {
    assert_eq!(
        Passage::parse("John"),
        Ok(Passage::Book(BookPassage::new(Book::John)))
    );
    assert_eq!(
        Passage::parse("John 3"),
        Ok(Passage::Chapter(
            ChapterPassage::single(Book::John, 3).unwrap()
        ))
    );
    assert_eq!(
        Passage::parse("John 3-4"),
        Ok(Passage::Chapter(
            ChapterPassage::range(Book::John, 3, 4).unwrap()
        ))
    );
}

#[test]
fn wraps_verses_ranges_and_cross_book_ranges() {
    for input in ["John 3:16", "John 3:16-4:1", "John 3:16-Acts 1:2"] {
        let parsed = Passage::parse(input).unwrap();
        assert!(matches!(parsed, Passage::Verses(_)), "{input}");
        assert_eq!(parsed.to_string(), input);
    }
}

#[test]
fn parses_and_compacts_comma_selections() {
    let passage = Passage::parse("John 3:16,18-20,4:1-5:2").unwrap();
    let Passage::Verses(passage) = passage else {
        panic!("expected verse passage");
    };
    assert_eq!(
        passage.selections(),
        &[
            verse(Book::John, 3, 16),
            range(Book::John, 3, 18, 3, 20),
            range(Book::John, 4, 1, 5, 2),
        ]
    );
    assert_eq!(passage.to_string(), "John 3:16,18-20,4:1-5:2");
}

#[test]
fn parses_semicolon_sequences() {
    let passage = Passage::parse("John 3:16; Acts 2:1-4; Romans 8").unwrap();
    let Passage::Sequence(sequence) = &passage else {
        panic!("expected passage sequence");
    };
    assert_eq!(sequence.passages().len(), 3);
    assert!(matches!(sequence.passages()[0], Passage::Verses(_)));
    assert!(matches!(sequence.passages()[1], Passage::Verses(_)));
    assert!(matches!(sequence.passages()[2], Passage::Chapter(_)));
    assert_eq!(passage.to_string(), "John 3:16; Acts 2:1-4; Romans 8");
}

#[test]
fn defaults_to_exactly_five_single_chapter_shorthands() {
    assert_eq!(DEFAULT_SINGLE_CHAPTER_BOOKS.len(), 5);
    for book in DEFAULT_SINGLE_CHAPTER_BOOKS {
        let input = format!("{} 3", book.full_name());
        let passage = Passage::parse(&input).unwrap();
        assert_eq!(passage.to_string(), format!("{} 1:3", book.full_name()));
    }
    assert!(matches!(Passage::parse("John 3"), Ok(Passage::Chapter(_))));
}

#[test]
fn single_chapter_shorthand_is_configurable() {
    let parser = PassageParser::new().with_single_chapter_books([Book::John]);
    assert_eq!(parser.single_chapter_books(), &[Book::John]);
    assert_eq!(parser.parse("John 3").unwrap().to_string(), "John 1:3");
    assert!(matches!(parser.parse("Jude 3"), Ok(Passage::Chapter(_))));
}

#[test]
fn reuses_reference_parser_configuration_and_metadata() {
    let references = ParserBuilder::default()
        .alias("favorite", Book::John)
        .build()
        .unwrap();
    let parser = PassageParser::from_reference_parser(references);
    let parsed = parser.parse_detailed("favorite 3:16,18").unwrap();

    assert!(matches!(parsed.value(), Passage::Verses(_)));
    assert_eq!(parsed.metadata().book_matches().len(), 1);
    assert!(parsed.metadata().book_matches()[0].selected().is_custom());
    assert_eq!(
        parsed.metadata().book_matches()[0].selected().book(),
        Book::John
    );
}

#[test]
fn supports_explicit_languages_and_aggregates_sequence_metadata() {
    let parsed = PassageParser::new()
        .parse_detailed_with_language("Juan 3:16; Hechos 2; Juan 4", Language::Spanish)
        .unwrap();
    assert_eq!(
        parsed.metadata().normalized_input(),
        "Juan 3:16; Hechos 2; Juan 4"
    );
    assert_eq!(
        parsed.metadata().detected_language(),
        Some(Language::Spanish)
    );
    assert_eq!(
        parsed
            .metadata()
            .book_matches()
            .iter()
            .map(|book_match| book_match.selected().book())
            .collect::<Vec<_>>(),
        [Book::John, Book::Acts, Book::John]
    );
}

#[test]
fn preserves_reference_parser_ambiguity_policy() {
    let references = ParserBuilder::default()
        .language_alias(Language::Spanish, "shared", Book::John)
        .language_alias(Language::French, "shared", Book::Jonah)
        .preferred_languages([Language::French])
        .ambiguity_policy(AmbiguityPolicy::Reject)
        .build()
        .unwrap();
    let error = PassageParser::from_reference_parser(references)
        .parse("shared 2")
        .unwrap_err();
    assert_eq!(error.kind(), ParseErrorKind::AmbiguousBook);
}

#[test]
fn normalizes_unicode_reference_syntax() {
    let passage = Passage::parse(
        "\u{ff2a}\u{ff4f}\u{ff48}\u{ff4e}\u{ff13}\u{ff1a}\u{ff11}\u{ff16}\u{ff0c}\u{ff11}\u{ff18}\u{ff0d}\u{ff12}\u{ff10}\u{ff1b}\u{ff21}\u{ff43}\u{ff54}\u{ff53}\u{ff12}",
    )
    .unwrap();
    assert_eq!(passage.to_string(), "John 3:16,18-20; Acts 2");
}

#[test]
fn rejects_empty_and_descending_expressions() {
    for input in ["", "John 3:16;", "John 3:16,,18"] {
        assert!(Passage::try_parse(input).is_none(), "{input:?}");
    }
    assert_eq!(
        Passage::parse("John 4-3").unwrap_err().kind(),
        ParseErrorKind::SameBookRangeNotAscending
    );
    assert_eq!(
        Passage::parse("John 3:18-16,20").unwrap_err().kind(),
        ParseErrorKind::SameBookRangeNotAscending
    );
}
