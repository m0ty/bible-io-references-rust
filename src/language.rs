//! Language identifiers and bundled parsing-support introspection.

use std::{error::Error, fmt, str::FromStr};

/// A language understood by the reference parser.
///
/// [`Language::Auto`] searches all bundled languages. The remaining variants
/// represent concrete languages, including languages for which callers may
/// register their own aliases even when this crate does not bundle book names.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Language {
    /// Search every language with bundled book aliases.
    #[default]
    Auto,
    /// Arabic (`ar`).
    Arabic,
    /// Chinese (`zh`).
    Chinese,
    /// English (`en`).
    English,
    /// Esperanto (`eo`), available for custom aliases.
    Esperanto,
    /// Finnish (`fi`), available for custom aliases.
    Finnish,
    /// French (`fr`).
    French,
    /// German (`de`).
    German,
    /// Greek (`el`), available for custom aliases.
    Greek,
    /// Hebrew (`he`).
    Hebrew,
    /// Hindi (`hi`).
    Hindi,
    /// Indonesian (`id`).
    Indonesian,
    /// Korean (`ko`).
    Korean,
    /// Portuguese (`pt`).
    Portuguese,
    /// Romanian (`ro`), available for custom aliases.
    Romanian,
    /// Russian (`ru`).
    Russian,
    /// Spanish (`es`).
    Spanish,
    /// Tagalog (`tl`).
    Tagalog,
    /// Vietnamese (`vi`), available for custom aliases.
    Vietnamese,
}

impl Language {
    /// Every language and parser mode, in stable declaration order.
    pub const ALL: [Self; 19] = [
        Self::Auto,
        Self::Arabic,
        Self::Chinese,
        Self::English,
        Self::Esperanto,
        Self::Finnish,
        Self::French,
        Self::German,
        Self::Greek,
        Self::Hebrew,
        Self::Hindi,
        Self::Indonesian,
        Self::Korean,
        Self::Portuguese,
        Self::Romanian,
        Self::Russian,
        Self::Spanish,
        Self::Tagalog,
        Self::Vietnamese,
    ];

    /// Languages and parser modes backed by bundled book aliases.
    pub const SUPPORTED: [Self; 14] = [
        Self::Auto,
        Self::Arabic,
        Self::Chinese,
        Self::English,
        Self::French,
        Self::German,
        Self::Hebrew,
        Self::Hindi,
        Self::Indonesian,
        Self::Korean,
        Self::Portuguese,
        Self::Russian,
        Self::Spanish,
        Self::Tagalog,
    ];

    /// The human-readable English name of this language.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Arabic => "Arabic",
            Self::Chinese => "Chinese",
            Self::English => "English",
            Self::Esperanto => "Esperanto",
            Self::Finnish => "Finnish",
            Self::French => "French",
            Self::German => "German",
            Self::Greek => "Greek",
            Self::Hebrew => "Hebrew",
            Self::Hindi => "Hindi",
            Self::Indonesian => "Indonesian",
            Self::Korean => "Korean",
            Self::Portuguese => "Portuguese",
            Self::Romanian => "Romanian",
            Self::Russian => "Russian",
            Self::Spanish => "Spanish",
            Self::Tagalog => "Tagalog",
            Self::Vietnamese => "Vietnamese",
        }
    }

    /// The canonical ISO 639-1 code, or `"auto"` for auto-detection.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Arabic => "ar",
            Self::Chinese => "zh",
            Self::English => "en",
            Self::Esperanto => "eo",
            Self::Finnish => "fi",
            Self::French => "fr",
            Self::German => "de",
            Self::Greek => "el",
            Self::Hebrew => "he",
            Self::Hindi => "hi",
            Self::Indonesian => "id",
            Self::Korean => "ko",
            Self::Portuguese => "pt",
            Self::Romanian => "ro",
            Self::Russian => "ru",
            Self::Spanish => "es",
            Self::Tagalog => "tl",
            Self::Vietnamese => "vi",
        }
    }

    /// The ISO 639-2 identifier accepted for this language.
    #[must_use]
    pub const fn identifier_prefix(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Arabic => "arb",
            Self::Chinese => "zho",
            Self::English => "eng",
            Self::Esperanto => "epo",
            Self::Finnish => "fin",
            Self::French => "fra",
            Self::German => "deu",
            Self::Greek => "ell",
            Self::Hebrew => "heb",
            Self::Hindi => "hin",
            Self::Indonesian => "ind",
            Self::Korean => "kor",
            Self::Portuguese => "por",
            Self::Romanian => "ron",
            Self::Russian => "rus",
            Self::Spanish => "spa",
            Self::Tagalog => "tgl",
            Self::Vietnamese => "vie",
        }
    }

    /// Supplemental identifiers in addition to the name and ISO codes.
    #[must_use]
    pub const fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::Auto => &["all", "global"],
            Self::Tagalog => &["fil"],
            _ => &[],
        }
    }

    /// Every accepted identifier for this language.
    ///
    /// This includes the lowercase variant name, display name, two-letter code,
    /// three-letter identifier, and supplemental aliases.
    pub fn all_aliases(self) -> impl Iterator<Item = &'static str> {
        [
            self.variant_name(),
            self.display_name(),
            self.code(),
            self.identifier_prefix(),
        ]
        .into_iter()
        .chain(self.aliases().iter().copied())
    }

    /// Whether the crate bundles parsing aliases for this language or mode.
    #[must_use]
    pub const fn is_parsing_supported(self) -> bool {
        matches!(
            self,
            Self::Auto
                | Self::Arabic
                | Self::Chinese
                | Self::English
                | Self::French
                | Self::German
                | Self::Hebrew
                | Self::Hindi
                | Self::Indonesian
                | Self::Korean
                | Self::Portuguese
                | Self::Russian
                | Self::Spanish
                | Self::Tagalog
        )
    }

    /// Whether this is the auto-detection parser mode rather than a language.
    #[must_use]
    pub const fn is_auto(self) -> bool {
        matches!(self, Self::Auto)
    }

    const fn variant_name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Arabic => "arabic",
            Self::Chinese => "chinese",
            Self::English => "english",
            Self::Esperanto => "esperanto",
            Self::Finnish => "finnish",
            Self::French => "french",
            Self::German => "german",
            Self::Greek => "greek",
            Self::Hebrew => "hebrew",
            Self::Hindi => "hindi",
            Self::Indonesian => "indonesian",
            Self::Korean => "korean",
            Self::Portuguese => "portuguese",
            Self::Romanian => "romanian",
            Self::Russian => "russian",
            Self::Spanish => "spanish",
            Self::Tagalog => "tagalog",
            Self::Vietnamese => "vietnamese",
        }
    }

    fn matches_identifier(self, identifier: &str) -> bool {
        self.all_aliases()
            .any(|alias| alias.eq_ignore_ascii_case(identifier))
    }
}

impl fmt::Display for Language {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}

impl FromStr for Language {
    type Err = ParseLanguageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim();
        if normalized.is_empty() {
            return Err(ParseLanguageError::new(value));
        }

        let prefix = normalized
            .split(['-', '_'])
            .next()
            .expect("a non-empty string always has a first segment");

        Self::ALL
            .into_iter()
            .find(|language| {
                language.matches_identifier(normalized) || language.matches_identifier(prefix)
            })
            .ok_or_else(|| ParseLanguageError::new(value))
    }
}

impl TryFrom<&str> for Language {
    type Error = ParseLanguageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl AsRef<str> for Language {
    fn as_ref(&self) -> &str {
        self.code()
    }
}

/// Returned when a language name, code, alias, or locale tag is not known.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseLanguageError {
    input: String,
}

impl ParseLanguageError {
    fn new(input: &str) -> Self {
        Self {
            input: input.to_owned(),
        }
    }

    /// The unmodified input that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for ParseLanguageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.input.trim().is_empty() {
            formatter.write_str("language must be a non-empty string")
        } else {
            write!(formatter, "unknown language: {}", self.input)
        }
    }
}

impl Error for ParseLanguageError {}

#[cfg(test)]
#[path = "../tests/unit/language.rs"]
mod tests;
