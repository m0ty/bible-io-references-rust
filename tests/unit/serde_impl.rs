use super::*;
use crate::ReferenceParser;
use serde_json::{json, to_value};

#[test]
fn references_use_dart_compatible_json() {
    let verse: Reference = "John 3:16".parse().unwrap();
    assert_eq!(
        to_value(verse).unwrap(),
        json!({"type": "verse", "book": "jo", "chapter": 3, "verse": 16})
    );

    let range: Reference = "John 3:16-17".parse().unwrap();
    let encoded = to_value(range).unwrap();
    assert_eq!(encoded["type"], "range");
    assert_eq!(encoded["start"]["book"], "jo");
    assert_eq!(serde_json::from_value::<Reference>(encoded).unwrap(), range);
}

#[test]
fn passages_round_trip_with_stable_tags() {
    for input in ["John", "John 3-4", "John 3:16,18-20", "John 3:16; Acts 2"] {
        let passage: Passage = input.parse().unwrap();
        let encoded = to_value(&passage).unwrap();
        assert_eq!(serde_json::from_value::<Passage>(encoded).unwrap(), passage);
    }
}

#[test]
fn metadata_uses_dart_compatible_json_and_preserves_aliases() {
    let parser = ReferenceParser::builder()
        .alias("Favorite", Book::John)
        .build()
        .unwrap();
    let parsed = parser.parse_detailed("favorite 3:16").unwrap();

    assert_eq!(
        to_value(parsed.metadata()).unwrap(),
        json!({
            "normalizedInput": "favorite 3:16",
            "detectedLanguage": null,
            "detectedLanguages": [],
            "bookMatches": [{
                "input": "favorite",
                "selected": {
                    "book": "jo",
                    "alias": "Favorite",
                    "language": null,
                    "custom": true
                },
                "alternatives": []
            }]
        })
    );
}

#[test]
fn mixed_book_verse_passages_round_trip() {
    let passage = Passage::Verses(
        VersePassage::new([
            Reference::Verse(VerseRef::new(Book::John, 3, 16).unwrap()),
            Reference::Verse(VerseRef::new(Book::Acts, 2, 1).unwrap()),
        ])
        .unwrap(),
    );
    let encoded = to_value(&passage).unwrap();
    assert_eq!(serde_json::from_value::<Passage>(encoded).unwrap(), passage);
}

#[test]
fn deserialization_revalidates_invariants() {
    let reversed = json!({
        "type": "range",
        "start": {"type": "verse", "book": "jo", "chapter": 3, "verse": 17},
        "end": {"type": "verse", "book": "jo", "chapter": 3, "verse": 16}
    });
    assert!(serde_json::from_value::<Reference>(reversed).is_err());
    assert!(
        serde_json::from_value::<Passage>(json!({
            "type": "verses",
            "selections": []
        }))
        .is_err()
    );
}

#[test]
fn json_book_lookup_uses_compact_codes_before_machine_identifiers() {
    // Dart JSON is case-insensitive over its compact code. `JUD` therefore
    // means Judges here, even though it is also Jude's strict USFM code.
    assert_eq!(
        serde_json::from_str::<Book>("\"JUD\"").unwrap(),
        Book::Judges
    );
    assert_eq!(serde_json::from_str::<Book>("\"jd\"").unwrap(), Book::Jude);
    assert_eq!(
        serde_json::from_str::<Book>("\" jo \"").unwrap(),
        Book::John
    );
}
