use crate::common_data::{self};
use crate::components::dataframe_select;

use crate::unwrap_or_continue;

use eframe::egui::{self};
use egui_plot::PlotBounds;
use h_analyzer_data::WorldFrame;
use polars::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum SeriesSource {
    DataFrame,
    GRPCClient,
    WorldFrame,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum PlotType {
    Point,
    Pose,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Hash, Clone, Copy)]
pub enum EntityVizTarget {
    Measurement,
    Estimate,
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SeriesInfo {
    title: String,
    source: SeriesSource,

    // entity plot settings
    entity_name: Option<String>,
    entity_elem_target: Option<EntityVizTarget>,
    entity_elem_id: Option<String>,

    // df plot settings
    plot_type: PlotType,
    df_id: Option<String>,
    visible: bool,
    x_column: Option<String>,
    y_column: Option<String>,
    theta_column: Option<String>,
    marker_radius: f64,
    track_this: bool,
}

impl Default for SeriesInfo {
    fn default() -> Self {
        Self {
            title: "".to_string(),
            source: SeriesSource::DataFrame,

            entity_name: None,
            entity_elem_target: None,
            entity_elem_id: None,

            plot_type: PlotType::Point,
            df_id: None,
            visible: true,
            x_column: None,
            y_column: None,
            theta_column: None,
            marker_radius: 1.0,
            track_this: false,
        }
    }
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
//#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Plotter2D {
    title: String,
    equal_aspect: bool,
    apply_x_limit: bool,
    limit_x_range: (f64, f64),
    series_infos: Vec<SeriesInfo>,
    series_df_selectors: Vec<dataframe_select::DataFrameSelect>,
}

impl Default for Plotter2D {
    fn default() -> Self {
        Self {
            title: "".to_string(),
            equal_aspect: true,
            apply_x_limit: false,
            limit_x_range: (0.0, 0.0),
            series_infos: Vec::new(),
            series_df_selectors: Vec::new(),
        }
    }
}

impl Plotter2D {
    fn entity_settings(
        idx: usize,
        info: &mut SeriesInfo,
        ui: &mut egui::Ui,
        world_frame_opt: &Option<WorldFrame>,
    ) -> Option<()> {
        if world_frame_opt.is_none() {
            return None;
        }
        let world_frame = world_frame_opt.as_ref().unwrap();
        ui.label("[Entity Settings] name:");
        egui::ComboBox::from_id_source(format!("entity_select_{}", idx))
            .selected_text(info.entity_name.as_deref().unwrap_or_default())
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for (name, _) in world_frame.entity_map.iter() {
                    ui.selectable_value(&mut info.entity_name, Some(name.clone()), name);
                }
            });
        egui::ComboBox::from_id_source(format!("entity_viz_target_{}", idx))
            .selected_text(match info.entity_elem_target {
                Some(EntityVizTarget::Estimate) => "Estimate",
                Some(EntityVizTarget::Measurement) => "Measurement",
                None => "",
            })
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                ui.selectable_value(
                    &mut info.entity_elem_target,
                    Some(EntityVizTarget::Measurement),
                    "Measurement",
                );
                ui.selectable_value(
                    &mut info.entity_elem_target,
                    Some(EntityVizTarget::Estimate),
                    "Estimate",
                );
            });
        let entity_name = info.entity_name.clone()?;
        if let Some(entity) = world_frame.entity_map.get(&entity_name) {
            egui::ComboBox::from_id_source(format!("entity_elem_select_{}", idx))
                .selected_text(info.entity_elem_id.as_deref().unwrap_or_default())
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    match info.entity_elem_target {
                        Some(EntityVizTarget::Measurement) => {
                            for (name, _) in entity.measurement_map.iter() {
                                ui.selectable_value(
                                    &mut info.entity_elem_id,
                                    Some(name.clone()),
                                    name,
                                );
                            }
                        }
                        Some(EntityVizTarget::Estimate) => {
                            for (name, _) in entity.estimate_map.iter() {
                                ui.selectable_value(
                                    &mut info.entity_elem_id,
                                    Some(name.clone()),
                                    name,
                                );
                            }
                        }
                        None => {}
                    }
                });
        }
        None
    }

    fn series_settings(
        idx: usize,
        info: &mut SeriesInfo,
        ui: &mut egui::Ui,
        df: Option<&mut DataFrame>,
    ) -> Option<()> {
        let df = df?;
        ui.label("x axis: ");
        egui::ComboBox::from_id_source(format!("x_select_{}", idx))
            .selected_text(info.x_column.as_deref().unwrap_or_default())
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for (_, &cname) in df.get_column_names().iter().enumerate() {
                    ui.selectable_value(&mut info.x_column, Some(cname.to_string()), cname);
                }
            });
        ui.label("y axis: ");
        egui::ComboBox::from_id_source(format!("y_select_{}", idx))
            .selected_text(info.y_column.as_deref().unwrap_or_default())
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for (_, &cname) in df.get_column_names().iter().enumerate() {
                    ui.selectable_value(&mut info.y_column, Some(cname.to_string()), cname);
                }
            });
        if info.plot_type == PlotType::Pose {
            ui.label("yaw: ");
            egui::ComboBox::from_id_source(format!("theta_select_{}", idx))
                .selected_text(info.theta_column.as_deref().unwrap_or_default())
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    for (_, &cname) in df.get_column_names().iter().enumerate() {
                        ui.selectable_value(&mut info.theta_column, Some(cname.to_string()), cname);
                    }
                });
        }

        None
    }

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
        egui::CollapsingHeader::new("Plot Settings")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.title).hint_text("title of the plot"),
                    );
                    ui.checkbox(&mut self.equal_aspect, "Equal Aspect Ratio");
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.apply_x_limit, "Limit x range");
                    ui.add(
                        egui::DragValue::new(&mut self.limit_x_range.0)
                            .speed(0.1)
                            .prefix(" min: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.limit_x_range.1)
                            .speed(0.1)
                            .prefix(" max: "),
                    );
                });

                let mut del_idx = None;
                let mut selector_iter = self.series_df_selectors.iter_mut();
                let num = self.series_infos.len();
                let mut sinfo_iter = self.series_infos.iter_mut();
                for idx in 0..num {
                    let info = unwrap_or_continue!(sinfo_iter.next());
                    ui.separator();
                    ui.horizontal(|ui| {
                        del_idx = Plotter2D::general_series_settings_ui(idx, info, ui);
                    });
                    if info.source == SeriesSource::DataFrame
                        || info.source == SeriesSource::GRPCClient
                    {
                        let mut series_df = None;
                        let selector = unwrap_or_continue!(selector_iter.next());
                        ui.horizontal(|ui| {
                            series_df = match info.source {
                                SeriesSource::DataFrame => {
                                    selector.select_df(idx + 1, ui, common_data)
                                }
                                SeriesSource::GRPCClient => {
                                    selector.select_backend_df(idx + 1, ui, common_data)
                                }
                                SeriesSource::WorldFrame => None,
                            };

                            ui.label("Plot Type");
                            egui::ComboBox::from_id_source(format!("plot_type_select_{}", idx))
                                .selected_text(match info.plot_type {
                                    PlotType::Point => "Point",
                                    PlotType::Pose => "Pose",
                                })
                                .show_ui(ui, |ui| {
                                    ui.style_mut().wrap = Some(false);
                                    ui.set_min_width(60.0);
                                    ui.selectable_value(
                                        &mut info.plot_type,
                                        PlotType::Point,
                                        "Point",
                                    );
                                    ui.selectable_value(
                                        &mut info.plot_type,
                                        PlotType::Pose,
                                        "Pose",
                                    );
                                });
                            ui.horizontal(|ui| {
                                Plotter2D::series_settings(idx + 1, info, ui, series_df);

                                ui.push_id(format!("track_this_{}", idx), |ui| {
                                    ui.checkbox(&mut info.track_this, "track_this");
                                });
                            });
                        });
                    } else if info.source == SeriesSource::WorldFrame {
                        ui.horizontal(|ui| {
                            Plotter2D::entity_settings(
                                idx,
                                info,
                                ui,
                                &common_data.latest_world_frame,
                            );
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut info.track_this, "track_this");
                        ui.add(
                            egui::DragValue::new(&mut info.marker_radius)
                                .speed(0.1)
                                .clamp_range(0.0..=f64::INFINITY)
                                .prefix("marker size: "),
                        );
                    });
                }

                if let Some(del_idx) = del_idx {
                    self.series_infos.remove(del_idx);
                    self.series_df_selectors.remove(del_idx);
                }

                ui.separator();
                if ui.button("Add Series").clicked() {
                    self.series_infos.push(SeriesInfo::default());
                    self.series_df_selectors
                        .push(dataframe_select::DataFrameSelect::default());
                }
            });

        ui.separator();
        ui.vertical_centered(|ui| {
            ui.label(&self.title);
        });
        let plot = egui_plot::Plot::new("lines_demo")
            .legend(egui_plot::Legend::default())
            //.y_axis_width(4)
            .x_axis_label("x[m]")
            .y_axis_label("y[m]")
            .auto_bounds_x()
            .auto_bounds_y()
            .show_axes(true)
            .show_grid(true);
        let plot = if self.equal_aspect {
            plot.data_aspect(1.0)
        } else {
            plot
        };

        let extract_series = |df: &DataFrame, cname: &str| -> Vec<f64> {
            let col = df.column(cname);
            if let Ok(col) = col {
                col.cast(&DataType::Float64)
                    .unwrap()
                    .f64()
                    .unwrap()
                    .into_no_null_iter()
                    .collect()
            } else {
                Vec::new()
            }
        };

        plot.show(ui, |plot_ui| {
            let mut df_select_iter = self.series_df_selectors.iter();
            for s_info in self.series_infos.iter() {
                if s_info.source == SeriesSource::WorldFrame {
                    let worldframe = unwrap_or_continue!(&common_data.latest_world_frame);
                    let entity_name = unwrap_or_continue!(s_info.entity_name.clone());
                    let entity = unwrap_or_continue!(worldframe.entity_map.get(&entity_name));
                    let elem_id = unwrap_or_continue!(s_info.entity_elem_id.clone());
                    match s_info.entity_elem_target {
                        Some(EntityVizTarget::Measurement) => {
                            let measurement =
                                unwrap_or_continue!(entity.measurement_map.get(&elem_id));
                            match measurement {
                                h_analyzer_data::Measurement::PointCloud2D(pc) => {
                                    let xs: Vec<f64> = pc.points.iter().map(|p| p.x).collect();
                                    let ys: Vec<f64> = pc.points.iter().map(|p| p.y).collect();
                                    if xs.len() == 0 || ys.len() == 0 {
                                        continue;
                                    }
                                    let xys: Vec<[f64; 2]> =
                                        (0..xs.len()).map(|i| [xs[i], ys[i]]).collect();
                                    plot_ui.points(
                                        egui_plot::Points::new(xys)
                                            .radius(s_info.marker_radius as f32)
                                            .filled(false)
                                            .shape(egui_plot::MarkerShape::Diamond)
                                            .name(&s_info.title),
                                    );
                                }
                                _ => {}
                            }
                        }
                        Some(EntityVizTarget::Estimate) => {
                            let estimate = unwrap_or_continue!(entity.estimate_map.get(&elem_id));
                            match estimate {
                                h_analyzer_data::Estimate::Pose2D(pose) => {
                                    log::info!("pose : {:?}", pose);
                                    let xs = vec![pose.position.x];
                                    let ys = vec![pose.position.y];
                                    if s_info.track_this {
                                        if let (Some(lx), Some(ly)) = (xs.last(), ys.last()) {
                                            let range = 3.0;
                                            plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                                                [lx - range, ly - range],
                                                [lx + range, ly + range],
                                            ));
                                        }
                                    }
                                    let xys: Vec<[f64; 2]> =
                                        (0..xs.len()).map(|i| [xs[i], ys[i]]).collect();
                                    let ts = vec![pose.theta];
                                    let xys2: Vec<[f64; 2]> = (0..xs.len())
                                        .map(|i| {
                                            [
                                                xs[i] + s_info.marker_radius * ts[i].cos(),
                                                ys[i] + s_info.marker_radius * ts[i].sin(),
                                            ]
                                        })
                                        .collect();
                                    let arrows = egui_plot::Arrows::new(xys.clone(), xys2);
                                    plot_ui.arrows(arrows);

                                    plot_ui.points(
                                        egui_plot::Points::new(xys)
                                            .radius(1.0)
                                            .filled(false)
                                            .name(&s_info.title),
                                    );
                                }
                            }
                        }
                        None => {}
                    }
                } else {
                    // use dataframe for plotting
                    let df_id = unwrap_or_continue!(unwrap_or_continue!(df_select_iter.next())
                        .dataframe_key
                        .clone());
                    if !s_info.visible {
                        continue;
                    }
                    let local_df = match s_info.source {
                        SeriesSource::DataFrame => common_data
                            .dataframes
                            .get(&df_id)
                            .map(|df_opt_pair| df_opt_pair.1.as_ref().map(|df_opt| df_opt.clone()))
                            .unwrap_or_default(),
                        SeriesSource::GRPCClient => common_data
                            .realtime_dataframes
                            .get(&df_id)
                            .map(|df_opt| df_opt.clone()),
                        SeriesSource::WorldFrame => None,
                    };
                    let mut local_df = unwrap_or_continue!(local_df);

                    if self.apply_x_limit {
                        log::info!("{:?}", self.limit_x_range);
                        let rt_col = local_df.column("Relative Time[s]").unwrap();
                        let mask = rt_col.gt(self.limit_x_range.0).unwrap();
                        local_df = local_df.filter(&mask).unwrap();
                        let rt_col = local_df.column("Relative Time[s]").unwrap();
                        let mask = rt_col.lt(self.limit_x_range.1).unwrap();
                        local_df = local_df.filter(&mask).unwrap();
                    }

                    let colx = unwrap_or_continue!(&s_info.x_column);
                    let coly = unwrap_or_continue!(&s_info.y_column);
                    let xs = extract_series(&local_df, colx.as_str());
                    let ys = extract_series(&local_df, coly.as_str());
                    if xs.len() == 0 || ys.len() == 0 {
                        continue;
                    }
                    let xys: Vec<[f64; 2]> = (0..xs.len()).map(|i| [xs[i], ys[i]]).collect();
                    //plot_ui.points(points)
                    if s_info.track_this {
                        if let (Some(lx), Some(ly)) = (xs.last(), ys.last()) {
                            let range = 0.1;
                            plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                                [lx - range, ly - range],
                                [lx + range, ly + range],
                            ));
                        }
                    }
                    match s_info.plot_type {
                        PlotType::Point => {
                            let line_obj = egui_plot::Line::new(xys.clone());
                            plot_ui.line(line_obj);
                            plot_ui.points(
                                egui_plot::Points::new(xys)
                                    .radius(5.0)
                                    .filled(false)
                                    .shape(egui_plot::MarkerShape::Diamond)
                                    .name(&s_info.title),
                            );
                        }
                        PlotType::Pose => {
                            let col_theta = unwrap_or_continue!(&s_info.theta_column);
                            let ts = extract_series(&local_df, col_theta.as_str());
                            let xys2: Vec<[f64; 2]> = (0..xs.len())
                                .map(|i| {
                                    [
                                        xs[i] + s_info.marker_radius * ts[i].cos(),
                                        ys[i] + s_info.marker_radius * ts[i].sin(),
                                    ]
                                })
                                .collect();
                            let arrows = egui_plot::Arrows::new(xys.clone(), xys2);
                            plot_ui.arrows(arrows);

                            plot_ui.points(
                                egui_plot::Points::new(xys)
                                    .radius(1.0)
                                    .filled(false)
                                    .name(&s_info.title),
                            );
                        }
                    }
                };
            }
        });
    }

    fn general_series_settings_ui(
        idx: usize,
        info: &mut SeriesInfo,
        ui: &mut egui::Ui,
    ) -> Option<usize> {
        let mut delete_idx = None;
        ui.push_id(format!("sname_{}", idx), |ui| {
            ui.label(format!("Series {}, ", idx));
            ui.add(egui::TextEdit::singleline(&mut info.title).hint_text("title of the series"));
            if ui.button("delete").clicked() {
                delete_idx = Some(idx);
            }
            ui.checkbox(&mut info.visible, "visible");
            ui.label("source: ");
        });
        ui.push_id(format!("df_source_{}", idx), |ui| {
            egui::ComboBox::from_id_source(idx)
                .selected_text(match info.source {
                    SeriesSource::DataFrame => "DataFrme",
                    SeriesSource::GRPCClient => "GRPC Client",
                    SeriesSource::WorldFrame => "WorldFrame",
                })
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    ui.selectable_value(&mut info.source, SeriesSource::DataFrame, "DataFrame");
                    ui.selectable_value(&mut info.source, SeriesSource::GRPCClient, "GRPC Client");
                    ui.selectable_value(&mut info.source, SeriesSource::WorldFrame, "WorldFrame");
                });
        });
        delete_idx
    }
}
