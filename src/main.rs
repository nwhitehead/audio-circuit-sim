// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

use eframe::egui;

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
}

impl Default for MyApp {
    fn default() -> Self {
        Self { }
    }
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "SaxMono".to_owned(),
            egui::FontData::from_static(include_bytes!("../fonts/saxmono.ttf")).tweak(
                egui::FontTweak {
                    scale: 1.0,
                    y_offset_factor: 0.0,
                    y_offset: 0.0,
                    baseline_offset_factor: 0.1,
                },
            ).into(),
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

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
        });
    }
}
