use std::borrow::Borrow;

//use egui_plotter::EguiBackend;
//use plotters::prelude::*;
use crate::common_data;
use crate::components::{dataframe_table, explorer, modal_window, plotter_2d};
use eframe::egui::{self, FontData};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    tree: egui_tiles::Tree<Pane>,

    #[serde(skip)]
    behavior: TreeBehavior,

    #[serde(skip)]
    last_tree_debug: String,

    #[serde(skip)]
    explorer: explorer::Explorer,

    #[serde(skip)]
    common_data: std::sync::Arc<std::sync::Mutex<common_data::CommonData>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let common_data = common_data::CommonData::default();
        let common_data_arc = std::sync::Arc::new(std::sync::Mutex::new(common_data));

        let mut next_view_nr = 0;
        let mut gen_view = |ptype: PaneType| {
            let view = Pane::new(ptype, next_view_nr);
            next_view_nr += 1;
            view
        };

        let mut tiles = egui_tiles::Tiles::default();

        let mut tabs = vec![];
        tabs.push({
            let mut cells = Vec::new();
            cells.push(tiles.insert_pane(gen_view(PaneType::None(0))));
            cells.push(tiles.insert_pane(gen_view(PaneType::Table(
                dataframe_table::DataFrameTable::default(),
            ))));
            cells.push(
                tiles.insert_pane(gen_view(PaneType::PerformancePlot(PerformancePlot::new()))),
            );
            cells.push(tiles.insert_pane(gen_view(PaneType::Plotter2D(
                plotter_2d::Plotter2D::default(),
            ))));
            tiles.insert_grid_tile(cells)
        });
        tabs.push(tiles.insert_pane(gen_view(PaneType::Table(
            dataframe_table::DataFrameTable::default(),
        ))));
        tabs.push(tiles.insert_pane(gen_view(PaneType::Plotter2D(
            plotter_2d::Plotter2D::default(),
        ))));

        let root = tiles.insert_tab_tile(tabs);

        let tree = egui_tiles::Tree::new(root, tiles);

        Self {
            tree: tree,
            behavior: TreeBehavior::new(common_data_arc.clone()),
            last_tree_debug: Default::default(),
            explorer: explorer::Explorer::default(),
            common_data: common_data_arc.clone(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let mut fonts = egui::FontDefinitions::default();

        // Install my own font (maybe supporting non-latin characters):
        fonts.font_data.insert(
            "my_font".to_owned(),
            FontData::from_static(include_bytes!("../NotoSansJP-Regular.ttf")),
        ); // .ttf and .otf supported
           // Put my font first (highest priority) for proportional text:
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "my_font".to_owned());

        // Put my font as last fallback for monospace:
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("my_font".to_owned());

        cc.egui_ctx.set_fonts(fonts);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            let mut loaded: TemplateApp =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            let common_data = common_data::CommonData::default();
            let common_data_arc = std::sync::Arc::new(std::sync::Mutex::new(common_data));
            loaded.behavior = TreeBehavior::new(common_data_arc.clone());
            loaded.common_data = common_data_arc.clone();
            return loaded;
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let current_path = {
            let cdata = self.common_data.lock();
            if cdata.is_err() {
                return;
            }
            let mut cdata = cdata.unwrap();
            let current_path = cdata.current_path.clone();
            cdata.current_path = cdata.default_path.clone();
            current_path.clone()
        };
        eframe::set_value(storage, eframe::APP_KEY, self);
        {
            let cdata = self.common_data.lock();
            if cdata.is_err() {
                return;
            }
            let mut cdata = cdata.unwrap();
            cdata.current_path = current_path;
        }
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut opening_modal_window = false;
        {
            let cdata = self.common_data.lock();
            if cdata.is_err() {
                return;
            }
            let mut cdata = cdata.unwrap();

            //        ctx.request_repaint();
            ctx.request_repaint_after(std::time::Duration::from_millis((1000 / 60) as u64));
            //
            // Data update
            //
            cdata.update();

            //
            // View update
            //
            for (_, (df_info, _)) in cdata.dataframes.iter_mut() {
                if df_info.load_state == modal_window::LoadState::OpenModalWindow {
                    modal_window::ModalWindow::default().show(ctx, df_info);
                    opening_modal_window = true;
                }
            }
        }
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.set_enabled(!opening_modal_window);

            egui::menu::bar(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            _frame.close();
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);

                if ui.button("reset dataframes").clicked() {
                    let cdata = self.common_data.lock();
                    if cdata.is_err() {
                        return;
                    }
                    let mut cdata = cdata.unwrap();
                    cdata.dataframes.clear();
                }

                if ui.button("save").clicked() {
                    if let Some(storage) = _frame.storage_mut() {
                        self.save(storage);
                        log::info!("saved");
                    }
                    let cdata = self.common_data.lock();
                    if cdata.is_err() {
                        return;
                    }
                    let mut cdata = cdata.unwrap();

                    cdata.save_df_list();
                }
            });
        });
        egui::SidePanel::left("info").show(ctx, |ui| {
            ui.set_enabled(!opening_modal_window);

            ui.heading("h_analyzer");

            self.explorer.show(ui, self.common_data.clone());
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!opening_modal_window);
            self.tree.ui(&mut self.behavior, ui);
        });

        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.label("Progress");
        });
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
enum PaneType {
    Plotter2D(plotter_2d::Plotter2D),
    Table(dataframe_table::DataFrameTable),
    PerformancePlot(PerformancePlot),
    None(i32),
}

#[derive(serde::Deserialize, serde::Serialize)]
struct PerformancePlot {
    history_map: std::collections::HashMap<String, std::collections::VecDeque<f64>>,
}

impl PerformancePlot {
    pub fn new() -> Self {
        let mut mp = std::collections::HashMap::new();
        mp.insert("test1".to_string(), std::collections::VecDeque::from([0.0]));
        mp.insert("test2".to_string(), std::collections::VecDeque::new());
        mp.insert("test3".to_string(), std::collections::VecDeque::new());
        Self { history_map: mp }
    }

    pub fn update(&mut self) {
        log::info!("pplot update {}", self.history_map.len());
        for (k, v) in self.history_map.iter_mut() {
            log::info!("key {}", k);
            log::info!("vs {:?}", v);
            v.clear();
            for i in 0..10 {
                v.push_back(i as f64);
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.update();

        ui.label("pplot");
        let mut chart1 = egui_plot::BarChart::new(vec![
            egui_plot::Bar::new(0.5, 1.0).name("Day 1"),
            egui_plot::Bar::new(1.5, 3.0).name("Day 2"),
            egui_plot::Bar::new(2.5, 1.0).name("Day 3"),
            egui_plot::Bar::new(3.5, 2.0).name("Day 4"),
            egui_plot::Bar::new(4.5, 4.0).name("Day 5"),
        ])
        .width(0.7)
        .name("Set 1");

        let mut chart2 = egui_plot::BarChart::new(vec![
            egui_plot::Bar::new(0.5, 1.0),
            egui_plot::Bar::new(1.5, 1.5),
            egui_plot::Bar::new(2.5, 0.1),
            egui_plot::Bar::new(3.5, 0.7),
            egui_plot::Bar::new(4.5, 0.8),
        ])
        .width(0.7)
        .name("Set 2")
        .stack_on(&[&chart1]);

        let mut chart3 = egui_plot::BarChart::new(vec![
            egui_plot::Bar::new(0.5, -0.5),
            egui_plot::Bar::new(1.5, 1.0),
            egui_plot::Bar::new(2.5, 0.5),
            egui_plot::Bar::new(3.5, -1.0),
            egui_plot::Bar::new(4.5, 0.3),
        ])
        .width(0.7)
        .name("Set 3")
        .stack_on(&[&chart1, &chart2]);

        let mut chart4 = egui_plot::BarChart::new(vec![
            egui_plot::Bar::new(0.5, 0.5),
            egui_plot::Bar::new(1.5, 1.0),
            egui_plot::Bar::new(2.5, 0.5),
            egui_plot::Bar::new(3.5, -0.5),
            egui_plot::Bar::new(4.5, -0.5),
        ])
        .width(0.7)
        .name("Set 4")
        .stack_on(&[&chart1, &chart2, &chart3]);

        egui_plot::Plot::new("Stacked Bar Chart Demo")
            .legend(egui_plot::Legend::default())
            .data_aspect(1.0)
            .allow_drag(true)
            .auto_bounds_x()
            .auto_bounds_y()
            .show(ui, |plot_ui| {
                let mut bar_charts = Vec::new();
                for (k, v) in self.history_map.iter_mut() {
                    let bars: Vec<egui_plot::Bar> = v
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| -> egui_plot::Bar { egui_plot::Bar::new(i as f64, v) })
                        .collect();
                    let bar_chart = egui_plot::BarChart::new(bars).width(0.8).name(k).stack_on(
                        bar_charts
                            .iter()
                            .collect::<Vec<&egui_plot::BarChart>>()
                            .as_slice(),
                    );
                    bar_charts.push(bar_chart);
                }
                for bar_chart in bar_charts.drain(..) {
                    plot_ui.bar_chart(bar_chart);
                }
                //plot_ui.bar_chart(chart1);
                //plot_ui.bar_chart(chart2);
                //plot_ui.bar_chart(chart3);
                //plot_ui.bar_chart(chart4);
            })
            .response;
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Pane {
    pane_type: PaneType,
    nr: usize,
}

impl std::fmt::Debug for Pane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("View").field("nr", &self.nr).finish()
    }
}

impl Pane {
    pub fn new(pane_type: PaneType, nr: usize) -> Self {
        Self {
            nr: nr,
            pane_type: pane_type,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        common_data_arc: std::sync::Arc<std::sync::Mutex<common_data::CommonData>>,
    ) -> egui_tiles::UiResponse {
        let mut sense_drag = false;
        ui.vertical(|ui| {
            let (rect, resp) = ui.allocate_at_least(
                egui::Vec2::new(ui.available_width(), 20.0),
                egui::Sense::click_and_drag(),
            );
            ui.allocate_ui_at_rect(rect, |ui| {
                let color = egui::epaint::Color32::DARK_GRAY;
                ui.painter().rect_filled(ui.max_rect(), 0.0, color);
                ui.label("test");
            });
            sense_drag = resp.dragged();

            match &mut self.pane_type {
                PaneType::Plotter2D(p2d) => {
                    p2d.show(ui, common_data_arc.clone());
                }
                PaneType::Table(ref mut tb) => {
                    tb.show(ui, common_data_arc.clone());
                }
                PaneType::PerformancePlot(pp) => {
                    pp.show(ui);
                }
                PaneType::None(_) => {
                    let color = egui::epaint::Hsva::new(0.103 * self.nr as f32, 0.5, 0.5, 1.0);
                    ui.painter().rect_filled(ui.max_rect(), 0.0, color);
                }
            }
        });

        let sense = ui
            .allocate_rect(egui::Rect::NOTHING, egui::Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::Grab);
        if sense.clicked() {
            log::info!("clicked {}", self.nr);
        }
        if sense_drag {
            egui_tiles::UiResponse::DragStarted
        } else {
            egui_tiles::UiResponse::None
        }
    }
}

struct TreeBehavior {
    simplification_options: egui_tiles::SimplificationOptions,
    tab_bar_height: f32,
    gap_width: f32,
    add_child_to: Option<egui_tiles::TileId>,
    common_data_arc: std::sync::Arc<std::sync::Mutex<common_data::CommonData>>,
}

impl TreeBehavior {
    fn new(common_data_arc: std::sync::Arc<std::sync::Mutex<common_data::CommonData>>) -> Self {
        Self {
            simplification_options: Default::default(),
            tab_bar_height: 24.0,
            gap_width: 4.0,
            add_child_to: None,
            common_data_arc: common_data_arc,
        }
    }
}

impl TreeBehavior {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            simplification_options,
            tab_bar_height,
            gap_width,
            add_child_to: _,
            common_data_arc,
        } = self;

        egui::Grid::new("behavior_ui")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("All panes must have tabs:");
                ui.checkbox(&mut simplification_options.all_panes_must_have_tabs, "");
                ui.end_row();

                ui.label("Join nested containers:");
                ui.checkbox(
                    &mut simplification_options.join_nested_linear_containerss,
                    "",
                );
                ui.end_row();

                ui.label("Tab bar height:");
                ui.add(
                    egui::DragValue::new(tab_bar_height)
                        .clamp_range(0.0..=100.0)
                        .speed(1.0),
                );
                ui.end_row();

                ui.label("Gap width:");
                ui.add(
                    egui::DragValue::new(gap_width)
                        .clamp_range(0.0..=20.0)
                        .speed(1.0),
                );
                ui.end_row();
            });
    }
}

impl egui_tiles::Behavior<Pane> for TreeBehavior {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        view: &mut Pane,
    ) -> egui_tiles::UiResponse {
        view.ui(ui, self.common_data_arc.clone())
    }

    fn tab_title_for_pane(&mut self, view: &Pane) -> egui::WidgetText {
        format!("View {}", view.nr).into()
    }

    fn top_bar_rtl_ui(
        &mut self,
        _tiles: &egui_tiles::Tiles<Pane>,
        ui: &mut egui::Ui,
        tile_id: egui_tiles::TileId,
        _tabs: &egui_tiles::Tabs,
    ) {
        if ui.button("➕").clicked() {
            self.add_child_to = Some(tile_id);
        }
    }

    // ---
    // Settings:

    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        self.tab_bar_height
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        self.gap_width
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        self.simplification_options
    }
}
