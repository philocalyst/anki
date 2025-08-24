use ariadne::Source;
use chumsky::prelude::*;
use semver::Version;
use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    fs, io,
    ops::Range,
    path::{Path, PathBuf},
};

use crate::crowd_anki::{CrowdAnkiEntity, NoteModelType};
use evalexpr::Node;

mod crowd_anki;

#[derive(Debug, Clone, PartialEq)]
pub struct Note<'a> {
    pub fields: Vec<NoteField>,
    pub model: &'a NoteModel,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoteField {
    name: String,
    content: Vec<TextElement>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct NoteModel {
    pub name: String,

    // The available templates
    pub templates: Vec<Template>,

    // The version of the schema that we're on
    pub schema_version: Version,

    // The default field configuration
    pub defaults: Option<Defaults>,

    // Anything with serde skip means I don't want it to be possible to be included in the TOML representation
    #[serde(skip)]
    pub css: String,

    pub fields: Vec<Field>,

    #[serde(skip)]
    pub latex_pre: Option<String>,
    #[serde(skip)]
    pub latex_post: Option<String>,

    // The field to sort around
    pub sort_field: Option<String>,
    pub tags: Option<Vec<String>>,

    // The required fields are determined at runtime, this String holds a boolean expression that affirms this.
    pub required: Node,
}

#[derive(Debug, Clone, PartialEq)]
pub struct P_NoteModel {
    pub name: String,
    pub aliases: HashMap<String, String>,
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
    Alias {
        from: String,
        to: String,
    },
    Tags(Vec<String>),
    Field {
        name: String,
        content: Vec<TextElement>,
    },
    Comment(String),
    BlankLine,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Defaults {
    pub font: String,
    pub size: u32,
    pub rtl: bool,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Field {
    pub name: String,
    pub sticky: Option<bool>,
    pub associated_media: Option<Vec<PathBuf>>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Template {
    pub name: String,

    #[serde(skip)]
    pub order: i32,

    #[serde(skip)]
    pub question_format: String,
    #[serde(skip)]
    pub answer_format: String,

    #[serde(skip)]
    pub browser_question_format: String,
    #[serde(skip)]
    pub browser_answer_format: String,
}

impl NoteModel {
    pub fn complete(&mut self, dir: &Path) -> Result<(), Box<dyn Error>> {
        // Load CSS if present
        let css_path = dir.join("style.css");
        if css_path.exists() {
            self.css = fs::read_to_string(css_path)?;
        }

        // Load LaTeX pre/post if present
        let pre_path = dir.join("pre.tex");
        if pre_path.exists() {
            self.latex_pre = Some(fs::read_to_string(pre_path)?);
        }

        let post_path = dir.join("post.tex");
        if post_path.exists() {
            self.latex_post = Some(fs::read_to_string(post_path)?);
        }

        // Load templates from .hbs files
        let mut templates = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "hbs" {
                    let filename = path.file_stem().unwrap().to_string_lossy().to_string();

                    // Parse naming convention: NAME+front.hbs, NAME+back.browser.hbs, etc.
                    let parts: Vec<&str> = filename.split('+').collect();
                    if parts.len() != 2 {
                        continue; // skip malformed
                    }

                    let template_name = parts[0].to_string();
                    let side = parts[1];

                    // Find or create template
                    let tmpl = templates
                        .iter_mut()
                        .find(|t: &&mut Template| t.name == template_name);

                    let tmpl = if let Some(t) = tmpl {
                        t
                    } else {
                        templates.push(Template {
                            name: template_name.clone(),
                            order: templates.len() as i32,
                            question_format: String::new(),
                            answer_format: String::new(),
                            browser_question_format: String::new(),
                            browser_answer_format: String::new(),
                        });
                        templates.last_mut().unwrap()
                    };

                    let content = fs::read_to_string(&path)?;

                    // Assign based on side
                    if side.starts_with("front") {
                        if side.contains("browser") {
                            tmpl.browser_question_format = content;
                        } else {
                            tmpl.question_format = content;
                        }
                    } else if side.starts_with("back") {
                        if side.contains("browser") {
                            tmpl.browser_answer_format = content;
                        } else {
                            tmpl.answer_format = content;
                        }
                    }
                }
            }
        }

        self.templates = templates;
        Ok(())
    }
}

fn parser<'a>(
    config: &'a NoteModel,
) -> impl Parser<'a, &'a str, Vec<Note>, extra::Err<Rich<'a, char>>> + Clone {
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
        .map(|(name, content)| FlashItem::Field { name, content });

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
        .map_with(move |items, _| {
            let mut models = Vec::new();
            let mut current_model: (Option<P_NoteModel>, Option<Range<usize>>) = (None, None);
            let mut cards = Vec::new();
            let mut current_tags: Vec<String> = Vec::new();
            let mut current_fields: Vec<NoteField> = Vec::new();

            for item in items {
                match item {
                    (FlashItem::NoteModel(name), span) => {
                        // Save previous model if exists
                        if let Some(mut model) = current_model.0.take() {
                            // Add any remaining card
                            if !current_fields.is_empty() {
                                cards.push(Note {
                                    fields: current_fields.clone(),
                                    tags: current_tags.clone(),
                                    model: config,
                                });
                                current_fields.clear();
                                current_tags.clear();
                            }
                            models.push(model);
                        }

                        current_model = (
                            Some(P_NoteModel {
                                name,
                                aliases: HashMap::new(),
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

                    (FlashItem::Field { name, content }, span) => {
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
                                cards.push(Note {
                                    fields: current_fields.clone(),
                                    tags: current_tags.clone(),
                                    model: config,
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
                    cards.push(Note {
                        fields: current_fields,
                        tags: current_tags,
                        model: &config,
                    });
                }
                models.push(model);
            }

            cards
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
    let mut config: NoteModel = toml::from_str(&example_config).unwrap();

    config.complete(Path::new(
        "/home/miles/Downloads/anki/COVID.deck/ClozeWithSource",
    ))?;

    dbg!(&config);

    let example_content = fs::read_to_string(cards[0].clone())?;

    let parse_result = parser(&config).parse(&example_content);

    match parse_result.into_result() {
        Ok(cards) => {
            for model in models {
                println!("  Cards:");
                for card in &cards {
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
