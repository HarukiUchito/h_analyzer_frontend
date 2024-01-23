//use egui_plotter::EguiBackend;
//use plotters::prelude::*;
use crate::common_data;
use crate::components::modal_window::ModalWindow;
use crate::components::{dataframe_table, explorer, modal_window, plotter_2d};
use eframe::egui::{self, FontData};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct WorldPlayer {
    selected_world_name: Option<String>,
}

impl Default for WorldPlayer {
    fn default() -> Self {
        Self {
            selected_world_name: None,
        }
    }
}

impl WorldPlayer {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        common_data_arc: std::sync::Arc<std::sync::Mutex<common_data::CommonData>>,
    ) {
        let common_data = common_data_arc.lock();
        if common_data.is_err() {
            return;
        }
        let common_data = &mut common_data.unwrap();
        ui.horizontal(|ui| {
            ui.label("World Player");
            if ui.button("Update World List").clicked() {
                common_data.update_world_list();
            }
            let world_list_opt = common_data.get_world_list();
            if let Some(world_list) = world_list_opt {
                egui::ComboBox::from_id_source("world_select")
                    .selected_text(self.selected_world_name.clone().unwrap_or_default())
                    .show_ui(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        for world_meta in world_list.list.iter() {
                            let txt = world_meta.id.clone().unwrap().id;
                            ui.selectable_value(
                                &mut self.selected_world_name,
                                Some(txt.clone()),
                                txt.clone(),
                            );
                        }
                    });
                // show world history length and current index
                let mut history_len = 0;
                if let Some(selected_name) = &self.selected_world_name {
                    for world_meta in world_list.list.iter() {
                        if world_meta.id.clone().unwrap().id == *selected_name {
                            history_len = world_meta.total_frame_num;
                        }
                    }
                }
                // controller buttons
                if ui.button("Previous Frame").clicked() {
                    common_data.world.previous();
                }
                let playing = common_data.world_playing;
                if ui.button(if playing { "Pause" } else { "Play" }).clicked() {
                    common_data.world_playing = !playing;
                }
                if ui.button("Next Frame").clicked() {
                    common_data.world.next();
                }
                // world infomation
                ui.label(format!(
                    "current frame : {} / {}",
                    common_data.world.current_index, history_len
                ));
                if let Some(wf) = common_data
                    .world
                    .history
                    .get(common_data.world.current_index)
                {
                    ui.label(format!("current time: {}", wf.timestamp));
                }
            }
        });
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    tree: egui_tiles::Tree<Pane>,

    world_player: WorldPlayer,

    #[serde(skip)]
    modal_window: ModalWindow,

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
            world_player: WorldPlayer::default(),
            modal_window: ModalWindow::default(),
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
            let selected_world_name = self.world_player.selected_world_name.clone();
            cdata.update(selected_world_name);

            //
            // View update
            //
            for (_, (df_info, _)) in cdata.dataframes.iter_mut() {
                if df_info.load_state == modal_window::LoadState::OpenModalWindow {
                    self.modal_window.show(ctx, df_info);
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
            self.world_player.show(ui, self.common_data.clone());
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

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        common_data_arc: std::sync::Arc<std::sync::Mutex<common_data::CommonData>>,
    ) -> Option<()> {
        let common_data = common_data_arc.lock();
        if common_data.is_err() {
            return None;
        }
        let common_data = &mut common_data.unwrap();

        egui_plot::Plot::new("Stacked Bar Chart Demo")
            .legend(egui_plot::Legend::default())
            .auto_bounds_x()
            .auto_bounds_y()
            .show(ui, |plot_ui| {
                //plot_ui.bar_chart(vec![egui_plot::BarChart::new()]);
                plot_ui.bar_chart(
                    egui_plot::BarChart::new(
                        common_data
                            .sl_time_history
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| -> egui_plot::Bar {
                                egui_plot::Bar::new(i as f64, v * 1e3)
                            })
                            .collect(),
                    )
                    .name("series list"),
                );
            })
            .response;
        Some(())
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
                    pp.show(ui, common_data_arc.clone());
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
