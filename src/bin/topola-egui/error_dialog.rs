//! dialog for error messages (e.g. for displaying file parser errors)

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::{translator::Translator, viewport::Viewport};

pub struct ErrorDialog {
    pub messages: Vec<(&'static str, String)>,
    pub window_open: bool,
}

impl ErrorDialog {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            window_open: false,
        }
    }

    // this is a separate method in order to intercept error messages
    // and also print them to the console.
    pub fn push_error(&mut self, component_id: &'static str, message: String) {
        // note that the message here is already localized, and perhaps less useful.
        log::info!("{}: {}", component_id, &message);
        self.messages.push((component_id, message));
        self.window_open = true;
    }

    pub fn update(&mut self, ctx: &egui::Context, tr: &Translator, viewport: &Viewport) {
        let mut messages_cleared = false;
        egui::Window::new(tr.text("title-error-messages"))
            .id("error-messages-dialog".into())
            .open(&mut self.window_open)
            .scroll(true)
            .show(ctx, |ui| {
                if ui.button(tr.text("reset-error-messages")).clicked() {
                    self.messages.clear();
                    messages_cleared = true;
                }

                egui::Grid::new("error-messages-grid").show(ui, |ui| {
                    let mut messages_to_discard = BTreeSet::<usize>::new();
                    let style = Arc::clone(ui.style());
                    for (msg_id, msg) in self.messages.iter().enumerate() {
                        use egui::style::{FontSelection, TextStyle};
                        use egui::text::{LayoutJob, TextWrapping};
                        use egui::widget_text::{RichText, WidgetText};
                        use egui::{Align, FontFamily, FontId, TextFormat};

                        let mut loj = LayoutJob::default();
                        loj.break_on_newline = true;
                        loj.wrap.max_width = 200.0;
                        RichText::new(&(tr.text(msg.0) + ": "))
                            .strong()
                            .color(style.visuals.text_color())
                            .append_to(&mut loj, &style, FontSelection::Default, Align::Min);
                        RichText::new(&msg.1)
                            .color(style.visuals.text_color())
                            .append_to(
                                &mut loj,
                                &style,
                                FontSelection::Style(TextStyle::Monospace),
                                Align::Min,
                            );

                        // TODO: perhaps alternatively, use small icon instead?
                        //   (provide alt text in that case!)
                        if ui.add(egui::Button::new(tr.text("discard-item"))).clicked() {
                            messages_to_discard.insert(msg_id);
                        }
                        ui.label(WidgetText::LayoutJob(loj));
                        ui.end_row();
                    }
                    if !messages_to_discard.is_empty() {
                        let mut count = 0;
                        self.messages.retain(|_| {
                            let ret = !messages_to_discard.contains(&count);
                            count += 1;
                            ret
                        });
                    }
                });
            });

        if messages_cleared {
            self.window_open = false;
        }
    }
}
