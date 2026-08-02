//! Rich Bible passage values and parsing.
//!
//! [`Reference`] represents one verse or one contiguous
//! verse range. A [`Passage`] can additionally represent a whole book, one or
//! more chapters, a comma-separated verse selection, or a semicolon-separated
//! sequence of passage expressions.

use core::{fmt, str::FromStr};

use crate::{
    Book, BookMatch, Language, ParseError, ParseErrorKind, ParseMetadata, Parsed, Reference,
    ReferenceParser, VerseRange, VerseRef,
    normalize::normalize_for_parsing,
    reference::{MAX_CHAPTER_NUMBER, MAX_VERSE_NUMBER},
};

/// Books whose conventional bare-number notation names a verse in chapter 1.
pub const DEFAULT_SINGLE_CHAPTER_BOOKS: [Book; 5] = [
    Book::Obadiah,
    Book::Philemon,
    Book::SecondJohn,
    Book::ThirdJohn,
    Book::Jude,
];

/// Failure to construct a passage value without satisfying its invariants.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PassageBuildError {
    /// A chapter coordinate was zero or exceeded the package sanity limit.
    InvalidChapter {
        /// The rejected chapter coordinate.
        value: u16,
    },
    /// An inclusive chapter range did not have a strictly ascending end.
    ChapterRangeNotAscending {
        /// The first chapter in the rejected range.
        start: u16,
        /// The final chapter in the rejected range.
        end: u16,
    },
    /// A verse passage contained no selections.
    EmptyVersePassage,
    /// A passage sequence contained no passages.
    EmptyPassageSequence,
}

impl fmt::Display for PassageBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidChapter { value } => write!(
                formatter,
                "chapter must be between 1 and {MAX_CHAPTER_NUMBER} (got {value})"
            ),
            Self::ChapterRangeNotAscending { start, end } => write!(
                formatter,
                "end chapter {end} must come after start chapter {start}"
            ),
            Self::EmptyVersePassage => {
                formatter.write_str("a verse passage must contain at least one reference")
            }
            Self::EmptyPassageSequence => {
                formatter.write_str("a passage sequence must contain at least one passage")
            }
        }
    }
}

impl std::error::Error for PassageBuildError {}

/// A complete Bible passage expression.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Passage {
    /// A complete book.
    Book(BookPassage),
    /// One chapter or an inclusive range of chapters.
    Chapter(ChapterPassage),
    /// One or more discrete verse references.
    Verses(VersePassage),
    /// A source-ordered sequence of passage expressions.
    Sequence(PassageSequence),
}

impl Passage {
    /// Parse a passage using automatic language detection.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        PassageParser::new().parse(input)
    }

    /// Parse a passage using one explicit book-name language.
    pub fn parse_with_language(input: &str, language: Language) -> Result<Self, ParseError> {
        PassageParser::new().parse_with_language(input, language)
    }

    /// Parse a passage and retain book-resolution metadata.
    pub fn parse_detailed(input: &str) -> Result<Parsed<Self>, ParseError> {
        PassageParser::new().parse_detailed(input)
    }

    /// Parse with one explicit language and retain book-resolution metadata.
    pub fn parse_detailed_with_language(
        input: &str,
        language: Language,
    ) -> Result<Parsed<Self>, ParseError> {
        PassageParser::new().parse_detailed_with_language(input, language)
    }

    /// Return `None` rather than an error for invalid input.
    #[must_use]
    pub fn try_parse(input: &str) -> Option<Self> {
        PassageParser::new().try_parse(input)
    }

    /// Return `None` rather than an error when parsing with one language.
    #[must_use]
    pub fn try_parse_with_language(input: &str, language: Language) -> Option<Self> {
        PassageParser::new().try_parse_with_language(input, language)
    }

    /// Borrow the whole-book value, if this is one.
    #[must_use]
    pub const fn as_book(&self) -> Option<&BookPassage> {
        match self {
            Self::Book(passage) => Some(passage),
            _ => None,
        }
    }

    /// Borrow the chapter value, if this is one.
    #[must_use]
    pub const fn as_chapter(&self) -> Option<&ChapterPassage> {
        match self {
            Self::Chapter(passage) => Some(passage),
            _ => None,
        }
    }

    /// Borrow the verse-selection value, if this is one.
    #[must_use]
    pub const fn as_verses(&self) -> Option<&VersePassage> {
        match self {
            Self::Verses(passage) => Some(passage),
            _ => None,
        }
    }

    /// Borrow the sequence value, if this is one.
    #[must_use]
    pub const fn as_sequence(&self) -> Option<&PassageSequence> {
        match self {
            Self::Sequence(passage) => Some(passage),
            _ => None,
        }
    }
}

impl fmt::Display for Passage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Book(passage) => passage.fmt(formatter),
            Self::Chapter(passage) => passage.fmt(formatter),
            Self::Verses(passage) => passage.fmt(formatter),
            Self::Sequence(passage) => passage.fmt(formatter),
        }
    }
}

impl FromStr for Passage {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

/// A passage covering an entire Bible book.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BookPassage {
    book: Book,
}

impl BookPassage {
    /// Construct a whole-book passage.
    #[must_use]
    pub const fn new(book: Book) -> Self {
        Self { book }
    }

    /// Return the represented book.
    #[must_use]
    pub const fn book(self) -> Book {
        self.book
    }
}

impl fmt::Display for BookPassage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.book.full_name())
    }
}

impl From<Book> for BookPassage {
    fn from(book: Book) -> Self {
        Self::new(book)
    }
}

impl From<BookPassage> for Passage {
    fn from(passage: BookPassage) -> Self {
        Self::Book(passage)
    }
}

/// A passage covering one chapter or an inclusive range of chapters.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ChapterPassage {
    book: Book,
    start_chapter: u16,
    end_chapter: Option<u16>,
}

impl ChapterPassage {
    /// Construct a validated chapter or chapter-range passage.
    pub const fn new(
        book: Book,
        start_chapter: u16,
        end_chapter: Option<u16>,
    ) -> Result<Self, PassageBuildError> {
        if start_chapter == 0 || start_chapter > MAX_CHAPTER_NUMBER {
            return Err(PassageBuildError::InvalidChapter {
                value: start_chapter,
            });
        }
        if let Some(end) = end_chapter {
            if end == 0 || end > MAX_CHAPTER_NUMBER {
                return Err(PassageBuildError::InvalidChapter { value: end });
            }
            if end <= start_chapter {
                return Err(PassageBuildError::ChapterRangeNotAscending {
                    start: start_chapter,
                    end,
                });
            }
        }
        Ok(Self {
            book,
            start_chapter,
            end_chapter,
        })
    }

    /// Construct a validated single-chapter passage.
    pub const fn single(book: Book, chapter: u16) -> Result<Self, PassageBuildError> {
        Self::new(book, chapter, None)
    }

    /// Construct a validated inclusive chapter range.
    pub const fn range(
        book: Book,
        start_chapter: u16,
        end_chapter: u16,
    ) -> Result<Self, PassageBuildError> {
        Self::new(book, start_chapter, Some(end_chapter))
    }

    /// Return the represented book.
    #[must_use]
    pub const fn book(self) -> Book {
        self.book
    }

    /// Return the first represented chapter.
    #[must_use]
    pub const fn start_chapter(self) -> u16 {
        self.start_chapter
    }

    /// Return the inclusive last chapter for a range, or `None` for one chapter.
    #[must_use]
    pub const fn end_chapter(self) -> Option<u16> {
        self.end_chapter
    }
}

impl fmt::Display for ChapterPassage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}",
            self.book.full_name(),
            self.start_chapter
        )?;
        if let Some(end) = self.end_chapter {
            write!(formatter, "-{end}")?;
        }
        Ok(())
    }
}

impl From<ChapterPassage> for Passage {
    fn from(passage: ChapterPassage) -> Self {
        Self::Chapter(passage)
    }
}

/// One or more discrete verse references belonging to one passage expression.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct VersePassage {
    selections: Box<[Reference]>,
}

impl VersePassage {
    /// Construct a non-empty verse-selection passage.
    pub fn new(selections: impl IntoIterator<Item = Reference>) -> Result<Self, PassageBuildError> {
        let selections = selections.into_iter().collect::<Box<[_]>>();
        if selections.is_empty() {
            return Err(PassageBuildError::EmptyVersePassage);
        }
        Ok(Self { selections })
    }

    /// Borrow the selections in source order.
    #[must_use]
    pub fn selections(&self) -> &[Reference] {
        &self.selections
    }
}

impl AsRef<[Reference]> for VersePassage {
    fn as_ref(&self) -> &[Reference] {
        self.selections()
    }
}

impl fmt::Display for VersePassage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first = self
            .selections
            .first()
            .expect("VersePassage guarantees at least one selection");
        first.fmt(formatter)?;

        let anchor = first.start();
        for selection in &self.selections[1..] {
            formatter.write_str(",")?;
            format_compact_selection(*selection, anchor, formatter)?;
        }
        Ok(())
    }
}

impl TryFrom<Vec<Reference>> for VersePassage {
    type Error = PassageBuildError;

    fn try_from(selections: Vec<Reference>) -> Result<Self, Self::Error> {
        Self::new(selections)
    }
}

impl From<VersePassage> for Passage {
    fn from(passage: VersePassage) -> Self {
        Self::Verses(passage)
    }
}

/// A source-ordered sequence of passage expressions.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PassageSequence {
    passages: Box<[Passage]>,
}

impl PassageSequence {
    /// Construct a non-empty passage sequence.
    pub fn new(passages: impl IntoIterator<Item = Passage>) -> Result<Self, PassageBuildError> {
        let passages = passages.into_iter().collect::<Box<[_]>>();
        if passages.is_empty() {
            return Err(PassageBuildError::EmptyPassageSequence);
        }
        Ok(Self { passages })
    }

    /// Borrow the component passages in source order.
    #[must_use]
    pub fn passages(&self) -> &[Passage] {
        &self.passages
    }
}

impl AsRef<[Passage]> for PassageSequence {
    fn as_ref(&self) -> &[Passage] {
        self.passages()
    }
}

impl fmt::Display for PassageSequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, passage) in self.passages.iter().enumerate() {
            if index != 0 {
                formatter.write_str("; ")?;
            }
            passage.fmt(formatter)?;
        }
        Ok(())
    }
}

impl TryFrom<Vec<Passage>> for PassageSequence {
    type Error = PassageBuildError;

    fn try_from(passages: Vec<Passage>) -> Result<Self, Self::Error> {
        Self::new(passages)
    }
}

impl From<PassageSequence> for Passage {
    fn from(passage: PassageSequence) -> Self {
        Self::Sequence(passage)
    }
}

/// A reusable parser for book, chapter, verse-list, and passage sequences.
#[derive(Clone, Debug)]
pub struct PassageParser {
    reference_parser: ReferenceParser,
    single_chapter_books: Vec<Book>,
}

impl Default for PassageParser {
    fn default() -> Self {
        Self {
            reference_parser: ReferenceParser::new(),
            single_chapter_books: DEFAULT_SINGLE_CHAPTER_BOOKS.to_vec(),
        }
    }
}

impl PassageParser {
    /// Construct a parser with bundled aliases and conventional shorthand.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a passage parser around an existing reference parser.
    #[must_use]
    pub fn from_reference_parser(reference_parser: ReferenceParser) -> Self {
        Self {
            reference_parser,
            ..Self::default()
        }
    }

    /// Replace the books for which a bare number denotes a chapter-one verse.
    #[must_use]
    pub fn with_single_chapter_books(mut self, books: impl IntoIterator<Item = Book>) -> Self {
        self.single_chapter_books.clear();
        for book in books {
            if !self.single_chapter_books.contains(&book) {
                self.single_chapter_books.push(book);
            }
        }
        self
    }

    /// Borrow the parser used for book-name resolution and reference parsing.
    #[must_use]
    pub const fn reference_parser(&self) -> &ReferenceParser {
        &self.reference_parser
    }

    /// Return the books configured for bare-number verse shorthand.
    #[must_use]
    pub fn single_chapter_books(&self) -> &[Book] {
        &self.single_chapter_books
    }

    /// Whether a book uses bare-number verse shorthand in this parser.
    #[must_use]
    pub fn is_single_chapter_book(&self, book: Book) -> bool {
        self.single_chapter_books.contains(&book)
    }

    /// Parse a passage using automatic language detection.
    pub fn parse(&self, input: &str) -> Result<Passage, ParseError> {
        self.parse_detailed(input).map(Parsed::into_value)
    }

    /// Parse a passage using one explicit book-name language.
    pub fn parse_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Passage, ParseError> {
        self.parse_detailed_with_language(input, language)
            .map(Parsed::into_value)
    }

    /// Parse a passage and retain normalization and book-resolution metadata.
    pub fn parse_detailed(&self, input: &str) -> Result<Parsed<Passage>, ParseError> {
        self.parse_detailed_inner(input, Language::Auto)
    }

    /// Parse with an explicit language and retain resolution metadata.
    pub fn parse_detailed_with_language(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<Passage>, ParseError> {
        self.parse_detailed_inner(input, language)
    }

    /// Return `None` rather than an error for invalid input.
    #[must_use]
    pub fn try_parse(&self, input: &str) -> Option<Passage> {
        self.parse(input).ok()
    }

    /// Return `None` rather than an error when parsing with one language.
    #[must_use]
    pub fn try_parse_with_language(&self, input: &str, language: Language) -> Option<Passage> {
        self.parse_with_language(input, language).ok()
    }

    fn parse_detailed_inner(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<Passage>, ParseError> {
        let normalized = normalize_for_parsing(input);
        if normalized.is_empty() {
            return Err(ParseError::new(
                ParseErrorKind::EmptyReference,
                "passage must not be empty",
            ));
        }

        let source_segments = normalized.split(';').collect::<Vec<_>>();
        if source_segments
            .iter()
            .any(|segment| segment.trim().is_empty())
        {
            return Err(ParseError::new(
                ParseErrorKind::PatternMismatch,
                "passage sequence contains an empty expression",
            ));
        }

        let mut passages = Vec::with_capacity(source_segments.len());
        let mut book_matches = Vec::new();
        for segment in source_segments {
            let parsed = self.parse_segment(segment.trim(), language)?;
            passages.push(parsed.value);
            book_matches.extend(parsed.book_matches);
        }

        let passage = if passages.len() == 1 {
            passages
                .pop()
                .expect("one parsed source segment produces one passage")
        } else {
            Passage::Sequence(
                PassageSequence::new(passages)
                    .expect("multiple parsed segments form a non-empty sequence"),
            )
        };

        Ok(Parsed::new(
            passage,
            ParseMetadata::from_parts(normalized, book_matches),
        ))
    }

    fn parse_segment(&self, input: &str, language: Language) -> Result<ParsedSegment, ParseError> {
        match self.parse_book(input, language) {
            Ok((book, book_matches)) => {
                return Ok(ParsedSegment::new(
                    Passage::Book(BookPassage::new(book)),
                    book_matches,
                ));
            }
            Err(error) if error.kind() == ParseErrorKind::UnknownBook => {}
            Err(error) => return Err(error),
        }

        let split = self.split_book_and_body(input, language)?;
        let book = split.book;
        let body = split.body;
        let is_single_chapter = self.is_single_chapter_book(book);

        if is_ascii_number(body) {
            let component = if is_single_chapter {
                "verse"
            } else {
                "chapter"
            };
            let maximum = if is_single_chapter {
                MAX_VERSE_NUMBER
            } else {
                MAX_CHAPTER_NUMBER
            };
            let number = parse_number(body, component, maximum)?;
            let value = if is_single_chapter {
                let verse = VerseRef::new(book, 1, number)
                    .expect("validated chapter and verse coordinates are valid");
                Passage::Verses(
                    VersePassage::new([Reference::Verse(verse)])
                        .expect("a singleton verse passage is non-empty"),
                )
            } else {
                Passage::Chapter(
                    ChapterPassage::single(book, number)
                        .expect("the parsed chapter coordinate is valid"),
                )
            };
            return Ok(ParsedSegment::new(value, split.book_matches));
        }

        if let Some((start_token, end_token)) = exact_numeric_range(body) {
            let component = if is_single_chapter {
                "verse"
            } else {
                "chapter"
            };
            let maximum = if is_single_chapter {
                MAX_VERSE_NUMBER
            } else {
                MAX_CHAPTER_NUMBER
            };
            let start = parse_number(start_token, &format!("start {component}"), maximum)?;
            let end = parse_number(end_token, &format!("end {component}"), maximum)?;
            if end <= start {
                return Err(ParseError::new(
                    ParseErrorKind::SameBookRangeNotAscending,
                    format!("end {component} must come after start {component}"),
                ));
            }

            let value = if is_single_chapter {
                let start = VerseRef::new(book, 1, start)
                    .expect("validated chapter and verse coordinates are valid");
                let end = VerseRef::new(book, 1, end)
                    .expect("validated chapter and verse coordinates are valid");
                let range = VerseRange::new(start, end)
                    .expect("the parsed range was checked to be ascending");
                Passage::Verses(
                    VersePassage::new([Reference::Range(range)])
                        .expect("a singleton verse passage is non-empty"),
                )
            } else {
                Passage::Chapter(
                    ChapterPassage::range(book, start, end)
                        .expect("the parsed chapter range is valid and ascending"),
                )
            };
            return Ok(ParsedSegment::new(value, split.book_matches));
        }

        if body.contains(',') {
            return self.parse_verse_list(split);
        }

        let reference_input = format!("{} {body}", split.book_token);
        let parsed = self.parse_reference(&reference_input, language)?;
        let (reference, metadata) = parsed.into_parts();
        Ok(ParsedSegment::new(
            Passage::Verses(
                VersePassage::new([reference]).expect("a singleton verse passage is non-empty"),
            ),
            metadata.book_matches().to_vec(),
        ))
    }

    fn parse_verse_list(&self, split: BookAndBody<'_>) -> Result<ParsedSegment, ParseError> {
        let (chapter_token, selection_text) =
            split_coordinate_once(split.body).ok_or_else(|| {
                ParseError::new(
                    ParseErrorKind::PatternMismatch,
                    format!("verse list {:?} does not match expected format", split.body),
                )
            })?;
        let chapter = parse_number(chapter_token, "chapter", MAX_CHAPTER_NUMBER)?;
        if selection_text.is_empty() {
            return Err(ParseError::new(
                ParseErrorKind::PatternMismatch,
                "verse list contains an empty selection",
            ));
        }

        let tokens = selection_text.split(',').collect::<Vec<_>>();
        if tokens.iter().any(|token| token.trim().is_empty()) {
            return Err(ParseError::new(
                ParseErrorKind::PatternMismatch,
                "verse list contains an empty selection",
            ));
        }

        let selections = tokens
            .into_iter()
            .map(|token| parse_verse_selection(token.trim(), split.book, chapter))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ParsedSegment::new(
            Passage::Verses(
                VersePassage::new(selections)
                    .expect("a syntactically valid list contains at least one selection"),
            ),
            split.book_matches,
        ))
    }

    fn split_book_and_body<'a>(
        &self,
        input: &'a str,
        language: Language,
    ) -> Result<BookAndBody<'a>, ParseError> {
        let digit_starts = input
            .char_indices()
            .filter_map(|(index, character)| {
                (index > 0 && character.is_ascii_digit()).then_some(index)
            })
            .collect::<Vec<_>>();
        let mut last_unknown_book = None;

        for position in digit_starts.into_iter().rev() {
            let book_token = input[..position].trim();
            let body = input[position..].trim();
            if book_token.is_empty()
                || !body.starts_with(|character: char| character.is_ascii_digit())
            {
                continue;
            }
            match self.parse_book(book_token, language) {
                Ok((book, book_matches)) => {
                    return Ok(BookAndBody {
                        book_token,
                        body,
                        book,
                        book_matches,
                    });
                }
                Err(error) if error.kind() == ParseErrorKind::UnknownBook => {
                    last_unknown_book = Some(error);
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_unknown_book.unwrap_or_else(|| {
            ParseError::new(
                ParseErrorKind::PatternMismatch,
                format!("passage {input:?} does not match expected format"),
            )
        }))
    }

    fn parse_book(
        &self,
        input: &str,
        language: Language,
    ) -> Result<(Book, Vec<BookMatch>), ParseError> {
        let parsed = self
            .reference_parser
            .parse_book_detailed_with_language(input, language)?;
        let (book, metadata) = parsed.into_parts();
        Ok((book, metadata.book_matches().to_vec()))
    }

    fn parse_reference(
        &self,
        input: &str,
        language: Language,
    ) -> Result<Parsed<Reference>, ParseError> {
        self.reference_parser
            .parse_detailed_with_language(input, language)
    }
}

struct ParsedSegment {
    value: Passage,
    book_matches: Vec<BookMatch>,
}

impl ParsedSegment {
    fn new(value: Passage, book_matches: Vec<BookMatch>) -> Self {
        Self {
            value,
            book_matches,
        }
    }
}

struct BookAndBody<'a> {
    book_token: &'a str,
    body: &'a str,
    book: Book,
    book_matches: Vec<BookMatch>,
}

fn is_ascii_number(input: &str) -> bool {
    !input.is_empty() && input.bytes().all(|byte| byte.is_ascii_digit())
}

fn exact_numeric_range(input: &str) -> Option<(&str, &str)> {
    let (start, end) = input.split_once('-')?;
    let start = start.trim();
    let end = end.trim();
    (!end.contains('-') && is_ascii_number(start) && is_ascii_number(end)).then_some((start, end))
}

fn split_coordinate_once(input: &str) -> Option<(&str, &str)> {
    let separator = input
        .char_indices()
        .find(|(_, character)| matches!(character, ':' | '.'))?;
    let left = input[..separator.0].trim();
    let right = input[separator.0 + separator.1.len_utf8()..].trim();
    (is_ascii_number(left) && !right.is_empty()).then_some((left, right))
}

fn parse_verse_selection(
    input: &str,
    book: Book,
    default_chapter: u16,
) -> Result<Reference, ParseError> {
    let mut parts = input.split('-');
    let start_text = parts.next().expect("split always returns one part").trim();
    let end_text = parts.next().map(str::trim);
    if parts.next().is_some() || start_text.is_empty() || end_text == Some("") {
        return Err(ParseError::new(
            ParseErrorKind::PatternMismatch,
            format!("verse selection {input:?} does not match expected format"),
        ));
    }

    let (start_chapter, start_verse) =
        parse_selection_endpoint(start_text, default_chapter, "start")?;
    let start = VerseRef::new(book, start_chapter, start_verse)
        .expect("parsed chapter and verse coordinates are valid");

    let Some(end_text) = end_text else {
        return Ok(Reference::Verse(start));
    };
    let (end_chapter, end_verse) = parse_selection_endpoint(end_text, start_chapter, "end")?;
    let end = VerseRef::new(book, end_chapter, end_verse)
        .expect("parsed chapter and verse coordinates are valid");
    let range = VerseRange::new(start, end).map_err(|_| {
        ParseError::new(
            ParseErrorKind::SameBookRangeNotAscending,
            "end reference must come after start reference",
        )
    })?;
    Ok(Reference::Range(range))
}

fn parse_selection_endpoint(
    input: &str,
    default_chapter: u16,
    position: &str,
) -> Result<(u16, u16), ParseError> {
    if let Some((chapter, verse)) = split_coordinate_once(input) {
        if verse.contains([':', '.']) || !is_ascii_number(verse) {
            return Err(ParseError::new(
                ParseErrorKind::PatternMismatch,
                format!("verse selection endpoint {input:?} does not match expected format"),
            ));
        }
        return Ok((
            parse_number(chapter, &format!("{position} chapter"), MAX_CHAPTER_NUMBER)?,
            parse_number(verse, &format!("{position} verse"), MAX_VERSE_NUMBER)?,
        ));
    }

    if !is_ascii_number(input) {
        return Err(ParseError::new(
            ParseErrorKind::PatternMismatch,
            format!("verse selection endpoint {input:?} does not match expected format"),
        ));
    }
    Ok((
        default_chapter,
        parse_number(input, &format!("{position} verse"), MAX_VERSE_NUMBER)?,
    ))
}

fn parse_number(token: &str, component: &str, maximum: u16) -> Result<u16, ParseError> {
    let value = token.parse::<u32>().map_err(|_| {
        ParseError::new(
            ParseErrorKind::InvalidNumericToken,
            format!("{component} token {token:?} is not an integer"),
        )
    })?;
    if value == 0 {
        return Err(ParseError::new(
            ParseErrorKind::NonPositiveNumericToken,
            format!("{component} must be greater than zero"),
        ));
    }
    if value > u32::from(maximum) {
        return Err(ParseError::new(
            ParseErrorKind::NumericTokenOutOfRange,
            format!("{component} {value} exceeds the sanity limit {maximum}"),
        ));
    }
    Ok(value as u16)
}

fn format_compact_selection(
    reference: Reference,
    anchor: VerseRef,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    let start = reference.start();
    if start.book() != anchor.book() {
        return fmt::Display::fmt(&reference, formatter);
    }

    if start.chapter() == anchor.chapter() {
        write!(formatter, "{}", start.verse())?;
    } else {
        write!(formatter, "{}:{}", start.chapter(), start.verse())?;
    }

    let Reference::Range(range) = reference else {
        return Ok(());
    };
    let end = range.end();
    if end.book() != start.book() {
        write!(formatter, "-{end}")
    } else if end.chapter() == start.chapter() {
        write!(formatter, "-{}", end.verse())
    } else {
        write!(formatter, "-{}:{}", end.chapter(), end.verse())
    }
}

#[cfg(test)]
#[path = "../tests/unit/passage.rs"]
mod tests;
