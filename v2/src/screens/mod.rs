//! The screens of the v2 window. Seven, and the sidebar shows them all:
//! nothing is two clicks deep that the counter uses every hour.

pub mod accueil;
pub mod agenda;
pub mod medicaments;
pub mod options;
pub mod patients;
pub mod stups;
pub mod verifier;

use crate::session::Session;
use crate::t::t;
use motif::Pict;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Screen {
    Accueil,
    Patients,
    Medicaments,
    Verifier,
    Agenda,
    Stups,
    Options,
}

impl Screen {
    pub const ALL: [Screen; 7] = [
        Screen::Accueil,
        Screen::Patients,
        Screen::Medicaments,
        Screen::Verifier,
        Screen::Agenda,
        Screen::Stups,
        Screen::Options,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Screen::Accueil => t("nav.accueil"),
            Screen::Patients => t("nav.patients"),
            Screen::Medicaments => t("nav.medicaments"),
            Screen::Verifier => t("nav.verifier"),
            Screen::Agenda => t("nav.agenda"),
            Screen::Stups => t("nav.stups"),
            Screen::Options => t("nav.options"),
        }
    }

    pub fn pict(self) -> Pict {
        match self {
            Screen::Accueil => Pict::Chart,
            Screen::Patients => Pict::Register,
            Screen::Medicaments => Pict::Pill,
            Screen::Verifier => Pict::Check,
            Screen::Agenda => Pict::Calendar,
            Screen::Stups => Pict::Lock,
            Screen::Options => Pict::Cog,
        }
    }

    /// The function key that reaches it from anywhere.
    pub fn key(self) -> eframe::egui::Key {
        use eframe::egui::Key;
        match self {
            Screen::Accueil => Key::F1,
            Screen::Patients => Key::F2,
            Screen::Medicaments => Key::F3,
            Screen::Verifier => Key::F4,
            Screen::Agenda => Key::F5,
            Screen::Stups => Key::F6,
            Screen::Options => Key::F7,
        }
    }
}

/// Where a screen asks the shell to go next.
#[derive(Clone, Debug, PartialEq)]
pub enum Go {
    Screen(Screen),
    Patient(i64),
    NewPatient,
    Drug(i64),
    /// Open the check with this file's treatments.
    Check(i64),
}

/// What a screen gets each frame: the base and a way to ask to move.
pub struct Ctx<'a> {
    pub s: &'a mut Session,
    pub go: &'a mut Option<Go>,
}

impl Ctx<'_> {
    pub fn go(&mut self, to: Go) {
        *self.go = Some(to);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_screen_has_its_own_key_label_and_mark() {
        let mut keys = std::collections::HashSet::new();
        let mut labels = std::collections::HashSet::new();
        for s in Screen::ALL {
            assert!(keys.insert(s.key()), "{s:?}");
            assert!(labels.insert(s.label()), "{s:?}");
            let _ = s.pict();
        }
    }
}
