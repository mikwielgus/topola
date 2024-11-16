use crate::{
    action::{Action, Switch, Trigger},
    translator::Translator,
};

pub struct FileActions {
    pub open_design: Trigger,
    pub export_session: Trigger,
    pub import_history: Trigger,
    pub export_history: Trigger,
    pub quit: Trigger,
}

impl FileActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            open_design: Trigger::new(Action::new(
                tr.text("tr-menu-file-open"),
                egui::Modifiers::CTRL,
                egui::Key::O,
            )),
            export_session: Trigger::new(Action::new(
                tr.text("tr-menu-file-export-session-file"),
                egui::Modifiers::CTRL,
                egui::Key::S,
            )),
            import_history: Trigger::new(Action::new(
                tr.text("tr-menu-file-import-history"),
                egui::Modifiers::CTRL,
                egui::Key::I,
            )),
            export_history: Trigger::new(Action::new(
                tr.text("tr-menu-file-export-history"),
                egui::Modifiers::CTRL,
                egui::Key::E,
            )),
            quit: Trigger::new(Action::new(
                tr.text("tr-menu-file-quit"),
                egui::Modifiers::CTRL,
                egui::Key::Q,
            )),
        }
    }
}

pub struct EditActions {
    pub undo: Trigger,
    pub redo: Trigger,
    pub abort: Trigger,
    pub remove_bands: Trigger,
}

impl EditActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            undo: Trigger::new(Action::new(
                tr.text("tr-menu-edit-undo"),
                egui::Modifiers::CTRL,
                egui::Key::Z,
            )),
            redo: Trigger::new(Action::new(
                tr.text("tr-menu-edit-redo"),
                egui::Modifiers::CTRL,
                egui::Key::Y,
            )),
            abort: Trigger::new(Action::new(
                tr.text("tr-menu-edit-abort"),
                egui::Modifiers::NONE,
                egui::Key::Escape,
            )),
            remove_bands: Trigger::new(Action::new(
                tr.text("tr-menu-edit-remove-bands"),
                egui::Modifiers::NONE,
                egui::Key::Delete,
            )),
        }
    }
}

pub struct PlaceActions {
    pub place_via: Switch,
}

impl PlaceActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            place_via: Switch::new(Action::new(
                tr.text("tr-menu-place-place-via"),
                egui::Modifiers::CTRL,
                egui::Key::P,
            )),
        }
    }
}

pub struct RouteActions {
    pub autoroute: Trigger,
}

impl RouteActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            autoroute: Trigger::new(Action::new(
                tr.text("tr-menu-route-autoroute"),
                egui::Modifiers::CTRL,
                egui::Key::A,
            )),
        }
    }
}

pub struct InspectActions {
    pub compare_detours: Trigger,
    pub measure_length: Trigger,
}

impl InspectActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            compare_detours: Trigger::new(Action::new(
                tr.text("tr-menu-inspect-compare-detours"),
                egui::Modifiers::NONE,
                egui::Key::Minus,
            )),
            measure_length: Trigger::new(Action::new(
                tr.text("tr-menu-inspect-measure-length"),
                egui::Modifiers::NONE,
                egui::Key::Plus,
            )),
        }
    }
}

pub struct HelpActions {
    pub online_documentation: Trigger,
}

impl HelpActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            online_documentation: Trigger::new(Action::new(
                tr.text("tr-menu-help-online-documentation"),
                egui::Modifiers::NONE,
                egui::Key::F1,
            )),
        }
    }
}

pub struct Actions {
    pub file: FileActions,
    pub edit: EditActions,
    pub place: PlaceActions,
    pub route: RouteActions,
    pub inspect: InspectActions,
    pub help: HelpActions,
}

impl Actions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            file: FileActions::new(tr),
            edit: EditActions::new(tr),
            place: PlaceActions::new(tr),
            route: RouteActions::new(tr),
            inspect: InspectActions::new(tr),
            help: HelpActions::new(tr),
        }
    }
}
