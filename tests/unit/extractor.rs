use super::*;

fn source_texts(matches: &[PassageMatch]) -> Vec<&str> {
    matches.iter().map(PassageMatch::source_text).collect()
}

#[test]
fn finds_repeated_references_with_exact_byte_ranges() {
    let source = "Read (John 3:16), then Juan 3:16!";
    let matches = ReferenceExtractor::new().extract(source);

    assert_eq!(source_texts(&matches), ["John 3:16", "Juan 3:16"]);
    for passage_match in &matches {
        assert_eq!(&source[passage_match.range()], passage_match.source_text());
        assert!(!passage_match.metadata().book_matches().is_empty());
    }
}

#[test]
fn prefers_complete_lists_and_sequences_over_fragments() {
    let source = "Study John 3:16,18-20; Acts 2:1-4 today.";
    let matches = ReferenceExtractor::new().extract(source);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].source_text(), "John 3:16,18-20; Acts 2:1-4");
    assert_eq!(matches[0].passage().to_string(), matches[0].source_text());
}

#[test]
fn reports_utf8_byte_offsets_for_cjk_and_directional_controls() {
    let reference = "约翰福音3:16";
    let source = format!("请读{reference}谢谢");
    let passage_match = ReferenceExtractor::new().extract(&source).remove(0);
    let expected_start = "请读".len();

    assert_eq!(passage_match.start(), expected_start);
    assert_eq!(passage_match.end(), expected_start + reference.len());
    assert_eq!(passage_match.source_text(), reference);

    let rtl_reference = "Jo\u{200f}hn ٣:١٦";
    let rtl_source = format!("Before \u{200f}{rtl_reference}\u{200f} after");
    let rtl_match = ReferenceExtractor::new().extract(&rtl_source).remove(0);
    assert_eq!(rtl_match.source_text(), rtl_reference);
    assert_eq!(&rtl_source[rtl_match.range()], rtl_reference);
}

#[test]
fn suppresses_common_words_but_keeps_capitalized_books() {
    let prose = "At 12:30, please mark 3:16; the job 2:4 is queued.";
    assert!(ReferenceExtractor::new().extract(prose).is_empty());

    let references = ReferenceExtractor::new().extract("Read Mark 3:16 and Job 2:4.");
    assert_eq!(source_texts(&references), ["Mark 3:16", "Job 2:4"]);
}

#[test]
fn respects_unicode_word_boundaries_without_breaking_cjk_adjacency() {
    assert!(
        ReferenceExtractor::new()
            .extract_with_language("словоИоанна 3:16", Language::Russian)
            .is_empty()
    );
    let joined_suffix = ReferenceExtractor::new().extract("John 3:16éclair");
    assert!(joined_suffix.is_empty(), "{joined_suffix:?}");
    assert_eq!(
        source_texts(
            &ReferenceExtractor::new()
                .extract_with_language("请读约翰福音3:16谢谢", Language::Chinese,)
        ),
        ["约翰福音3:16"]
    );
}

#[test]
fn bare_books_are_opt_in() {
    let source = "Read Mark and Job.";
    assert!(ReferenceExtractor::new().extract(source).is_empty());

    let matches = ReferenceExtractor::new()
        .with_include_bare_books(true)
        .extract(source);
    assert_eq!(source_texts(&matches), ["Mark", "Job"]);
}

#[test]
fn validates_lookaround_configuration() {
    assert!(
        ReferenceExtractor::builder()
            .max_lookbehind(0)
            .build()
            .is_err()
    );
    assert!(
        ReferenceExtractor::builder()
            .max_lookahead(MAX_LOOKAROUND + 1)
            .build()
            .is_err()
    );
    let extractor = ReferenceExtractor::builder()
        .max_lookbehind(MIN_LOOKAROUND)
        .max_lookahead(MAX_LOOKAROUND)
        .build()
        .unwrap();
    assert_eq!(extractor.max_lookbehind(), MIN_LOOKAROUND);
    assert_eq!(extractor.max_lookahead(), MAX_LOOKAROUND);
}

#[test]
fn replacement_is_one_pass_and_preserves_unmatched_text() {
    let source = "Read John 3:16 once.";
    let mut calls = 0;
    let replaced = ReferenceExtractor::new().replace_matches(source, |_| {
        calls += 1;
        "Acts 1:1 and Mark 2:2"
    });

    assert_eq!(calls, 1);
    assert_eq!(replaced, "Read Acts 1:1 and Mark 2:2 once.");
}

#[test]
fn markdown_linkification_escapes_labels_and_destinations() {
    struct Destination;
    impl fmt::Display for Destination {
        fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
            output.write_str("https://example.test/(passage)?q=John+3")
        }
    }

    let mut uri_calls = 0;
    let result = ReferenceExtractor::new().linkify_markdown_with_label(
        "Read John 3:16.",
        |_| {
            uri_calls += 1;
            Destination
        },
        |_| "[John]".to_owned(),
    );

    assert_eq!(uri_calls, 1);
    assert_eq!(
        result,
        "Read [\\[John\\]](https://example.test/%28passage%29?q=John+3)."
    );
}

#[test]
fn markdown_linkification_supports_explicit_languages() {
    let linked = ReferenceExtractor::new().linkify_markdown_with_language(
        "Read Jn 3:16",
        Language::Spanish,
        |passage_match| {
            if passage_match.passage().to_string().starts_with("John") {
                "/john"
            } else {
                "/other"
            }
        },
    );
    assert_eq!(linked, "Read [Jn 3:16](/john)");
}
