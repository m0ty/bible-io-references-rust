use super::*;

fn verse(book: Book, chapter: u16, verse: u16) -> VerseRef {
    VerseRef::new(book, chapter, verse).unwrap()
}

fn range(start: VerseRef, end: VerseRef) -> VerseRange {
    VerseRange::new(start, end).unwrap()
}

#[test]
fn defaults_and_builder_methods_are_deterministic() {
    let formatter = ReferenceFormatter::new();
    assert_eq!(formatter.language(), Language::English);
    assert_eq!(formatter.book_name_style(), BookNameStyle::Long);
    assert!(formatter.compact_ranges());
    assert_eq!(formatter.format_book(Book::John).to_string(), "John");

    let configured = formatter
        .with_language(Language::Spanish)
        .with_book_name_style(BookNameStyle::Short)
        .with_compact_ranges(false);
    assert_eq!(configured.language(), Language::Spanish);
    assert_eq!(configured.book_name_style(), BookNameStyle::Short);
    assert!(!configured.compact_ranges());
    assert_eq!(configured.format_book(Book::John).to_string(), "Jn");
}

#[test]
fn formats_localized_long_and_short_names_with_english_fallback() {
    let john = verse(Book::John, 3, 16);

    assert_eq!(
        ReferenceFormatter::for_language(Language::Spanish)
            .format(john)
            .to_string(),
        "Juan 3:16"
    );
    assert_eq!(
        ReferenceFormatter::for_language(Language::Spanish)
            .with_book_name_style(BookNameStyle::Short)
            .format(john)
            .to_string(),
        "Jn 3:16"
    );
    assert_eq!(
        ReferenceFormatter::for_language(Language::Greek)
            .format(john)
            .to_string(),
        "John 3:16"
    );
    assert_eq!(
        ReferenceFormatter::for_language(Language::Auto)
            .with_book_name_style(BookNameStyle::Short)
            .format(john)
            .to_string(),
        "jo 3:16"
    );
}

#[test]
fn compacts_and_expands_ranges_like_the_dart_formatter() {
    let formatter = ReferenceFormatter::for_language(Language::Spanish);
    let start = verse(Book::John, 3, 16);

    assert_eq!(
        formatter
            .format(range(start, verse(Book::John, 3, 18)))
            .to_string(),
        "Juan 3:16-18"
    );
    assert_eq!(
        formatter
            .format(range(start, verse(Book::John, 4, 1)))
            .to_string(),
        "Juan 3:16-4:1"
    );
    assert_eq!(
        formatter
            .with_compact_ranges(false)
            .format(range(start, verse(Book::John, 3, 18)))
            .to_string(),
        "Juan 3:16-Juan 3:18"
    );
    assert_eq!(
        formatter
            .format(range(start, verse(Book::Acts, 1, 2)))
            .to_string(),
        "Juan 3:16-Hechos 1:2"
    );
}

#[test]
fn formats_each_passage_shape() {
    let formatter = ReferenceFormatter::for_language(Language::Spanish);
    let book = BookPassage::new(Book::John);
    let chapters = ChapterPassage::new(Book::John, 3, Some(4)).unwrap();
    let selections = VersePassage::new([
        Reference::from(verse(Book::John, 3, 16)),
        Reference::from(range(verse(Book::John, 3, 18), verse(Book::John, 3, 20))),
        Reference::from(verse(Book::John, 4, 1)),
    ])
    .unwrap();

    assert_eq!(formatter.format_book_passage(book).to_string(), "Juan");
    assert_eq!(
        formatter.format_chapter_passage(chapters).to_string(),
        "Juan 3-4"
    );
    assert_eq!(
        formatter.format_verse_passage(&selections).to_string(),
        "Juan 3:16,18-20,4:1"
    );
    assert_eq!(
        formatter
            .with_compact_ranges(false)
            .format_verse_passage(&selections)
            .to_string(),
        "Juan 3:16,Juan 3:18-Juan 3:20,Juan 4:1"
    );
}

#[test]
fn formats_context_changing_verse_selections() {
    let selections = VersePassage::new([
        Reference::from(verse(Book::John, 3, 16)),
        Reference::from(verse(Book::Acts, 2, 1)),
    ])
    .unwrap();

    assert_eq!(
        ReferenceFormatter::for_language(Language::Spanish)
            .format_verse_passage(&selections)
            .to_string(),
        "Juan 3:16,Hechos 2:1"
    );
}

#[test]
fn localizes_every_expression_in_a_sequence() {
    let sequence = PassageSequence::new([
        Passage::Book(BookPassage::new(Book::John)),
        Passage::Chapter(ChapterPassage::new(Book::John, 3, Some(4)).unwrap()),
        Passage::Verses(
            VersePassage::new([
                Reference::from(verse(Book::John, 3, 16)),
                Reference::from(range(verse(Book::John, 3, 18), verse(Book::John, 3, 20))),
            ])
            .unwrap(),
        ),
        Passage::Verses(
            VersePassage::new([Reference::from(range(
                verse(Book::Acts, 2, 1),
                verse(Book::Acts, 2, 4),
            ))])
            .unwrap(),
        ),
    ])
    .unwrap();
    let passage = Passage::Sequence(sequence);

    assert_eq!(
        ReferenceFormatter::for_language(Language::Spanish)
            .format_passage(&passage)
            .to_string(),
        "Juan; Juan 3-4; Juan 3:16,18-20; Hechos 2:1-4"
    );
}
