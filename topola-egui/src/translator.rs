// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use derive_getters::Getters;
use fluent_templates::{Loader, static_loader};
use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;

static_loader! {
    static LOCALES = {
        locales: "../locales",
        fallback_language: "en-US",
    };
}

#[derive(Debug, Serialize, Deserialize, Getters)]
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

    pub fn langid_mut(&mut self) -> &mut LanguageIdentifier {
        &mut self.langid
    }

    pub fn locales() -> Box<dyn Iterator<Item = &'static LanguageIdentifier> + 'static> {
        LOCALES.locales()
    }
}
