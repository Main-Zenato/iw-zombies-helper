//! Language of the text shown to the user: interface, log, command line messages.
//! English by default, French on request (`[interface] language` in the configuration). Full
//! survey files stay in French.

use serde::Deserialize;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    En,
    Fr,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Fr];

    /// Value written in the configuration file.
    pub fn key(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
        }
    }

    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|lang| lang.key() == key)
    }

    /// Name of the language, written in that language.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Fr => "Français",
        }
    }

    /// The text for this language.
    pub fn pick<T>(self, en: T, fr: T) -> T {
        match self {
            Lang::En => en,
            Lang::Fr => fr,
        }
    }
}

/// Current language, for the whole program: the interface changes it without restarting the engine.
static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == Lang::Fr as u8 { Lang::Fr } else { Lang::En }
}

/// The text for the current language.
pub fn tr(en: &'static str, fr: &'static str) -> &'static str {
    current().pick(en, fr)
}

/// Like `tr`, for built text.
pub fn trs(en: impl Into<String>, fr: impl Into<String>) -> String {
    current().pick(en.into(), fr.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_language_gets_its_own_text() {
        assert_eq!(Lang::En.pick("Infinite money", "Argent infini"), "Infinite money");
        assert_eq!(Lang::Fr.pick("Infinite money", "Argent infini"), "Argent infini");
        assert_eq!(Lang::default(), Lang::En);
        assert_eq!(Lang::parse("fr"), Some(Lang::Fr));
        assert_eq!(Lang::parse("de"), None);
        assert!(Lang::ALL.iter().all(|&lang| Lang::parse(lang.key()) == Some(lang)));
    }
}
