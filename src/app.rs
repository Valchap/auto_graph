use std::collections::HashMap;

pub struct App {
    grid_values: HashMap<(u32, u32), String>,
    col_count: usize,
}

impl App {
    pub fn new() -> Self {
        App {
            grid_values: HashMap::new(),
            col_count: 3,
        }
    }

    pub fn get_value_mut(&mut self, key: (u32, u32)) -> &mut String {
        self.grid_values.entry(key).or_default()
    }
}

impl eframe::App for App {
    #[cfg(not(target_arch = "wasm32"))]
    fn on_close_event(&mut self) -> bool {
        true
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::light());

        egui::SidePanel::right("my_right_panel").show(ctx, |ui| {
            let _sin: egui::plot::PlotPoints = (0..1000)
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

            let line = egui::plot::Line::new(egui::plot::PlotPoints::from_explicit_callback(
                |x| 0.5 * (2.0 * x).sin(),
                ..,
                1024,
            ));
            let line2 = egui::plot::Line::new(sin2);
            egui::plot::Plot::new("my_plot").show(ui, |plot_ui| {
                plot_ui.line(line);
                plot_ui.line(line2)
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui_extras::TableBuilder::new(ui)
                .columns(egui_extras::Size::initial(100.0), self.col_count)
                .column(egui_extras::Size::exact(85.0))
                .striped(true)
                .resizable(true)
                .header(20.0, |mut header| {
                    for i in 0..self.col_count {
                        header.col(|ui| {
                            ui.heading(format!("Column {}", i));
                        });
                    }

                    header.col(|ui| {
                        if ui.button("New Column").clicked() {
                            self.col_count += 1;
                        }
                    });
                })
                .body(|mut body| {
                    for y in 0..3 {
                        body.row(20.0, |mut row| {
                            for x in 0..3 {
                                row.col(|ui| {
                                    ui.add(egui::widgets::TextEdit::singleline(
                                        self.get_value_mut((x, y)),
                                    ));
                                });
                            }
                        })
                    }
                });
        });
    }
}
