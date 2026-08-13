# Dart Upstream Parity

The authoritative behavioral source for this crate is the Dart package at
<https://github.com/m0ty/bible-io-references-dart>. Rust APIs follow Rust
conventions, but parsing, formatting, data, identifiers, passage grammar, and
command-line behavior are kept compatible unless a difference is explicitly
listed below.

## Pinned Baseline

| Field | Value |
| --- | --- |
| Repository | <https://github.com/m0ty/bible-io-references-dart> |
| Branch | `main` |
| Commit | [`8399fac944f0d44bb539a628bf81419a2e435bbc`](https://github.com/m0ty/bible-io-references-dart/commit/8399fac944f0d44bb539a628bf81419a2e435bbc) |
| Release | `Release-1.1.0` |
| Package version | `1.1.0` |
| Last upstream verification | 2026-08-13 |

The same baseline is recorded in `[package.metadata.dart-parity]` in
`Cargo.toml` so automated checks do not need to scrape this document.

## Verified Parity Matrix

The evidence column names both the upstream contract and the corresponding
Rust implementation/tests. A row marked **Parity** is expected to remain
behaviorally compatible, subject only to the documented differences in the
next section.

| Area | Status | Evidence |
| --- | --- | --- |
| Package scope and limits | **Parity** | Dart [`README.md`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/README.md) and [`references.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/references.dart); Rust `README.md`, `src/reference.rs`, and `tests/unit/reference.rs` |
| Ordered 83-book model and compact abbreviations | **Parity** | Dart [`bible_book_enum.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/bible_book_enum.dart); Rust `src/book.rs` and `tests/unit/book.rs` |
| English plus 12 complete localized packs | **Parity** | Dart [`languages.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/languages.dart) and [`lib/languages/`](https://github.com/m0ty/bible-io-references-dart/tree/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/languages); Rust `src/language.rs`, `src/language_data.rs`, and their unit tests |
| Single verses and ascending same-chapter, cross-chapter, and cross-book ranges | **Parity** | Dart `lib/references.dart`, `test/reference_api_test.dart`, and `test/package_test.dart`; Rust `src/reference.rs`, `src/parser.rs`, and their unit tests |
| Configurable aliases, language priority, ambiguity policy, and parse metadata | **Parity** | Dart [`src/reference_parser.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/src/reference_parser.dart) and `test/configurable_parser_test.dart`; Rust `src/parser.rs` and `tests/unit/parser.rs` |
| Whole books, chapters, chapter ranges, verse lists, sequences, and single-chapter shorthand | **Parity** | Dart [`src/passage_parser.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/src/passage_parser.dart) and `test/passage_grammar_test.dart`; Rust `src/passage.rs` and `tests/unit/passage.rs` |
| Reference-oriented Unicode normalization and source mapping | **Parity with native offsets** | Dart [`reference_input_normalizer.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/reference_input_normalizer.dart) and its tests; Rust `src/normalize.rs` and `tests/unit/normalize.rs` |
| Localized long/short formatting and compact ranges | **Parity** | Dart [`reference_formatter.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/reference_formatter.dart); Rust `src/formatter.rs` and `tests/unit/formatter.rs` |
| Complete OSIS/USFM book mappings, strict reference parsing, and passage serialization | **Parity** | Dart [`reference_identifiers.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/reference_identifiers.dart); Rust `src/identifiers.rs` and `tests/unit/identifiers.rs` |
| Prose extraction, non-overlap rules, replacement, and Markdown linkification | **Parity with native offsets/boundaries** | Dart [`reference_extractor.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/lib/reference_extractor.dart); Rust `src/extractor.rs` and `tests/unit/extractor.rs` |
| Stable reference/passage JSON shapes | **Parity when enabled** | Dart `toJson`/`fromJson` tests; Rust `src/serde_impl.rs` and `tests/unit/serde_impl.rs` with `--features serde` |
| UTF-8 single/batch CLI, JSON Lines, text/JSON/OSIS/USFM output, and exit codes 0/64/65/66 | **Parity with Rust executable naming** | Dart [`bin/bible_io_references.dart`](https://github.com/m0ty/bible-io-references-dart/blob/8399fac944f0d44bb539a628bf81419a2e435bbc/bin/bible_io_references.dart); Rust `src/bin/bible_io_references.rs` and `tests/unit/bible_io_references_cli.rs` |
| Public API smoke coverage | **Parity** | Dart `test/`; Rust `tests/public_api.rs` plus module unit suites documented in `tests/README.md` |

## Intentional Rust-Native and Superset Differences

These differences are allowed and are not parity regressions:

1. **UTF-8 spans, windows, and Unicode boundaries.** Dart reports source
   positions and extractor windows in UTF-16 code units. Rust reports
   slice-safe UTF-8 byte ranges, measures extractor windows in bytes, and uses
   Unicode-aware boundaries where Dart uses UTF-16/ASCII-oriented boundaries.
2. **Additional common English aliases.** Rust accepts a small superset of
   conventional English book aliases. It must not change the Dart-compatible
   result for aliases present upstream.
3. **Additional punctuation normalization.** Rust normalizes a small superset
   of reference punctuation. Inputs accepted by Dart must retain the same
   normalized syntax and parse result.
4. **Component-specific error precedence.** When one malformed input violates
   several rules, a strict Rust component parser may report a more specific
   error before the error selected by Dart's dispatch order. Stable error
   categories and codes remain compatible for equivalent validation failures.
5. **Optional Serde.** Dart exposes JSON maps unconditionally. Rust gates the
   same wire shapes behind the optional `serde` feature to keep the default
   dependency set empty.
6. **Underscore locale tags.** Rust accepts locale tags such as `es_MX` in
   addition to Dart-compatible hyphenated tags such as `es-MX`.
7. **Hyphenated executable name.** The Dart executable is
   `bible_io_references`; the Rust executable is `bible-io-references`.
8. **Modernized Rust convenience entry points.** Rust's `FromStr` support for
   `Book` accepts compact codes, English names, OSIS identifiers, and USFM
   identifiers. The `verse_ref_from_str` and `verse_range_ref_from_str`
   compatibility helpers use the modern Unicode-aware parser rather than
   preserving the Dart legacy helpers' pre-normalization behavior. The Rust
   CLI also provides the conventional `--version`/`-V` option.

Any other observable difference is treated as a parity bug until it is either
fixed or deliberately added to this list with evidence.

## Explicitly Out of Scope

Upstream commits `7fe9554` and `10e7508` temporarily introduced canon profiles,
versification profiles, `BibleProfile`, and reference-range expansion. The
author then removed that work in the pinned release commit with the message
`release: prepare 1.1.0 without versification validation`.

Those transient APIs are not part of Dart 1.1.0 or the parity target. Both
packages intentionally apply only broad `1..=999` coordinate checks; they do
not assert that a verse exists in a particular edition or that every modeled
book belongs to one universal canon.

## Checking and Advancing the Baseline

Given a checkout of the Dart repository, run:

```powershell
pwsh tools/parity/check.ps1 -DartCheckout PATH
```

When upstream advances:

1. Fetch `main` and review the complete diff from the commit pinned above.
2. Inventory exported API, language data, parsing/formatting behavior, wire
   formats, CLI behavior, tests, and changelog entries.
3. Port behavioral changes and add Rust regression tests, or document an
   intentionally approved Rust difference.
4. Run the parity check and the release checks from `README.md`.
5. Update the commit/version in this file and `Cargo.toml` together.

The baseline must never be advanced merely to silence a drift check.
