#![forbid(unsafe_code)]
#![warn(clippy::all)]

use bible_io_references::{
    BookNameStyle, Language, MachineIdentifiers, Passage, ReferenceExtractor, ReferenceFormatter,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let passage: Passage = "John 3:16,18-20; Acts 2".parse()?;
    println!("{passage}");
    println!("OSIS: {}", passage.osis_identifier());
    println!("USFM: {}", passage.usfm_identifier());

    let spanish = ReferenceFormatter::for_language(Language::Spanish)
        .with_book_name_style(BookNameStyle::Long);
    println!("Spanish: {}", spanish.format_passage(&passage));

    let source = "Read John 3:16 and Acts 2 today.";
    for found in ReferenceExtractor::new().extract(source) {
        println!("{}: {}", found.start(), found.passage());
    }

    Ok(())
}
