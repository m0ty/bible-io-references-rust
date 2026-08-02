//! Serde support using the Dart package's stable JSON shapes.

use serde::{
    Deserialize, Deserializer, Serialize, Serializer, de::Error as _, ser::SerializeStruct,
};

use crate::{
    Book, BookCandidate, BookMatch, BookPassage, ChapterPassage, Language, ParseMetadata, Passage,
    PassageSequence, Reference, VersePassage, VerseRange, VerseRef,
};

impl Serialize for Book {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.abbreviation())
    }
}

impl<'de> Deserialize<'de> for Book {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let normalized = value.trim();
        Book::ALL
            .iter()
            .copied()
            .find(|book| {
                book.abbreviation().eq_ignore_ascii_case(normalized)
                    || book.full_name().eq_ignore_ascii_case(normalized)
                    || format!("{book:?}").eq_ignore_ascii_case(normalized)
            })
            .ok_or_else(|| D::Error::custom(format!("unknown Bible book: {value}")))
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(D::Error::custom)
    }
}

impl Serialize for BookCandidate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("BookCandidate", 4)?;
        state.serialize_field("book", &self.book())?;
        state.serialize_field("alias", self.alias())?;
        state.serialize_field("language", &self.language())?;
        state.serialize_field("custom", &self.is_custom())?;
        state.end()
    }
}

impl Serialize for BookMatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("BookMatch", 3)?;
        state.serialize_field("input", self.token())?;
        state.serialize_field("selected", self.selected())?;
        state.serialize_field("alternatives", self.alternatives())?;
        state.end()
    }
}

impl Serialize for ParseMetadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let detected_languages = self.detected_languages();
        let mut state = serializer.serialize_struct("ParseMetadata", 4)?;
        state.serialize_field("normalizedInput", self.normalized_input())?;
        state.serialize_field("detectedLanguage", &self.detected_language())?;
        state.serialize_field("detectedLanguages", &detected_languages)?;
        state.serialize_field("bookMatches", self.book_matches())?;
        state.end()
    }
}

impl Serialize for VerseRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("VerseRef", 4)?;
        state.serialize_field("type", "verse")?;
        state.serialize_field("book", &self.book())?;
        state.serialize_field("chapter", &self.chapter())?;
        state.serialize_field("verse", &self.verse())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for VerseRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match ReferenceWire::deserialize(deserializer)? {
            ReferenceWire::Verse {
                book,
                chapter,
                verse,
            } => VerseRef::new(book, chapter, verse).map_err(D::Error::custom),
            ReferenceWire::Range { .. } => {
                Err(D::Error::custom("expected a verse object, found a range"))
            }
        }
    }
}

impl Serialize for VerseRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("VerseRange", 3)?;
        state.serialize_field("type", "range")?;
        state.serialize_field("start", &self.start())?;
        state.serialize_field("end", &self.end())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for VerseRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match ReferenceWire::deserialize(deserializer)? {
            ReferenceWire::Range { start, end } => {
                let start = start.into_verse().map_err(D::Error::custom)?;
                let end = end.into_verse().map_err(D::Error::custom)?;
                VerseRange::new(start, end).map_err(D::Error::custom)
            }
            ReferenceWire::Verse { .. } => {
                Err(D::Error::custom("expected a range object, found a verse"))
            }
        }
    }
}

impl Serialize for Reference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Verse(verse) => verse.serialize(serializer),
            Self::Range(range) => range.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Reference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ReferenceWire::deserialize(deserializer)?
            .into_reference()
            .map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ReferenceWire {
    #[serde(rename = "verse")]
    Verse {
        book: Book,
        chapter: u16,
        verse: u16,
    },
    #[serde(rename = "range")]
    Range {
        start: Box<ReferenceWire>,
        end: Box<ReferenceWire>,
    },
}

impl ReferenceWire {
    fn into_verse(self) -> Result<VerseRef, String> {
        match self {
            Self::Verse {
                book,
                chapter,
                verse,
            } => VerseRef::new(book, chapter, verse).map_err(|error| error.to_string()),
            Self::Range { .. } => Err("range endpoints must be verse objects".to_owned()),
        }
    }

    fn into_reference(self) -> Result<Reference, String> {
        match self {
            Self::Verse { .. } => self.into_verse().map(Reference::Verse),
            Self::Range { start, end } => {
                let start = start.into_verse()?;
                let end = end.into_verse()?;
                VerseRange::new(start, end)
                    .map(Reference::Range)
                    .map_err(|error| error.to_string())
            }
        }
    }
}

impl Serialize for BookPassage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("BookPassage", 2)?;
        state.serialize_field("type", "book")?;
        state.serialize_field("book", &self.book())?;
        state.end()
    }
}

impl Serialize for ChapterPassage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ChapterPassage", 4)?;
        state.serialize_field("type", "chapter")?;
        state.serialize_field("book", &self.book())?;
        state.serialize_field("startChapter", &self.start_chapter())?;
        state.serialize_field("endChapter", &self.end_chapter())?;
        state.end()
    }
}

impl Serialize for VersePassage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("VersePassage", 2)?;
        state.serialize_field("type", "verses")?;
        state.serialize_field("selections", self.selections())?;
        state.end()
    }
}

impl Serialize for PassageSequence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("PassageSequence", 2)?;
        state.serialize_field("type", "sequence")?;
        state.serialize_field("passages", self.passages())?;
        state.end()
    }
}

impl Serialize for Passage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Book(passage) => passage.serialize(serializer),
            Self::Chapter(passage) => passage.serialize(serializer),
            Self::Verses(passage) => passage.serialize(serializer),
            Self::Sequence(passage) => passage.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Passage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match PassageWire::deserialize(deserializer)? {
            PassageWire::Book { book } => Ok(Self::Book(BookPassage::new(book))),
            PassageWire::Chapter {
                book,
                start_chapter,
                end_chapter,
            } => ChapterPassage::new(book, start_chapter, end_chapter)
                .map(Self::Chapter)
                .map_err(D::Error::custom),
            PassageWire::Verses { selections } => VersePassage::new(selections)
                .map(Self::Verses)
                .map_err(D::Error::custom),
            PassageWire::Sequence { passages } => PassageSequence::new(passages)
                .map(Self::Sequence)
                .map_err(D::Error::custom),
        }
    }
}

macro_rules! deserialize_passage_variant {
    ($type:ty, $variant:ident, $expected:literal) => {
        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                match Passage::deserialize(deserializer)? {
                    Passage::$variant(value) => Ok(value),
                    _ => Err(D::Error::custom(concat!(
                        "expected a ",
                        $expected,
                        " passage"
                    ))),
                }
            }
        }
    };
}

deserialize_passage_variant!(BookPassage, Book, "book");
deserialize_passage_variant!(ChapterPassage, Chapter, "chapter");
deserialize_passage_variant!(VersePassage, Verses, "verses");
deserialize_passage_variant!(PassageSequence, Sequence, "sequence");

#[derive(Deserialize)]
#[serde(tag = "type")]
enum PassageWire {
    #[serde(rename = "book")]
    Book { book: Book },
    #[serde(rename = "chapter")]
    Chapter {
        book: Book,
        #[serde(rename = "startChapter")]
        start_chapter: u16,
        #[serde(rename = "endChapter")]
        end_chapter: Option<u16>,
    },
    #[serde(rename = "verses")]
    Verses { selections: Vec<Reference> },
    #[serde(rename = "sequence")]
    Sequence { passages: Vec<Passage> },
}

#[cfg(test)]
#[path = "../tests/unit/serde_impl.rs"]
mod tests;
