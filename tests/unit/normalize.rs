use super::*;

#[test]
fn keeps_ascii_case_and_whitespace_runs() {
    let input = "JoHn  3:16-18, 20; Acts\t2:1";
    assert_eq!(normalize(input), input);
}

#[test]
fn normalizes_fullwidth_ascii_and_digits() {
    let fullwidth = "\u{ff2a}\u{ff4f}\u{ff48}\u{ff4e}\u{3000}\u{ff13}\u{ff1a}\u{ff11}\u{ff16}\u{ff0d}\u{ff11}\u{ff18}";
    assert_eq!(normalize(fullwidth), "John 3:16-18");
    assert_eq!(
        normalize("\u{660}\u{661}\u{662}\u{663}\u{664}\u{665}\u{666}\u{667}\u{668}\u{669}"),
        "0123456789"
    );
    assert_eq!(
        normalize("\u{6f0}\u{6f1}\u{6f2}\u{6f3}\u{6f4}\u{6f5}\u{6f6}\u{6f7}\u{6f8}\u{6f9}"),
        "0123456789"
    );
}

#[test]
fn normalizes_reference_punctuation_variants() {
    for variant in [
        '\u{060c}', '\u{3001}', '\u{fe10}', '\u{fe11}', '\u{fe50}', '\u{fe51}', '\u{ff64}',
    ] {
        assert_eq!(normalize(&format!("16{variant}18")), "16,18");
    }
    for variant in ['\u{061b}', '\u{fe14}', '\u{fe54}'] {
        assert_eq!(normalize(&format!("16{variant}Acts")), "16;Acts");
    }
    for variant in ['\u{2236}', '\u{fe13}', '\u{fe55}'] {
        assert_eq!(normalize(&format!("3{variant}16")), "3:16");
    }
    for variant in [
        '\u{066b}', '\u{06d4}', '\u{2024}', '\u{3002}', '\u{fe52}', '\u{ff61}',
    ] {
        assert_eq!(normalize(&format!("3{variant}16")), "3.16");
    }
    for variant in [
        '\u{05be}', '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}', '\u{2015}',
        '\u{2212}', '\u{2e3a}', '\u{2e3b}', '\u{fe58}', '\u{fe63}',
    ] {
        assert_eq!(normalize(&format!("16{variant}18")), "16-18");
    }
}

#[test]
fn normalizes_unicode_spaces_without_collapsing_them() {
    for variant in [
        '\u{00a0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}',
        '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{202f}',
        '\u{205f}', '\u{3000}',
    ] {
        assert_eq!(normalize(&format!("John{variant}3")), "John 3");
    }
    assert_eq!(normalize("John\u{a0}\u{3000}3"), "John  3");
}

#[test]
fn removes_controls_selectors_and_script_marks() {
    assert_eq!(
        normalize("\u{feff}Jo\u{ad}\u{200b}hn\u{2060} 3\u{2236}16\u{05be}18\u{fe0f}"),
        "John 3:16-18"
    );
    assert_eq!(normalize("Ge\u{301}ne\u{302}sis JOHN"), "Genesis JOHN");
    assert_eq!(
        normalize("\u{5d1}\u{5b0}\u{5bc}\u{5e8}\u{5b5}\u{5d0}\u{5e9}\u{5b4}\u{5c1}\u{5d9}\u{5ea}"),
        "\u{5d1}\u{5e8}\u{5d0}\u{5e9}\u{5d9}\u{5ea}"
    );
    assert_eq!(
        normalize("\u{64a}\u{64f}\u{648}\u{62d}\u{64e}\u{646}\u{64e}\u{651}\u{627}"),
        "\u{64a}\u{648}\u{62d}\u{646}\u{627}"
    );
}

#[test]
fn detailed_mapping_uses_original_utf8_byte_spans() {
    let input = "\u{200f}Jo\u{301}hn\u{200e} \u{663}\u{ff1a}\u{661}\u{666}";
    let result = normalize_detailed(input);

    assert_eq!(result.normalized(), "John 3:16");
    assert_eq!(result.original(), input);
    assert!(result.changed());
    assert_eq!(result.len(), 9);
    assert_eq!(result.source_spans().len(), 9);
    assert_eq!(result.map_span(0..4), Some(SourceSpan::new(0, 12)));
    assert_eq!(
        result.original_text_for(0..4),
        Some("\u{200f}Jo\u{301}hn\u{200e}")
    );
    assert_eq!(result.map_span(1..2), Some(SourceSpan::new(4, 7)));
    assert_eq!(result.original_text_for(1..2), Some("o\u{301}"));
    assert_eq!(result.map_span(5..9), Some(SourceSpan::new(13, 22)));
    assert_eq!(
        result.map_span(0..result.len()),
        Some(SourceSpan::new(0, 22))
    );
}

#[test]
fn detailed_mapping_expands_partial_multibyte_scalars() {
    let result = normalize_detailed("A\u{1f600}\u{200f}B");

    assert_eq!(result.normalized(), "A\u{1f600}B");
    assert_eq!(result.len(), 6);
    assert_eq!(result.source_spans().len(), 6);
    assert_eq!(result.map_span(1..5), Some(SourceSpan::new(1, 8)));
    assert_eq!(result.map_span(1..2), Some(SourceSpan::new(1, 8)));
    assert_eq!(result.map_span(4..5), Some(SourceSpan::new(1, 8)));
    assert_eq!(result.original_text_for(1..2), Some("\u{1f600}\u{200f}"));
}

#[test]
fn detailed_mapping_handles_empty_ranges_and_removed_input() {
    let result = normalize_detailed("A\u{200f}B");
    assert_eq!(result.map_span(0..0), Some(SourceSpan::new(0, 0)));
    assert_eq!(result.map_span(1..1), Some(SourceSpan::new(4, 4)));
    assert_eq!(result.map_span(2..2), Some(SourceSpan::new(5, 5)));
    assert_eq!(result.map_span(3..3), None);
    let reversed = Range { start: 2, end: 1 };
    assert_eq!(result.map_span(reversed), None);

    let removed = "\u{200f}\u{301}\u{200e}";
    let result = normalize_detailed(removed);
    assert!(result.is_empty());
    assert_eq!(
        result.map_span(0..0),
        Some(SourceSpan::new(0, removed.len()))
    );
    assert_eq!(result.original_text_for(0..0), Some(removed));
}

#[test]
fn whitespace_helpers_prepare_parser_input() {
    assert_eq!(collapse_whitespace(" \tJohn\n  3:16\u{3000}"), "John 3:16");
    assert_eq!(
        normalize_for_parsing(
            "\u{200f}\u{ff2a}\u{ff4f}\u{ff48}\u{ff4e}\u{3000}\u{663}\u{ff1a}\u{661}\u{666}  "
        ),
        "John 3:16"
    );
}

#[test]
fn source_span_extracts_only_valid_utf8_ranges() {
    let text = "A\u{1f600}B";
    assert_eq!(SourceSpan::new(1, 5).text_from(text), Some("\u{1f600}"));
    assert_eq!(SourceSpan::new(2, 5).text_from(text), None);
    assert_eq!(SourceSpan::new(1, 5).to_string(), "1..5");
}
