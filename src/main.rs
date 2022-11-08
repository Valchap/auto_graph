#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Confirm exit", options, Box::new(|_cc| Box::new(MyApp {})));
}

struct MyApp {}

impl eframe::App for MyApp {
    fn on_close_event(&mut self) -> bool {
        true
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::light());

        egui::SidePanel::right("my_right_panel").show(ctx, |ui| {
            let sin: egui::plot::PlotPoints = (0..1000)
                .map(|i| {
                    let x = i as f64 * 0.01;
                    [x, x.sin()]
                })
                .collect();

            let sin2: egui::plot::PlotPoints = (0..1000)
                .map(|i| {
                    let x = i as f64 * 0.01;
                    [x, (2.0 * x).sin()]
                })
                .collect();

            let line = egui::plot::Line::new(sin);
            let line2 = egui::plot::Line::new(sin2);
            egui::plot::Plot::new("my_plot").show(ui, |plot_ui| {
                plot_ui.line(line);
                plot_ui.line(line2)
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let sin: egui::plot::PlotPoints = (0..1000)
                .map(|i| {
                    let x = i as f64 * 0.01;
                    [x, x.sin()]
                })
                .collect();

            let sin2: egui::plot::PlotPoints = (0..1000)
                .map(|i| {
                    let x = i as f64 * 0.01;
                    [x, (2.0 * x).sin()]
                })
                .collect();

            let line = egui::plot::Line::new(sin);
            let line2 = egui::plot::Line::new(sin2);
            egui::plot::Plot::new("my_plot").show(ui, |plot_ui| {
                plot_ui.line(line);
                plot_ui.line(line2)
            });
        });
    }
}
