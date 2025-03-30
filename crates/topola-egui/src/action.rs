// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

pub struct Action {
    name: String,
    shortcut: Option<egui::KeyboardShortcut>,
}

pub struct Trigger {
    action: Action,
    triggered: bool,
}

pub struct Switch {
    action: Action,
}

impl Action {
    pub fn new(name: String, modifiers: egui::Modifiers, key: egui::Key) -> Self {
        Self {
            name,
            shortcut: Some(egui::KeyboardShortcut::new(modifiers, key)),
        }
    }

    pub fn new_keyless(name: String) -> Self {
        Self {
            name,
            shortcut: None,
        }
    }

    fn widget_text(&self) -> String {
        if let Some(shortcut) = self.shortcut {
            format!(
                "{} ({})",
                self.name,
                shortcut.format(&egui::ModifierNames::NAMES, false)
            )
        } else {
            self.name.clone()
        }
    }

    #[inline]
    pub fn into_trigger(self) -> Trigger {
        Trigger {
            action: self,
            triggered: false,
        }
    }

    #[inline(always)]
    pub fn into_switch(self) -> Switch {
        Switch { action: self }
    }
}

impl Trigger {
    pub fn button(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        self.triggered = ui.button(self.action.widget_text()).clicked();
    }

    pub fn hyperlink(&self, _ctx: &egui::Context, ui: &mut egui::Ui, url: &str) {
        ui.hyperlink_to(self.action.widget_text(), url);
    }

    pub fn consume_key_triggered(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
        self.consume_key(ctx, ui);
        self.triggered
    }

    fn consume_key(&mut self, ctx: &egui::Context, _ui: &mut egui::Ui) {
        if let Some(shortcut) = self.action.shortcut {
            if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                self.triggered = true;
            }
        }
    }
}

impl Switch {
    pub fn toggle_widget(&self, _ctx: &egui::Context, ui: &mut egui::Ui, selected: &mut bool) {
        ui.toggle_value(selected, self.action.widget_text());
    }

    pub fn consume_key_enabled(
        &mut self,
        ctx: &egui::Context,
        _ui: &mut egui::Ui,
        selected: &mut bool,
    ) -> bool {
        if let Some(shortcut) = self.action.shortcut {
            if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                *selected = !*selected;
            }
        }

        *selected
    }
}
