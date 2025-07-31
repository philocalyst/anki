use chumsky::prelude::*;
use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct FlashCard {
    pub fields: Vec<(String, String)>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoteModel {
    pub name: String,
    pub aliases: HashMap<String, String>,
    pub cards: Vec<FlashCard>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlashItem {
    NoteModel(String),
    Alias { from: String, to: String },
    Tags(Vec<String>),
    Pair((String, String)),
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

    // Get valid field names from config
    let valid_fields: Vec<String> = config.fields.iter().map(|f| f.name.clone()).collect();

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

    // Tags parser (["Biology", "Covid-19", "Health"])
    let string_literal = just('"')
        .ignore_then(none_of('"').repeated().collect::<String>())
        .then_ignore(just('"'));

    let tags = just('[')
        .ignore_then(
            string_literal
                .separated_by(just(',').padded_by(inline_whitespace.clone()))
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(']'))
        .map(FlashItem::Tags);

    let field = text::ident()
        .then_ignore(just(':'))
        .try_map(|field_name: &str, span| {
            if config.fields.iter().any(|f| f.name.as_str() == field_name) {
                Ok(field_name.to_string())
            } else {
                Ok(field_name.to_string())
            }
        });

    // Content parser - content that follows a field
    let content = none_of('\n').repeated().collect::<String>();

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
    let line = choice((
        note_model,
        alias,
        tags,
        pair,
        comment,
        blank_line,
    ));

    // Full parser
    line.repeated()
        .collect::<Vec<_>>()
        .then_ignore(end())
        .map(|items| {
            let mut models = Vec::new();
            let mut current_model: Option<NoteModel> = None;
            let mut current_tags: Vec<String> = Vec::new();
            let mut current_fields: Vec<(String, String)> = Vec::new();

            for item in items {
                match item {
                    FlashItem::NoteModel(name) => {
                        // Save previous model if exists
                        if let Some(mut model) = current_model.take() {
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

                        current_model = Some(NoteModel {
                            name,
                            aliases: HashMap::new(),
                            cards: Vec::new(),
                        });
                    }

                    FlashItem::Alias { from, to } => {
                        if let Some(ref mut model) = current_model {
                            model.aliases.insert(from, to);
                        }
                    }

                    FlashItem::Tags(tags) => {
                        current_tags = tags;
                    }

                    FlashItem::Pair((field, content)) => {
                        // If we have a store of alises, and none of them match, then there's an issue!!
                        if let Some(ref model) = current_model {
             if !config.fields.iter().any(|f| f.name.as_str() == field) && model.aliases.get(&field).is_none() {
                 println!("{:?}ZZ{:?}ZZ{}", model.aliases, config.fields, field);
                 }
                 }
                        current_fields.push((field, content));
                    }

                    FlashItem::Comment(_) => {
                        // Ignore comments
                    }

                    FlashItem::BlankLine => {
                        // Blank line indicates end of current card
                        if !current_fields.is_empty() {
                            if let Some(ref mut model) = current_model {
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
            if let Some(mut model) = current_model {
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

fn main() {
    // Load config first
    let example_config = include_str!("/home/miles/Downloads/oh/src/ClozeWithSource/config.toml");
    let config: Config = toml::from_str(&example_config).unwrap();

    let example_content = include_str!("/home/miles/Downloads/oh/example.flash");

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
                    println!("    Fields: {:?}", card.fields);
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
}
