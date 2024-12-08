use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {
    pub dark_theme: Colors,
    pub light_theme: Colors,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Colors {
    pub layers: LayerColors,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LayerColors {
    default: LayerColor,
    colors: HashMap<String, LayerColor>,
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
        let dark_theme = Colors {
            layers: LayerColors {
                default: LayerColor {
                    normal: egui::Color32::from_rgb(255, 255, 255),
                    highlighted: egui::Color32::from_rgb(255, 255, 255),
                },
                colors: HashMap::from([
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

        Self {
            dark_theme: dark_theme.clone(),
            light_theme: dark_theme,
        }
    }
}
