use topola::board::{mesadata::MesadataTrait, Board};

pub struct Layers {
    // TODO:
    // In1.Cu shall be #7fc87f (#d5ecd5 when selected).
    // In2.Cu shall be #ce7d2c (#e8c39e when selected).
    pub visible: Box<[bool]>,
    pub colors: Box<[egui::Color32]>,
    pub highlight_colors: Box<[egui::Color32]>,
}

impl Layers {
    pub fn new(board: &Board<impl MesadataTrait>) -> Self {
        let layer_count = board.layout().drawing().layer_count();
        let visible = std::iter::repeat(true)
            .take(layer_count)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let colors = std::iter::repeat(egui::Color32::from_rgb(255, 255, 255))
            .enumerate()
            .map(|(i, color)| {
                if matches!(board.mesadata().layer_layername(i), Some("F.Cu")) {
                    egui::Color32::from_rgb(255, 52, 52)
                } else if matches!(board.mesadata().layer_layername(i), Some("B.Cu")) {
                    egui::Color32::from_rgb(52, 52, 255)
                } else {
                    color
                }
            })
            .take(layer_count)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let highlight_colors = std::iter::repeat(egui::Color32::from_rgb(255, 255, 255))
            .enumerate()
            .map(|(i, color)| {
                if matches!(board.mesadata().layer_layername(i), Some("F.Cu")) {
                    egui::Color32::from_rgb(255, 100, 100)
                } else if matches!(board.mesadata().layer_layername(i), Some("B.Cu")) {
                    egui::Color32::from_rgb(100, 100, 255)
                } else {
                    color
                }
            })
            .take(layer_count)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            visible,
            colors,
            highlight_colors,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, board: &Board<impl MesadataTrait>) {
        egui::SidePanel::right("right_side_panel").show(ctx, |ui| {
            ui.label("Layers");

            for (layer, visible) in self.visible.iter_mut().enumerate() {
                let layername = board
                    .layout()
                    .drawing()
                    .rules()
                    .layer_layername(layer.try_into().unwrap() /* FIXME */)
                    .unwrap_or("Unnamed layer");

                ui.checkbox(visible, layername);
            }
        });
    }
}
