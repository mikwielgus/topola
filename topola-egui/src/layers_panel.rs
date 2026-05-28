// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use egui::{Context, Grid, ScrollArea, SidePanel, widget_text::WidgetText};
use serde::{Deserialize, Serialize};
use topola::{Board, LayerDesc, LayerId, LayerSide, LayerType};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Colors {
    pub layers: ColorLayers,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ColorLayers {
    default: LayerColors,
    colors: BTreeMap<LayerDesc, LayerColors>,
}

impl ColorLayers {
    pub fn colors(&self, layer_desc: Option<&LayerDesc>) -> &LayerColors {
        layer_desc
            .map(|layer_desc| self.colors.get(layer_desc).unwrap_or(&self.default))
            .unwrap_or(&self.default)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LayerColors {
    pub normal: egui::Color32,
    pub highlighted: egui::Color32,
    pub pin_selected: egui::Color32,
    pub pin_selected_highlighted: egui::Color32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LayersPanel {
    // TODO:
    // In1.Cu shall be #7fc87f (#d5ecd5 when selected).
    // In2.Cu shall be #ce7d2c (#e8c39e when selected).
    dark_colors: Colors,
    light_colors: Colors,

    #[serde(skip)]
    pub active: LayerId,
    #[serde(skip)]
    pub visible: Box<[bool]>,
}

impl LayersPanel {
    pub fn new(board: &Board) -> Self {
        let mut dark_layer_colors = BTreeMap::new();
        let mut light_layer_colors = BTreeMap::new();

        for layer_index in 0..*board.layout().layer_count() {
            let layer = LayerId::new(layer_index);
            let Some(layer_desc) = board.layer_desc(layer) else {
                continue;
            };

            let dark_light_colors = match layer_desc.typ {
                LayerType::Copper => match layer_desc.side {
                    LayerSide::Top => Some((
                        LayerColors {
                            normal: egui::Color32::from_rgb(255, 52, 52),
                            highlighted: egui::Color32::from_rgb(255, 100, 100),
                            pin_selected: egui::Color32::from_rgb(190, 200, 70),
                            pin_selected_highlighted: egui::Color32::from_rgb(210, 240, 110),
                        },
                        LayerColors {
                            normal: egui::Color32::from_rgb(255, 27, 27),
                            highlighted: egui::Color32::from_rgb(255, 52, 52),
                            pin_selected: egui::Color32::from_rgb(160, 170, 50),
                            pin_selected_highlighted: egui::Color32::from_rgb(190, 210, 80),
                        },
                    )),
                    LayerSide::Bottom => Some((
                        LayerColors {
                            normal: egui::Color32::from_rgb(52, 52, 255),
                            highlighted: egui::Color32::from_rgb(100, 100, 255),
                            pin_selected: egui::Color32::from_rgb(70, 190, 190),
                            pin_selected_highlighted: egui::Color32::from_rgb(100, 230, 230),
                        },
                        LayerColors {
                            normal: egui::Color32::from_rgb(27, 27, 255),
                            highlighted: egui::Color32::from_rgb(52, 52, 255),
                            pin_selected: egui::Color32::from_rgb(50, 160, 160),
                            pin_selected_highlighted: egui::Color32::from_rgb(70, 200, 200),
                        },
                    )),
                    LayerSide::Inner => (layer_desc.index % 2 == 0)
                        .then_some((
                            LayerColors {
                                normal: egui::Color32::from_rgb(127, 200, 127),
                                highlighted: egui::Color32::from_rgb(213, 236, 213),
                                pin_selected: egui::Color32::from_rgb(100, 230, 100),
                                pin_selected_highlighted: egui::Color32::from_rgb(170, 250, 170),
                            },
                            LayerColors {
                                normal: egui::Color32::from_rgb(76, 169, 76),
                                highlighted: egui::Color32::from_rgb(127, 200, 127),
                                pin_selected: egui::Color32::from_rgb(60, 200, 60),
                                pin_selected_highlighted: egui::Color32::from_rgb(100, 230, 100),
                            },
                        ))
                        .or(Some((
                            LayerColors {
                                normal: egui::Color32::from_rgb(206, 125, 44),
                                highlighted: egui::Color32::from_rgb(232, 195, 158),
                                pin_selected: egui::Color32::from_rgb(170, 200, 70),
                                pin_selected_highlighted: egui::Color32::from_rgb(200, 230, 120),
                            },
                            LayerColors {
                                normal: egui::Color32::from_rgb(183, 80, 12),
                                highlighted: egui::Color32::from_rgb(206, 125, 44),
                                pin_selected: egui::Color32::from_rgb(140, 170, 50),
                                pin_selected_highlighted: egui::Color32::from_rgb(170, 200, 80),
                            },
                        ))),
                },
                LayerType::Outline => match layer_desc.side {
                    LayerSide::Top => Some((
                        LayerColors {
                            normal: egui::Color32::from_rgb(222, 217, 141),
                            highlighted: egui::Color32::from_rgb(255, 255, 215),
                            pin_selected: egui::Color32::from_rgb(170, 235, 100),
                            pin_selected_highlighted: egui::Color32::from_rgb(210, 255, 160),
                        },
                        LayerColors {
                            normal: egui::Color32::from_rgb(185, 180, 110),
                            highlighted: egui::Color32::from_rgb(245, 240, 165),
                            pin_selected: egui::Color32::from_rgb(140, 210, 80),
                            pin_selected_highlighted: egui::Color32::from_rgb(190, 245, 130),
                        },
                    )),
                    LayerSide::Bottom => Some((
                        LayerColors {
                            normal: egui::Color32::from_rgb(212, 158, 147),
                            highlighted: egui::Color32::from_rgb(255, 228, 210),
                            pin_selected: egui::Color32::from_rgb(160, 210, 110),
                            pin_selected_highlighted: egui::Color32::from_rgb(200, 245, 165),
                        },
                        LayerColors {
                            normal: egui::Color32::from_rgb(170, 130, 120),
                            highlighted: egui::Color32::from_rgb(235, 195, 185),
                            pin_selected: egui::Color32::from_rgb(130, 190, 85),
                            pin_selected_highlighted: egui::Color32::from_rgb(175, 230, 140),
                        },
                    )),
                    LayerSide::Inner => None,
                },
            };

            if let Some((dark_color, light_color)) = dark_light_colors {
                dark_layer_colors.insert(layer_desc.clone(), dark_color);
                light_layer_colors.insert(layer_desc.clone(), light_color);
            }
        }

        let dark_colors = Colors {
            layers: ColorLayers {
                default: LayerColors {
                    normal: egui::Color32::from_rgb(255, 255, 255),
                    highlighted: egui::Color32::from_rgb(255, 255, 255),
                    pin_selected: egui::Color32::from_rgb(200, 255, 200),
                    pin_selected_highlighted: egui::Color32::from_rgb(220, 255, 220),
                },
                colors: dark_layer_colors,
            },
        };
        let light_colors = Colors {
            layers: ColorLayers {
                default: LayerColors {
                    normal: egui::Color32::from_rgb(0, 0, 0),
                    highlighted: egui::Color32::from_rgb(0, 0, 0),
                    pin_selected: egui::Color32::from_rgb(0, 100, 0),
                    pin_selected_highlighted: egui::Color32::from_rgb(0, 140, 0),
                },
                colors: light_layer_colors,
            },
        };

        let layer_count = board.layout().layer_count();
        let visible = core::iter::repeat(true)
            .take(*layer_count)
            .collect::<Box<[_]>>();
        Self {
            dark_colors,
            light_colors,
            active: board.layer_id("F.Cu").unwrap_or(LayerId::new(0)),
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
                            for layer_index in row_range {
                                let layer = LayerId::new(layer_index);
                                let visible = &mut self.visible[layer.index()];
                                let layer_name = board.layer_name(layer);

                                ui.radio_value(&mut self.active, layer, WidgetText::default());
                                ui.checkbox(visible, WidgetText::default());
                                ui.label(layer_name.unwrap_or_else(|| {
                                    format!("{} - Unnamed layer", layer.index())
                                }));
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

    pub fn layer_colors(&self, ctx: &Context, layer_desc: Option<&LayerDesc>) -> &LayerColors {
        self.colors(ctx).layers.colors(layer_desc)
    }

    pub fn layer_color(
        &self,
        ctx: &Context,
        layer_desc: Option<&LayerDesc>,
        pin_selected: bool,
        highlight: bool,
    ) -> egui::Color32 {
        let colors = self.colors(ctx).layers.colors(layer_desc);
        if pin_selected {
            if highlight {
                colors.pin_selected_highlighted
            } else {
                colors.pin_selected
            }
        } else if highlight {
            colors.highlighted
        } else {
            colors.normal
        }
    }

    pub fn layers_in_display_order(&self, layer_count: usize) -> Vec<LayerId> {
        let active = self.active.index();
        let mut layers = (0..layer_count)
            .rev()
            .filter(|&layer| layer != active)
            .map(LayerId::new)
            .collect::<Vec<_>>();

        // The active layer should be drawn on top.
        layers.push(self.active);
        layers
    }
}
