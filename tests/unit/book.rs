use std::collections::HashSet;

use super::*;

#[test]
fn every_representation_is_unique_and_round_trips() {
    assert_eq!(Book::ALL.len(), 83);
    assert_eq!(Book::ALL.first(), Some(&Book::Genesis));
    assert_eq!(Book::ALL.get(65), Some(&Book::Revelation));
    assert_eq!(Book::ALL.last(), Some(&Book::FourthMaccabees));

    let mut abbreviations = HashSet::new();
    let mut names = HashSet::new();
    let mut osis_identifiers = HashSet::new();
    let mut usfm_identifiers = HashSet::new();

    for &book in Book::ALL {
        assert!(abbreviations.insert(book.abbreviation()));
        assert!(names.insert(book.full_name()));
        assert!(osis_identifiers.insert(book.osis()));
        assert!(usfm_identifiers.insert(book.usfm()));

        assert_eq!(Book::from_abbreviation(book.abbreviation()), Some(book));
        assert_eq!(
            Book::from_abbreviation(&book.abbreviation().to_ascii_uppercase()),
            Some(book)
        );
        assert_eq!(Book::from_osis(book.osis()), Some(book));
        assert_eq!(Book::from_usfm(book.usfm()), Some(book));
    }
}

#[test]
fn identifier_lookups_are_strict() {
    assert_eq!(Book::from_abbreviation("gN"), Some(Book::Genesis));
    assert_eq!(Book::from_abbreviation(" gn"), None);
    assert_eq!(Book::from_abbreviation("Gen"), None);

    assert_eq!(Book::from_osis("John"), Some(Book::John));
    assert_eq!(Book::from_osis("john"), None);
    assert_eq!(Book::from_osis("John "), None);

    assert_eq!(Book::from_usfm("JHN"), Some(Book::John));
    assert_eq!(Book::from_usfm("jhn"), None);
    assert_eq!(Book::from_usfm("JHN "), None);
}

#[test]
fn parses_supported_forms_and_displays_the_full_name() {
    assert_eq!("1sm".parse(), Ok(Book::FirstSamuel));
    assert_eq!("SONG OF SOLOMON".parse(), Ok(Book::SongOfSolomon));
    assert_eq!("2Cor".parse(), Ok(Book::SecondCorinthians));
    assert_eq!("2CO".parse(), Ok(Book::SecondCorinthians));
    assert_eq!(Book::SongOfSolomon.to_string(), "Song of Solomon");
}

#[test]
fn compact_codes_take_precedence_in_general_parsing() {
    assert_eq!("jud".parse(), Ok(Book::Judges));
    assert_eq!("JUD".parse(), Ok(Book::Judges));
    assert_eq!(Book::from_usfm("JUD"), Some(Book::Jude));
}

#[test]
fn parse_errors_retain_the_rejected_input() {
    let error = " John ".parse::<Book>().unwrap_err();
    assert_eq!(error.input(), " John ");
    assert_eq!(error.to_string(), "unrecognized Bible book: \" John \"");
}
