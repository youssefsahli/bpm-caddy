//! La traçabilité des accès : qui a ouvert quel dossier, et quand.
//!
//! # L'exact contraire de `telemetry.rs`, et c'est le dessin
//!
//! Les deux modules comptent ce que fait l'officine, et ils sont bâtis
//! sur des règles opposées, chacune écrite chez elle. **La télémétrie
//! compte le logiciel et jamais la personne** : aucun opérateur, aucune
//! initiale, parce que « combien de dossiers ont été ouverts » est une
//! question sur laquelle on décide et « combien un tel en a ouverts »
//! n'en est pas une. **La traçabilité nomme la personne et jamais le
//! logiciel** : elle n'existe que pour répondre à « qui a regardé ce
//! dossier-là, le 12 mars », qui est la seule question qu'un accès à une
//! donnée de santé ait à pouvoir rendre.
//!
//! Confondre les deux serait le pire des deux : des chiffres d'usage
//! nominatifs que personne n'a demandés, et une traçabilité anonyme qui
//! ne trace rien.
//!
//! # Ce qu'une ligne porte, et ce qu'elle ne porte pas
//!
//! Un **numéro de dossier**, jamais un nom — la règle que l'ordonnancier
//! suit déjà, et pour la même raison : ce qui s'imprime et se pose sur
//! un comptoir doit permettre de *remonter* au patient, pas de
//! l'afficher. Un accès nominatif imprimé est un second fichier de
//! patients, en clair, que personne n'a décidé de créer.
//!
//! Et **ce qui manque se dit** : sans opérateur déclaré dans
//! `[ui] operator`, la ligne écrit un tiret. Une trace qui devine qui
//! c'était est pire qu'une trace qui dit qu'elle ne sait pas — la
//! première se lit comme une preuve.
//!
//! # La conservation
//!
//! Un journal d'accès se garde une durée **fixée et proportionnée**, et
//! pas pour toujours : c'est un fichier de qui-a-vu-quoi sur les gens
//! qui travaillent là. La durée est dans `[audit] keep_days`, l'officine
//! la règle, et la purge **s'écrit dans le journal qu'elle purge** —
//! sinon un journal qui a rétréci et un journal qu'on a vidé se lisent
//! pareil.
//!
//! # Ce qui est tracé, et ce qui ne l'est pas
//!
//! Deux gestes : **ouvrir un dossier** et **exporter** — les deux actes
//! par lesquels une donnée de patient est regardée ou sort. L'ouverture
//! est prise à la porte unique par laquelle passent la vingtaine
//! d'endroits qui ouvrent un dossier.
//!
//! Ce qui n'est **pas** tracé, et il vaut mieux l'écrire que de le
//! laisser croire : l'impression. Les vingt et un documents imprimables
//! passent bien par une fonction unique, mais elle n'a en main ni
//! l'opérateur ni le dossier — elle reçoit du Typst déjà composé. Les
//! tracer demanderait de faire descendre les deux jusque-là, et ce
//! jour-là ce sera une ligne de plus dans [`Act`] et pas un autre
//! dessin. Un journal qui prétendrait tout voir serait pire que celui-ci.
//!
//! Pur, testé, sans horloge : le jour est passé.

use crate::strings::{tr, trf};

/// Ce qui a été fait d'un dossier.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    /// Le dossier a été ouvert à l'écran.
    Ouvert,
    /// Des données sont sorties : l'export CSV.
    Exporte,
    /// La purge du journal lui-même, écrite dedans.
    ///
    /// Elle n'a pas de dossier — c'est le seul acte qui n'en ait pas —
    /// et elle est ici plutôt que dans un fichier à côté parce qu'un
    /// journal qui a rétréci et un journal qu'on a vidé se lisent
    /// autrement pareil.
    Purge,
}

impl Act {
    pub const ALL: [Act; 3] = [Act::Ouvert, Act::Exporte, Act::Purge];

    /// Ce qui est écrit dans la base. Ne change jamais.
    pub fn key(self) -> &'static str {
        match self {
            Act::Ouvert => "ouvert",
            Act::Exporte => "exporte",
            Act::Purge => "purge",
        }
    }

    /// Un acte qu'une version postérieure a inventé est rendu tel quel
    /// par la base et n'est simplement pas nommé ici : la ligne reste,
    /// et c'est ce qui compte dans un journal d'accès.
    pub fn from_key(key: &str) -> Option<Act> {
        Act::ALL.into_iter().find(|a| a.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            Act::Ouvert => tr("audit_act_ouvert"),
            Act::Exporte => tr("audit_act_exporte"),
            Act::Purge => tr("audit_act_purge"),
        }
    }
}

/// Une ligne du journal d'accès.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Access {
    /// `YYYY-MM-DD HH:MM:SS`, heure locale du poste qui a écrit.
    pub at: String,
    /// Les initiales déclarées dans `[ui] operator`. Vides quand le
    /// poste n'en déclare pas — et c'est alors écrit, pas deviné.
    pub operator: String,
    pub act: Act,
    /// Le **numéro** du dossier. Jamais un nom : voir l'en-tête.
    /// Zéro pour un acte qui ne porte pas sur un dossier.
    pub file: i64,
}

impl Access {
    /// La journée, pour grouper et pour purger.
    pub fn day(&self) -> &str {
        self.at.get(..10).unwrap_or(&self.at)
    }

    /// Qui, tel qu'on l'écrit. Un tiret quand le poste ne déclarait
    /// personne — dire « je ne sais pas » est une réponse, en inventer
    /// une n'en est pas.
    pub fn who(&self) -> &str {
        let who = self.operator.trim();
        if who.is_empty() {
            "—"
        } else {
            who
        }
    }
}

/// Le jour le plus ancien que l'officine garde.
///
/// `keep_days` à zéro veut dire « ne rien purger » et non « tout
/// purger » : un réglage vide ou oublié ne doit pas effacer un journal.
/// C'est la même direction que les horaires d'ouverture livrés vides —
/// sans consigne écrite, on ne fait rien plutôt que de tout faire.
pub fn horizon(today: &str, keep_days: u32) -> Option<String> {
    if keep_days == 0 {
        return None;
    }
    crate::date::add_days(today, -(i64::from(keep_days) - 1))
}

/// Ce qu'un relevé dit de lui-même.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Summary {
    pub lines: usize,
    /// Sur combien de journées distinctes. Un cumul sans période à côté
    /// se lit comme s'il couvrait tout ce qui a jamais existé — la règle
    /// du récapitulatif de la caisse.
    pub days: usize,
    pub first: String,
    pub last: String,
    /// Combien de dossiers distincts ont été touchés.
    pub files: usize,
    /// Par acte, dans l'ordre de [`Act::ALL`].
    pub by_act: Vec<(Act, usize)>,
    /// Par opérateur, du plus actif au moins actif, puis par nom — un
    /// ordre **total**, sans quoi deux relevés du même journal ne se
    /// ressemblent pas.
    pub by_operator: Vec<(String, usize)>,
}

/// Lit un relevé. Ne trie rien à la source : le journal arrive dans
/// l'ordre où il a été écrit, et c'est cet ordre qui a une valeur.
pub fn summarize(accesses: &[Access]) -> Summary {
    use std::collections::{BTreeMap, BTreeSet};

    let mut days: BTreeSet<&str> = BTreeSet::new();
    let mut files: BTreeSet<i64> = BTreeSet::new();
    let mut per_act: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut per_who: BTreeMap<&str, usize> = BTreeMap::new();

    for a in accesses {
        days.insert(a.day());
        // Zéro n'est pas un dossier : c'est ce que porte un acte qui
        // n'en vise aucun, et le compter gonflerait le nombre de
        // dossiers touchés d'exactement un, toujours.
        if a.file != 0 {
            files.insert(a.file);
        }
        *per_act.entry(a.act.key()).or_default() += 1;
        *per_who.entry(a.who()).or_default() += 1;
    }

    let mut by_operator: Vec<(String, usize)> = per_who
        .into_iter()
        .map(|(who, n)| (who.to_owned(), n))
        .collect();
    by_operator.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    Summary {
        lines: accesses.len(),
        days: days.len(),
        first: days
            .iter()
            .next()
            .map(|d| (*d).to_owned())
            .unwrap_or_default(),
        last: days
            .iter()
            .next_back()
            .map(|d| (*d).to_owned())
            .unwrap_or_default(),
        files: files.len(),
        by_act: Act::ALL
            .into_iter()
            .map(|act| (act, per_act.get(act.key()).copied().unwrap_or(0)))
            .collect(),
        by_operator,
    }
}

/// Ce qu'une officine a fait sur la période — **compté, jamais nommé**.
///
/// Les actes portent une nature et un opérateur ; ils ne portent pas le
/// patient. Le rapport est un relevé de décision, pas une liste de
/// dossiers : la liste de dossiers existe déjà, elle est dans la base,
/// et elle y est chiffrée.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Activity {
    /// Par nature d'acte, du plus fréquent au moins fréquent.
    pub acts: Vec<(String, usize)>,
    /// Par opérateur, même ordre.
    pub by_operator: Vec<(String, usize)>,
    pub register_lines: usize,
    /// Combien de soirs ont été comptés, et l'écart cumulé en centimes.
    ///
    /// `None` quand aucune recette attendue n'a été saisie : **sans
    /// attendu il n'y a pas d'écart**, et annoncer zéro serait annoncer
    /// que tout tombe juste. La règle de `caisse.rs`, telle quelle.
    pub till_evenings: usize,
    /// Sur combien de ces soirs une recette attendue avait été saisie.
    ///
    /// **Ce n'est pas `till_evenings`**, et confondre les deux est
    /// précisément l'erreur que le récapitulatif de la caisse refuse :
    /// un écart cumulé annoncé « sur vingt-quatre soirs » quand il n'en
    /// couvre que vingt-deux est un écart qu'on croit plus petit qu'il
    /// n'est.
    pub till_expected_evenings: usize,
    pub till_gap_cents: Option<i64>,
}

/// Ce que la base dit d'elle-même.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Conformity {
    /// Libellés de classe que `classes.rs` ne sait pas replier, et
    /// combien de fiches les portent. C'est la dérive que ce
    /// référentiel existe pour montrer : deux orthographes d'une classe
    /// sont deux classes, et une fiche se retrouve seule de son espèce
    /// sans que rien n'ait l'air cassé.
    pub unknown_classes: Vec<(String, usize)>,
    /// Réécritures que la version livrée a périmées : elles attendent
    /// une relecture et **ne s'impriment pas**.
    pub outdated_rewrites: usize,
    pub cards_without_dci: usize,
    pub cards_without_class: usize,
}

/// L'en-tête : de qui, de quoi, sur quand.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Head {
    pub officine: String,
    pub base: String,
    pub made_on: String,
    pub from: String,
    pub to: String,
    pub days: usize,
}

/// Écrit le rapport.
///
/// Trois sections, et **aucune n'est laissée vide** : une section qui ne
/// trouve rien l'écrit en toutes lettres. Un blanc se lit comme une
/// panne du programme, ce qui est exactement le contraire de ce qu'un
/// rapport d'audit doit produire.
pub fn render(
    head: &Head,
    activity: &Activity,
    access: &Summary,
    conformity: &Conformity,
) -> String {
    let mut out = String::new();
    let line = |out: &mut String, s: &str| {
        out.push_str(s);
        out.push('\n');
    };
    let rule = |out: &mut String| line(out, "────────────────────────────────────────────────────");
    // Un libellé, un chiffre, et des points entre les deux. Le libellé
    // **plus long que la colonne** collait son chiffre contre lui :
    // « au registre des stupéfiants     5 » se lisait comme une
    // quantité de stupéfiants. Il reste toujours une espace.
    let figure = |label: &str, n: &dyn std::fmt::Display, indent: &str| {
        let n = n.to_string();
        let width = 40usize.saturating_sub(indent.chars().count());
        let dots = width.saturating_sub(label.chars().count());
        format!("{indent}{label}{} {n}", ".".repeat(dots))
    };

    // Une officine qui n'a pas encore écrit son nom donne un titre qui
    // s'arrête sur un tiret en l'air : c'est le même défaut qu'un état
    // vide laissé blanc, et il se lit comme une panne du programme.
    line(
        &mut out,
        &if head.officine.trim().is_empty() {
            tr("audit_report_title_none").to_owned()
        } else {
            trf("audit_report_title", head.officine.trim())
        },
    );
    line(&mut out, &trf("audit_report_base", &head.base));
    line(
        &mut out,
        &crate::strings::trn("audit_report_period", &[&head.from, &head.to, &head.days]),
    );
    line(&mut out, &trf("audit_report_made", &head.made_on));
    line(&mut out, tr("audit_report_scope"));
    line(&mut out, "");

    rule(&mut out);
    line(&mut out, tr("audit_report_activity"));
    rule(&mut out);
    if activity.acts.is_empty() && activity.register_lines == 0 && activity.till_evenings == 0 {
        line(&mut out, tr("audit_report_nothing"));
    } else {
        line(&mut out, tr("audit_report_acts"));
        for (kind, n) in &activity.acts {
            line(&mut out, &figure(kind, n, "  "));
        }
        if activity.acts.is_empty() {
            line(&mut out, &format!("  {}", tr("audit_report_nothing")));
        }
        line(&mut out, "");
        line(&mut out, tr("audit_by_operator"));
        for (who, n) in &activity.by_operator {
            line(&mut out, &figure(who, n, "  "));
        }
        line(&mut out, "");
        line(
            &mut out,
            &figure(tr("audit_report_register"), &activity.register_lines, ""),
        );
        line(
            &mut out,
            &figure(tr("audit_report_tills"), &activity.till_evenings, ""),
        );
        match activity.till_gap_cents {
            Some(cents) => line(
                &mut out,
                &crate::strings::trn(
                    "audit_report_gap",
                    &[
                        &activity.till_expected_evenings,
                        &format!("{} €", crate::caisse::euros(cents)),
                    ],
                ),
            ),
            None => line(&mut out, tr("audit_report_gap_none")),
        }
    }
    line(&mut out, "");

    rule(&mut out);
    line(&mut out, tr("audit_report_access"));
    rule(&mut out);
    if access.lines == 0 {
        line(&mut out, tr("audit_report_nothing"));
    } else {
        for (act, n) in &access.by_act {
            line(&mut out, &figure(act.label(), n, "  "));
        }
        line(&mut out, "");
        line(&mut out, tr("audit_by_operator"));
        for (who, n) in &access.by_operator {
            line(&mut out, &figure(who, n, "  "));
        }
        line(&mut out, "");
        line(
            &mut out,
            &crate::strings::trn(
                "audit_span",
                &[
                    &access.lines,
                    &access.days,
                    &crate::db::format_french_date(&access.first),
                    &crate::db::format_french_date(&access.last),
                    &access.files,
                ],
            ),
        );
    }
    line(&mut out, "");

    rule(&mut out);
    line(&mut out, tr("audit_report_conformity"));
    rule(&mut out);
    let clean = conformity.unknown_classes.is_empty()
        && conformity.outdated_rewrites == 0
        && conformity.cards_without_dci == 0
        && conformity.cards_without_class == 0;
    if clean {
        line(&mut out, tr("audit_report_conform"));
    } else {
        if !conformity.unknown_classes.is_empty() {
            line(&mut out, tr("audit_report_unknown_classes"));
            for (label, n) in &conformity.unknown_classes {
                line(&mut out, &figure(label, n, "  "));
            }
            line(&mut out, "");
        }
        line(
            &mut out,
            &trf("audit_report_outdated", conformity.outdated_rewrites),
        );
        line(
            &mut out,
            &trf("audit_report_no_dci", conformity.cards_without_dci),
        );
        line(
            &mut out,
            &trf("audit_report_no_class", conformity.cards_without_class),
        );
    }
    out
}

/// Ce que le programme répond quand il ne peut pas faire le rapport.
/// Écrites ici parce que c'est ici que vivent les clés de ce module —
/// une clé employée dans `main.rs` seul serait une clé que le garde des
/// libellés compte comme inutilisée.
pub fn linux_only() -> &'static str {
    tr("audit_report_linux")
}

pub fn usage() -> &'static str {
    tr("audit_report_usage")
}

pub fn locked() -> &'static str {
    tr("audit_report_locked")
}

pub fn unreadable(why: &str) -> String {
    trf("audit_report_unreadable", why)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: &str, hour: &str, who: &str, act: Act, file: i64) -> Access {
        Access {
            at: format!("{day} {hour}"),
            operator: who.to_owned(),
            act,
            file,
        }
    }

    /// Une trace nomme la personne — et c'est tout l'écart avec la
    /// télémétrie, qui ne le fait jamais. Les deux modules répondent à
    /// deux questions, et les confondre donnerait le pire des deux.
    #[test]
    fn a_trace_names_the_person_and_a_missing_name_is_said() {
        let named = at("2026-09-19", "09:12:00", "CL", Act::Ouvert, 4021);
        assert_eq!(named.who(), "CL");
        // Sans opérateur déclaré : dit, jamais deviné.
        let anonymous = at("2026-09-19", "09:12:00", "   ", Act::Ouvert, 4021);
        assert_eq!(anonymous.who(), "—");
        assert_eq!(at("2026-09-19", "09:12:00", "", Act::Ouvert, 1).who(), "—");
    }

    /// Un accès porte un **numéro** de dossier, jamais un nom : ce qui
    /// s'imprime doit permettre de remonter au patient, pas de
    /// l'afficher. La règle est tenue par le texte du module, comme
    /// celle du registre l'est dans `db.rs`.
    #[test]
    fn an_access_carries_a_file_number_and_never_a_name() {
        let text = include_str!("audit.rs");
        // Les champs seuls : la documentation, elle, parle de noms —
        // c'est précisément son travail de dire lesquels sont absents.
        let shape: String = text
            .split("pub struct Access {")
            .nth(1)
            .and_then(|t| t.split("\n}").next())
            .expect("la forme d'une ligne")
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for named in ["name", "nom", "first", "last", "prenom", "nir", "birth"] {
            assert!(
                !shape.contains(named),
                "une ligne d'accès ne porte pas « {named} »"
            );
        }
        assert!(shape.contains("pub file: i64"));
    }

    /// La journée se lit dans l'horodatage, et une valeur trop courte ne
    /// fait pas tomber l'application : ces chaînes viennent de la base.
    #[test]
    fn a_day_is_read_out_of_the_stamp_without_panicking() {
        assert_eq!(
            at("2026-09-19", "09:12:00", "CL", Act::Ouvert, 1).day(),
            "2026-09-19"
        );
        let short = Access {
            at: "2026".to_owned(),
            operator: String::new(),
            act: Act::Ouvert,
            file: 1,
        };
        assert_eq!(short.day(), "2026");
    }

    /// La durée est comptée depuis aujourd'hui, et un réglage vide ne
    /// purge rien — un oubli ne doit pas effacer un journal.
    #[test]
    fn a_kept_duration_is_counted_from_today_and_zero_purges_nothing() {
        // Trente jours au 19 septembre : on garde à partir du 21 août.
        // Trente jours au 19 septembre : on garde à partir du 21 août.
        assert_eq!(horizon("2026-09-19", 30).as_deref(), Some("2026-08-21"));
        // Un jour de conservation garde la journée en cours, et elle
        // seule. Que la borne soit *gardée* est tenu par la purge
        // elle-même, dans `db.rs` : la comparaison se fait en SQL, et
        // la réécrire ici en Rust serait deux écritures d'une même
        // chose — le jour où elles divergent, c'est celle que personne
        // ne fait tourner qui a l'air juste.
        assert_eq!(horizon("2026-09-19", 1).as_deref(), Some("2026-09-19"));
        // Zéro : rien à purger, et surtout pas tout.
        assert_eq!(horizon("2026-09-19", 0), None);
    }

    /// Un relevé dit sur combien de journées il court, combien de
    /// dossiers il touche, et par qui — dans un ordre total, sans quoi
    /// deux relevés du même journal ne se ressemblent pas.
    #[test]
    fn a_reading_says_over_how_many_days_it_runs() {
        let lines = [
            at("2026-09-17", "09:00:00", "CL", Act::Ouvert, 4021),
            at("2026-09-17", "09:05:00", "CL", Act::Ouvert, 4022),
            at("2026-09-18", "10:00:00", "MB", Act::Ouvert, 4021),
            at("2026-09-18", "18:00:00", "MB", Act::Exporte, 0),
            at("2026-09-19", "07:00:00", "", Act::Purge, 0),
        ];
        let s = summarize(&lines);
        assert_eq!((s.lines, s.days), (5, 3));
        assert_eq!(
            (s.first.as_str(), s.last.as_str()),
            ("2026-09-17", "2026-09-19")
        );
        // Deux dossiers distincts : le zéro de l'export et celui de la
        // purge ne sont pas des dossiers.
        assert_eq!(s.files, 2);
        assert_eq!(
            s.by_act,
            vec![(Act::Ouvert, 3), (Act::Exporte, 1), (Act::Purge, 1)]
        );
        // Le plus actif d'abord, puis par nom.
        assert_eq!(
            s.by_operator,
            vec![
                ("CL".to_owned(), 2),
                ("MB".to_owned(), 2),
                ("—".to_owned(), 1)
            ]
        );
        // Et un relevé vide ne prétend rien.
        assert_eq!(summarize(&[]).days, 0);
        assert!(summarize(&[]).first.is_empty());
    }

    fn full() -> (Head, Activity, Summary, Conformity) {
        let head = Head {
            officine: "Pharmacie des Lilas".to_owned(),
            base: "/srv/officine/bpm_caddy.db".to_owned(),
            made_on: "19/09/2026".to_owned(),
            from: "21/08/2026".to_owned(),
            to: "19/09/2026".to_owned(),
            days: 30,
        };
        let activity = Activity {
            acts: vec![("BPM".to_owned(), 4), ("AOD".to_owned(), 3)],
            by_operator: vec![("CL".to_owned(), 5), ("—".to_owned(), 2)],
            register_lines: 5,
            till_evenings: 24,
            till_expected_evenings: 22,
            till_gap_cents: Some(1975),
        };
        let access = summarize(&[
            at("2026-09-17", "09:00:00", "CL", Act::Ouvert, 4021),
            at("2026-09-18", "18:00:00", "MB", Act::Exporte, 0),
        ]);
        let conformity = Conformity {
            unknown_classes: vec![("anti-TNF alpha".to_owned(), 3)],
            outdated_rewrites: 2,
            cards_without_dci: 7,
            cards_without_class: 1,
        };
        (head, activity, access, conformity)
    }

    /// Le rapport dit de qui il parle, de quelle base, et **sur quelle
    /// période** — un relevé sans période se lit comme s'il couvrait
    /// tout ce qui a jamais existé.
    #[test]
    fn a_report_says_whose_it_is_and_over_what_period() {
        let (head, activity, access, conformity) = full();
        let text = render(&head, &activity, &access, &conformity);
        assert!(text.contains("Pharmacie des Lilas"));
        assert!(text.contains("/srv/officine/bpm_caddy.db"));
        assert!(text.contains("du 21/08/2026 au 19/09/2026"));
        assert!(text.contains("30 jour(s)"));
        // Les trois sections sont là, dans l'ordre.
        let one = text.find("1. Activité").expect("activité");
        let two = text.find("2. Accès aux dossiers").expect("accès");
        let three = text.find("3. Conformité").expect("conformité");
        assert!(one < two && two < three);
        // Et la réserve, **avant** le contenu.
        let scope = text.find("désigné par son numéro").expect("la réserve");
        assert!(scope < one, "la réserve se lit avant ce qu'elle qualifie");
    }

    /// Une officine qui n'a pas encore écrit son nom donne un titre qui
    /// s'arrête, et non un titre qui pend sur un tiret — lequel se lit
    /// comme une panne du programme.
    #[test]
    fn a_report_with_no_officine_name_does_not_end_on_a_dash() {
        let (mut head, activity, access, conformity) = full();
        head.officine = "   ".to_owned();
        let text = render(&head, &activity, &access, &conformity);
        let title = text.lines().next().expect("un titre");
        assert_eq!(title, "Rapport d'audit");
        assert!(!title.ends_with('—'));
    }

    /// **L'écart est annoncé sur les soirs où il est calculé**, et non
    /// sur ceux qui ont été comptés : ce sont deux nombres, et les
    /// confondre fait croire l'écart plus petit qu'il n'est.
    #[test]
    fn a_cumulative_gap_says_over_how_many_evenings_it_was_computed() {
        let (head, activity, access, conformity) = full();
        let text = render(&head, &activity, &access, &conformity);
        assert!(text.contains("sur 22 soir(s)"), "{text}");
        assert!(!text.contains("sur 24 soir(s)"));
        // Le nombre de soirs comptés est dit ailleurs, et il n'a pas
        // disparu.
        assert!(text.contains("Caisses comptées"));
        assert!(text.contains("24"));
        // Et l'unité : un montant sans monnaie est un nombre.
        assert!(text.contains("19,75 €"), "{text}");
    }

    /// Sans recette attendue il n'y a **pas** d'écart : la règle de la
    /// caisse, telle quelle. Annoncer zéro serait annoncer que tout
    /// tombe juste.
    #[test]
    fn with_no_expected_takings_there_is_no_gap() {
        let (head, mut activity, access, conformity) = full();
        activity.till_expected_evenings = 0;
        activity.till_gap_cents = None;
        let text = render(&head, &activity, &access, &conformity);
        assert!(text.contains("il n'y a pas d'écart à dire"), "{text}");
        assert!(!text.contains("Écart cumulé"));
    }

    /// Aucune section n'est laissée vide : un blanc se lit comme une
    /// panne, ce qui est l'exact contraire de ce qu'un rapport d'audit
    /// doit produire.
    #[test]
    fn a_section_that_finds_nothing_writes_it_in_words() {
        let head = Head {
            officine: "Pharmacie des Lilas".to_owned(),
            days: 30,
            ..Head::default()
        };
        let text = render(
            &head,
            &Activity::default(),
            &Summary::default(),
            &Conformity::default(),
        );
        assert_eq!(text.matches("Rien sur la période.").count(), 2);
        assert!(text.contains("Rien à signaler."));
        // Et aucune section n'est suivie de rien : après chaque titre
        // il y a au moins une ligne qui parle.
        for title in ["1. Activité", "2. Accès aux dossiers", "3. Conformité"] {
            let after: Vec<&str> = text
                .lines()
                .skip_while(|l| !l.starts_with(title))
                .skip(2)
                .take(1)
                .collect();
            assert!(
                after.first().is_some_and(|l| !l.trim().is_empty()),
                "{title} ne dit rien"
            );
        }
    }

    /// Un libellé plus long que la colonne garde son espace.
    ///
    /// « Lignes portées au registre des stupéfiants » fait plus de
    /// quarante caractères, et son chiffre venait se coller contre lui :
    /// « …des stupéfiants5 » se lit comme une quantité.
    #[test]
    fn a_label_longer_than_its_column_keeps_its_space() {
        let (head, activity, access, conformity) = full();
        let text = render(&head, &activity, &access, &conformity);
        let line = text
            .lines()
            .find(|l| l.starts_with("Lignes portées"))
            .expect("la ligne du registre");
        assert!(line.ends_with(" 5"), "« {line} »");
        // Et une colonne ordinaire garde ses points de conduite.
        let short = text
            .lines()
            .find(|l| l.trim_start().starts_with("BPM"))
            .unwrap();
        assert!(short.contains("...."), "« {short} »");
        assert!(short.ends_with(" 4"));
    }

    /// Un rapport ne nomme personne d'autre qu'un opérateur : ce qu'on
    /// lui donne à écrire ne porte aucun patient. La règle est tenue
    /// par le texte du module, comme celle d'une ligne d'accès.
    #[test]
    fn a_report_is_given_nothing_that_names_a_patient() {
        let text = include_str!("audit.rs");
        for shape in [
            "pub struct Activity {",
            "pub struct Conformity {",
            "pub struct Head {",
        ] {
            let fields: String = text
                .split(shape)
                .nth(1)
                .and_then(|t| t.split("\n}").next())
                .expect(shape)
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            for named in ["patient", "name", "nom", "nir", "birth"] {
                assert!(!fields.contains(named), "{shape} porte « {named} »");
            }
        }
    }

    /// Les clés sont écrites dans la base : stables pour toujours, et
    /// chacune la sienne.
    #[test]
    fn an_acts_key_is_stable_and_its_own() {
        let mut seen = std::collections::HashSet::new();
        for act in Act::ALL {
            assert!(seen.insert(act.key()), "{act:?} partage sa clé");
            assert_eq!(Act::from_key(act.key()), Some(act));
            assert!(act.key().chars().all(|c| c.is_ascii_lowercase()));
            assert!(!act.label().is_empty());
        }
        assert_eq!(Act::ALL.map(Act::key), ["ouvert", "exporte", "purge"]);
        assert_eq!(Act::from_key("ce-que-fera-la-suite"), None);
    }
}
