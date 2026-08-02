//! Command-line interface for parsing Bible references and passages.

#![forbid(unsafe_code)]
#![warn(clippy::all)]

use std::{
    env,
    fmt::Write as _,
    fs::File,
    io::{self, BufRead, BufReader, Write as IoWrite},
    process::ExitCode,
    str::FromStr,
};

use bible_io_references::{
    Language, MachineIdentifiers, ParseError, Passage, PassageParser, Reference,
    ReferenceFormatter, VerseRef,
};

const SUCCESS: u8 = 0;
const USAGE_ERROR: u8 = 64;
const DATA_ERROR: u8 = 65;
const NO_INPUT_ERROR: u8 = 66;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let standard_input = io::stdin();
    let standard_output = io::stdout();
    let standard_error = io::stderr();

    let code = run_cli(
        &arguments,
        &mut standard_input.lock(),
        &mut standard_output.lock(),
        &mut standard_error.lock(),
    );
    ExitCode::from(code)
}

fn run_cli(
    arguments: &[String],
    standard_input: &mut dyn BufRead,
    output: &mut dyn IoWrite,
    errors: &mut dyn IoWrite,
) -> u8 {
    if arguments
        .iter()
        .any(|argument| matches!(argument.as_str(), "--help" | "-h"))
    {
        return print_usage(output).map_or(NO_INPUT_ERROR, |()| SUCCESS);
    }
    if arguments
        .iter()
        .any(|argument| matches!(argument.as_str(), "--version" | "-V"))
    {
        return writeln!(output, "bible-io-references {}", env!("CARGO_PKG_VERSION"))
            .map_or(NO_INPUT_ERROR, |()| SUCCESS);
    }

    let options = match Options::parse(arguments) {
        Ok(options) => options,
        Err(message) => return usage_error(&message, errors),
    };

    let parser = PassageParser::new();
    if let Some(path) = options.input_path.as_deref() {
        match File::open(path) {
            Ok(file) => {
                let mut input = BufReader::new(file);
                match run_batch(&parser, &mut input, &options, output, errors) {
                    Ok(code) => code,
                    Err(error) => input_error(Some(path), &error, errors),
                }
            }
            Err(error) => input_error(Some(path), &error, errors),
        }
    } else if options.batch {
        match run_batch(&parser, standard_input, &options, output, errors) {
            Ok(code) => code,
            Err(error) => input_error(None, &error, errors),
        }
    } else {
        let input = options.input_parts.join(" ");
        run_single(&parser, &input, &options, output, errors)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
    Osis,
    Usfm,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "osis" => Ok(Self::Osis),
            "usfm" => Ok(Self::Usfm),
            _ => Err(format!(
                "Unsupported format {value:?}. Use text, json, osis, or usfm."
            )),
        }
    }
}

#[derive(Debug)]
struct Options {
    language: Option<Language>,
    output_format: OutputFormat,
    batch: bool,
    input_path: Option<String>,
    input_parts: Vec<String>,
}

impl Options {
    fn parse(arguments: &[String]) -> Result<Self, String> {
        let mut language = None;
        let mut output_format = OutputFormat::Text;
        let mut batch = false;
        let mut input_path = None;
        let mut input_parts = Vec::new();
        let mut index = 0;

        while index < arguments.len() {
            let argument = &arguments[index];
            match argument.as_str() {
                "--language" | "-l" => {
                    index += 1;
                    let value = arguments
                        .get(index)
                        .ok_or_else(|| format!("Missing value for {argument}."))?;
                    language = Some(parse_language(value)?);
                }
                "--format" | "-f" => {
                    index += 1;
                    let value = arguments
                        .get(index)
                        .ok_or_else(|| format!("Missing value for {argument}."))?;
                    output_format = value.parse()?;
                }
                "--batch" | "-b" => batch = true,
                "--input" | "-i" => {
                    index += 1;
                    input_path = Some(
                        arguments
                            .get(index)
                            .ok_or_else(|| format!("Missing value for {argument}."))?
                            .clone(),
                    );
                }
                _ if argument.starts_with("--language=") => {
                    language = Some(parse_language(&argument[11..])?);
                }
                _ if argument.starts_with("--format=") => {
                    output_format = argument[9..].parse()?;
                }
                _ if argument.starts_with("--input=") => {
                    let value = &argument[8..];
                    if value.is_empty() {
                        return Err("Missing value for --input.".to_owned());
                    }
                    input_path = Some(value.to_owned());
                }
                _ if argument.starts_with('-') => {
                    return Err(format!("Unknown option: {argument}"));
                }
                _ => input_parts.push(argument.clone()),
            }
            index += 1;
        }

        if let Some(language) = language {
            if !language.is_parsing_supported() {
                return Err(format!(
                    "Language {:?} has no registered parser data.",
                    language.code()
                ));
            }
        }
        if batch && input_path.is_some() {
            return Err("--batch and --input cannot be used together.".to_owned());
        }

        let is_batch = batch || input_path.is_some();
        if is_batch && !input_parts.is_empty() {
            return Err("A positional reference cannot be combined with batch input.".to_owned());
        }
        if !is_batch && input_parts.is_empty() {
            return Err("A Bible reference is required.".to_owned());
        }

        Ok(Self {
            language,
            output_format,
            batch,
            input_path,
            input_parts,
        })
    }
}

fn parse_language(value: &str) -> Result<Language, String> {
    value.parse::<Language>().map_err(|error| error.to_string())
}

struct ParsedInput {
    passage: Passage,
    narrow_reference: Option<Reference>,
}

fn parse_input(
    parser: &PassageParser,
    input: &str,
    language: Option<Language>,
) -> Result<ParsedInput, ParseError> {
    let passage = match language {
        Some(language) => parser.parse_with_language(input, language)?,
        None => parser.parse(input)?,
    };

    // Preserve the Dart CLI's legacy JSON shape for inputs that are also
    // valid narrow references, while parsing every expression as a Passage.
    let narrow_reference = match language {
        Some(language) => parser
            .reference_parser()
            .parse_with_language(input, language)
            .ok(),
        None => parser.reference_parser().parse(input).ok(),
    };

    Ok(ParsedInput {
        passage,
        narrow_reference,
    })
}

fn run_single(
    parser: &PassageParser,
    input: &str,
    options: &Options,
    output: &mut dyn IoWrite,
    errors: &mut dyn IoWrite,
) -> u8 {
    match parse_input(parser, input, options.language) {
        Ok(parsed) => {
            let rendered = render_parsed(&parsed, options);
            writeln!(output, "{rendered}").map_or(NO_INPUT_ERROR, |()| SUCCESS)
        }
        Err(error) => {
            let written = if options.output_format == OutputFormat::Json {
                writeln!(output, "{}", error_json(input, &error, None))
            } else {
                write_text_diagnostic(errors, input, &error, None)
            };
            written.map_or(NO_INPUT_ERROR, |()| DATA_ERROR)
        }
    }
}

fn run_batch(
    parser: &PassageParser,
    source: &mut dyn BufRead,
    options: &Options,
    output: &mut dyn IoWrite,
    errors: &mut dyn IoWrite,
) -> io::Result<u8> {
    let mut had_failure = false;
    let mut line_number = 0;
    let mut raw_line = String::new();

    loop {
        raw_line.clear();
        if source.read_line(&mut raw_line)? == 0 {
            break;
        }
        line_number += 1;
        let input = raw_line.trim();
        if input.is_empty() {
            continue;
        }

        match parse_input(parser, input, options.language) {
            Ok(parsed) => {
                let written = if options.output_format == OutputFormat::Json {
                    writeln!(
                        output,
                        "{}",
                        batch_success_json(line_number, input, &parsed)
                    )
                } else {
                    writeln!(output, "{}", render_parsed(&parsed, options))
                };
                if written.is_err() {
                    return Ok(NO_INPUT_ERROR);
                }
            }
            Err(error) => {
                had_failure = true;
                let written = if options.output_format == OutputFormat::Json {
                    writeln!(output, "{}", error_json(input, &error, Some(line_number)))
                } else {
                    write_text_diagnostic(errors, input, &error, Some(line_number))
                };
                if written.is_err() {
                    return Ok(NO_INPUT_ERROR);
                }
            }
        }
    }

    Ok(if had_failure { DATA_ERROR } else { SUCCESS })
}

fn render_parsed(parsed: &ParsedInput, options: &Options) -> String {
    match options.output_format {
        OutputFormat::Text => {
            ReferenceFormatter::for_language(options.language.unwrap_or(Language::English))
                .format_passage(&parsed.passage)
                .to_string()
        }
        OutputFormat::Json => parsed
            .narrow_reference
            .map_or_else(|| passage_json(&parsed.passage), reference_json),
        OutputFormat::Osis => parsed.passage.osis_identifier(),
        OutputFormat::Usfm => parsed.passage.usfm_identifier(),
    }
}

fn write_text_diagnostic(
    output: &mut dyn IoWrite,
    input: &str,
    error: &ParseError,
    line_number: Option<usize>,
) -> io::Result<()> {
    if let Some(line_number) = line_number {
        write!(output, "Line {line_number}: ")?;
    }
    writeln!(output, "Unable to parse {input:?} ({}).", error.code())?;
    writeln!(output, "{}", error.details())
}

fn usage_error(message: &str, output: &mut dyn IoWrite) -> u8 {
    writeln!(output, "{message}")
        .and_then(|()| print_usage(output))
        .map_or(NO_INPUT_ERROR, |()| USAGE_ERROR)
}

fn input_error(path: Option<&str>, error: &io::Error, output: &mut dyn IoWrite) -> u8 {
    let source = path.map_or_else(|| "standard input".to_owned(), |path| format!("{path:?}"));
    if error.kind() == io::ErrorKind::InvalidData {
        let _ = writeln!(output, "Unable to read {source}: input is not valid UTF-8.");
    } else {
        let message = error.to_string();
        let suffix = if message.ends_with('.') { "" } else { "." };
        let _ = writeln!(output, "Unable to read {source}: {message}{suffix}");
    }
    NO_INPUT_ERROR
}

fn print_usage(output: &mut dyn IoWrite) -> io::Result<()> {
    writeln!(output, "Usage:")?;
    writeln!(
        output,
        "  bible-io-references [--language CODE] [--format text|json|osis|usfm] \"John 3:16\""
    )?;
    writeln!(
        output,
        "  bible-io-references [--language CODE] [--format text|json|osis|usfm] --batch"
    )?;
    writeln!(
        output,
        "  bible-io-references [--language CODE] [--format text|json|osis|usfm] --input FILE"
    )?;
    writeln!(output)?;
    writeln!(
        output,
        "Batch input is UTF-8 with one passage per nonblank line."
    )?;
    writeln!(
        output,
        "JSON batch output is JSON Lines, including per-line errors."
    )?;
    writeln!(
        output,
        "Exit codes: 0 success, 64 usage, 65 parse failure, 66 input error."
    )
}

fn batch_success_json(line_number: usize, input: &str, parsed: &ParsedInput) -> String {
    let mut output = format!(
        "{{\"line\":{line_number},\"input\":{},\"ok\":true,",
        json_string(input)
    );
    if let Some(reference) = parsed.narrow_reference {
        output.push_str("\"reference\":");
        output.push_str(&reference_json(reference));
    } else {
        output.push_str("\"passage\":");
        output.push_str(&passage_json(&parsed.passage));
    }
    output.push('}');
    output
}

fn error_json(input: &str, error: &ParseError, line_number: Option<usize>) -> String {
    let mut output = String::from("{");
    if let Some(line_number) = line_number {
        write!(output, "\"line\":{line_number},").expect("writing to String cannot fail");
    }
    write!(
        output,
        "\"input\":{},\"ok\":false,\"error\":{{\"code\":{},\"details\":{}}}}}",
        json_string(input),
        json_string(error.code()),
        json_string(error.details())
    )
    .expect("writing to String cannot fail");
    output
}

fn passage_json(passage: &Passage) -> String {
    match passage {
        Passage::Book(passage) => format!(
            "{{\"type\":\"book\",\"book\":{}}}",
            json_string(passage.book().abbreviation())
        ),
        Passage::Chapter(passage) => {
            let end = passage
                .end_chapter()
                .map_or_else(|| "null".to_owned(), |chapter| chapter.to_string());
            format!(
                "{{\"type\":\"chapter\",\"book\":{},\"startChapter\":{},\"endChapter\":{end}}}",
                json_string(passage.book().abbreviation()),
                passage.start_chapter()
            )
        }
        Passage::Verses(passage) => {
            let selections = passage
                .selections()
                .iter()
                .copied()
                .map(reference_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("{{\"type\":\"verses\",\"selections\":[{selections}]}}")
        }
        Passage::Sequence(sequence) => {
            let passages = sequence
                .passages()
                .iter()
                .map(passage_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("{{\"type\":\"sequence\",\"passages\":[{passages}]}}")
        }
    }
}

fn reference_json(reference: Reference) -> String {
    match reference {
        Reference::Verse(verse) => verse_json(verse),
        Reference::Range(range) => format!(
            "{{\"type\":\"range\",\"start\":{},\"end\":{}}}",
            verse_json(range.start()),
            verse_json(range.end())
        ),
    }
}

fn verse_json(verse: VerseRef) -> String {
    format!(
        "{{\"type\":\"verse\",\"book\":{},\"chapter\":{},\"verse\":{}}}",
        json_string(verse.book().abbreviation()),
        verse.chapter(),
        verse.verse()
    )
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{2028}' => output.push_str("\\u2028"),
            '\u{2029}' => output.push_str("\\u2029"),
            character if character <= '\u{1f}' => {
                write!(output, "\\u{:04x}", character as u32)
                    .expect("writing to String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
#[path = "../../tests/unit/bible_io_references_cli.rs"]
mod tests;
