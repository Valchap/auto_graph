use egui::plot::{BoxElem, BoxPlot, BoxSpread};
use egui_extras::Column;
use evalexpr::{
    eval_number_with_context, ContextWithMutableVariables, EvalexprError, HashMapContext,
};

const SAMPLE_COUNT: isize = 10;

#[derive(Clone)]
enum PopupStatus {
    None,
    ColumnSettings(usize),
}

#[derive(Clone)]
struct Value {
    raw_value: String,
    raw_uncertainty: String,
    value: f64,
    uncertainty: f64,
}

impl Value {
    const fn new() -> Self {
        Self {
            raw_value: String::new(),
            raw_uncertainty: String::new(),
            value: f64::NAN,
            uncertainty: 0.0,
        }
    }
}

pub struct App {
    grid_values: Vec<Vec<Value>>,
    columns: Vec<ColumnSettings>,
    popup_status: PopupStatus,
}

pub struct ColumnSettings {
    name: String,
    expression: String,
    precision: usize,
}

impl ColumnSettings {
    const fn new(name: String) -> Self {
        Self {
            name,
            expression: String::new(),
            precision: 3,
        }
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            grid_values: Vec::new(),
            columns: Vec::new(),
            popup_status: PopupStatus::None,
        };

        app.add_column("y".to_owned());
        app.add_column("x".to_owned());

        app
    }

    fn add_column(&mut self, name: String) {
        self.columns.push(ColumnSettings::new(name));

        for line in &mut self.grid_values {
            line.push(Value::new());
        }
    }

    fn remove_column(&mut self, index: usize) {
        self.columns.remove(index);

        for line in &mut self.grid_values {
            line.remove(index);
        }
    }

    fn add_line(&mut self) {
        self.grid_values
            .push(vec![Value::new(); self.columns.len()]);
    }

    fn ensure_empty_line(&mut self) {
        let mut last_empty_line = 0;

        'outer: for i in (0..self.grid_values.len()).rev() {
            for x in 0..self.columns.len() {
                if self.columns[x].expression.is_empty()
                    && (!self.grid_values[i][x].raw_value.is_empty()
                        || !self.grid_values[i][x].raw_uncertainty.is_empty())
                {
                    last_empty_line = i + 1;
                    break 'outer;
                }
            }
        }

        if last_empty_line >= self.grid_values.len() {
            self.add_line();
        } else {
            self.grid_values
                .drain((last_empty_line + 1)..self.grid_values.len());
        }
    }

    fn compute_line_value(&mut self, line_n: usize) {
        for column_n in (0..self.columns.len()).rev() {
            if !self.columns[column_n].expression.is_empty() {
                let mut context = HashMapContext::new();

                for i in (column_n + 1)..self.columns.len() {
                    context
                        .set_value(
                            self.columns[i].name.clone(),
                            self.grid_values[line_n][i].value.into(),
                        )
                        .unwrap();
                }

                let result = eval_number_with_context(&self.columns[column_n].expression, &context);

                self.grid_values[line_n][column_n].value = result.unwrap_or(f64::NAN);
            }
        }
    }

    fn compute_line_with_uncertainty(&mut self, line_n: usize) {
        self.compute_line_value(line_n);

        let mut reference_values: Vec<f64> = Vec::new();

        for column_n in 0..self.columns.len() {
            if self.columns[column_n].expression.is_empty() {
                reference_values.push(self.grid_values[line_n][column_n].value);
            }
        }

        if reference_values.is_empty() {
            return;
        }

        let mut samplers = vec![-SAMPLE_COUNT; reference_values.len()];

        let mut maxs = vec![f64::NAN; self.columns.len() - reference_values.len()];
        let mut mins = vec![f64::NAN; self.columns.len() - reference_values.len()];

        'outer: loop {
            let mut index = 0;

            for column_n in 0..self.columns.len() {
                if self.columns[column_n].expression.is_empty() {
                    self.grid_values[line_n][column_n].value = reference_values[index]
                        + samplers[index] as f64 * self.grid_values[line_n][column_n].uncertainty
                            / SAMPLE_COUNT as f64;

                    index += 1;
                }
            }

            self.compute_line_value(line_n);

            index = 0;

            for column_n in 0..self.columns.len() {
                if !self.columns[column_n].expression.is_empty() {
                    maxs[index] = maxs[index].max(self.grid_values[line_n][column_n].value);
                    mins[index] = mins[index].min(self.grid_values[line_n][column_n].value);
                    index += 1;
                }
            }

            for i in (0..samplers.len()).rev() {
                if samplers[i] < SAMPLE_COUNT {
                    samplers[i] += 1;
                    break;
                } else if i == 0 {
                    break 'outer;
                }
                samplers[i] = -SAMPLE_COUNT;
            }
        }

        let mut index = 0;
        for column_n in (0..self.columns.len()).rev() {
            if self.columns[column_n].expression.is_empty() {
                self.grid_values[line_n][column_n].value = reference_values.pop().unwrap();
            } else {
                self.grid_values[line_n][column_n].uncertainty = (maxs[index] - mins[index]) / 2.0;
                index += 1;
            }
        }

        self.compute_line_value(line_n);
    }

    fn compute_all(&mut self) {
        for line_n in 0..self.grid_values.len() - 1 {
            self.compute_line_with_uncertainty(line_n);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        if let PopupStatus::ColumnSettings(column_index) = self.popup_status.clone() {
            let mut open = true;
            egui::Window::new("⚙ column settings")
                .collapsible(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        let mut context = HashMapContext::new();

                        for i in (column_index + 1)..self.columns.len() {
                            context
                                .set_value(self.columns[i].name.clone(), f64::NAN.into())
                                .unwrap();
                        }

                        ui.label("Name");

                        if column_index > 1 {
                            let text_edit = egui::widgets::TextEdit::singleline(
                                &mut self.columns[column_index].name,
                            );

                            let name_input = ui.add(text_edit);

                            if name_input.lost_focus() {
                                self.columns[column_index].name =
                                    self.columns[column_index].name.trim().to_owned();
                            }
                        } else {
                            ui.label(self.columns[column_index].name.clone());
                        }

                        ui.label("Expression");

                        let result: Result<f64, EvalexprError> = eval_number_with_context(
                            &self.columns[column_index].expression,
                            &context,
                        );

                        let mut text_edit = egui::widgets::TextEdit::singleline(
                            &mut self.columns[column_index].expression,
                        );

                        if result.is_err() {
                            text_edit = text_edit.text_color(egui::Color32::RED);
                        }

                        let expression_input = ui.add(text_edit);

                        if expression_input.lost_focus() {
                            self.columns[column_index].expression =
                                self.columns[column_index].expression.trim().to_owned();
                        }

                        if expression_input.changed() {
                            self.compute_all();
                        }

                        ui.label("Precision");

                        let precision_edit = egui::widgets::DragValue::new(
                            &mut self.columns[column_index].precision,
                        )
                        .clamp_range(0..=10);

                        ui.add(precision_edit);

                        if column_index > 1 && ui.button("remove column").clicked() {
                            self.popup_status = PopupStatus::None;
                            self.remove_column(column_index);
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

            let mut box_list: Vec<BoxElem> = Vec::new();

            let mut x_square_sum = 0.;
            let mut xy_sum = 0.;

            for line in 0..self.grid_values.len() {
                let x = self.grid_values[line][1].value;
                let y = self.grid_values[line][0].value;
                let uncertainty = self.grid_values[line][0].uncertainty;

                if x.is_nan() || y.is_nan() {
                    continue;
                }

                box_list.push(BoxElem::new(
                    x,
                    BoxSpread::new(
                        y - uncertainty,
                        y - uncertainty / 2.,
                        y,
                        y + uncertainty / 2.,
                        y + uncertainty,
                    ),
                ));

                x_square_sum += x * x;
                xy_sum += x * y;
            }

            let slope = xy_sum / x_square_sum;

            ui.label(format!("Slope : {slope}"));

            let line = egui::plot::Line::new(egui::plot::PlotPoints::from_explicit_callback(
                move |x| slope * x,
                ..,
                1024,
            ))
            .width(2.)
            .color(egui::Color32::from_rgb(255, 63, 63));

            let box_plot = BoxPlot::new(box_list);

            //let line2 = egui::plot::Line::new(sin2);
            egui::plot::Plot::new("my_plot").show(ui, |plot_ui| {
                plot_ui.line(line);
                //plot_ui.line(line2);
                plot_ui.box_plot(box_plot);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let column_count = self.columns.len();

            egui_extras::TableBuilder::new(ui)
                .column(Column::initial(30.))
                .columns(Column::initial(50.), column_count * 2)
                .column(Column::remainder())
                .striped(true)
                .resizable(true)
                .header(20., |mut header| {
                    header.col(|ui| {
                        ui.heading("i");
                    });

                    for column_index in 0..self.columns.len() {
                        header.col(|ui| {
                            ui.horizontal(|ui| {
                                ui.heading(self.columns[column_index].name.clone());
                                if ui.button("⚙".to_owned()).clicked() {
                                    self.popup_status = PopupStatus::ColumnSettings(column_index);
                                }
                            });
                        });

                        header.col(|ui| {
                            ui.heading(format!("Δ{}", self.columns[column_index].name));
                        });
                    }

                    header.col(|ui| {
                        if ui.button("Add Column").clicked() {
                            self.add_column("a".to_owned());
                            self.popup_status = PopupStatus::ColumnSettings(column_count);
                        }
                    });
                })
                .body(|mut body| {
                    for y in 0..self.grid_values.len() {
                        body.row(20., |mut row| {
                            row.col(|ui| {
                                if y != self.grid_values.len() - 1 {
                                    ui.label((y + 1).to_string());
                                }
                            });

                            for x in 0..column_count {
                                // Values column
                                row.col(|ui| {
                                    if self.columns[x].expression.is_empty() {
                                        let invalid = self.grid_values[y][x].value.is_nan();

                                        let mut text_edit = egui::widgets::TextEdit::singleline(
                                            &mut self.grid_values[y][x].raw_value,
                                        );

                                        if invalid {
                                            text_edit = text_edit.text_color(egui::Color32::RED);
                                        }

                                        let input = ui.add(text_edit);

                                        if input.lost_focus() {
                                            self.grid_values[y][x].raw_value =
                                                self.grid_values[y][x].raw_value.trim().to_owned();
                                        }

                                        if input.changed() {
                                            self.grid_values[y][x].value = self.grid_values[y][x]
                                                .raw_value
                                                .trim()
                                                .parse::<f64>()
                                                .unwrap_or(f64::NAN);

                                            self.compute_line_with_uncertainty(y);
                                        }
                                    } else if y != self.grid_values.len() - 1 {
                                        let value = self.grid_values[y][x].value;

                                        let mut rich_text = egui::RichText::new(format!(
                                            "{:.*}",
                                            self.columns[x].precision, value
                                        ));

                                        if value.is_nan() {
                                            rich_text = rich_text.color(egui::Color32::RED);
                                        }

                                        ui.label(rich_text);
                                    }
                                });

                                // Uncertainty column
                                row.col(|ui| {
                                    if self.columns[x].expression.is_empty() {
                                        let invalid = self.grid_values[y][x].uncertainty.is_nan();

                                        let mut text_edit = egui::widgets::TextEdit::singleline(
                                            &mut self.grid_values[y][x].raw_uncertainty,
                                        );

                                        if invalid {
                                            text_edit = text_edit.text_color(egui::Color32::RED);
                                        }

                                        let input = ui.add(text_edit);

                                        if input.lost_focus() {
                                            self.grid_values[y][x].raw_uncertainty = self
                                                .grid_values[y][x]
                                                .raw_uncertainty
                                                .trim()
                                                .to_owned();
                                        }

                                        if input.changed() {
                                            if self.grid_values[y][x].raw_uncertainty.is_empty() {
                                                self.grid_values[y][x].uncertainty = 0.0;
                                            } else {
                                                self.grid_values[y][x].uncertainty = self
                                                    .grid_values[y][x]
                                                    .raw_uncertainty
                                                    .trim()
                                                    .parse::<f64>()
                                                    .unwrap_or(f64::NAN);
                                            }

                                            self.compute_line_with_uncertainty(y);
                                        }
                                    } else if y != self.grid_values.len() - 1 {
                                        let uncertainty = self.grid_values[y][x].uncertainty;

                                        let mut rich_text = egui::RichText::new(format!(
                                            "{:.*}",
                                            self.columns[x].precision, uncertainty
                                        ));
                                        if uncertainty.is_nan() {
                                            rich_text = rich_text.color(egui::Color32::RED);
                                        }

                                        ui.label(rich_text);
                                    }
                                });
                            }
                        });
                    }
                });

            self.ensure_empty_line();
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn on_close_event(&mut self) -> bool {
        true
    }
}
