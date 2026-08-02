# bible-io-references

An idiomatic Rust implementation of the parsing model in
[`bible-io-references-dart` Release 1.1.0](https://github.com/m0ty/bible-io-references-dart/releases/tag/Release-1.1.0).

The crate parses multilingual Bible references and richer passage expressions
without assuming a particular translation, canon, or versification table.

## Features

- Single verses and inclusive same-chapter, cross-chapter, or cross-book ranges
- Whole books, chapter ranges, comma-separated verse selections, and sequences
- English plus 12 complete localized parsing and formatting packs
- Unicode-aware syntax normalization and UTF-8 source-span mapping
- Configurable aliases, language priority, ambiguity handling, and parse metadata
- Read-only localized alias registries and automatic-collision introspection
- OSIS and USFM serialization and strict reverse parsing for all 83 books
- Parser-driven extraction, replacement, and Markdown linkification in prose
- Optional Dart-compatible JSON serialization through `serde`
- No required runtime dependencies

## Installation

```bash
cargo add bible-io-references
```

Enable Serde support when JSON interoperability is needed:

```bash
cargo add bible-io-references --features serde
```

## Reference Parsing

Rust's standard parsing traits are the primary API:

```rust
use bible_io_references::{Book, Reference, VerseRef};

let verse: VerseRef = "John 3:16".parse()?;
assert_eq!(verse.book(), Book::John);
assert_eq!(verse.chapter(), 3);
assert_eq!(verse.verse(), 16);

let range: Reference = "John 3:16-Acts 1:2".parse()?;
assert_eq!(range.to_string(), "John 3:16-Acts 1:2");

# Ok::<(), Box<dyn std::error::Error>>(())
```

`VerseRef`, `VerseRange`, and `Reference` expose strict subtype parsing. All
constructors enforce chapter and verse coordinates in `1..=999`; ranges must
move forward in the crate's documented 83-book order.

## Rich Passages

Use `Passage` for expressions broader than one contiguous range:

```rust
use bible_io_references::Passage;

let passage: Passage = "John 3:16,18-20; Acts 2; Romans 8".parse()?;
assert_eq!(
    passage.to_string(),
    "John 3:16,18-20; Acts 2; Romans 8"
);

# Ok::<(), Box<dyn std::error::Error>>(())
```

Bare numbers in Obadiah, Philemon, 2 John, 3 John, and Jude use standard
single-chapter shorthand. `PassageParser::with_single_chapter_books` can replace
that set for a specific application.

## Languages And Formatting

Automatic detection is the default. An explicit language is useful for
short aliases or predictable user interfaces:

```rust
use bible_io_references::{
    BookNameStyle, Language, Passage, ReferenceFormatter,
};

let passage = Passage::parse_with_language("Juan 3:16,18-20", Language::Spanish)?;
let formatter = ReferenceFormatter::for_language(Language::Spanish)
    .with_book_name_style(BookNameStyle::Short);

assert_eq!(formatter.format_passage(&passage).to_string(), "Jn 3:16,18-20");

# Ok::<(), Box<dyn std::error::Error>>(())
```

Bundled parsing packs: Arabic, Chinese, English, French, German, Hebrew,
Hindi, Indonesian, Korean, Portuguese, Russian, Spanish, and Tagalog. Reserved
language variants remain available for custom aliases.

`localized_books` exposes every registered long name and abbreviation without
allowing mutation. `auto_language_collisions` returns the normalized aliases
that can identify distinct books during automatic detection.

## Configurable Parsing

```rust
use bible_io_references::{
    AmbiguityPolicy, Book, Language, ReferenceParser,
};

let parser = ReferenceParser::builder()
    .alias("favorite", Book::John)
    .preferred_languages([Language::Spanish])
    .ambiguity_policy(AmbiguityPolicy::Reject)
    .build()?;

let parsed = parser.parse_detailed("favorite 3:16")?;
assert_eq!(parsed.value().start().book(), Book::John);
assert!(parsed.metadata().book_matches()[0].selected().is_custom());
assert_eq!(parsed.metadata().book_matches()[0].selected().alias(), "favorite");

# Ok::<(), Box<dyn std::error::Error>>(())
```

Errors carry a typed `ParseErrorKind`, a stable snake-case code, and a detailed
diagnostic. Ordinary Rust `Result` is the non-throwing parse API; `try_parse`
helpers return `Option` where error detail is not needed.

## OSIS And USFM

```rust
use bible_io_references::{
    MachineIdentifiers, Reference, reference_from_osis_identifier,
};

let reference: Reference = "2 Corinthians 6:14-7:1".parse()?;
assert_eq!(reference.osis_identifier(), "2Cor.6.14-2Cor.7.1");
assert_eq!(reference.usfm_identifier(), "2CO 6:14-7:1");
assert_eq!(
    reference_from_osis_identifier("2Cor.6.14-2Cor.7.1")?,
    reference
);

# Ok::<(), Box<dyn std::error::Error>>(())
```

Machine identifier parsing is intentionally case-sensitive. Cross-book USFM
ranges use the Dart package's reversible extension, for example
`JHN-ACT 21:25-1:2`.

## Extraction

Extraction reports Rust-native UTF-8 byte ranges and preserves exact source
text:

```rust
use bible_io_references::ReferenceExtractor;

let source = "Study John 3:16,18-20 and Acts 2 today.";
let matches = ReferenceExtractor::new().extract(source);

assert_eq!(matches.len(), 2);
assert_eq!(matches[0].source_text(), "John 3:16,18-20");
assert_eq!(&source[matches[0].range()], matches[0].source_text());
```

`replace_matches` and `linkify_markdown` process matches once, so text returned
by a callback is never scanned again. Bare-book extraction is opt-in.

## Unicode Normalization

The syntax normalizer handles fullwidth ASCII, Arabic-Indic and Persian
digits, common punctuation variants, Unicode spaces, combining marks, and
directional controls. It is intentionally reference-specific rather than a
general transliteration or Unicode normalization library.

`normalize_detailed` maps normalized UTF-8 byte ranges back to slice-safe byte
ranges in the original input. This is the intentional Rust equivalent of the
Dart package's UTF-16 mapping API.

## JSON

With the `serde` feature, `Book`, references, and passages use the stable Dart
wire shapes. For example, John 3:16 serializes as:

```json
{"type":"verse","book":"jo","chapter":3,"verse":16}
```

Deserialization calls the same checked constructors as normal Rust code, so
zero coordinates, empty passage collections, and reversed ranges are rejected.
Parse candidates, token matches, and parse metadata also serialize with the
field names used by Dart 1.1.0.

## Command Line

```bash
cargo install bible-io-references
bible-io-references "John 3:16,18-20"
cargo run -- "John 3:16,18-20"
cargo run -- --language es --format osis "Juan 3:16-17"
cargo run -- --format usfm --input references.txt
```

Batch mode reads one nonblank UTF-8 expression per line. Exit codes follow the
Dart package: `0` success, `64` usage, `65` parse failure, and `66` input error.

## Scope

- Coordinates receive broad sanity checks, not edition-specific validation.
- The ordered 83-book model includes Protestant books, separately modeled
  Catholic additions, and Eastern Orthodox additions. It is a deterministic
  range order, not a claim that every tradition uses one universal canon.
- Passage lists use commas and sequences use semicolons; prose words such as
  `and` are not grammar separators.

## Development

Test implementations live under `tests/unit/` and are included from their
corresponding production modules so private invariants remain covered. Run the
release checks with:

```bash
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo package --allow-dirty --locked
```

## License

GNU Affero General Public License v3.0 only. See `LICENSE`.
