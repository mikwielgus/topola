use topola::board::{mesadata::MesadataTrait, Board};

pub struct Layers {
    pub visible: Box<[bool]>,
}

impl Layers {
    pub fn new(board: &Board<impl MesadataTrait>) -> Self {
        let layer_count = board.layout().drawing().layer_count();

        Self {
            visible: std::iter::repeat(true)
                .take(layer_count.try_into().unwrap() /* FIXME */)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
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
