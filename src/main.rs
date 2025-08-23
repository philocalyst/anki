use ariadne::Source;
use chumsky::prelude::*;
use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    fs, io,
    ops::Range,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq)]
pub struct FlashCard {
    pub fields: Vec<NoteField>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoteField {
    name: String,
    content: Vec<TextElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoteModel {
    pub name: String,
    pub aliases: HashMap<String, String>,
    pub cards: Vec<FlashCard>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cloze {
    pub id: u32,
    pub answer: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextElement {
    Text(String),
    Cloze(Cloze),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlashItem {
    NoteModel(String),
    Alias { from: String, to: String },
    Tags(Vec<String>),
    Pair((String, Vec<TextElement>)), // Updated to use TextElement
    Comment(String),
    BlankLine,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub schema_version: String,
    pub name: String,
    pub tags: Vec<String>,
    pub sort_field: String,

    // this corresponds to the `[defaults]` table
    pub defaults: Defaults,

    // this corresponds to the `[[fields]]` array of tables
    pub fields: Vec<Field>,

    pub template_order: Option<Vec<String>>,

    // this corresponds to the `[[templates]]` array of tables
    pub templates: Vec<Template>,
}

#[derive(Deserialize, Debug)]
pub struct Defaults {
    pub font: String,
    pub size: u32,
    pub rtl: bool,
}

#[derive(Deserialize, Debug)]
pub struct Field {
    pub name: String,
    pub sticky: bool,
    pub media: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Template {
    pub name: String,
    pub required_fields: Vec<String>,
}

fn parser<'a>(
    config: &'a Config,
) -> impl Parser<'a, &'a str, Vec<NoteModel>, extra::Err<Rich<'a, char>>> + Clone {
    // Whitespace parser (excluding newlines), ignored for padding/ignoring
    let inline_whitespace = one_of(" \t").repeated().ignored();

    let note_model = just('=')
        .ignore_then(none_of('=').repeated().collect::<String>())
        .then_ignore(just('='))
        .map(|name| FlashItem::NoteModel(name.trim().to_string()));

    // Alias parser (alias Question to Q)
    let alias = text::keyword("alias")
        .padded_by(inline_whitespace.clone())
        .ignore_then(
            none_of([' ', '\t', '\n'])
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then_ignore(inline_whitespace.clone())
        .then_ignore(text::keyword("to"))
        .then_ignore(inline_whitespace.clone())
        .then(
            none_of([' ', '\t', '\n'])
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .map(|(from, to)| FlashItem::Alias { from, to });

    // Complete cloze parser
    let cloze_id = text::int(10).map(|s: &str| s.to_string().parse::<u32>().unwrap());

    let cloze_content = none_of(['|', '}', '\n'])
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|s| s.trim().to_string());

    let hint = just('|').ignore_then(cloze_content.clone()).or_not();

    let cloze = just('#')
        .ignore_then(cloze_id)
        .then_ignore(just('{'))
        .then(cloze_content)
        .then(hint)
        .then_ignore(just('}'))
        .map(|((id, answer), hint)| TextElement::Cloze(Cloze { id, answer, hint }));

    let tag = none_of(",[]")
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|s: String| s.trim().to_string());

    let tags = tag
        .separated_by(just(',').padded())
        .collect::<Vec<_>>() // <-- required; otherwise output is ()
        .delimited_by(just('['), just(']'))
        .map(FlashItem::Tags);

    let field = text::ident()
        .then_ignore(just(':'))
        .try_map(|field_name: &str, _| {
            if config.fields.iter().any(|f| f.name.as_str() == field_name) {
                Ok(field_name.to_string())
            } else {
                Ok(field_name.to_string())
            }
        });

    // Updated content parser that handles mixed text and cloze deletions
    let regular_text = none_of(['#', '\n'])
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(TextElement::Text);

    let content_element = cloze.or(regular_text);

    let content = content_element
        .repeated()
        .collect::<Vec<TextElement>>()
        .validate(|elements, _extra, emitter| {
            // Merge adjacent text elements for cleaner output
            let mut merged = Vec::new();
            let mut current_text = String::new();

            for element in elements {
                match element {
                    TextElement::Text(text) => {
                        current_text.push_str(&text);
                    }
                    TextElement::Cloze(cloze) => {
                        if !current_text.is_empty() {
                            merged.push(TextElement::Text(current_text.clone()));
                            current_text.clear();
                        }
                        merged.push(TextElement::Cloze(cloze));
                    }
                }
            }

            if !current_text.is_empty() {
                merged.push(TextElement::Text(current_text));
            }

            merged
        });

    let pair = field
        .then_ignore(inline_whitespace.clone())
        .then(content)
        .map(|(field, content)| FlashItem::Pair((field, content)));

    // Comment parser (// comment)
    let comment = just("//")
        .ignore_then(none_of('\n').repeated().collect::<String>())
        .map(FlashItem::Comment);

    // Blank line parser
    let blank_line = text::newline().to(FlashItem::BlankLine);

    // Line parser
    let line =
        choice((note_model, alias, tags, pair, comment, blank_line)).map_with(|item, extra| {
            let span: SimpleSpan = extra.span(); // Get the span for the parsed item
            (item, span) // Return the item along with its span
        });

    // Full parser
    line.repeated()
        .collect::<Vec<_>>()
        .then_ignore(end())
        .map_with(|items, _| {
            let mut models = Vec::new();
            let mut current_model: (Option<NoteModel>, Option<Range<usize>>) = (None, None);
            let mut current_tags: Vec<String> = Vec::new();
            let mut current_fields: Vec<NoteField> = Vec::new();

            for item in items {
                match item {
                    (FlashItem::NoteModel(name), span) => {
                        // Save previous model if exists
                        if let Some(mut model) = current_model.0.take() {
                            // Add any remaining card
                            if !current_fields.is_empty() {
                                model.cards.push(FlashCard {
                                    fields: current_fields.clone(),
                                    tags: current_tags.clone(),
                                });
                                current_fields.clear();
                                current_tags.clear();
                            }
                            models.push(model);
                        }

                        current_model = (
                            Some(NoteModel {
                                name,
                                aliases: HashMap::new(),
                                cards: Vec::new(),
                            }),
                            Some(span.into_range()),
                        );
                    }

                    (FlashItem::Alias { from, to }, _) => {
                        if let Some(ref mut model) = current_model.0 {
                            model.aliases.insert(from, to);
                        }
                    }

                    (FlashItem::Tags(tags), _) => {
                        current_tags = tags;
                    }

                    (FlashItem::Pair((name, content)), span) => {
                        use ariadne::{Color, ColorGenerator, Fmt, Label, Report, ReportKind};

                        let mut colors = ColorGenerator::new();

                        let path = "/home/miles/Downloads/anki/COVID.deck/example.flash";
                        let path_content = fs::read_to_string(path).unwrap();

                        // pick some colours
                        let a = colors.next();
                        // let b = colors.next();
                        let out = Color::Fixed(81);

                        if let Some(ref model) = current_model.0 {
                            if !config.fields.iter().any(|f| f.name == name)
                                && model.aliases.get(&name).is_none()
                            {
                                let range: Range<usize> = span.into_range();
                                // build the error
                                let report = Report::build(
                                    ReportKind::Error,
                                    (path, current_model.1.clone().unwrap()),
                                )
                                .with_code(3)
                                .with_message("Unknown field!")
                                .with_label(
                                    Label::new((path, range))
                                        .with_message(format!("No field named '{}'", name))
                                        .with_color(a),
                                )
                                .with_note(format!(
                                    "For the model {}, the available fields are {}",
                                    model.name.clone().fg(out),
                                    format!(
                                        "{:?}",
                                        config
                                            .fields
                                            .iter()
                                            .map(|item| item.name.clone())
                                            .collect::<Vec<_>>()
                                    )
                                    .fg(out)
                                ))
                                .finish();

                                // write it out
                                let mut stdout = io::stdout();
                                report
                                    .write((path, Source::from(&path_content)), &mut stdout)
                                    .unwrap();
                                continue;
                            }
                        }
                        current_fields.push(NoteField { name, content });
                    }

                    (FlashItem::Comment(_), _) => {
                        // Ignore comments
                    }

                    (FlashItem::BlankLine, _) => {
                        // Blank line indicates end of current card
                        if !current_fields.is_empty() {
                            if let Some(ref mut model) = current_model.0 {
                                model.cards.push(FlashCard {
                                    fields: current_fields.clone(),
                                    tags: current_tags.clone(),
                                });
                                current_fields.clear();
                                current_tags.clear();
                            }
                        }
                    }
                }
            }

            // Don't forget the last model and card
            if let Some(mut model) = current_model.0 {
                if !current_fields.is_empty() {
                    model.cards.push(FlashCard {
                        fields: current_fields,
                        tags: current_tags,
                    });
                }
                models.push(model);
            }

            models
        })
}

fn is_deck_dir<P: AsRef<Path>>(path: P) -> bool {
    let p = path.as_ref();
    // is_dir() will return false if it doesn't exist or isn't a dir
    p.is_dir() && p.extension().and_then(|e| e.to_str()) == Some("deck")
}

fn main() -> Result<(), Box<dyn Error>> {
    // Check for a deck
    let mut dirs = Vec::new();
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            dirs.push(entry.path());
        }
    }

    dbg!(&dirs);
    let deck: PathBuf = dirs.into_iter().find(|dir| is_deck_dir(dir)).unwrap();

    // Get the models and flashcards in the deck
    let mut models = Vec::new();
    let mut cards = Vec::new();
    for entry in fs::read_dir(deck)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            models.push(entry.path());
        } else if entry.path().extension().and_then(|ext| ext.to_str()) == Some("flash") {
            cards.push(entry.path());
        }
    }

    let mut config = PathBuf::new();

    for entry in fs::read_dir(models[0].clone())? {
        let entry = entry?.path();
        if entry.to_str().unwrap().contains("config.toml") {
            config = entry;
        }
    }

    // Load config first
    let example_config = fs::read_to_string(config)?;
    let config: Config = toml::from_str(&example_config).unwrap();

    let example_content = fs::read_to_string(cards[0].clone())?;

    let parse_result = parser(&config).parse(&example_content);

    match parse_result.into_result() {
        Ok(models) => {
            for model in models {
                println!("Note Model: {}", model.name);

                if !model.aliases.is_empty() {
                    println!("  Aliases:");
                    for (from, to) in &model.aliases {
                        println!("    {} -> {}", from, to);
                    }
                }

                println!("  Cards:");
                for card in &model.cards {
                    for field in card.fields.clone() {
                        println!("{} : {:?}", field.name, field.content);
                    }
                    if !card.tags.is_empty() {
                        println!("    Tags: {:?}", card.tags);
                    }
                    println!();
                }
                println!("---");
            }
        }

        Err(errors) => {
            eprintln!("Parsing errors:");
            for error in errors {
                eprintln!("  {}", error);
            }
        }
    }

    Ok(())
}
