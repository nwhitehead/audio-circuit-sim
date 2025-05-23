// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

use eframe::egui;
use serde_json::{Value};
use crate::egui::{Color32, Pos2, Shape, Stroke};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

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

struct MyApp {
    lib: serde_json::Value,
    n: usize,
}

impl Default for MyApp {
    fn default() -> Self {
        let bytes = include_bytes!("./BJT.json");
        let parsed: Value = serde_json::from_slice(bytes).unwrap();
        Self {
            lib: parsed,
            n: 0,
        }
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

fn find_draw(v: &Value) -> Option<&Value> {
    for i in 0..v.as_array().unwrap().len() {
        if v[i][0] == serde_json::Value::String("DRAW".into()) {
            return Some(&v[i][1]);
        }
    }
    return None;
}

fn parse_number(v: &Value) -> Option<f32> {
    match v.as_number() {
        Some(n) => Some(n.as_f64().unwrap() as f32),
        None => None,
    }
}

fn drawline_to_shape(v: &Value) -> Option<Shape> {
    let a = v.as_array().unwrap();
    let tag = &a[0];
    if tag.is_string() {
        let ts = tag.as_str().unwrap();
        match ts {
            "C" => {
                let (x, y, r, w);
                x = parse_number(&a[1]).unwrap() + 150.0;
                y = parse_number(&a[2]).unwrap() + 200.0;
                r = parse_number(&a[3]).unwrap();
                w = parse_number(&a[6]).unwrap();
                if a[7].as_str().unwrap() == "N" {
                    return Some(Shape::circle_stroke(Pos2::new(x, y), r, Stroke::new(w, Color32::WHITE)));
                    println!("C {:?} : {} {}", a, x, y);
                }
            },
            "P" => {
                println!("{:?}", a);
                let (n, w);
                n = parse_number(&a[1]).unwrap() as usize;
                // Get width, make sure it is at least 1.0 (0 means 1 pixel wide)
                w = parse_number(&a[4]).unwrap().max(2.0);
                let mut v: std::vec::Vec<Pos2> = vec![];
                for i in 0..n {
                    let (x, y);
                    x = parse_number(&a[5 + 2 * i]).unwrap() + 150.0;
                    y = parse_number(&a[6 + 2 * i]).unwrap() + 200.0;
                    v.push(Pos2::new(x, y));
                }
                let filled = a[5 + 2 * n].as_str().unwrap() == "F";
                //return Some(Shape::convex_polygon(v, Color32::WHITE, Stroke::new(w, Color32::WHITE)));
                if filled {
                    return Some(Shape::convex_polygon(v, Color32::WHITE, Stroke::new(w, Color32::WHITE)));
                } else {
                    return Some(Shape::line(v, Stroke::new(w, Color32::WHITE)));
                }
            },
            &_ => return None,
        }
    }
    //return Some(Shape::circle_filled(Pos2::new(100.0, 100.0), 20.0, Color32::WHITE));
    return None;
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let nth = &self.lib[self.n][1];
        let draw = find_draw(&nth).unwrap();
        //println!("{:?}", draw);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(heading("Circuit"));
            let painter = ui.painter();
            let c = Shape::circle_filled(Pos2::new(100.0, 100.0), 20.0, Color32::WHITE);
            let n = draw.as_array().unwrap().len();
            for i in 0..n {
                if let Some(s) = drawline_to_shape(&draw[i]) {
                    painter.add(s);
                }    
            }
        });
    }
}
