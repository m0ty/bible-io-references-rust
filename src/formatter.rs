//! Localized, configurable formatting for references and passages.

use core::fmt;

use crate::{
    book::Book,
    language::Language,
    language_data::{long_name, short_name},
    passage::{BookPassage, ChapterPassage, Passage, PassageSequence, VersePassage},
    reference::{Reference, VerseRange, VerseRef},
};

/// Controls whether localized book names are written in long or short form.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum BookNameStyle {
    /// Use a localized full name, such as `Juan`.
    #[default]
    Long,
    /// Use the shortest localized abbreviation, such as `Jn`.
    Short,
}

/// Immutable options for localized reference and passage formatting.
///
/// The default formatter uses English long book names and compact ranges.
/// A language without bundled names falls back to English deterministically.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ReferenceFormatter {
    language: Language,
    book_name_style: BookNameStyle,
    compact_ranges: bool,
}

impl ReferenceFormatter {
    /// The default English, long-name, compact-range formatter.
    pub const DEFAULT: Self = Self::new();

    /// Construct a formatter with the package defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            language: Language::English,
            book_name_style: BookNameStyle::Long,
            compact_ranges: true,
        }
    }

    /// Construct a formatter from all configuration values.
    #[must_use]
    pub const fn with_options(
        language: Language,
        book_name_style: BookNameStyle,
        compact_ranges: bool,
    ) -> Self {
        Self {
            language,
            book_name_style,
            compact_ranges,
        }
    }

    /// Construct a formatter for one language, retaining the other defaults.
    #[must_use]
    pub const fn for_language(language: Language) -> Self {
        Self::new().with_language(language)
    }

    /// Return the requested output language.
    #[must_use]
    pub const fn language(self) -> Language {
        self.language
    }

    /// Return the configured book-name style.
    #[must_use]
    pub const fn book_name_style(self) -> BookNameStyle {
        self.book_name_style
    }

    /// Return whether repeated components are omitted from ranges and lists.
    #[must_use]
    pub const fn compact_ranges(self) -> bool {
        self.compact_ranges
    }

    /// Return a copy configured for `language`.
    #[must_use]
    pub const fn with_language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    /// Return a copy configured to use `book_name_style`.
    #[must_use]
    pub const fn with_book_name_style(mut self, book_name_style: BookNameStyle) -> Self {
        self.book_name_style = book_name_style;
        self
    }

    /// Return a copy configured to compact or expand repeated range components.
    #[must_use]
    pub const fn with_compact_ranges(mut self, compact_ranges: bool) -> Self {
        self.compact_ranges = compact_ranges;
        self
    }

    /// Return the preferred localized display name for `book`.
    #[must_use]
    pub fn book_name(self, book: Book) -> &'static str {
        match self.book_name_style {
            BookNameStyle::Long => long_name(book, self.language),
            BookNameStyle::Short => short_name(book, self.language),
        }
    }

    /// Alias for [`Self::book_name`] matching the Dart package terminology.
    #[must_use]
    pub fn format_book_name(self, book: Book) -> &'static str {
        self.book_name(book)
    }

    /// Wrap a book in a lazily evaluated [`fmt::Display`] adapter.
    #[must_use = "formatting adapters must be displayed or converted to a string"]
    pub const fn format_book(self, book: Book) -> FormattedBook {
        FormattedBook {
            formatter: self,
            book,
        }
    }

    /// Wrap a verse or verse range in a localized [`fmt::Display`] adapter.
    #[must_use = "formatting adapters must be displayed or converted to a string"]
    pub fn format(self, reference: impl Into<Reference>) -> FormattedReference {
        self.format_reference(reference)
    }

    /// Wrap a verse or verse range in a localized [`fmt::Display`] adapter.
    #[must_use = "formatting adapters must be displayed or converted to a string"]
    pub fn format_reference(self, reference: impl Into<Reference>) -> FormattedReference {
        FormattedReference {
            formatter: self,
            reference: reference.into(),
        }
    }

    /// Wrap a whole passage expression in a localized [`fmt::Display`] adapter.
    #[must_use = "formatting adapters must be displayed or converted to a string"]
    pub const fn format_passage<'a>(self, passage: &'a Passage) -> FormattedPassage<'a> {
        FormattedPassage {
            formatter: self,
            passage,
        }
    }

    /// Wrap a whole-book passage in a localized [`fmt::Display`] adapter.
    #[must_use = "formatting adapters must be displayed or converted to a string"]
    pub const fn format_book_passage(self, passage: BookPassage) -> FormattedBookPassage {
        FormattedBookPassage {
            formatter: self,
            passage,
        }
    }

    /// Wrap a chapter passage in a localized [`fmt::Display`] adapter.
    #[must_use = "formatting adapters must be displayed or converted to a string"]
    pub const fn format_chapter_passage(self, passage: ChapterPassage) -> FormattedChapterPassage {
        FormattedChapterPassage {
            formatter: self,
            passage,
        }
    }

    /// Wrap a verse-selection passage in a localized [`fmt::Display`] adapter.
    #[must_use = "formatting adapters must be displayed or converted to a string"]
    pub const fn format_verse_passage<'a>(
        self,
        passage: &'a VersePassage,
    ) -> FormattedVersePassage<'a> {
        FormattedVersePassage {
            formatter: self,
            passage,
        }
    }

    /// Wrap a passage sequence in a localized [`fmt::Display`] adapter.
    #[must_use = "formatting adapters must be displayed or converted to a string"]
    pub const fn format_passage_sequence<'a>(
        self,
        passage: &'a PassageSequence,
    ) -> FormattedPassageSequence<'a> {
        FormattedPassageSequence {
            formatter: self,
            passage,
        }
    }

    fn write_verse(self, output: &mut fmt::Formatter<'_>, verse: VerseRef) -> fmt::Result {
        write!(
            output,
            "{} {}:{}",
            self.book_name(verse.book()),
            verse.chapter(),
            verse.verse()
        )
    }

    fn write_range(self, output: &mut fmt::Formatter<'_>, range: VerseRange) -> fmt::Result {
        let start = range.start();
        let end = range.end();

        if !self.compact_ranges || start.book() != end.book() {
            self.write_verse(output, start)?;
            output.write_str("-")?;
            return self.write_verse(output, end);
        }

        self.write_verse(output, start)?;
        if start.chapter() == end.chapter() {
            write!(output, "-{}", end.verse())
        } else {
            write!(output, "-{}:{}", end.chapter(), end.verse())
        }
    }

    fn write_reference(self, output: &mut fmt::Formatter<'_>, reference: Reference) -> fmt::Result {
        match reference {
            Reference::Verse(verse) => self.write_verse(output, verse),
            Reference::Range(range) => self.write_range(output, range),
        }
    }

    fn write_chapter_passage(
        self,
        output: &mut fmt::Formatter<'_>,
        passage: ChapterPassage,
    ) -> fmt::Result {
        write!(
            output,
            "{} {}",
            self.book_name(passage.book()),
            passage.start_chapter()
        )?;
        if let Some(end) = passage.end_chapter() {
            write!(output, "-{end}")?;
        }
        Ok(())
    }

    fn write_verse_passage(
        self,
        output: &mut fmt::Formatter<'_>,
        passage: &VersePassage,
    ) -> fmt::Result {
        let Some((first, rest)) = passage.selections().split_first() else {
            return Ok(());
        };

        self.write_reference(output, *first)?;
        if !self.compact_ranges || rest.is_empty() {
            for selection in rest {
                output.write_str(",")?;
                self.write_reference(output, *selection)?;
            }
            return Ok(());
        }

        let anchor = first.start();
        for selection in rest {
            output.write_str(",")?;
            self.write_relative_selection(output, *selection, anchor)?;
        }
        Ok(())
    }

    fn write_relative_selection(
        self,
        output: &mut fmt::Formatter<'_>,
        selection: Reference,
        anchor: VerseRef,
    ) -> fmt::Result {
        match selection {
            Reference::Verse(verse) => self.write_relative_verse(output, verse, anchor),
            Reference::Range(range) => self.write_relative_range(output, range, anchor),
        }
    }

    fn write_relative_verse(
        self,
        output: &mut fmt::Formatter<'_>,
        verse: VerseRef,
        anchor: VerseRef,
    ) -> fmt::Result {
        if verse.book() != anchor.book() {
            return self.write_verse(output, verse);
        }
        if verse.chapter() == anchor.chapter() {
            write!(output, "{}", verse.verse())
        } else {
            write!(output, "{}:{}", verse.chapter(), verse.verse())
        }
    }

    fn write_relative_range(
        self,
        output: &mut fmt::Formatter<'_>,
        range: VerseRange,
        anchor: VerseRef,
    ) -> fmt::Result {
        let start = range.start();
        let end = range.end();
        if start.book() != anchor.book() || end.book() != anchor.book() {
            return self.write_range(output, range);
        }

        if start.chapter() == anchor.chapter() {
            write!(output, "{}", start.verse())?;
        } else {
            write!(output, "{}:{}", start.chapter(), start.verse())?;
        }
        output.write_str("-")?;
        if start.chapter() == end.chapter() {
            write!(output, "{}", end.verse())
        } else {
            write!(output, "{}:{}", end.chapter(), end.verse())
        }
    }

    fn write_passage(self, output: &mut fmt::Formatter<'_>, passage: &Passage) -> fmt::Result {
        match passage {
            Passage::Book(passage) => output.write_str(self.book_name(passage.book())),
            Passage::Chapter(passage) => self.write_chapter_passage(output, *passage),
            Passage::Verses(passage) => self.write_verse_passage(output, passage),
            Passage::Sequence(sequence) => self.write_passage_sequence(output, sequence),
        }
    }

    fn write_passage_sequence(
        self,
        output: &mut fmt::Formatter<'_>,
        sequence: &PassageSequence,
    ) -> fmt::Result {
        let mut separator = "";
        for passage in sequence.passages() {
            output.write_str(separator)?;
            self.write_passage(output, passage)?;
            separator = "; ";
        }
        Ok(())
    }
}

impl Default for ReferenceFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Language> for ReferenceFormatter {
    fn from(language: Language) -> Self {
        Self::for_language(language)
    }
}

/// A lazily formatted, localized book name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormattedBook {
    formatter: ReferenceFormatter,
    book: Book,
}

impl fmt::Display for FormattedBook {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(self.formatter.book_name(self.book))
    }
}

/// A lazily formatted, localized verse or verse range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormattedReference {
    formatter: ReferenceFormatter,
    reference: Reference,
}

impl fmt::Display for FormattedReference {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.formatter.write_reference(output, self.reference)
    }
}

/// A lazily formatted, localized whole passage expression.
#[derive(Clone, Copy, Debug)]
pub struct FormattedPassage<'a> {
    formatter: ReferenceFormatter,
    passage: &'a Passage,
}

impl fmt::Display for FormattedPassage<'_> {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.formatter.write_passage(output, self.passage)
    }
}

/// A lazily formatted, localized whole-book passage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormattedBookPassage {
    formatter: ReferenceFormatter,
    passage: BookPassage,
}

impl fmt::Display for FormattedBookPassage {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(self.formatter.book_name(self.passage.book()))
    }
}

/// A lazily formatted, localized chapter passage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormattedChapterPassage {
    formatter: ReferenceFormatter,
    passage: ChapterPassage,
}

impl fmt::Display for FormattedChapterPassage {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.formatter.write_chapter_passage(output, self.passage)
    }
}

/// A lazily formatted, localized verse-selection passage.
#[derive(Clone, Copy, Debug)]
pub struct FormattedVersePassage<'a> {
    formatter: ReferenceFormatter,
    passage: &'a VersePassage,
}

impl fmt::Display for FormattedVersePassage<'_> {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.formatter.write_verse_passage(output, self.passage)
    }
}

/// A lazily formatted, localized passage sequence.
#[derive(Clone, Copy, Debug)]
pub struct FormattedPassageSequence<'a> {
    formatter: ReferenceFormatter,
    passage: &'a PassageSequence,
}

impl fmt::Display for FormattedPassageSequence<'_> {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.formatter.write_passage_sequence(output, self.passage)
    }
}

#[cfg(test)]
#[path = "../tests/unit/formatter.rs"]
mod tests;
