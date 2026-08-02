use super::*;

fn verse(book: Book, chapter: u16, number: u16) -> VerseRef {
    VerseRef::new(book, chapter, number).unwrap()
}

fn range(start: VerseRef, end: VerseRef) -> VerseRange {
    VerseRange::new(start, end).unwrap()
}

#[test]
fn every_book_identifier_round_trips_strictly() {
    for &book in Book::ALL {
        assert_eq!(book_from_osis_identifier(book.osis()), Ok(book));
        assert_eq!(book_from_usfm_identifier(book.usfm()), Ok(book));
        assert_eq!(book.osis_identifier(), book.osis());
        assert_eq!(book.usfm_identifier(), book.usfm());
    }

    for input in ["", "john", "John ", "Ps151", "Unknown"] {
        assert_eq!(
            book_from_osis_identifier(input).unwrap_err().kind(),
            IdentifierErrorKind::UnknownBook,
            "{input:?}"
        );
    }
    for input in ["", "jhn", "JHN ", "JOH", "ESG", "XXX"] {
        assert_eq!(
            book_from_usfm_identifier(input).unwrap_err().kind(),
            IdentifierErrorKind::UnknownBook,
            "{input:?}"
        );
    }
}

#[test]
fn serializes_osis_verses_and_full_endpoint_ranges() {
    let single = verse(Book::John, 3, 16);
    let same_book = range(
        verse(Book::SecondCorinthians, 6, 14),
        verse(Book::SecondCorinthians, 7, 1),
    );
    let cross_book = range(verse(Book::John, 21, 25), verse(Book::Acts, 1, 2));

    assert_eq!(single.osis_identifier(), "John.3.16");
    assert_eq!(same_book.osis_identifier(), "2Cor.6.14-2Cor.7.1");
    assert_eq!(cross_book.osis_identifier(), "John.21.25-Acts.1.2");
}

#[test]
fn parses_and_round_trips_osis_reference_shapes() {
    for input in [
        "John.3.16",
        "John.3.16-John.3.17",
        "John.3.16-John.4.1",
        "John.21.25-Acts.1.2",
    ] {
        let reference = reference_from_osis_identifier(input).unwrap();
        assert_eq!(reference.osis_identifier(), input);
    }

    for &book in Book::ALL {
        let input = format!("{}.1.1", book.osis());
        assert_eq!(
            reference_from_osis_identifier(&input),
            Ok(Reference::Verse(verse(book, 1, 1)))
        );
    }
}

#[test]
fn rejects_non_osis_reference_identifiers() {
    for input in [
        "",
        "John",
        "John.3",
        "John.3.16-17",
        "john.3.16",
        "Unknown.3.16",
        "John.0.16",
        "John.3.0",
        "John.1000.1",
        "John.1.1000",
        "John.4.1-John.3.16",
        "Acts.1.2-John.21.25",
        "John.3.16-John.3.16",
        " John.3.16",
    ] {
        assert!(reference_from_osis_identifier(input).is_err(), "{input:?}");
    }
}

#[test]
fn serializes_standard_usfm_ranges_and_cross_book_extension() {
    let single = verse(Book::John, 3, 16);
    let same_chapter = range(verse(Book::John, 3, 16), verse(Book::John, 3, 17));
    let cross_chapter = range(verse(Book::John, 3, 16), verse(Book::John, 4, 1));
    let cross_book = range(verse(Book::John, 21, 25), verse(Book::Acts, 1, 2));

    assert_eq!(single.usfm_identifier(), "JHN 3:16");
    assert_eq!(same_chapter.usfm_identifier(), "JHN 3:16-17");
    assert_eq!(cross_chapter.usfm_identifier(), "JHN 3:16-4:1");
    assert_eq!(cross_book.usfm_identifier(), "JHN-ACT 21:25-1:2");
}

#[test]
fn parses_and_round_trips_usfm_reference_shapes() {
    for input in [
        "JHN 3:16",
        "JHN 3:16-17",
        "JHN 3:16-4:1",
        "JHN-ACT 21:25-1:2",
    ] {
        let reference = reference_from_usfm_identifier(input).unwrap();
        assert_eq!(reference.usfm_identifier(), input);
    }

    for &book in Book::ALL {
        let input = format!("{} 1:1", book.usfm());
        assert_eq!(
            reference_from_usfm_identifier(&input),
            Ok(Reference::Verse(verse(book, 1, 1)))
        );
    }
}

#[test]
fn rejects_non_usfm_reference_identifiers() {
    for input in [
        "",
        "JHN",
        "JHN 3",
        "JHN.3.16",
        "jhn 3:16",
        "XXX 3:16",
        "JHN 0:16",
        "JHN 3:0",
        "JHN 1000:1",
        "JHN 1:1000",
        "JHN 4:1-3:16",
        "ACT-JHN 1:2-21:25",
        "JHN 3:16-16",
        " JHN 3:16",
    ] {
        assert!(reference_from_usfm_identifier(input).is_err(), "{input:?}");
    }
}

#[test]
fn parse_errors_are_typed_and_retain_the_input() {
    let syntax = reference_from_usfm_identifier("jhn 3:16").unwrap_err();
    assert_eq!(syntax.format(), IdentifierFormat::Usfm);
    assert_eq!(syntax.kind(), IdentifierErrorKind::InvalidSyntax);
    assert_eq!(syntax.input(), "jhn 3:16");

    let book = reference_from_osis_identifier("Unknown.3.16").unwrap_err();
    assert_eq!(book.kind(), IdentifierErrorKind::UnknownBook);

    let malformed = reference_from_osis_identifier("Unknown.1.1-bad").unwrap_err();
    assert_eq!(malformed.kind(), IdentifierErrorKind::InvalidSyntax);
    let malformed = reference_from_usfm_identifier("XXX nope").unwrap_err();
    assert_eq!(malformed.kind(), IdentifierErrorKind::InvalidSyntax);

    let chapter = reference_from_osis_identifier("John.1000.16").unwrap_err();
    assert_eq!(chapter.kind(), IdentifierErrorKind::InvalidChapter);

    let verse = reference_from_usfm_identifier("JHN 3:0").unwrap_err();
    assert_eq!(verse.kind(), IdentifierErrorKind::InvalidVerse);

    let order = reference_from_osis_identifier("John.3.16-John.3.16").unwrap_err();
    assert_eq!(order.kind(), IdentifierErrorKind::RangeNotAscending);
}

#[test]
fn accepts_dart_compatible_leading_zero_coordinates() {
    let osis = reference_from_osis_identifier("John.003.016").unwrap();
    let usfm = reference_from_usfm_identifier("JHN 003:016").unwrap();
    assert_eq!(osis.osis_identifier(), "John.3.16");
    assert_eq!(usfm.usfm_identifier(), "JHN 3:16");
}

#[test]
fn serializes_passages_in_osis_and_compact_usfm_forms() {
    let book = BookPassage::new(Book::John);
    let chapters = ChapterPassage::range(Book::John, 3, 4).unwrap();
    assert_eq!(book.osis_identifier(), "John");
    assert_eq!(book.usfm_identifier(), "JHN");
    assert_eq!(chapters.osis_identifier(), "John.3-John.4");
    assert_eq!(chapters.usfm_identifier(), "JHN 3-4");

    let selections = VersePassage::new([
        Reference::Verse(verse(Book::John, 3, 16)),
        Reference::Range(range(verse(Book::John, 3, 18), verse(Book::John, 3, 20))),
        Reference::Range(range(verse(Book::John, 4, 1), verse(Book::John, 4, 2))),
    ])
    .unwrap();
    assert_eq!(
        selections.osis_identifier(),
        "John.3.16 John.3.18-John.3.20 John.4.1-John.4.2"
    );
    assert_eq!(selections.usfm_identifier(), "JHN 3:16,18-20,JHN 4:1-2");
}

#[test]
fn serializes_context_changing_verse_selections() {
    let selections = VersePassage::new([
        Reference::Verse(verse(Book::John, 3, 16)),
        Reference::Verse(verse(Book::Acts, 2, 1)),
    ])
    .unwrap();

    assert_eq!(selections.osis_identifier(), "John.3.16 Acts.2.1");
    assert_eq!(selections.usfm_identifier(), "JHN 3:16,ACT 2:1");
}

#[test]
fn passage_sequences_use_format_specific_separators() {
    let sequence = PassageSequence::new([
        Passage::Book(BookPassage::new(Book::John)),
        Passage::Chapter(ChapterPassage::single(Book::John, 3).unwrap()),
        Passage::Verses(
            VersePassage::new([Reference::Range(range(
                verse(Book::Acts, 2, 1),
                verse(Book::Acts, 2, 4),
            ))])
            .unwrap(),
        ),
    ])
    .unwrap();

    assert_eq!(sequence.osis_identifier(), "John John.3 Acts.2.1-Acts.2.4");
    assert_eq!(sequence.usfm_identifier(), "JHN; JHN 3; ACT 2:1-4");
}
