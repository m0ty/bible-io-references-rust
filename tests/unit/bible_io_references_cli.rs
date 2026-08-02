use std::io::Cursor;

use serde_json::Value;

use super::*;

fn run(arguments: &[&str], input: &[u8]) -> (u8, String, String) {
    let arguments = arguments
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect::<Vec<_>>();
    let mut input = Cursor::new(input);
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let code = run_cli(&arguments, &mut input, &mut output, &mut errors);
    (
        code,
        String::from_utf8(output).unwrap(),
        String::from_utf8(errors).unwrap(),
    )
}

struct FailingWriter;

impl IoWrite for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed output"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn parses_single_rich_passages_and_localizes_text() {
    let (code, output, errors) = run(&["John 3:16,18-20; Acts 2"], b"");
    assert_eq!(code, SUCCESS);
    assert_eq!(output, "John 3:16,18-20; Acts 2\n");
    assert!(errors.is_empty());

    let (code, output, errors) = run(&["--language", "es", "Juan", "3:16"], b"");
    assert_eq!(code, SUCCESS);
    assert_eq!(output, "Juan 3:16\n");
    assert!(errors.is_empty());
}

#[test]
fn renders_machine_identifiers() {
    let (code, output, _) = run(&["--format=osis", "John 3:16-4:1"], b"");
    assert_eq!(code, SUCCESS);
    assert_eq!(output, "John.3.16-John.4.1\n");

    let (code, output, _) = run(&["-f", "usfm", "John 3:16,18-20"], b"");
    assert_eq!(code, SUCCESS);
    assert_eq!(output, "JHN 3:16,18-20\n");
}

#[test]
fn preserves_narrow_and_rich_json_shapes() {
    let (code, output, errors) = run(&["--format=json", "John", "3:16"], b"");
    assert_eq!(code, SUCCESS);
    assert!(errors.is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(&output).unwrap(),
        serde_json::json!({
            "type": "verse",
            "book": "jo",
            "chapter": 3,
            "verse": 16
        })
    );

    let (code, output, _) = run(&["--format", "json", "John 3"], b"");
    assert_eq!(code, SUCCESS);
    assert_eq!(
        serde_json::from_str::<Value>(&output).unwrap(),
        serde_json::json!({
            "type": "chapter",
            "book": "jo",
            "startChapter": 3,
            "endChapter": null
        })
    );
}

#[test]
fn batch_continues_after_errors_and_ignores_blank_lines() {
    let (code, output, errors) = run(&["--batch"], b"John 3:16\n\nNotABook 1:1\nActs 2:1-4\n");
    assert_eq!(code, DATA_ERROR);
    assert_eq!(output, "John 3:16\nActs 2:1-4\n");
    assert!(errors.contains("Line 3: Unable to parse"));
    assert!(errors.contains("(unknown_book)"));
}

#[test]
fn batch_json_emits_one_object_per_nonblank_line() {
    let (code, output, errors) = run(
        &["--batch", "--format=json"],
        b"John\n\nNotABook 1:1\nJohn 3:16,18\n",
    );
    assert_eq!(code, DATA_ERROR);
    assert!(errors.is_empty());
    let records = output
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["line"], 1);
    assert_eq!(records[0]["ok"], true);
    assert_eq!(records[0]["passage"]["type"], "book");
    assert_eq!(records[1]["line"], 3);
    assert_eq!(records[1]["ok"], false);
    assert_eq!(records[1]["error"]["code"], "unknown_book");
    assert_eq!(records[2]["line"], 4);
    assert_eq!(records[2]["passage"]["type"], "verses");
}

#[test]
fn json_diagnostics_escape_untrusted_input() {
    let (code, output, errors) = run(&["--format=json", "bad\"book 1:1"], b"");
    assert_eq!(code, DATA_ERROR);
    assert!(errors.is_empty());
    let record = serde_json::from_str::<Value>(&output).unwrap();
    assert_eq!(record["input"], "bad\"book 1:1");
    assert_eq!(record["ok"], false);
}

#[test]
fn rejects_bad_option_combinations_with_usage_exit() {
    for arguments in [
        &["--batch", "--input", "references.txt"][..],
        &["--batch", "John 3:16"][..],
        &["--format", "yaml", "John 3:16"][..],
        &["--language", "el", "John 3:16"][..],
    ] {
        let (code, _, errors) = run(arguments, b"");
        assert_eq!(code, USAGE_ERROR, "{arguments:?}");
        assert!(errors.contains("Usage:"), "{arguments:?}");
    }
}

#[test]
fn malformed_utf8_is_an_input_error() {
    let (code, output, errors) = run(&["--batch"], &[0xc3, 0x28]);
    assert_eq!(code, NO_INPUT_ERROR);
    assert!(output.is_empty());
    assert!(errors.contains("input is not valid UTF-8"));
}

#[test]
fn batch_streams_valid_records_before_late_malformed_utf8() {
    let (code, output, errors) = run(&["--batch"], b"John 3:16\n\xc3(");
    assert_eq!(code, NO_INPUT_ERROR);
    assert_eq!(output, "John 3:16\n");
    assert!(errors.contains("input is not valid UTF-8"));
}

#[test]
fn help_and_version_exit_successfully() {
    let (code, output, errors) = run(&["--help"], b"");
    assert_eq!(code, SUCCESS);
    assert!(output.contains("Exit codes: 0 success"));
    assert!(errors.is_empty());

    let (code, output, errors) = run(&["--version"], b"");
    assert_eq!(code, SUCCESS);
    assert_eq!(
        output,
        format!("bible-io-references {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(errors.is_empty());
}

#[test]
fn output_failures_return_an_io_exit_code() {
    let arguments = vec!["John 3:16".to_owned()];
    assert_eq!(
        run_cli(
            &arguments,
            &mut Cursor::new([]),
            &mut FailingWriter,
            &mut Vec::new(),
        ),
        NO_INPUT_ERROR
    );

    let invalid = vec!["NotABook 1:1".to_owned()];
    assert_eq!(
        run_cli(
            &invalid,
            &mut Cursor::new([]),
            &mut Vec::new(),
            &mut FailingWriter,
        ),
        NO_INPUT_ERROR
    );
}
