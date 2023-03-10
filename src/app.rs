use eframe::Storage;
use egui::{
    plot::{BoxElem, BoxPlot, BoxSpread, Line, Plot, PlotPoints},
    CentralPanel, Color32, Context, DragValue, RichText, SidePanel, Stroke, TextEdit,
    TopBottomPanel, Visuals, Window,
};
use egui_extras::{Column, TableBuilder};
use evalexpr::{
    eval_number_with_context, ContextWithMutableVariables, EvalexprError, HashMapContext,
};

const SAMPLE_COUNT: isize = 1;

const DARK_THEME_KEY: &str = "dark_them";
const VERTICAL_BOX_PLOT_KEY: &str = "vertical_box_plot";
const FULL_BOX_PLOT_KEY: &str = "full_box_plot";
const LINEAR_REGRESSION_KEY: &str = "linear_regression";

const COLUMN_COUNT_KEY: &str = "column_count";
const LINE_COUNT_KEY: &str = "line_count";
const COLUMN_NAME_KEY: &str = "column_name";
const COLUMN_EXPRESSION_KEY: &str = "column_expression";
const COLUMN_PRECISION_KEY: &str = "column_precision";
const GRID_VALUE_KEY: &str = "grid_value";
const GRID_UNCERTAINTY_KEY: &str = "grid_uncertainty";

#[derive(Clone)]
enum PopupStatus {
    None,
    ColumnSettings(usize),
    GlobalSettings,
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
            uncertainty: 0.,
        }
    }
}

pub struct App {
    grid: Vec<Vec<Value>>,
    columns: Vec<ColumnSettings>,
    popup_status: PopupStatus,
    dark_theme: bool,
    vertical_box_plot: bool,
    full_box_plot: bool,
    linear_regression: bool,
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
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut app = Self {
            grid: Vec::new(),
            columns: Vec::new(),
            popup_status: PopupStatus::None,
            dark_theme: false,
            vertical_box_plot: true,
            full_box_plot: false,
            linear_regression: true,
        };

        if let Some(storage) = cc.storage {
            if let Some(dark_theme_str) = storage.get_string(DARK_THEME_KEY) {
                if let Ok(dark_theme) = dark_theme_str.parse::<bool>() {
                    app.dark_theme = dark_theme;
                }
            }

            if let Some(vertical_box_plot_str) = storage.get_string(VERTICAL_BOX_PLOT_KEY) {
                if let Ok(vertical_box_plot) = vertical_box_plot_str.parse::<bool>() {
                    app.vertical_box_plot = vertical_box_plot;
                }
            }

            if let Some(full_box_plot_str) = storage.get_string(FULL_BOX_PLOT_KEY) {
                if let Ok(full_box_plot) = full_box_plot_str.parse::<bool>() {
                    app.full_box_plot = full_box_plot;
                }
            }

            if let Some(linear_regression_str) = storage.get_string(LINEAR_REGRESSION_KEY) {
                if let Ok(linear_regression) = linear_regression_str.parse::<bool>() {
                    app.linear_regression = linear_regression;
                }
            }

            if let Some(column_count_str) = storage.get_string(COLUMN_COUNT_KEY) {
                if let Ok(column_count) = column_count_str.parse::<usize>() {
                    if column_count >= 2 {
                        for column_n in 0..column_count {
                            let column_name = storage
                                .get_string(&format!("{COLUMN_NAME_KEY}_{column_n}"))
                                .unwrap();

                            let column_expression = storage
                                .get_string(&format!("{COLUMN_EXPRESSION_KEY}_{column_n}"))
                                .unwrap();

                            let column_precision = storage
                                .get_string(&format!("{COLUMN_PRECISION_KEY}_{column_n}"))
                                .unwrap()
                                .parse::<usize>()
                                .unwrap();

                            app.add_column(column_name);
                            let last = app.columns.last_mut().unwrap();
                            last.expression = column_expression;
                            last.precision = column_precision;
                        }

                        if let Some(line_count_str) = storage.get_string(LINE_COUNT_KEY) {
                            if let Ok(line_count) = line_count_str.parse::<usize>() {
                                for column_n in 0..column_count {
                                    for line_n in 0..line_count {
                                        let value = storage
                                            .get_string(&format!(
                                                "{GRID_VALUE_KEY}_{line_n}_{column_n}"
                                            ))
                                            .unwrap();

                                        let uncertainty = storage
                                            .get_string(&format!(
                                                "{GRID_UNCERTAINTY_KEY}_{line_n}_{column_n}"
                                            ))
                                            .unwrap();

                                        app.add_line();

                                        app.grid[line_n][column_n].raw_value = value;
                                        app.grid[line_n][column_n].raw_uncertainty = uncertainty;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        app.compute_and_parse_all();

        if app.columns.is_empty() {
            app.add_column("y".to_owned());
            app.add_column("x".to_owned());
        }

        app
    }

    fn add_column(&mut self, name: String) {
        self.columns.push(ColumnSettings::new(name));

        for line in &mut self.grid {
            line.push(Value::new());
        }
    }

    fn remove_column(&mut self, index: usize) {
        self.columns.remove(index);

        for line in &mut self.grid {
            line.remove(index);
        }
    }

    fn add_line(&mut self) {
        self.grid.push(vec![Value::new(); self.columns.len()]);
    }

    fn ensure_empty_line(&mut self) {
        let mut last_empty_line = 0;

        'outer: for i in (0..self.grid.len()).rev() {
            for x in 0..self.columns.len() {
                if self.columns[x].expression.is_empty()
                    && (!self.grid[i][x].raw_value.is_empty()
                        || !self.grid[i][x].raw_uncertainty.is_empty())
                {
                    last_empty_line = i + 1;
                    break 'outer;
                }
            }
        }

        if last_empty_line >= self.grid.len() {
            self.add_line();
        } else {
            self.grid.drain((last_empty_line + 1)..self.grid.len());
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
                            self.grid[line_n][i].value.into(),
                        )
                        .unwrap();
                }

                let result = eval_number_with_context(&self.columns[column_n].expression, &context);

                self.grid[line_n][column_n].value = result.unwrap_or(f64::NAN);
            }
        }
    }

    fn compute_line_with_uncertainty(&mut self, line_n: usize) {
        self.compute_line_value(line_n);

        let mut reference_values: Vec<f64> = Vec::new();

        for column_n in 0..self.columns.len() {
            if self.columns[column_n].expression.is_empty() {
                reference_values.push(self.grid[line_n][column_n].value);
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
                    self.grid[line_n][column_n].value = reference_values[index]
                        + samplers[index] as f64 * self.grid[line_n][column_n].uncertainty
                            / SAMPLE_COUNT as f64;

                    index += 1;
                }
            }

            self.compute_line_value(line_n);

            index = 0;

            for column_n in 0..self.columns.len() {
                if !self.columns[column_n].expression.is_empty() {
                    maxs[index] = maxs[index].max(self.grid[line_n][column_n].value);
                    mins[index] = mins[index].min(self.grid[line_n][column_n].value);
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

        let mut index = self.columns.len() - reference_values.len();
        for column_n in (0..self.columns.len()).rev() {
            if self.columns[column_n].expression.is_empty() {
                self.grid[line_n][column_n].value = reference_values.pop().unwrap();
            } else {
                index -= 1;
                self.grid[line_n][column_n].uncertainty = (maxs[index] - mins[index]) / 2.;
            }
        }

        self.compute_line_value(line_n);
    }

    fn compute_all(&mut self) {
        for line_n in 0..self.grid.len() {
            self.compute_line_with_uncertainty(line_n);
        }
    }

    fn compute_and_parse_all(&mut self) {
        for line_n in 0..self.grid.len() {
            for column_n in 0..self.columns.len() {
                self.grid[line_n][column_n].value = self.grid[line_n][column_n]
                    .raw_value
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(f64::NAN);
                self.grid[line_n][column_n].uncertainty = self.grid[line_n][column_n]
                    .raw_uncertainty
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(f64::NAN);
            }
        }

        self.compute_all();
    }

    fn show_global_settings(&mut self, ctx: &Context) {
        let mut open = true;
        Window::new("⚙ settings")
            .collapsible(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Box plot orientation");
                    ui.radio_value(&mut self.vertical_box_plot, true, "Vertical");
                    ui.radio_value(&mut self.vertical_box_plot, false, "Horizontal");
                    ui.label("Box plot style");
                    ui.radio_value(&mut self.full_box_plot, true, "Full box plot");
                    ui.radio_value(&mut self.full_box_plot, false, "Whisker box plot");
                    ui.label("Regression type");
                    ui.radio_value(&mut self.linear_regression, true, "Linear regression");
                    ui.radio_value(&mut self.linear_regression, false, "Affine regression");
                })
            });

        if !open {
            self.popup_status = PopupStatus::None;
        }
    }

    fn show_column_settings(&mut self, ctx: &Context, column_index: usize) {
        let mut open = true;
        Window::new("⚙ column settings")
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
                        let text_edit = TextEdit::singleline(&mut self.columns[column_index].name);

                        let name_input = ui.add(text_edit);

                        if name_input.lost_focus() {
                            self.columns[column_index].name =
                                self.columns[column_index].name.trim().to_owned();
                        }
                    } else {
                        ui.label(self.columns[column_index].name.clone());
                    }

                    ui.label("Expression");

                    let result: Result<f64, EvalexprError> =
                        eval_number_with_context(&self.columns[column_index].expression, &context);

                    let mut text_edit =
                        TextEdit::singleline(&mut self.columns[column_index].expression);

                    if result.is_err() {
                        text_edit = text_edit.text_color(Color32::RED);
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

                    let precision_edit = DragValue::new(&mut self.columns[column_index].precision)
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
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn Storage) {
        storage.set_string(DARK_THEME_KEY, self.dark_theme.to_string());
        storage.set_string(VERTICAL_BOX_PLOT_KEY, self.vertical_box_plot.to_string());
        storage.set_string(FULL_BOX_PLOT_KEY, self.full_box_plot.to_string());
        storage.set_string(LINEAR_REGRESSION_KEY, self.linear_regression.to_string());

        storage.set_string(COLUMN_COUNT_KEY, self.columns.len().to_string());
        storage.set_string(LINE_COUNT_KEY, self.grid.len().to_string());

        for column_n in 0..self.columns.len() {
            storage.set_string(
                &format!("{COLUMN_NAME_KEY}_{column_n}"),
                self.columns[column_n].name.clone(),
            );

            storage.set_string(
                &format!("{COLUMN_EXPRESSION_KEY}_{column_n}"),
                self.columns[column_n].expression.clone(),
            );

            storage.set_string(
                &format!("{COLUMN_PRECISION_KEY}_{column_n}"),
                self.columns[column_n].precision.to_string(),
            );

            for line_n in 0..self.grid.len() {
                storage.set_string(
                    &format!("{GRID_VALUE_KEY}_{line_n}_{column_n}"),
                    self.grid[line_n][column_n].raw_value.clone(),
                );
                storage.set_string(
                    &format!("{GRID_UNCERTAINTY_KEY}_{line_n}_{column_n}"),
                    self.grid[line_n][column_n].raw_uncertainty.clone(),
                );
            }
        }

        // Prevent eframe from saving unneeded data
        storage.set_string("egui", String::new());
        storage.set_string("window", String::new());
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(if self.dark_theme {
            Visuals::dark()
        } else {
            Visuals::light()
        });

        TopBottomPanel::top("settings_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |bar_ui| {
                if bar_ui
                    .button(if self.dark_theme { "Light" } else { "Dark" })
                    .clicked()
                {
                    self.dark_theme = !self.dark_theme;
                }

                if bar_ui.button("Settings").clicked() {
                    self.popup_status = PopupStatus::GlobalSettings;
                }
            })
        });

        match self.popup_status {
            PopupStatus::ColumnSettings(column_index) => {
                self.show_column_settings(ctx, column_index);
            }
            PopupStatus::GlobalSettings => self.show_global_settings(ctx),
            PopupStatus::None => {}
        }

        CentralPanel::default().show(ctx, |ui| {
            let column_count = self.columns.len();

            TableBuilder::new(ui)
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
                    for y in 0..self.grid.len() {
                        body.row(20., |mut row| {
                            row.col(|ui| {
                                if y != self.grid.len() - 1 {
                                    ui.label((y + 1).to_string());
                                }
                            });

                            for x in 0..column_count {
                                // Values column
                                row.col(|ui| {
                                    if self.columns[x].expression.is_empty() {
                                        let invalid = self.grid[y][x].value.is_nan();

                                        let mut text_edit =
                                            TextEdit::singleline(&mut self.grid[y][x].raw_value);

                                        if invalid {
                                            text_edit = text_edit.text_color(Color32::RED);
                                        }

                                        let input = ui.add(text_edit);

                                        if input.lost_focus() {
                                            self.grid[y][x].raw_value =
                                                self.grid[y][x].raw_value.trim().to_owned();
                                        }

                                        if input.changed() {
                                            self.grid[y][x].value = self.grid[y][x]
                                                .raw_value
                                                .trim()
                                                .parse::<f64>()
                                                .unwrap_or(f64::NAN);

                                            self.compute_line_with_uncertainty(y);
                                        }
                                    } else if y != self.grid.len() - 1 {
                                        let value = self.grid[y][x].value;

                                        let mut rich_text = RichText::new(format!(
                                            "{value:.*}",
                                            self.columns[x].precision
                                        ));

                                        if value.is_nan() {
                                            rich_text = rich_text.color(Color32::RED);
                                        }

                                        ui.label(rich_text);
                                    }
                                });

                                // Uncertainty column
                                row.col(|ui| {
                                    if self.columns[x].expression.is_empty() {
                                        let invalid = self.grid[y][x].uncertainty.is_nan();

                                        let mut text_edit = TextEdit::singleline(
                                            &mut self.grid[y][x].raw_uncertainty,
                                        );

                                        if invalid {
                                            text_edit = text_edit.text_color(Color32::RED);
                                        }

                                        let input = ui.add(text_edit);

                                        if input.lost_focus() {
                                            self.grid[y][x].raw_uncertainty =
                                                self.grid[y][x].raw_uncertainty.trim().to_owned();
                                        }

                                        if input.changed() {
                                            if self.grid[y][x].raw_uncertainty.is_empty() {
                                                self.grid[y][x].uncertainty = 0.;
                                            } else {
                                                self.grid[y][x].uncertainty = self.grid[y][x]
                                                    .raw_uncertainty
                                                    .trim()
                                                    .parse::<f64>()
                                                    .unwrap_or(f64::NAN);
                                            }

                                            self.compute_line_with_uncertainty(y);
                                        }
                                    } else if y != self.grid.len() - 1 {
                                        let uncertainty = self.grid[y][x].uncertainty;

                                        let mut rich_text = RichText::new(format!(
                                            "{uncertainty:.*}",
                                            self.columns[x].precision
                                        ));
                                        if uncertainty.is_nan() {
                                            rich_text = rich_text.color(Color32::RED);
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

        SidePanel::right("graph_panel").show(ctx, |ui| {
            let mut box_list: Vec<BoxElem> = Vec::new();

            let mut x_sum = 0.;
            let mut y_sum = 0.;

            let mut xx_sum = 0.;

            let mut xy_sum = 0.;

            let mut n = 0.0;

            let mut min_x = 0f64;
            let mut max_x = 0f64;

            for line in 0..self.grid.len() {
                let x = self.grid[line][1].value;
                let y = self.grid[line][0].value;
                let uncertainty_x = self.grid[line][1].uncertainty;
                let uncertainty_y = self.grid[line][0].uncertainty;

                if x.is_nan() || y.is_nan() {
                    continue;
                }

                min_x = min_x.min(x);
                max_x = max_x.max(x);

                let quartile_factor = if self.full_box_plot { 1.0 } else { 0.5 };

                if self.vertical_box_plot {
                    box_list.push(
                        BoxElem::new(
                            x,
                            BoxSpread::new(
                                y - uncertainty_y,
                                y - uncertainty_y * quartile_factor,
                                y,
                                y + uncertainty_y * quartile_factor,
                                y + uncertainty_y,
                            ),
                        )
                        .stroke(Stroke::new(2.0, Color32::TRANSPARENT))
                        .fill(Color32::TRANSPARENT)
                        .box_width(uncertainty_x * 2.)
                        .whisker_width(uncertainty_x * 2.),
                    );
                } else {
                    box_list.push(
                        BoxElem::new(
                            y,
                            BoxSpread::new(
                                x - uncertainty_x,
                                x - uncertainty_x * quartile_factor,
                                x,
                                x + uncertainty_x * quartile_factor,
                                x + uncertainty_x,
                            ),
                        )
                        .box_width(uncertainty_y * 2.)
                        .whisker_width(uncertainty_y * 2.),
                    );
                }

                x_sum += x;
                y_sum += y;

                xx_sum += x * x;

                xy_sum += x * y;

                n += 1.0;
            }

            let line = if self.linear_regression {
                let slope = xy_sum / xx_sum;

                ui.label(format!("Slope : {slope}"));

                Line::new(PlotPoints::from_explicit_callback(
                    move |x| slope * x,
                    min_x..max_x,
                    1024,
                ))
            } else {
                let slope = (n * xy_sum - x_sum * y_sum) / (n * xx_sum - x_sum.powi(2));
                let height = (y_sum - slope * x_sum) / n;

                ui.label(format!("Slope : {slope}"));
                ui.label(format!("Height : {height}"));

                Line::new(PlotPoints::from_explicit_callback(
                    move |x| slope * x + height,
                    min_x..max_x,
                    1024,
                ))
            }
            .width(2.)
            .highlight(false)
            .color(Color32::from_rgb(255, 63, 63));

            let mut box_plot = BoxPlot::new(box_list);

            box_plot = if self.vertical_box_plot {
                box_plot.vertical()
            } else {
                box_plot.horizontal()
            };

            Plot::new("my_plot").show(ui, |plot_ui| {
                plot_ui.box_plot(box_plot);
                plot_ui.line(line);
            });
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn on_close_event(&mut self) -> bool {
        true
    }
}
