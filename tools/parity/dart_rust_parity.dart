import 'dart:convert';
import 'dart:io';

import 'package:bible_io_references/bible_io_references.dart';

Future<ProcessResult> runBatch(
  String executable,
  List<String> arguments,
  String input,
) async {
  final process = await Process.start(
    executable,
    arguments,
    workingDirectory: Directory.current.path,
  );
  final stdoutFuture = utf8.decoder.bind(process.stdout).join();
  final stderrFuture = utf8.decoder.bind(process.stderr).join();
  process.stdin.write(input);
  await process.stdin.close();
  final stdoutText = await stdoutFuture;
  final stderrText = await stderrFuture;
  final exitCode = await process.exitCode;
  return ProcessResult(process.pid, exitCode, stdoutText, stderrText);
}

String normalizeNewlines(Object? value) =>
    value.toString().replaceAll('\r\n', '\n');

Future<void> main(List<String> arguments) async {
  if (arguments.length != 1) {
    stderr.writeln('usage: dart_rust_parity.dart RUST_BINARY');
    exitCode = 64;
    return;
  }

  final rust = arguments.single;
  var comparisons = 0;
  final failures = <String>[];
  final allInputs = <String>[];
  final explicitBatches = <BibleLanguageEnum, List<String>>{};

  final languages = [
    ...supportedParsingLanguages.where(
      (language) => language != BibleLanguageEnum.auto,
    ),
    BibleLanguageEnum.auto,
  ];
  for (final language in languages) {
    final inputs = language == BibleLanguageEnum.auto
        ? List<String>.of(allInputs)
        : <String>[];
    if (language == BibleLanguageEnum.english) {
      for (final book in BibleBookEnum.values) {
        inputs
          ..add('${book.fullName} 3:16')
          ..add('${book.abbreviation} 3:16');
      }
    } else if (language != BibleLanguageEnum.auto) {
      final names = bookNamesByLanguage[language.code]!;
      final abbreviations = bookAbbreviationsByLanguage[language.code]!;
      for (final book in BibleBookEnum.values) {
        for (final alias in {...names[book]!, ...abbreviations[book]!}) {
          inputs.add('$alias 3:16');
        }
      }
    }

    // Cover rich grammar for every book in every concrete language too.
    if (language != BibleLanguageEnum.auto) {
      final preferredNames = language == BibleLanguageEnum.english
          ? {for (final book in BibleBookEnum.values) book: book.fullName}
          : {
              for (final book in BibleBookEnum.values)
                book: bookNamesByLanguage[language.code]![book]!.first,
            };
      for (final book in BibleBookEnum.values) {
        final alias = preferredNames[book]!;
        inputs
          ..add('$alias 3:16-4:1')
          ..add('$alias 3:16,18-20,4:1');
      }
      allInputs.addAll(inputs);
      explicitBatches[language] = List.of(inputs);
    }

    final batch = '${inputs.join('\n')}\n';
    final formats = language == BibleLanguageEnum.auto
        ? ['text', 'json', 'osis', 'usfm']
        : ['text'];
    for (final format in formats) {
      final cliArguments = [
        '--language',
        language.code,
        '--format',
        format,
        '--batch',
      ];
      final rustResult = await runBatch(rust, cliArguments, batch);
      comparisons += inputs.length;

      final expectedLines = <String>[];
      for (var index = 0; index < inputs.length; index++) {
        final input = inputs[index];
        final referenceResult = Reference.parseResult(
          input,
          language: language,
        );
        final Object parsed =
            referenceResult.valueOrNull ??
            Passage.parse(input, language: language);
        switch (format) {
          case 'json':
            expectedLines.add(
              jsonEncode({
                'line': index + 1,
                'input': input,
                'ok': true,
                if (parsed is Reference)
                  'reference': parsed.toJson()
                else
                  'passage': (parsed as Passage).toJson(),
              }),
            );
          case 'text':
            expectedLines.add(switch (parsed) {
              Reference value => value.format(language: language),
              Passage value => value.format(language: language),
              _ => throw StateError('unsupported parsed value'),
            });
          case 'osis':
            expectedLines.add(switch (parsed) {
              Reference value => value.osisIdentifier,
              Passage value => value.osisIdentifier,
              _ => throw StateError('unsupported parsed value'),
            });
          case 'usfm':
            expectedLines.add(switch (parsed) {
              Reference value => value.usfmIdentifier,
              Passage value => value.usfmIdentifier,
              _ => throw StateError('unsupported parsed value'),
            });
          default:
            throw StateError('unsupported format: $format');
        }
      }

      final expectedOutput = '${expectedLines.join('\n')}\n';
      final rustOutput = normalizeNewlines(rustResult.stdout);
      final rustErrors = normalizeNewlines(rustResult.stderr);
      if (rustResult.exitCode == 0 &&
          expectedOutput == rustOutput &&
          rustErrors.isEmpty) {
        continue;
      }

      var line = 0;
      final expectedOutputLines = expectedOutput.split('\n');
      final rustOutputLines = rustOutput.split('\n');
      while (line < expectedOutputLines.length &&
          line < rustOutputLines.length &&
          expectedOutputLines[line] == rustOutputLines[line]) {
        line++;
      }
      failures.add(
        '${language.code}/$format differs at output line ${line + 1}; '
        'input=${line < inputs.length ? jsonEncode(inputs[line]) : '<eof>'}; '
        'dart=${line < expectedOutputLines.length ? jsonEncode(expectedOutputLines[line]) : '<eof>'}; '
        'rust=${line < rustOutputLines.length ? jsonEncode(rustOutputLines[line]) : '<eof>'}; '
        'exit=${rustResult.exitCode}; stderr=${jsonEncode(rustErrors)}',
      );
    }
  }

  final dataChecks = _checkRegistryContract();
  comparisons += dataChecks.comparisons;
  failures.addAll(dataChecks.failures);
  final metadataChecks = _checkMetadataContract(explicitBatches);
  comparisons += metadataChecks.comparisons;
  failures.addAll(metadataChecks.failures);
  final errorChecks = await _checkCliErrors(rust);
  comparisons += errorChecks.comparisons;
  failures.addAll(errorChecks.failures);

  stdout.writeln('Compared $comparisons Dart/Rust contract observations.');
  if (failures.isNotEmpty) {
    for (final failure in failures) {
      stderr.writeln(failure);
    }
    exitCode = 1;
  }
}

({int comparisons, List<String> failures}) _checkRegistryContract() {
  const expectedTermCounts = {
    BibleLanguageEnum.arabic: (226, 314),
    BibleLanguageEnum.chinese: (335, 166),
    BibleLanguageEnum.french: (323, 338),
    BibleLanguageEnum.german: (147, 343),
    BibleLanguageEnum.hebrew: (230, 286),
    BibleLanguageEnum.hindi: (180, 218),
    BibleLanguageEnum.indonesian: (145, 430),
    BibleLanguageEnum.korean: (111, 416),
    BibleLanguageEnum.portuguese: (180, 335),
    BibleLanguageEnum.russian: (193, 160),
    BibleLanguageEnum.spanish: (191, 443),
    BibleLanguageEnum.tagalog: (83, 247),
  };
  final failures = <String>[];
  var comparisons = 0;
  var totalTerms = 0;
  for (final entry in expectedTermCounts.entries) {
    final names = bookNamesByLanguage[entry.key.code]!;
    final abbreviations = bookAbbreviationsByLanguage[entry.key.code]!;
    final nameCount = names.values.expand((terms) => terms).length;
    final abbreviationCount = abbreviations.values
        .expand((terms) => terms)
        .length;
    totalTerms += nameCount + abbreviationCount;
    comparisons += 3;
    if (names.length != BibleBookEnum.values.length) {
      failures.add('${entry.key.code}: incomplete name registry');
    }
    if (nameCount != entry.value.$1 || abbreviationCount != entry.value.$2) {
      failures.add(
        '${entry.key.code}: term counts are '
        '$nameCount/$abbreviationCount, expected ${entry.value.$1}/${entry.value.$2}',
      );
    }
  }
  comparisons += 4;
  if (BibleBookEnum.values.length != 83) failures.add('expected 83 books');
  if (BibleLanguageEnum.values.length != 19)
    failures.add('expected 19 languages');
  if (supportedParsingLanguages.length != 14) {
    failures.add('expected 14 supported languages/modes');
  }
  if (totalTerms != 6040) failures.add('expected 6040 localized terms');
  const expectedCollisions = {
    'jc': {'jm', 'jud'},
    'jn': {'jn', 'jo'},
    'jud': {'jud', 'jd'},
    'so': {'so', 'zp'},
  };
  comparisons += expectedCollisions.length + 1;
  if (autoLanguageCollisions.keys
          .toSet()
          .difference(expectedCollisions.keys.toSet())
          .isNotEmpty ||
      expectedCollisions.keys
          .toSet()
          .difference(autoLanguageCollisions.keys.toSet())
          .isNotEmpty) {
    failures.add('automatic collision keys changed');
  }
  for (final entry in expectedCollisions.entries) {
    final actual = autoLanguageCollisions[entry.key]
        ?.map((book) => book.abbreviation)
        .toSet();
    if (actual == null ||
        actual.difference(entry.value).isNotEmpty ||
        entry.value.difference(actual).isNotEmpty) {
      failures.add('${entry.key}: automatic collision set changed');
    }
  }
  return (comparisons: comparisons, failures: failures);
}

({int comparisons, List<String> failures}) _checkMetadataContract(
  Map<BibleLanguageEnum, List<String>> explicitBatches,
) {
  final failures = <String>[];
  var comparisons = 0;
  for (final languageEntry in explicitBatches.entries) {
    for (final input in languageEntry.value) {
      final result = Passage.parseResult(input, language: languageEntry.key);
      comparisons++;
      if (!result.isSuccess || result.metadataOrNull!.bookMatches.isEmpty) {
        failures.add('${languageEntry.key.code}: missing metadata for $input');
      }
    }
  }
  return (comparisons: comparisons, failures: failures);
}

Future<({int comparisons, List<String> failures})> _checkCliErrors(
  String rust,
) async {
  const inputs = [
    'Unknown 3:16',
    'John 0:1',
    'John 1000:1',
    'John 3:16-15',
    'Acts 1:1-John 1:1',
    'John 3:',
    'John 3:x',
    'John 3:16-',
    'John 3:16,,18',
    'John 3:16;',
  ];
  final failures = <String>[];
  var comparisons = 0;
  for (final input in inputs) {
    final result = await runBatch(rust, ['--format', 'json', input], '');
    comparisons++;
    String? dartCode;
    final reference = Reference.parseResult(input);
    if (reference.errorOrNull case final error?) {
      final passage = Passage.parseResult(input);
      dartCode = passage.errorOrNull?.code ?? error.code;
    }
    if (result.exitCode != 65) {
      failures.add('$input: Rust exit ${result.exitCode}, expected 65');
      continue;
    }
    final rustJson =
        jsonDecode(result.stdout.toString()) as Map<String, Object?>;
    final rustError = rustJson['error']! as Map<String, Object?>;
    final rustCode = rustError['code'];
    // Error precedence is intentionally native, but every Rust code must be a
    // member of the Dart stable classification vocabulary.
    final knownCodes = ReferenceParseErrorCode.values
        .map((code) => code.wireName)
        .toSet();
    if (!knownCodes.contains(rustCode)) {
      failures.add(
        '$input: unknown Rust error code $rustCode (Dart: $dartCode)',
      );
    }
  }
  return (comparisons: comparisons, failures: failures);
}
