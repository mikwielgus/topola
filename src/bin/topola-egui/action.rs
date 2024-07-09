pub struct Action {
    name: String,
    shortcut: egui::KeyboardShortcut,
}

impl Action {
    pub fn new(name: &str, modifiers: egui::Modifiers, key: egui::Key) -> Self {
        Self {
            name: String::from(name),
            shortcut: egui::KeyboardShortcut::new(modifiers, key),
        }
    }

    fn widget_text(&self) -> String {
        format!(
            "{} ({})",
            self.name,
            self.shortcut.format(&egui::ModifierNames::NAMES, false)
        )
    }
}

pub struct Trigger {
    action: Action,
    triggered: bool,
}

impl Trigger {
    pub fn new(action: Action) -> Self {
        Self {
            action,
            triggered: false,
        }
    }

    pub fn button(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        self.triggered = ui.button(self.action.widget_text()).clicked();
    }

    pub fn consume_key_triggered(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
        self.consume_key(ctx, ui);
        self.triggered()
    }

    fn consume_key(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if ctx.input_mut(|i| i.consume_shortcut(&self.action.shortcut)) {
            self.triggered = true;
        }
    }

    fn triggered(&self) -> bool {
        self.triggered
    }
}

pub struct Switch {
    action: Action,
}

impl Switch {
    pub fn new(action: Action) -> Self {
        Self { action }
    }

    pub fn toggle_widget(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, selected: &mut bool) {
        ui.toggle_value(selected, self.action.widget_text());
    }

    pub fn consume_key_enabled(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        selected: &mut bool,
    ) -> bool {
        if ctx.input_mut(|i| i.consume_shortcut(&self.action.shortcut)) {
            *selected = !*selected;
        }

        *selected
    }
}
