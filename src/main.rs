use chumsky::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct FlashCard {
    pub question: String,
    pub answer: String,
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
    Question(String),
    Answer(String),
    Comment(String),
}

fn parser<'a>() -> impl Parser<'a, &'a str, Vec<NoteModel>, extra::Err<Rich<'a, char>>> {
    // Whitespace parser (excluding newlines)
    let inline_whitespace = one_of(" \t").repeated();
    
    // Note model parser (=Basic=)
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
                .collect::<String>()
        )
        .then_ignore(inline_whitespace.clone())
        .then_ignore(text::keyword("to"))
        .then_ignore(inline_whitespace.clone())
        .then(
            none_of([' ', '\t', '\n'])
                .repeated()
                .at_least(1)
                .collect::<String>()
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
                .collect::<Vec<_>>()
        )
        .then_ignore(just(']'))
        .map(FlashItem::Tags);

    // Question parser (Question: content or Q: content)
    let question_label = text::keyword("Question").or(text::keyword("Q"));
    
    let question = question_label
        .then_ignore(just(':'))
        .then_ignore(inline_whitespace.clone())
        .ignore_then(
            none_of('\n')
                .repeated()
                .collect::<String>()
        )
        .map(|content| FlashItem::Question(content.trim().to_string()));

    // Answer parser (Answer: content or A: content)  
    let answer_label = text::keyword("Answer").or(text::keyword("A"));
    
    let answer = answer_label
        .then_ignore(just(':'))
        .then_ignore(inline_whitespace.clone())
        .ignore_then(
            none_of('\n')
                .repeated()
                .collect::<String>()
        )
        .map(|content| FlashItem::Answer(content.trim().to_string()));

    // Comment parser (// comment)
    let comment = just("//")
        .ignore_then(
            none_of('\n')
                .repeated()
                .collect::<String>()
        )
        .map(FlashItem::Comment);

    // Line parser
    let line = choice((
        note_model,
        alias, 
        tags,
        question,
        answer,
        comment,
    ));

    // Full parser
    line
        .separated_by(text::newline())
        .allow_trailing()
        .collect::<Vec<_>>()
        .then_ignore(end())
        .map(|items| {
            let mut models = Vec::new();
            let mut current_model: Option<NoteModel> = None;
            let mut current_question: Option<String> = None;
            let mut current_tags: Vec<String> = Vec::new();

            for item in items {
                match item {
                    FlashItem::NoteModel(name) => {
                        // Save previous model if exists
                        if let Some(model) = current_model.take() {
                            models.push(model);
                        }
                        
                        current_model = Some(NoteModel {
                            name,
                            aliases: HashMap::new(),
                            cards: Vec::new(),
                        });
                        current_tags.clear();
                    }
                    
                    FlashItem::Alias { from, to } => {
                        if let Some(ref mut model) = current_model {
                            model.aliases.insert(from, to);
                        }
                    }
                    
                    FlashItem::Tags(tags) => {
                        current_tags = tags;
                    }
                    
                    FlashItem::Question(q) => {
                        current_question = Some(q);
                    }
                    
                    FlashItem::Answer(a) => {
                        if let (Some(question), Some(model)) = 
                            (current_question.take(), &mut current_model) {
                            model.cards.push(FlashCard {
                                question,
                                answer: a,
                                tags: current_tags.clone(),
                            });
                        }
                    }
                    
                    FlashItem::Comment(_) => {
                        // Ignore comments
                    }
                }
            }

            // Don't forget the last model
            if let Some(model) = current_model {
                models.push(model);
            }

            models
        })
}

fn main() {
    let example_content = include_str!("/home/miles/Downloads/oh/example.flash");

    let parse_result = parser().parse(example_content);
    
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
                    println!("    Q: {}", card.question);
                    println!("    A: {}", card.answer);
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
