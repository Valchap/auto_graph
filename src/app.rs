use evalexpr::*;

#[derive(Clone)]
enum PopupStatus {
    None,
    ColumnSettings(String),
}

pub struct App {
    grid_values: Vec<Vec<String>>,
    columns: Vec<ColumnSettings>,
    popup_status: PopupStatus,
}

pub struct ColumnSettings {
    name: String,
    formula: String,
}

impl ColumnSettings {
    pub fn new(name: String) -> Self {
        ColumnSettings {
            name,
            formula: "".to_owned(),
        }
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = App {
            grid_values: vec![vec![]],
            columns: vec![],
            popup_status: PopupStatus::None,
        };

        app.add_column("y".to_owned());
        app.add_column("x".to_owned());

        app
    }

    pub fn get_column_by_name_mut(&mut self, name: &String) -> Option<&mut ColumnSettings> {
        self.columns.iter_mut().find(|column| column.name == *name)
    }

    pub fn add_column(&mut self, name: String) {
        self.columns.push(ColumnSettings::new(name));

        for line in self.grid_values.iter_mut() {
            line.push("".to_owned());
        }
    }

    pub fn add_line(&mut self) {
        self.grid_values
            .push(vec!["".to_owned(); self.columns.len()]);
    }

    pub fn ensure_empty_line(&mut self) {
        let mut last_empty_line = 0;

        for i in (0..self.grid_values.len()).rev() {
            if !self.grid_values[i].iter().all(|s| s.is_empty()) {
                last_empty_line = i + 1;
                break;
            }
        }

        for _ in ((last_empty_line + 1)..self.grid_values.len()).rev() {
            self.grid_values.pop();
        }

        if last_empty_line >= self.grid_values.len() {
            self.add_line();
        }
    }

    pub fn get_value(&self, line: usize, column: usize) -> f64 {
        if self.columns[column].formula.is_empty() {
            self.grid_values[line][column]
                .parse::<f64>()
                .unwrap_or(f64::NAN)
        } else {
            let mut context = HashMapContext::new();

            for i in (column + 1)..self.columns.len() {
                context
                    .set_value(self.columns[i].name.clone(), self.get_value(line, i).into())
                    .unwrap();
            }

            let result: Result<f64, EvalexprError> =
                eval_number_with_context(&self.columns[column].formula, &context);

            result.unwrap_or(f64::NAN)
        }
    }
}

impl eframe::App for App {
    #[cfg(not(target_arch = "wasm32"))]
    fn on_close_event(&mut self) -> bool {
        true
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let PopupStatus::ColumnSettings(name) = self.popup_status.clone() {
            let mut open = true;
            egui::Window::new(format!("⚙ column {name}"))
                .collapsible(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        let mut context = HashMapContext::new();

                        for column in self.columns.iter() {
                            context
                                .set_value(column.name.clone(), f64::NAN.into())
                                .unwrap();
                        }

                        if let Some(column) = self.get_column_by_name_mut(&name) {
                            ui.label("Expression");

                            let result: Result<f64, EvalexprError> =
                                eval_number_with_context(&column.formula, &context);

                            let mut text_edit =
                                egui::widgets::TextEdit::singleline(&mut column.formula);

                            if result.is_err() {
                                text_edit = text_edit.text_color(egui::Color32::RED);
                            }

                            let input = ui.add(text_edit);

                            if input.lost_focus() {
                                column.formula = column.formula.trim().to_owned();
                            }
                        }
                    });
                });
            if !open {
                self.popup_status = PopupStatus::None;
            }
        }

        egui::SidePanel::right("my_right_panel").show(ctx, |ui| {
            /*let _sin: egui::plot::PlotPoints = (0..1000)
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
                .collect();*/

            /*let mut plot_ements:Vec<BoxElem> = Vec::new();

            let mut box1 = BoxPlot::new(vec![
                BoxElem::new(0.5, BoxSpread::new(1.5, 2.2, 2.5, 2.6, 3.1)).name("Day 1"),
                BoxElem::new(2.5, BoxSpread::new(0.4, 1.0, 1.1, 1.4, 2.1)).name("Day 2"),
                BoxElem::new(4.5, BoxSpread::new(1.7, 2.0, 2.2, 2.5, 2.9)).name("Day 3"),
            ]);*/

            let mut point_list: Vec<[f64; 2]> = Vec::new();

            let mut x_square_sum = 0.0;
            let mut xy_sum = 0.0;

            for line in 0..self.grid_values.len() {
                let x = self.get_value(line, 0);
                let y = self.get_value(line, 1);

                if x.is_nan() || y.is_nan() {
                    continue;
                }

                point_list.push([x, y]);

                x_square_sum += x * x;
                xy_sum += x * y;
            }

            let slope = xy_sum / x_square_sum;

            ui.label(format!("Slope : {slope}"));

            let line = egui::plot::Line::new(egui::plot::PlotPoints::from_explicit_callback(
                move |x| slope * x,
                ..,
                1024,
            ));

            let points = egui::plot::Points::new(point_list)
                .radius(5.0)
                .color(egui::Color32::RED);

            //let line2 = egui::plot::Line::new(sin2);
            egui::plot::Plot::new("my_plot").show(ui, |plot_ui| {
                plot_ui.line(line);
                //plot_ui.line(line2);
                //plot_ui.box_plot(box1);
                plot_ui.points(points);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui_extras::TableBuilder::new(ui)
                .column(egui_extras::Size::exact(40.0))
                .columns(egui_extras::Size::initial(100.0), self.columns.len())
                .column(egui_extras::Size::exact(85.0))
                .striped(true)
                .resizable(true)
                .header(20.0, |mut header| {
                    header.col(|_ui| {});

                    for column in &self.columns {
                        header.col(|ui| {
                            ui.horizontal(|ui| {
                                ui.heading(column.name.to_owned());
                                if ui.button("⚙".to_owned()).clicked() {
                                    self.popup_status =
                                        PopupStatus::ColumnSettings(column.name.clone());
                                }
                            });
                        });
                    }

                    header.col(|ui| {
                        if ui.button("New Column").clicked() {
                            self.add_column("a".to_owned());
                        }
                    });
                })
                .body(|mut body| {
                    for y in 0..self.grid_values.len() {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                if y != self.grid_values.len() - 1 {
                                    ui.label(format!("{}", y + 1));
                                }
                            });

                            for x in 0..self.columns.len() {
                                row.col(|ui| {
                                    let valid = !self.get_value(y, x).is_nan();

                                    let mut text_edit = egui::widgets::TextEdit::singleline(
                                        &mut self.grid_values[y][x],
                                    );

                                    if !valid {
                                        text_edit = text_edit.text_color(egui::Color32::RED);
                                    }

                                    let input = ui.add(text_edit);

                                    if input.lost_focus() {
                                        self.grid_values[y][x] =
                                            self.grid_values[y][x].trim().to_owned();
                                    }
                                });
                            }
                        })
                    }
                });

            self.ensure_empty_line();
        });
    }
}
