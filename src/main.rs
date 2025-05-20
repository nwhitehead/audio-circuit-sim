// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

use eframe::egui;
use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "csv.pest"]
pub struct CSVParser;

#[derive(Debug, PartialEq)]
enum Entry {
    Identifier(String),
    QuotedString(String),
    Number(f64),
}

type Record = std::vec::Vec<Entry>;
type File = std::vec::Vec<Record>;

fn parse_value(pair: Pair<Rule>) -> Entry {
    match pair.as_rule() {
        // Always try parsing identifier as number
        Rule::identifier => {
            let txt = pair.as_str();
            match txt.parse::<f64>() {
                Ok(n) => Entry::Number(n),
                // If it doesn't work as number, just make it an identifier
                Err(..) => Entry::Identifier(txt.into()),
            }
        },
        Rule::string => Entry::QuotedString(pair.into_inner().as_str().into()),
        _ => unreachable!(),
    }
}

fn parse_record(pair: Pair<Rule>) -> Record {
    pair.into_inner().map(parse_value).collect::<Record>()
}

fn parse_file(contents: &str) -> Result<File, pest::error::Error<Rule>> {
    let data = CSVParser::parse(Rule::file, contents)?.next().unwrap();
    Ok(vec![])
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let bytes = include_bytes!("./component.lib");
    let data = parse_file(&String::from_utf8_lossy(bytes));

    assert_eq!(
        parse_value(
            CSVParser::parse(Rule::field, "0")
                .unwrap()
                .next()
                .unwrap()
        ),
        Entry::Number(0.0)
    );
    assert_eq!(
        parse_value(
            CSVParser::parse(Rule::field, "-273.15")
                .unwrap()
                .next()
                .unwrap()
        ),
        Entry::Number(-273.15)
    );
    assert_eq!(
        parse_value(
            CSVParser::parse(Rule::field, "2N2222")
                .unwrap()
                .next()
                .unwrap()
        ),
        Entry::Identifier("2N2222".into())
    );
    assert_eq!(
        parse_value(
            CSVParser::parse(Rule::field, "2N2222")
                .unwrap()
                .next()
                .unwrap()
        ),
        Entry::Identifier("2N2222".into())
    );
    assert_eq!(
        parse_value(
            CSVParser::parse(Rule::field, "\"abc\"")
                .unwrap()
                .next()
                .unwrap()
        ),
        Entry::QuotedString("abc".into())
    );
    assert_eq!(
        parse_record(
            CSVParser::parse(Rule::record, "F 1")
                .unwrap()
                .next()
                .unwrap()
        ),
        vec![Entry::Identifier("F".into()), Entry::Number(1.0)]
    );

    // println!("{:?}", CSVParser::parse(Rule::field, "-273.15"));
    // println!("{:?}", CSVParser::parse(Rule::field, "F2 V"));
    // println!("{:?}", CSVParser::parse(Rule::field, "\"this\""));
    // println!("{:?}", CSVParser::parse(Rule::record, "X \"this\" 37 0 F"));
    // println!("{:?}", CSVParser::parse(Rule::file, "# comment\nX \"this\" 37 0 F\n"));

    // println!("{:?}", CSVParser::parse(Rule::file, " $FPLIST\n"));
    // println!("{:?}", CSVParser::parse(Rule::record, "DEF 2N3906 Q 0 0 Y N 1 F N"));
    // println!("{:?}", CSVParser::parse(Rule::file, &String::from_utf8_lossy(bytes)).expect("parsed"));
    // println!("{:?}", CSVParser::parse(Rule::field, "2F2"));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Nathan's App",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::new(MyApp::new(cc)))
        }),
    )
}

struct MyApp {}

impl Default for MyApp {
    fn default() -> Self {
        Self {}
    }
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "SaxMono".to_owned(),
            egui::FontData::from_static(include_bytes!("../fonts/saxmono.ttf"))
                .tweak(egui::FontTweak {
                    scale: 1.0,
                    y_offset_factor: 0.0,
                    y_offset: 0.0,
                    baseline_offset_factor: 0.1,
                })
                .into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("SaxMono".to_owned());
        cc.egui_ctx.set_fonts(fonts);
        Self::default()
    }
}

fn heading(text: &str) -> egui::Label {
    egui::Label::new(egui::RichText::new(text).font(egui::FontId::proportional(20.0)))
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(heading("Circuit"));
        });
    }
}
