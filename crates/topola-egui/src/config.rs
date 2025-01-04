// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {
    dark_colors: Colors,
    light_colors: Colors,
}

impl Config {
    pub fn colors(&self, ctx: &egui::Context) -> &Colors {
        match ctx.theme() {
            egui::Theme::Dark => &self.dark_colors,
            egui::Theme::Light => &self.light_colors,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Colors {
    pub layers: LayerColors,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LayerColors {
    default: LayerColor,
    colors: BTreeMap<String, LayerColor>,
}

impl LayerColors {
    pub fn color(&self, layername: Option<&str>) -> &LayerColor {
        layername
            .and_then(|layername| Some(self.colors.get(layername).unwrap_or(&self.default)))
            .unwrap_or(&self.default)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LayerColor {
    pub normal: egui::Color32,
    pub highlighted: egui::Color32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dark_colors: Colors {
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
            },
            light_colors: Colors {
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
            },
        }
    }
}
