// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use egui::{Context, Grid, ScrollArea, SidePanel, widget_text::WidgetText};
use serde::{Deserialize, Serialize};
use topola::Board;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Colors {
    pub layers: LayerColors,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LayerColors {
    default: LayerColor,
    colors: BTreeMap<String, LayerColor>,
}

impl LayerColors {
    pub fn color(&self, layer_name: Option<&str>) -> &LayerColor {
        layer_name
            .map(|layername| self.colors.get(layername).unwrap_or(&self.default))
            .unwrap_or(&self.default)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LayerColor {
    pub normal: egui::Color32,
    pub highlighted: egui::Color32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppearancePanel {
    // TODO:
    // In1.Cu shall be #7fc87f (#d5ecd5 when selected).
    // In2.Cu shall be #ce7d2c (#e8c39e when selected).
    pub dark_colors: Colors,
    pub light_colors: Colors,

    #[serde(skip)]
    pub visible: Box<[bool]>,
}

impl AppearancePanel {
    pub fn new(board: &Board) -> Self {
        let dark_colors = Colors {
            layers: LayerColors {
                default: LayerColor {
                    normal: egui::Color32::from_rgb(255, 255, 255),
                    highlighted: egui::Color32::from_rgb(255, 255, 255),
                },
                colors: BTreeMap::from([
                    (
                        "F.Cu".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(255, 52, 52),
                            highlighted: egui::Color32::from_rgb(255, 100, 100),
                        },
                    ),
                    (
                        "1".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(255, 52, 52),
                            highlighted: egui::Color32::from_rgb(255, 100, 100),
                        },
                    ),
                    (
                        "B.Cu".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(52, 52, 255),
                            highlighted: egui::Color32::from_rgb(100, 100, 255),
                        },
                    ),
                    (
                        "2".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(52, 52, 255),
                            highlighted: egui::Color32::from_rgb(100, 100, 255),
                        },
                    ),
                    (
                        "In1.Cu".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(127, 200, 127),
                            highlighted: egui::Color32::from_rgb(213, 236, 213),
                        },
                    ),
                    (
                        "In2.Cu".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(206, 125, 44),
                            highlighted: egui::Color32::from_rgb(232, 195, 158),
                        },
                    ),
                ]),
            },
        };
        let light_colors = Colors {
            layers: LayerColors {
                default: LayerColor {
                    normal: egui::Color32::from_rgb(0, 0, 0),
                    highlighted: egui::Color32::from_rgb(0, 0, 0),
                },
                colors: BTreeMap::from([
                    (
                        "F.Cu".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(255, 27, 27),
                            highlighted: egui::Color32::from_rgb(255, 52, 52),
                        },
                    ),
                    (
                        "1".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(255, 27, 27),
                            highlighted: egui::Color32::from_rgb(255, 52, 52),
                        },
                    ),
                    (
                        "B.Cu".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(27, 27, 255),
                            highlighted: egui::Color32::from_rgb(52, 52, 255),
                        },
                    ),
                    (
                        "2".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(27, 27, 255),
                            highlighted: egui::Color32::from_rgb(52, 52, 255),
                        },
                    ),
                    (
                        "In1.Cu".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(76, 169, 76),
                            highlighted: egui::Color32::from_rgb(127, 200, 127),
                        },
                    ),
                    (
                        "In2.Cu".to_string(),
                        LayerColor {
                            normal: egui::Color32::from_rgb(183, 80, 12),
                            highlighted: egui::Color32::from_rgb(206, 125, 44),
                        },
                    ),
                ]),
            },
        };

        let layer_count = board.layout().layer_count();
        let visible = core::iter::repeat(true)
            .take(*layer_count)
            .collect::<Box<[_]>>();
        Self {
            dark_colors,
            light_colors,
            visible,
        }
    }

    pub fn update(&mut self, ctx: &Context, board: &Board) {
        SidePanel::right("appearance_panel").show(ctx, |ui| {
            ui.label("Layers");
            let row_height = ui.spacing().interact_size.y;
            ScrollArea::vertical().show_rows(
                ui,
                row_height,
                self.visible.len(),
                |ui, row_range| {
                    let start_row = row_range.start;
                    Grid::new("appearance_layers")
                        .min_col_width(ui.spacing().icon_width)
                        .num_columns(3)
                        .start_row(start_row)
                        .show(ui, |ui| {
                            for (layer, visible) in self.visible[row_range].iter_mut().enumerate() {
                                let layer = layer + start_row;
                                let layer_name = board.layer_name(layer);

                                // unnamed layers can't be used for routing
                                /*if layer_name.is_some() {
                                    ui.radio_value(
                                        &mut options.planar.principal_layer,
                                        layer,
                                        WidgetText::default(),
                                    );
                                } else {
                                    // dummy item to bump the grid
                                    ui.label("");
                                }*/

                                ui.checkbox(visible, WidgetText::default());
                                ui.label(
                                    layer_name
                                        .map(|i| i.to_string())
                                        .unwrap_or_else(|| format!("{} - Unnamed layer", layer)),
                                );
                                ui.end_row();
                            }
                        })
                },
            );
        });
    }

    pub fn colors(&self, ctx: &Context) -> &Colors {
        match ctx.theme() {
            egui::Theme::Dark => &self.dark_colors,
            egui::Theme::Light => &self.light_colors,
        }
    }
}
