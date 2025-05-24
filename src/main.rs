// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

use eframe::egui;
use serde_json::{Value};
use crate::egui::{Color32, Pos2, Shape, Stroke};
use glam::{Vec2, Affine2};

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

fn apply_affine2(v: &Pos2, a: &Affine2) -> Pos2 {
    let p = a.transform_point2(Vec2::new(v.x, v.y));
    let sx = a.transform_vector2(Vec2::new(1.0, 0.0));
    return Pos2::new(p.x, p.y);
}

fn drawline_to_shape(v: &Value, transform: &Affine2, color: Color32) -> Option<Shape> {
    let a = v.as_array().unwrap();
    let tag = &a[0];
    if tag.is_string() {
        let ts = tag.as_str().unwrap();
        match ts {
            "C" => {
                let (x, y, r, w);
                x = parse_number(&a[1]).unwrap();
                y = parse_number(&a[2]).unwrap();
                r = parse_number(&a[3]).unwrap();
                w = parse_number(&a[6]).unwrap();
                if a[7].as_str().unwrap() == "N" {
                    let c = apply_affine2(&Pos2::new(x, y), &transform);
                    return Some(Shape::circle_stroke(c, r, Stroke::new(w, color)));
                }
            },
            "P" => {
                let (n, w);
                n = parse_number(&a[1]).unwrap() as usize;
                // Get width, make sure it is at least 1.0 (0 means 1 pixel wide)
                w = parse_number(&a[4]).unwrap().max(2.0);
                let mut v: std::vec::Vec<Pos2> = vec![];
                for i in 0..n {
                    let (x, y);
                    x = parse_number(&a[5 + 2 * i]).unwrap();
                    y = parse_number(&a[6 + 2 * i]).unwrap();
                    let c = apply_affine2(&Pos2::new(x, y), &transform);
                    v.push(c);
                }
                let filled = a[5 + 2 * n].as_str().unwrap() == "F" && w == 2.0;
                if filled {
                    return Some(Shape::convex_polygon(v, color, Stroke::default()));
                } else {
                    return Some(Shape::line(v, Stroke::new(w, color)));
                }
            },
            "X" => {
                let (x, y, l, d, w);
                x = parse_number(&a[3]).unwrap();
                y = parse_number(&a[4]).unwrap();
                l = parse_number(&a[5]).unwrap();
                d = a[6].as_str().unwrap();
                w = 2.0;
                let vl = match d {
                    "U" => Pos2::new(0.0, 1.0),
                    "D" => Pos2::new(0.0, -1.0),
                    "L" => Pos2::new(-1.0, 0.0),
                    "R" => Pos2::new(1.0, 0.0),
                    &_ => unreachable!(),
                };
                let c1 = apply_affine2(&Pos2::new(x, y), &transform);
                let c2 = apply_affine2(&Pos2::new(x + l * vl.x, y + l * vl.y), &transform);
                return Some(Shape::line_segment([c1, c2], Stroke::new(w, color)))
            },
            &_ => return None,
        }
    }
    return None;
}

fn nth_color(i: usize) -> Color32 {
    match i % 6 {
        0 => Color32::WHITE,
        1 => Color32::YELLOW,
        2 => Color32::RED,
        3 => Color32::PURPLE,
        4 => Color32::LIGHT_RED,
        5 => Color32::LIGHT_BLUE,
        _ => unreachable!(),
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let nth = &self.lib[self.n][1];
        let draw = find_draw(&nth).unwrap();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(heading("Circuit"));
            if ui.add(egui::Button::new("Prev symbol")).clicked() {
                if self.n > 0 {
                    self.n -= 1;
                }
            }
            if ui.add(egui::Button::new("Next symbol")).clicked() {
                self.n += 1;
            }
            let max_n = self.lib.as_array().unwrap().len() - 1;
            if self.n > max_n {
                self.n = max_n;
            }
            if ui.add(egui::Button::new("Show draw")).clicked() {
                let partname = &nth[1][1].as_str().unwrap();
                println!("================  {}  ==========", partname);
                for i in draw.as_array().unwrap() {
                    println!("{:?}", i);
                }
            }
            let painter = ui.painter();
            let transform = Affine2::from_scale_angle_translation(Vec2::new(1.0, 1.0), 3.14159*0.0, Vec2::new(300.0, 250.0));
            let n = draw.as_array().unwrap().len();
            for i in 0..n {
                if let Some(s) = drawline_to_shape(&draw[i], &transform, nth_color(i)) {
                    painter.add(s);
                }    
            }
        });
    }
}
