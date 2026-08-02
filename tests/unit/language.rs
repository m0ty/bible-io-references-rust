use super::*;

#[test]
fn parses_names_codes_aliases_and_locale_tags() {
    assert_eq!("Spanish".parse(), Ok(Language::Spanish));
    assert_eq!("ES".parse(), Ok(Language::Spanish));
    assert_eq!("spa-MX".parse(), Ok(Language::Spanish));
    assert_eq!("pt_BR".parse(), Ok(Language::Portuguese));
    assert_eq!("fil-PH".parse(), Ok(Language::Tagalog));
    assert_eq!("global".parse(), Ok(Language::Auto));
}

#[test]
fn exposes_every_accepted_identifier_in_source_order() {
    assert_eq!(
        Language::Tagalog.all_aliases().collect::<Vec<_>>(),
        ["tagalog", "Tagalog", "tl", "tgl", "fil"]
    );
}

#[test]
fn rejects_empty_and_unknown_identifiers() {
    assert!("   ".parse::<Language>().is_err());
    assert!("xx".parse::<Language>().is_err());
}

#[test]
fn reports_bundled_parsing_support() {
    assert!(Language::Auto.is_parsing_supported());
    assert!(Language::English.is_parsing_supported());
    assert!(Language::Spanish.is_parsing_supported());
    assert!(!Language::Greek.is_parsing_supported());
}
