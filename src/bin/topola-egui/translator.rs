use fluent_templates::{static_loader, Loader};
use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en-US",
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Translator {
    langid: LanguageIdentifier,
}

impl Translator {
    pub fn new(langid: LanguageIdentifier) -> Self {
        Self { langid }
    }

    pub fn text(&self, fluent_id: &str) -> String {
        LOCALES.lookup(&self.langid, fluent_id)
    }
}
