//! Canonical Bible book names and standard machine-readable identifiers.

use core::{fmt, str::FromStr};

macro_rules! define_books {
    (
        $(
            $variant:ident => {
                abbreviation: $abbreviation:literal,
                full_name: $full_name:literal,
                osis: $osis:literal,
                usfm: $usfm:literal,
            }
        ),+ $(,)?
    ) => {
        /// A Bible book supported by this crate.
        ///
        /// Variants are ordered canonically: the 66-book Protestant canon,
        /// Catholic deuterocanonical books, then Eastern Orthodox additions.
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(u8)]
        pub enum Book {
            $(
                #[doc = $full_name]
                $variant
            ),+
        }

        impl Book {
            /// Every supported book in canonical order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// Return the package's compact, lowercase abbreviation.
            #[must_use]
            pub const fn abbreviation(self) -> &'static str {
                match self {
                    $(Self::$variant => $abbreviation),+
                }
            }

            /// Return the canonical English display name.
            #[must_use]
            pub const fn full_name(self) -> &'static str {
                match self {
                    $(Self::$variant => $full_name),+
                }
            }

            /// Return the case-sensitive OSIS book identifier.
            #[must_use]
            pub const fn osis(self) -> &'static str {
                match self {
                    $(Self::$variant => $osis),+
                }
            }

            /// Return the case-sensitive USFM book identifier.
            #[must_use]
            pub const fn usfm(self) -> &'static str {
                match self {
                    $(Self::$variant => $usfm),+
                }
            }

            /// Resolve an exact compact abbreviation, ignoring only ASCII case.
            ///
            /// Surrounding whitespace and alternate abbreviations are rejected.
            #[must_use]
            pub fn from_abbreviation(value: &str) -> Option<Self> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|book| book.abbreviation().eq_ignore_ascii_case(value))
            }

            /// Resolve an exact, case-sensitive OSIS book identifier.
            #[must_use]
            pub fn from_osis(value: &str) -> Option<Self> {
                match value {
                    $($osis => Some(Self::$variant),)+
                    _ => None,
                }
            }

            /// Resolve an exact, case-sensitive USFM book identifier.
            #[must_use]
            pub fn from_usfm(value: &str) -> Option<Self> {
                match value {
                    $($usfm => Some(Self::$variant),)+
                    _ => None,
                }
            }

            fn from_full_name(value: &str) -> Option<Self> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|book| book.full_name().eq_ignore_ascii_case(value))
            }
        }
    };
}

define_books! {
    Genesis => {
        abbreviation: "gn",
        full_name: "Genesis",
        osis: "Gen",
        usfm: "GEN",
    },
    Exodus => {
        abbreviation: "ex",
        full_name: "Exodus",
        osis: "Exod",
        usfm: "EXO",
    },
    Leviticus => {
        abbreviation: "lv",
        full_name: "Leviticus",
        osis: "Lev",
        usfm: "LEV",
    },
    Numbers => {
        abbreviation: "nm",
        full_name: "Numbers",
        osis: "Num",
        usfm: "NUM",
    },
    Deuteronomy => {
        abbreviation: "dt",
        full_name: "Deuteronomy",
        osis: "Deut",
        usfm: "DEU",
    },
    Joshua => {
        abbreviation: "js",
        full_name: "Joshua",
        osis: "Josh",
        usfm: "JOS",
    },
    Judges => {
        abbreviation: "jud",
        full_name: "Judges",
        osis: "Judg",
        usfm: "JDG",
    },
    Ruth => {
        abbreviation: "rt",
        full_name: "Ruth",
        osis: "Ruth",
        usfm: "RUT",
    },
    FirstSamuel => {
        abbreviation: "1sm",
        full_name: "1 Samuel",
        osis: "1Sam",
        usfm: "1SA",
    },
    SecondSamuel => {
        abbreviation: "2sm",
        full_name: "2 Samuel",
        osis: "2Sam",
        usfm: "2SA",
    },
    FirstKings => {
        abbreviation: "1kgs",
        full_name: "1 Kings",
        osis: "1Kgs",
        usfm: "1KI",
    },
    SecondKings => {
        abbreviation: "2kgs",
        full_name: "2 Kings",
        osis: "2Kgs",
        usfm: "2KI",
    },
    FirstChronicles => {
        abbreviation: "1ch",
        full_name: "1 Chronicles",
        osis: "1Chr",
        usfm: "1CH",
    },
    SecondChronicles => {
        abbreviation: "2ch",
        full_name: "2 Chronicles",
        osis: "2Chr",
        usfm: "2CH",
    },
    Ezra => {
        abbreviation: "ezr",
        full_name: "Ezra",
        osis: "Ezra",
        usfm: "EZR",
    },
    Nehemiah => {
        abbreviation: "ne",
        full_name: "Nehemiah",
        osis: "Neh",
        usfm: "NEH",
    },
    Esther => {
        abbreviation: "et",
        full_name: "Esther",
        osis: "Esth",
        usfm: "EST",
    },
    Job => {
        abbreviation: "job",
        full_name: "Job",
        osis: "Job",
        usfm: "JOB",
    },
    Psalms => {
        abbreviation: "ps",
        full_name: "Psalms",
        osis: "Ps",
        usfm: "PSA",
    },
    Proverbs => {
        abbreviation: "prv",
        full_name: "Proverbs",
        osis: "Prov",
        usfm: "PRO",
    },
    Ecclesiastes => {
        abbreviation: "ec",
        full_name: "Ecclesiastes",
        osis: "Eccl",
        usfm: "ECC",
    },
    SongOfSolomon => {
        abbreviation: "so",
        full_name: "Song of Solomon",
        osis: "Song",
        usfm: "SNG",
    },
    Isaiah => {
        abbreviation: "is",
        full_name: "Isaiah",
        osis: "Isa",
        usfm: "ISA",
    },
    Jeremiah => {
        abbreviation: "jr",
        full_name: "Jeremiah",
        osis: "Jer",
        usfm: "JER",
    },
    Lamentations => {
        abbreviation: "lm",
        full_name: "Lamentations",
        osis: "Lam",
        usfm: "LAM",
    },
    Ezekiel => {
        abbreviation: "ez",
        full_name: "Ezekiel",
        osis: "Ezek",
        usfm: "EZK",
    },
    Daniel => {
        abbreviation: "dn",
        full_name: "Daniel",
        osis: "Dan",
        usfm: "DAN",
    },
    Hosea => {
        abbreviation: "ho",
        full_name: "Hosea",
        osis: "Hos",
        usfm: "HOS",
    },
    Joel => {
        abbreviation: "jl",
        full_name: "Joel",
        osis: "Joel",
        usfm: "JOL",
    },
    Amos => {
        abbreviation: "am",
        full_name: "Amos",
        osis: "Amos",
        usfm: "AMO",
    },
    Obadiah => {
        abbreviation: "ob",
        full_name: "Obadiah",
        osis: "Obad",
        usfm: "OBA",
    },
    Jonah => {
        abbreviation: "jn",
        full_name: "Jonah",
        osis: "Jonah",
        usfm: "JON",
    },
    Micah => {
        abbreviation: "mi",
        full_name: "Micah",
        osis: "Mic",
        usfm: "MIC",
    },
    Nahum => {
        abbreviation: "na",
        full_name: "Nahum",
        osis: "Nah",
        usfm: "NAM",
    },
    Habakkuk => {
        abbreviation: "hk",
        full_name: "Habakkuk",
        osis: "Hab",
        usfm: "HAB",
    },
    Zephaniah => {
        abbreviation: "zp",
        full_name: "Zephaniah",
        osis: "Zeph",
        usfm: "ZEP",
    },
    Haggai => {
        abbreviation: "hg",
        full_name: "Haggai",
        osis: "Hag",
        usfm: "HAG",
    },
    Zechariah => {
        abbreviation: "zc",
        full_name: "Zechariah",
        osis: "Zech",
        usfm: "ZEC",
    },
    Malachi => {
        abbreviation: "ml",
        full_name: "Malachi",
        osis: "Mal",
        usfm: "MAL",
    },
    Matthew => {
        abbreviation: "mt",
        full_name: "Matthew",
        osis: "Matt",
        usfm: "MAT",
    },
    Mark => {
        abbreviation: "mk",
        full_name: "Mark",
        osis: "Mark",
        usfm: "MRK",
    },
    Luke => {
        abbreviation: "lk",
        full_name: "Luke",
        osis: "Luke",
        usfm: "LUK",
    },
    John => {
        abbreviation: "jo",
        full_name: "John",
        osis: "John",
        usfm: "JHN",
    },
    Acts => {
        abbreviation: "act",
        full_name: "Acts",
        osis: "Acts",
        usfm: "ACT",
    },
    Romans => {
        abbreviation: "rm",
        full_name: "Romans",
        osis: "Rom",
        usfm: "ROM",
    },
    FirstCorinthians => {
        abbreviation: "1co",
        full_name: "1 Corinthians",
        osis: "1Cor",
        usfm: "1CO",
    },
    SecondCorinthians => {
        abbreviation: "2co",
        full_name: "2 Corinthians",
        osis: "2Cor",
        usfm: "2CO",
    },
    Galatians => {
        abbreviation: "gl",
        full_name: "Galatians",
        osis: "Gal",
        usfm: "GAL",
    },
    Ephesians => {
        abbreviation: "eph",
        full_name: "Ephesians",
        osis: "Eph",
        usfm: "EPH",
    },
    Philippians => {
        abbreviation: "ph",
        full_name: "Philippians",
        osis: "Phil",
        usfm: "PHP",
    },
    Colossians => {
        abbreviation: "cl",
        full_name: "Colossians",
        osis: "Col",
        usfm: "COL",
    },
    FirstThessalonians => {
        abbreviation: "1ts",
        full_name: "1 Thessalonians",
        osis: "1Thess",
        usfm: "1TH",
    },
    SecondThessalonians => {
        abbreviation: "2ts",
        full_name: "2 Thessalonians",
        osis: "2Thess",
        usfm: "2TH",
    },
    FirstTimothy => {
        abbreviation: "1tm",
        full_name: "1 Timothy",
        osis: "1Tim",
        usfm: "1TI",
    },
    SecondTimothy => {
        abbreviation: "2tm",
        full_name: "2 Timothy",
        osis: "2Tim",
        usfm: "2TI",
    },
    Titus => {
        abbreviation: "tt",
        full_name: "Titus",
        osis: "Titus",
        usfm: "TIT",
    },
    Philemon => {
        abbreviation: "phm",
        full_name: "Philemon",
        osis: "Phlm",
        usfm: "PHM",
    },
    Hebrews => {
        abbreviation: "hb",
        full_name: "Hebrews",
        osis: "Heb",
        usfm: "HEB",
    },
    James => {
        abbreviation: "jm",
        full_name: "James",
        osis: "Jas",
        usfm: "JAS",
    },
    FirstPeter => {
        abbreviation: "1pe",
        full_name: "1 Peter",
        osis: "1Pet",
        usfm: "1PE",
    },
    SecondPeter => {
        abbreviation: "2pe",
        full_name: "2 Peter",
        osis: "2Pet",
        usfm: "2PE",
    },
    FirstJohn => {
        abbreviation: "1jo",
        full_name: "1 John",
        osis: "1John",
        usfm: "1JN",
    },
    SecondJohn => {
        abbreviation: "2jo",
        full_name: "2 John",
        osis: "2John",
        usfm: "2JN",
    },
    ThirdJohn => {
        abbreviation: "3jo",
        full_name: "3 John",
        osis: "3John",
        usfm: "3JN",
    },
    Jude => {
        abbreviation: "jd",
        full_name: "Jude",
        osis: "Jude",
        usfm: "JUD",
    },
    Revelation => {
        abbreviation: "re",
        full_name: "Revelation",
        osis: "Rev",
        usfm: "REV",
    },
    Tobit => {
        abbreviation: "tb",
        full_name: "Tobit",
        osis: "Tob",
        usfm: "TOB",
    },
    Judith => {
        abbreviation: "jdt",
        full_name: "Judith",
        osis: "Jdt",
        usfm: "JDT",
    },
    Wisdom => {
        abbreviation: "ws",
        full_name: "Wisdom",
        osis: "Wis",
        usfm: "WIS",
    },
    Sirach => {
        abbreviation: "sir",
        full_name: "Sirach",
        osis: "Sir",
        usfm: "SIR",
    },
    Baruch => {
        abbreviation: "bar",
        full_name: "Baruch",
        osis: "Bar",
        usfm: "BAR",
    },
    FirstMaccabees => {
        abbreviation: "1mc",
        full_name: "1 Maccabees",
        osis: "1Macc",
        usfm: "1MA",
    },
    SecondMaccabees => {
        abbreviation: "2mc",
        full_name: "2 Maccabees",
        osis: "2Macc",
        usfm: "2MA",
    },
    EstherAdditions => {
        abbreviation: "etg",
        full_name: "Esther (Greek)",
        osis: "AddEsth",
        usfm: "ADE",
    },
    DanielSongOfThree => {
        abbreviation: "dn3",
        full_name: "Daniel (Song of Three)",
        osis: "PrAzar",
        usfm: "S3Y",
    },
    DanielSusanna => {
        abbreviation: "dns",
        full_name: "Daniel (Susanna)",
        osis: "Sus",
        usfm: "SUS",
    },
    DanielBelAndTheDragon => {
        abbreviation: "dnb",
        full_name: "Daniel (Bel and the Dragon)",
        osis: "Bel",
        usfm: "BEL",
    },
    FirstEsdras => {
        abbreviation: "1es",
        full_name: "1 Esdras",
        osis: "1Esd",
        usfm: "1ES",
    },
    SecondEsdras => {
        abbreviation: "2es",
        full_name: "2 Esdras",
        osis: "2Esd",
        usfm: "2ES",
    },
    PrayerOfManasseh => {
        abbreviation: "pmn",
        full_name: "Prayer of Manasseh",
        osis: "PrMan",
        usfm: "MAN",
    },
    Psalm151 => {
        abbreviation: "ps151",
        full_name: "Psalm 151",
        osis: "AddPs",
        usfm: "PS2",
    },
    ThirdMaccabees => {
        abbreviation: "3mc",
        full_name: "3 Maccabees",
        osis: "3Macc",
        usfm: "3MA",
    },
    FourthMaccabees => {
        abbreviation: "4mc",
        full_name: "4 Maccabees",
        osis: "4Macc",
        usfm: "4MA",
    },
}

impl fmt::Display for Book {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.full_name())
    }
}

/// Error returned when a string does not name a supported Bible book.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseBookError {
    input: String,
}

impl ParseBookError {
    /// Return the rejected input.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for ParseBookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unrecognized Bible book: {:?}", self.input)
    }
}

impl std::error::Error for ParseBookError {}

impl FromStr for Book {
    type Err = ParseBookError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Human-facing compact codes take precedence over machine formats.
        // Strict OSIS/USFM callers should use their dedicated lookup APIs.
        Self::from_abbreviation(value)
            .or_else(|| Self::from_full_name(value))
            .or_else(|| Self::from_osis(value))
            .or_else(|| Self::from_usfm(value))
            .ok_or_else(|| ParseBookError {
                input: value.to_owned(),
            })
    }
}

#[cfg(test)]
#[path = "../tests/unit/book.rs"]
mod tests;
