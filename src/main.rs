// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

use crate::egui::{Color32, Pos2, Rect, Shape, Stroke, StrokeKind};
use eframe::egui;
use serde_json::Value;

#[derive(Debug, Eq, Hash, PartialEq)]
enum ComponentType {
    Capacitor,
    Diode,
    DiodeSchottky,
    DiodeZener,
    Inductor,
    Led,
    OpAmp,
    TransistorNPN,
    TransistorNPNDarlington,
    TransistorPNP,
    TransistorPNPDarlington,
    Resistor,
    ResistorUS,
    Potentiometer,
    PotentiometerUS,
    VoltmeterDC,
    Wire,
}

fn string_to_componenttype(n: &str) -> Option<ComponentType> {
    match n {
        "C" => Some(ComponentType::Capacitor),
        "D" => Some(ComponentType::Diode),
        "D_Schottky" => Some(ComponentType::DiodeSchottky),
        "D_Zener" => Some(ComponentType::DiodeZener),
        "L" => Some(ComponentType::Inductor),
        "LED" => Some(ComponentType::Led),
        "Opamp_Dual" => Some(ComponentType::OpAmp),
        "Q_NPN_BCE" => Some(ComponentType::TransistorNPN),
        "Q_NPN_Darlington_BCE" => Some(ComponentType::TransistorNPNDarlington),
        "Q_PNP_BCE" => Some(ComponentType::TransistorPNP),
        "Q_PNP_Darlington_BCE" => Some(ComponentType::TransistorPNPDarlington),
        "R" => Some(ComponentType::Resistor),
        "R_Potentiometer" => Some(ComponentType::Potentiometer),
        "R_US" => Some(ComponentType::ResistorUS),
        "R_Potentiometer_US" => Some(ComponentType::PotentiometerUS),
        "Voltmeter_DC" => Some(ComponentType::VoltmeterDC),
        &_ => None,
    }
}

/// Maps component type into draw instructions
type ComponentDrawLibrary = std::collections::HashMap<ComponentType, Value>;

#[derive(Debug)]
struct GraphicalComponent {
    component_type: ComponentType,
    position: Pos2,
    angle: f32,
}

impl GraphicalComponent {
    fn new(component_type: ComponentType, position: Pos2, angle: f32) -> Self {
        Self {
            component_type,
            position,
            angle,
        }
    }
}
fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Circuit App",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::new(MyApp::new(cc)))
        }),
    )
}

struct MyApp {
    draw_lib: ComponentDrawLibrary,
    graphical_parts: Vec<GraphicalComponent>,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut draw_lib = ComponentDrawLibrary::new();
        let bytes = include_bytes!("./circuit.json");
        let lib: Value = serde_json::from_slice(bytes).unwrap();
        for elem in lib.as_array().unwrap() {
            let name = elem[1][1][1]
                .as_str()
                .expect("Could not parse string in library");
            let comp = string_to_componenttype(name).expect("Unknown component type");
            println!("{:?}", comp);
            draw_lib.insert(
                comp,
                find_draw(&elem[1])
                    .expect("Could not find DRAW line in library")
                    .clone(),
            );
        }
        let graphical_parts = vec![
            GraphicalComponent::new(ComponentType::Resistor, Pos2::new(200.0, 200.0), 0.0),
            GraphicalComponent::new(ComponentType::Resistor, Pos2::new(300.0, 200.0), 0.0),
        ];

        Self { draw_lib, graphical_parts }
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

/// Add heading text UI element (big font)
fn heading(text: &str) -> egui::Label {
    egui::Label::new(egui::RichText::new(text).font(egui::FontId::proportional(20.0)))
}

/// Given JSON library chunk of a component, extract part that is DRAW if found
fn find_draw(v: &Value) -> Option<&Value> {
    for i in 0..v.as_array().unwrap().len() {
        if v[i][0] == serde_json::Value::String("DRAW".into()) {
            return Some(&v[i][1]);
        }
    }
    return None;
}

/// Given a JSON value, try to parse as a f32 number
fn parse_number(v: &Value) -> Option<f32> {
    match v.as_number() {
        Some(n) => Some(n.as_f64().unwrap() as f32),
        None => None,
    }
}

struct Transform {
    scale: f32,
    rotate: f32,
    translate: Pos2,
}

impl Transform {
    fn new(scale: f32, rotate: f32, translate_x: f32, translate_y: f32) -> Self {
        Self {
            scale,
            rotate,
            translate: Pos2::new(translate_x, translate_y),
        }
    }
    /// Chain two transforms into one transform (order is other then this one)
    fn chain(&self, other: &Self) -> Self {
        Transform::new(self.scale * other.scale, self.rotate + other.rotate, self.translate.x + other.translate.x, self.translate.y + other.translate.y)
    }
    fn apply(&self, a: &Pos2) -> Pos2 {
        let c = self.rotate.cos();
        let s = self.rotate.sin();
        return Pos2::new(
            (a.x * c - a.y * s) * self.scale + self.translate.x,
            (a.x * s + a.y * c) * self.scale + self.translate.y,
        );
    }
    fn apply_scalar(&self, a: f32) -> f32 {
        return self.scale * a;
    }
}

/// Helper function for draw_to_shape
// Turns one line of DRAW section into a Shape
fn drawline_to_shape(v: &Value, transform: &Transform, color: Color32) -> Option<Shape> {
    let a = v.as_array().unwrap();
    let tag = &a[0];
    let w_fine_orig = 2.0;
    let w_fine = transform.apply_scalar(w_fine_orig);
    if tag.is_string() {
        let ts = tag.as_str().unwrap();
        match ts {
            "A" => {
                // Arc
                let (x, y, r, angle_start, angle_end, w);
                x = parse_number(&a[1]).unwrap();
                y = parse_number(&a[2]).unwrap();
                r = parse_number(&a[3]).unwrap();
                // Angles measured in 1/10s of degrees
                angle_start = parse_number(&a[4]).unwrap() / 10.0 / 360.0 * 2.0 * 3.14159265;
                angle_end = parse_number(&a[5]).unwrap() / 10.0 / 360.0 * 2.0 * 3.14159265;
                w = parse_number(&a[8]).unwrap().max(w_fine_orig);
                let w = transform.apply_scalar(w);
                let mut v: std::vec::Vec<Pos2> = vec![];
                let num = 10;
                for i in 0..num + 1 {
                    let a = angle_start + (i as f32 / num as f32) * (angle_end - angle_start);
                    let xx = a.cos() * r + x;
                    let yy = a.sin() * r + y;
                    let c = transform.apply(&Pos2::new(xx, yy));
                    v.push(c);
                }
                return Some(Shape::line(v, Stroke::new(w, color)));
            }
            "C" => {
                // Circle
                let (x, y, r, w);
                x = parse_number(&a[1]).unwrap();
                y = parse_number(&a[2]).unwrap();
                r = parse_number(&a[3]).unwrap();
                w = parse_number(&a[6]).unwrap();
                if a[7].as_str().unwrap() == "N" {
                    let c = transform.apply(&Pos2::new(x, y));
                    let r = transform.apply_scalar(r);
                    let w = transform.apply_scalar(w);
                    return Some(Shape::circle_stroke(c, r, Stroke::new(w, color)));
                }
            }
            "P" => {
                // Polyline
                let (n, w);
                n = parse_number(&a[1]).unwrap() as usize;
                // Get width, make sure it is at least 1.0 (0 in lib means 1 pixel wide)
                w = parse_number(&a[4]).unwrap().max(w_fine_orig);
                let w = transform.apply_scalar(w);
                let mut v: std::vec::Vec<Pos2> = vec![];
                for i in 0..n {
                    let (x, y);
                    x = parse_number(&a[5 + 2 * i]).unwrap();
                    y = parse_number(&a[6 + 2 * i]).unwrap();
                    let c = transform.apply(&Pos2::new(x, y));
                    v.push(c);
                }
                let filled = a[5 + 2 * n].as_str().unwrap() == "F" && w == w_fine;
                if filled {
                    return Some(Shape::convex_polygon(v, color, Stroke::default()));
                } else {
                    let mut res = vec![];
                    // Add individual line segments connecting pairs.
                    // This avoids jagged connectors that extend beyond radius of line bend.
                    for i in 0..v.len() - 1 {
                        res.push(Shape::line_segment([v[i], v[i + 1]], Stroke::new(w, color)));
                    }
                    // Add circles to connect lines.
                    // Smaller than width / 2 to make it visually look better.
                    for p in v {
                        res.push(Shape::circle_filled(p, w * 0.48, color));
                    }
                    return Some(Shape::Vec(res));
                }
            }
            "S" => {
                // Rectangle
                let (sx, sy, ex, ey, w);
                sx = parse_number(&a[1]).unwrap();
                sy = parse_number(&a[2]).unwrap();
                ex = parse_number(&a[3]).unwrap();
                ey = parse_number(&a[4]).unwrap();
                w = parse_number(&a[7]).unwrap().max(w_fine_orig);
                let w = transform.apply_scalar(w);

                // Create vector with points on rectangle
                let v = vec![
                    transform.apply(&Pos2::new(sx, sy)),
                    transform.apply(&Pos2::new(ex, sy)),
                    transform.apply(&Pos2::new(ex, ey)),
                    transform.apply(&Pos2::new(sx, ey)),
                    transform.apply(&Pos2::new(sx, sy)),
                ];
                // Add individual line segments connecting pairs.
                // This avoids jagged connectors that extend beyond radius of line bend.
                let mut res = vec![];
                for i in 0..v.len() - 1 {
                    res.push(Shape::line_segment([v[i], v[i + 1]], Stroke::new(w, color)));
                }
                // Add circles to connect lines.
                // Smaller than width / 2 to make it visually look better.
                for p in v {
                    res.push(Shape::circle_filled(p, w * 0.48, color));
                }
                return Some(Shape::Vec(res));
            }
            "X" => {
                // Pin
                let (x, y, l, d, w);
                x = parse_number(&a[3]).unwrap();
                y = parse_number(&a[4]).unwrap();
                l = parse_number(&a[5]).unwrap();
                d = a[6].as_str().unwrap();
                w = w_fine;
                let vl = match d {
                    "U" => Pos2::new(0.0, 1.0),
                    "D" => Pos2::new(0.0, -1.0),
                    "L" => Pos2::new(-1.0, 0.0),
                    "R" => Pos2::new(1.0, 0.0),
                    &_ => unreachable!(),
                };
                let c1 = transform.apply(&Pos2::new(x, y));
                let c2 = transform.apply(&Pos2::new(x + l * vl.x, y + l * vl.y));
                return Some(Shape::line_segment([c1, c2], Stroke::new(w, color)));
            }
            &_ => return None,
        }
    }
    return None;
}

/// Given DRAW JSON value, turn section into single Shape for drawing
fn draw_to_shape(v: &Value, transform: &Transform, color: Color32) -> Shape {
    let mut shapes = vec![];
    let n = v.as_array().unwrap().len();
    for i in 0..n {
        if let Some(s) = drawline_to_shape(&v[i], &transform, color) {
            shapes.push(s);
        }
    }
    return Shape::Vec(shapes);
}

/// Find pad positions of symbol, given DRAW section
fn draw_to_padpos(v: &Value, transform: &Transform) -> Vec<Pos2> {
    let mut res = vec![];
    let n = v.as_array().unwrap().len();
    for i in 0..n {
        let vi = &v[i];
        let a = vi.as_array().unwrap();
        let tag = &a[0];
        let ts = tag.as_str().unwrap();
        if ts == "X" {
            let (x, y);
            x = parse_number(&a[3]).unwrap();
            y = parse_number(&a[4]).unwrap();
            res.push(transform.apply(&Pos2::new(x, y)));
        }
    }
    return res;
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(heading("Circuit"));
            let painter = ui.painter();
            let color = Color32::WHITE;
            let lead_color = Color32::YELLOW;
            let global_transform = Transform::new(1.0, 0.0, 0.0, 0.0);
            for component in &self.graphical_parts {
                let draw_instr = &self.draw_lib[&component.component_type];
                let transform = global_transform.chain(&Transform::new(1.0, 3.14159*0.1, component.position.x, component.position.y));
                painter.add(draw_to_shape(&draw_instr, &transform, color));
                let leads = draw_to_padpos(&draw_instr, &transform);
                for lead in leads {
                    painter.add(Shape::circle_filled(lead, 5.0, lead_color));
                }
            }
        });
    }
}
