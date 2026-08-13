# Changelog

## 1.1.1 - 2026-08-13

- Pinned the authoritative Dart upstream to an exact release commit
- Added exhaustive language-data and collision-contract regression tests
- Added a cross-runtime Dart/Rust parity checker and scheduled drift workflow
- Added Dart-compatible fallback errors, named parsing helpers, and checked
  verse-range copy helpers

## 1.1.0 - 2026-08-02

- Initial Rust implementation based on `bible-io-references-dart` Release 1.1.0
- Added checked book, verse, range, passage, language, and error value types
- Added multilingual configurable parsing and Unicode source normalization
- Added localized formatting, OSIS/USFM interoperability, and extraction
- Added optional Dart-compatible Serde representations and a batch CLI
- Added Dart-compatible metadata serialization and public alias introspection
- Added streaming CLI batch input and a crates.io-style executable name
- Moved all test implementations into `tests/unit`
