//! La conciliation médicamenteuse : ce qu'une ordonnance de sortie a
//! changé à celle que le dossier porte.
//!
//! Le patient revient de l'hôpital avec une feuille. La question du
//! comptoir n'est pas « que dit cette feuille », c'est « qu'est-ce qui a
//! changé » — et surtout « qu'est-ce qui a disparu », parce que ce qui a
//! disparu est ce que le patient continuera de prendre s'il lui reste
//! une boîte et que personne ne le lui dit.
//!
//! Ce module lit la feuille telle qu'elle est tapée ou collée, rapproche
//! chaque ligne d'une fiche de la base, et dit de chaque traitement s'il
//! est reconduit, arrêté, ajouté, changé de dose, ou remplacé par une
//! molécule de la même classe. Il ne décide rien : la conciliation est
//! un acte du pharmacien, et la ligne qu'il n'a pas su rapprocher est
//! affichée telle quelle plutôt qu'écartée en silence.
//!
//! Pur et testé, comme la biologie et la revue. Aucune base ici : la
//! base est passée en argument.

/// Ce que le rapprochement d'une ligne demande d'une fiche.
pub struct Known<'a> {
    pub name: &'a str,
    pub dci: &'a str,
    pub class: &'a str,
}

/// Un traitement du dossier : la fiche, et la posologie que l'équipe a
/// notée pour ce patient (souvent vide, et c'est un état normal).
pub struct Held<'a> {
    pub name: &'a str,
    pub dci: &'a str,
    pub class: &'a str,
    pub posology: &'a str,
}

/// Ce que la comparaison dit d'un traitement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Change {
    /// La ligne n'a été rapprochée d'aucune fiche : personne ne l'a
    /// vérifiée, et c'est la seule catégorie qui demande un geste avant
    /// toutes les autres.
    Unmatched,
    /// Une molécule de la classe d'un traitement arrêté prend sa place.
    /// C'est la divergence dont le patient repart avec les deux boîtes.
    Switched,
    /// Sur le dossier, absent de la nouvelle liste.
    Stopped,
    /// Sur les deux, avec deux posologies différentes.
    DoseChanged,
    /// Sur la nouvelle liste seulement.
    Added,
    /// Sur les deux, sans changement lisible.
    Kept,
}

impl Change {
    /// L'ordre d'affichage : ce qui demande une décision d'abord, ce qui
    /// n'a pas bougé en dernier.
    fn rank(self) -> u8 {
        match self {
            Change::Unmatched => 0,
            Change::Switched => 1,
            Change::Stopped => 2,
            Change::DoseChanged => 3,
            Change::Added => 4,
            Change::Kept => 5,
        }
    }
}

/// Une ligne du tableau de conciliation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Divergence {
    pub kind: Change,
    /// Le produit, tel qu'il s'affiche : le nom de la fiche quand elle a
    /// été trouvée, la ligne telle qu'elle a été tapée sinon. Pour un
    /// remplacement, c'est le produit qui *arrive*.
    pub label: String,
    /// Le produit qui part, pour un remplacement et pour lui seul.
    ///
    /// Deux champs et non une phrase toute faite : la façon de dire
    /// « X remplacé par Y » est du texte d'interface, il vit dans
    /// `strings.fr.toml` avec le reste, et un module pur n'a pas à
    /// choisir la flèche.
    pub replaces: String,
    /// Ce que le dossier portait — vide pour un ajout.
    pub before: String,
    /// Ce que la nouvelle liste porte — vide pour un arrêt.
    pub after: String,
    /// La raison, quand elle se dit en quelques mots : la classe pour un
    /// remplacement, la ligne d'origine pour une ligne non rapprochée.
    pub note: String,
}

/// Le décompte, pour la ligne de résumé et pour le journal.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Counts {
    pub unmatched: usize,
    pub switched: usize,
    pub stopped: usize,
    pub dose_changed: usize,
    pub added: usize,
    pub kept: usize,
}

impl Counts {
    /// Tout ce qui n'est pas une reconduction à l'identique.
    pub fn divergences(self) -> usize {
        self.unmatched + self.switched + self.stopped + self.dose_changed + self.added
    }
}

pub fn counts(list: &[Divergence]) -> Counts {
    let mut c = Counts::default();
    for d in list {
        match d.kind {
            Change::Unmatched => c.unmatched += 1,
            Change::Switched => c.switched += 1,
            Change::Stopped => c.stopped += 1,
            Change::DoseChanged => c.dose_changed += 1,
            Change::Added => c.added += 1,
            Change::Kept => c.kept += 1,
        }
    }
    c
}

/// Séparer une ligne d'ordonnance en « le produit » et « le reste ».
///
/// Les feuilles de sortie sont écrites de vingt façons : « KARDEGIC
/// 75 mg — 1 sachet le matin », « - Ramipril 5 mg : 1 cp le soir »,
/// « 3) Lévothyrox 75 µg à jeun ». Trois règles suffisent et aucune
/// n'invente : on retire la puce ou le numéro de tête, on coupe au
/// deux-points s'il en vient un avant le premier chiffre, sinon au
/// premier mot qui commence par un chiffre — c'est le dosage, et le nom
/// est ce qui le précède.
///
/// Le nom est plafonné à quatre mots : une ligne de prose entière n'est
/// pas un nom de produit, et la laisser passer ferait chercher la base
/// sur une phrase.
pub fn split_line(raw: &str) -> (&str, &str) {
    let line = raw.trim();
    // La puce de tête, sous toutes ses formes.
    let line = line
        .trim_start_matches(['-', '–', '—', '*', '•', '·', '>', '+'])
        .trim_start();
    // Un numéro de tête : « 3) », « 3. », « 3 - ». Retiré seulement s'il
    // est suivi d'un séparateur, faute de quoi « 5 mg » y passerait.
    let line = {
        let digits = line
            .char_indices()
            .take_while(|(_, c)| c.is_ascii_digit())
            .count();
        let rest = &line[digits..];
        if digits > 0 && rest.starts_with([')', '.', '-', '/']) {
            rest[1..].trim_start()
        } else {
            line
        }
    };
    let cut = |at: usize| {
        let (name, detail) = line.split_at(at);
        (
            name.trim_end_matches([':', '-', '–', '—', ' ']).trim(),
            {
                let d = detail.trim_start();
                d.trim_start_matches([':', '-', '–', '—']).trim()
            },
        )
    };
    // Le deux-points, s'il vient avant le premier chiffre.
    let colon = line.find(':');
    let digit = line.find(|c: char| c.is_ascii_digit());
    if let Some(i) = colon {
        if digit.is_none_or(|d| i < d) {
            return cut(i);
        }
    }
    // Sinon le premier mot qui commence par un chiffre.
    let mut words = 0;
    for (i, word) in word_starts(line) {
        if word.starts_with(|c: char| c.is_ascii_digit()) && i > 0 {
            return cut(i);
        }
        words += 1;
        if words == 4 {
            // Quatre mots et toujours pas de dosage : le nom s'arrête
            // là, le reste est de la prose.
            let end = i + word.len();
            if end < line.len() {
                return cut(end);
            }
        }
    }
    (line, "")
}

/// Les mots d'une ligne, avec l'octet où chacun commence.
fn word_starts(line: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in line.char_indices() {
        if c.is_whitespace() {
            if let Some(s) = start.take() {
                out.push((s, &line[s..i]));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push((s, &line[s..]));
    }
    out
}

/// La fiche que cette ligne désigne, si l'une d'elles la désigne.
///
/// Le nom de marque d'abord, la DCI ensuite, et jamais sur moins de
/// trois lettres : « AS » rapproché de tout n'aide personne. Un score et
/// non un premier trouvé, parce que « Kardegic » doit gagner contre
/// « Kardegic 75 » aussi bien que l'inverse.
pub fn match_name(name: &str, base: &[Known]) -> Option<usize> {
    let needle = crate::fuzzy::sort_key(name.trim());
    if needle.chars().filter(|c| c.is_alphanumeric()).count() < 3 {
        return None;
    }
    let mut best: Option<(i32, usize)> = None;
    for (i, k) in base.iter().enumerate() {
        let mut score = 0;
        for (field, weight) in [(k.name, 100), (k.dci, 90)] {
            if field.trim().is_empty() {
                continue;
            }
            let hay = crate::fuzzy::sort_key(field.trim());
            let s = if hay == needle {
                weight
            } else if hay.starts_with(&needle) || needle.starts_with(&hay) {
                // Le plus court des deux décide de la qualité : « ramipril »
                // contre « ramipril hydrochlorothiazide » est un moins bon
                // rapprochement que « ramipril » contre « ramipril ».
                weight - 20 + (hay.len().min(needle.len()) as i32).min(15)
            } else if needle.len() >= 5 && hay.contains(&needle) {
                weight - 50
            } else {
                0
            };
            score = score.max(s);
        }
        if score > 0 {
            // À égalité, la première fiche de la liste : l'ordre de la
            // base est stable, donc la réponse l'est aussi.
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, i));
            }
        }
    }
    best.map(|(_, i)| i)
}

/// Comparer l'ordonnance du dossier à une liste tapée ou collée.
///
/// `base` sert au rapprochement des lignes tapées : c'est elle qui dit
/// que « Doliprane » et « paracétamol » sont le même traitement, et
/// c'est d'elle que vient la classe sur laquelle un remplacement se
/// reconnaît. Rien n'est écrit nulle part : la fonction rend un tableau.
pub fn compare(held: &[Held], list: &str, base: &[Known]) -> Vec<Divergence> {
    // La nouvelle liste, lue ligne par ligne.
    struct Entry {
        key: String,
        label: String,
        class: String,
        detail: String,
        matched: bool,
        raw: String,
    }
    // Le dossier d'abord, la base ensuite. Une ligne qui nomme quelque
    // chose que le patient prend déjà désigne *ce* traitement-là, et pas
    // un homonyme trouvé ailleurs dans les huit cents fiches : deux
    // spécialités portent le même nom plus souvent qu'on ne croit, et le
    // rapprochement raté transforme une reconduction en un arrêt suivi
    // d'un ajout — la pire des deux erreurs, puisqu'elle invente une
    // divergence là où il n'y en a pas.
    let from_file: Vec<Known> = held
        .iter()
        .map(|h| Known {
            name: h.name,
            dci: h.dci,
            class: h.class,
        })
        .collect();
    let mut entries: Vec<Entry> = Vec::new();
    for raw in list.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (name, detail) = split_line(raw);
        if name.is_empty() {
            continue;
        }
        let found = match_name(name, &from_file)
            .map(|i| &from_file[i])
            .or_else(|| match_name(name, base).map(|i| &base[i]));
        match found {
            Some(k) => {
                entries.push(Entry {
                    key: identity(k.name, k.dci),
                    label: k.name.to_owned(),
                    class: k.class.trim().to_owned(),
                    detail: detail.to_owned(),
                    matched: true,
                    raw: raw.to_owned(),
                });
            }
            None => entries.push(Entry {
                key: crate::fuzzy::sort_key(name),
                label: name.to_owned(),
                class: String::new(),
                detail: detail.to_owned(),
                matched: false,
                raw: raw.to_owned(),
            }),
        }
    }
    // Une même molécule écrite deux fois sur la feuille (le générique et
    // la marque, la forme du matin et celle du soir) est un traitement,
    // pas deux : les détails se rejoignent sur une ligne.
    let mut merged: Vec<Entry> = Vec::new();
    for e in entries {
        match merged.iter_mut().find(|m| m.key == e.key) {
            Some(m) => {
                if !e.detail.trim().is_empty() {
                    if m.detail.trim().is_empty() {
                        m.detail = e.detail;
                    } else if !m.detail.contains(e.detail.trim()) {
                        m.detail = format!("{} ; {}", m.detail, e.detail.trim());
                    }
                }
            }
            None => merged.push(e),
        }
    }
    let entries = merged;

    let mut out: Vec<Divergence> = Vec::new();
    let mut taken = vec![false; entries.len()];
    for h in held {
        let key = identity(h.name, h.dci);
        let hit = entries
            .iter()
            .enumerate()
            .find(|(i, e)| e.key == key && !taken[*i])
            .map(|(i, _)| i);
        match hit {
            Some(i) => {
                taken[i] = true;
                let e = &entries[i];
                let kind = if same_dose(h.posology, &e.detail) {
                    Change::Kept
                } else {
                    Change::DoseChanged
                };
                out.push(Divergence {
                    kind,
                    label: h.name.to_owned(),
                    replaces: String::new(),
                    before: h.posology.trim().to_owned(),
                    after: e.detail.clone(),
                    note: String::new(),
                });
            }
            None => out.push(Divergence {
                kind: Change::Stopped,
                label: h.name.to_owned(),
                replaces: String::new(),
                before: h.posology.trim().to_owned(),
                after: String::new(),
                note: h.class.trim().to_owned(),
            }),
        }
    }
    for (i, e) in entries.iter().enumerate() {
        if taken[i] {
            continue;
        }
        out.push(Divergence {
            kind: if e.matched {
                Change::Added
            } else {
                Change::Unmatched
            },
            label: e.label.clone(),
            replaces: String::new(),
            before: String::new(),
            after: e.detail.clone(),
            note: if e.matched {
                e.class.clone()
            } else {
                e.raw.clone()
            },
        });
    }
    fold_switches(&mut out);
    // Stable, donc à catégorie égale l'ordre de lecture est conservé :
    // le dossier d'abord, la feuille ensuite.
    out.sort_by_key(|d| d.kind.rank());
    out
}

/// Un arrêt et un ajout de la même classe sont un remplacement, et se
/// disent sur une ligne. C'est la divergence qui compte le plus : le
/// patient qui garde la boîte de l'un et commence l'autre prend deux
/// fois la même chose.
fn fold_switches(list: &mut Vec<Divergence>) {
    loop {
        let pair = list
            .iter()
            .enumerate()
            .filter(|(_, d)| d.kind == Change::Stopped && !d.note.trim().is_empty())
            .find_map(|(si, s)| {
                let key = crate::fuzzy::sort_key(s.note.trim());
                list.iter()
                    .enumerate()
                    .find(|(_, a)| {
                        a.kind == Change::Added
                            && crate::fuzzy::sort_key(a.note.trim()) == key
                            && !a.note.trim().is_empty()
                    })
                    .map(|(ai, _)| (si, ai))
            });
        let Some((si, ai)) = pair else { return };
        let added = list[ai].clone();
        let stopped = list[si].clone();
        // L'ajout part, l'arrêt devient le remplacement : retirer le
        // plus grand indice d'abord garde l'autre valide.
        let (first, second) = if si > ai { (si, ai) } else { (ai, si) };
        list.remove(first);
        list.remove(second);
        list.push(Divergence {
            kind: Change::Switched,
            label: added.label,
            replaces: stopped.label,
            before: stopped.before,
            after: added.after,
            note: stopped.note,
        });
    }
}

/// L'identité d'un traitement : sa DCI si la fiche en porte une, son nom
/// sinon. C'est ce qui fait que « Doliprane » sur le dossier et
/// « paracétamol » sur la feuille sont le même traitement.
fn identity(name: &str, dci: &str) -> String {
    let dci = dci.trim();
    if dci.is_empty() {
        crate::fuzzy::sort_key(name.trim())
    } else {
        crate::fuzzy::sort_key(dci)
    }
}

/// Deux posologies disent-elles la même chose ?
///
/// Un côté muet ne contredit rien : un dossier qui ne note pas la
/// posologie n'est pas un dossier qui la conteste, et l'annoncer comme
/// un changement de dose remplirait le tableau de faux.
///
/// Et l'une des deux en dit souvent plus que l'autre sans rien dire
/// d'autre : le dossier note « 1 sachet le matin », la feuille de
/// l'hôpital écrit « 75 mg : 1 sachet le matin ». La règle est donc :
/// la plus courte est un **suffixe** de la plus longue, mot à mot. Elle
/// laisse passer le dosage rappelé en tête et attrape le reste — « 5 mg
/// le soir » contre « 10 mg le soir » n'est le suffixe de personne, et
/// « 1 cp le matin » contre « 1 cp le matin et le soir » non plus.
fn same_dose(before: &str, after: &str) -> bool {
    let words = |s: &str| -> Vec<String> {
        crate::fuzzy::sort_key(s.trim())
            .replace(',', ".")
            .split_whitespace()
            .map(|w| w.trim_matches([':', ';', '.', '-', '(', ')']).to_owned())
            .filter(|w| !w.is_empty())
            .collect()
    };
    let a = words(before);
    let b = words(after);
    if a.is_empty() || b.is_empty() {
        return true;
    }
    let (short, long) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    long.ends_with(&short[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Vec<Known<'static>> {
        vec![
            Known {
                name: "Kardegic 75",
                dci: "acétylsalicylate de lysine",
                class: "antiagrégant",
            },
            Known {
                name: "Triatec",
                dci: "ramipril",
                class: "IEC",
            },
            Known {
                name: "Coversyl",
                dci: "périndopril",
                class: "IEC",
            },
            Known {
                name: "Lévothyrox",
                dci: "lévothyroxine",
                class: "hormone thyroïdienne",
            },
            Known {
                name: "Doliprane",
                dci: "paracétamol",
                class: "antalgique",
            },
            Known {
                name: "Eliquis",
                dci: "apixaban",
                class: "AOD",
            },
        ]
    }

    #[test]
    fn a_line_gives_up_its_product_and_its_dose() {
        // Les cinq façons dont une feuille de sortie est écrite.
        assert_eq!(
            split_line("KARDEGIC 75 mg — 1 sachet le matin"),
            ("KARDEGIC", "75 mg — 1 sachet le matin")
        );
        assert_eq!(
            split_line("- Ramipril 5 mg : 1 cp le soir"),
            ("Ramipril", "5 mg : 1 cp le soir")
        );
        assert_eq!(
            split_line("3) Lévothyrox 75 µg à jeun"),
            ("Lévothyrox", "75 µg à jeun")
        );
        // Le deux-points avant tout chiffre : c'est lui qui coupe.
        assert_eq!(
            split_line("Doliprane : 1 g si douleur"),
            ("Doliprane", "1 g si douleur")
        );
        // Une ligne sans dosage est un nom entier.
        assert_eq!(split_line("Eliquis"), ("Eliquis", ""));
        // Un nom en plusieurs mots survit tant qu'aucun chiffre ne vient.
        assert_eq!(
            split_line("Acide folique 5 mg 1 cp par semaine"),
            ("Acide folique", "5 mg 1 cp par semaine")
        );
        // Et la prose est plafonnée : quatre mots font un nom, pas plus.
        let (name, _) = split_line("le patient prend aussi de la vitamine D tous les mois");
        assert_eq!(name.split_whitespace().count(), 4);
    }

    #[test]
    fn a_line_finds_its_fiche_by_brand_or_by_molecule() {
        let base = base();
        let by = |n: &str| match_name(n, &base).map(|i| base[i].name);
        assert_eq!(by("Kardegic"), Some("Kardegic 75"));
        assert_eq!(by("KARDEGIC 75"), Some("Kardegic 75"));
        // Par la DCI, que la feuille de l'hôpital écrit plus volontiers
        // que la marque.
        assert_eq!(by("ramipril"), Some("Triatec"));
        assert_eq!(by("Paracétamol"), Some("Doliprane"));
        // Les accents ne comptent pas.
        assert_eq!(by("levothyroxine"), Some("Lévothyrox"));
        // Rien d'inventé : deux lettres ne rapprochent de rien, et un
        // produit que la base ne connaît pas reste inconnu.
        assert_eq!(by("AS"), None);
        assert_eq!(by("Vogalène"), None);
    }

    #[test]
    fn the_comparison_names_what_changed_and_what_did_not() {
        let base = base();
        let held = vec![
            Held {
                name: "Kardegic 75",
                dci: "acétylsalicylate de lysine",
                class: "antiagrégant",
                posology: "1 sachet le matin",
            },
            Held {
                name: "Triatec",
                dci: "ramipril",
                class: "IEC",
                posology: "5 mg le soir",
            },
            Held {
                name: "Lévothyrox",
                dci: "lévothyroxine",
                class: "hormone thyroïdienne",
                posology: "75 µg à jeun",
            },
        ];
        let list = "\
Kardegic 75 mg : 1 sachet le matin
Triatec 10 mg : 1 cp le soir
Eliquis 5 mg x2/j
Vogalène lyoc si nausées
";
        let out = compare(&held, list, &base);
        let find = |label: &str| {
            out.iter()
                .find(|d| d.label.contains(label))
                .unwrap_or_else(|| panic!("{label} absent du tableau"))
        };
        // Reconduit à l'identique.
        assert_eq!(find("Kardegic").kind, Change::Kept);
        // Même molécule, dose différente : c'est la divergence la plus
        // fréquente et la plus silencieuse.
        assert_eq!(find("Triatec").kind, Change::DoseChanged);
        assert_eq!(find("Triatec").before, "5 mg le soir");
        assert_eq!(find("Triatec").after, "10 mg : 1 cp le soir");
        // Sur le dossier, absent de la feuille.
        assert_eq!(find("Lévothyrox").kind, Change::Stopped);
        // Sur la feuille, absent du dossier.
        assert_eq!(find("Eliquis").kind, Change::Added);
        // La base ne connaît pas ce produit : la ligne est montrée telle
        // qu'elle a été tapée, jamais écartée.
        let unknown = find("Vogalène");
        assert_eq!(unknown.kind, Change::Unmatched);
        assert_eq!(unknown.note, "Vogalène lyoc si nausées");

        // Le tableau est trié : ce qui demande une décision en tête.
        let ranks: Vec<u8> = out.iter().map(|d| d.kind.rank()).collect();
        let mut sorted = ranks.clone();
        sorted.sort_unstable();
        assert_eq!(ranks, sorted);

        let c = counts(&out);
        assert_eq!(c.kept, 1);
        assert_eq!(c.dose_changed, 1);
        assert_eq!(c.stopped, 1);
        assert_eq!(c.added, 1);
        assert_eq!(c.unmatched, 1);
        assert_eq!(c.divergences(), 4);
    }

    #[test]
    fn one_molecule_of_a_class_replacing_another_is_one_line_not_two() {
        let base = base();
        let held = vec![Held {
            name: "Triatec",
            dci: "ramipril",
            class: "IEC",
            posology: "5 mg le soir",
        }];
        let out = compare(&held, "Coversyl 5 mg le matin", &base);
        assert_eq!(out.len(), 1, "un remplacement n'est pas deux lignes");
        assert_eq!(out[0].kind, Change::Switched);
        assert_eq!(out[0].label, "Coversyl");
        assert_eq!(out[0].replaces, "Triatec");
        assert_eq!(out[0].before, "5 mg le soir");
        assert_eq!(out[0].after, "5 mg le matin");
        assert_eq!(out[0].note, "IEC");
    }

    #[test]
    fn the_brand_on_the_file_and_the_molecule_on_the_sheet_are_one_treatment() {
        let base = base();
        let held = vec![Held {
            name: "Doliprane",
            dci: "paracétamol",
            class: "antalgique",
            posology: "",
        }];
        // La DCI rapproche, et le dossier muet sur la posologie ne
        // conteste pas celle de la feuille.
        let out = compare(&held, "paracétamol 1 g si douleur", &base);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, Change::Kept);
        assert_eq!(out[0].after, "1 g si douleur");
    }

    #[test]
    fn the_same_molecule_written_twice_on_the_sheet_is_one_line() {
        let base = base();
        let held = vec![];
        // Le matin et le soir sur deux lignes, la marque puis la DCI :
        // c'est un traitement.
        let out = compare(
            &held,
            "Doliprane 1 g le matin\nparacétamol 1 g le soir",
            &base,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, Change::Added);
        assert!(out[0].after.contains("le matin") && out[0].after.contains("le soir"));
    }

    #[test]
    fn an_empty_sheet_stops_everything_and_an_empty_file_adds_everything() {
        let base = base();
        let held = vec![Held {
            name: "Triatec",
            dci: "ramipril",
            class: "IEC",
            posology: "5 mg",
        }];
        // Rien de collé : le tableau ne prétend pas que tout est arrêté
        // par accident — il le dit, et le compte le montre.
        let out = compare(&held, "", &base);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, Change::Stopped);
        // Les lignes vides et les blancs ne comptent pour rien.
        assert_eq!(compare(&held, "\n\n   \n", &base), out);
        // Dossier vide : tout est un ajout.
        let out = compare(&[], "Triatec 5 mg", &base);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, Change::Added);
    }

    #[test]
    fn a_line_naming_something_the_file_already_carries_is_that_treatment() {
        // Deux fiches du même nom : celle que le patient prend, sans
        // DCI, et une homonyme de la base qui en porte une. Rapprochée
        // de la seconde, la ligne aurait une autre identité que celle du
        // dossier, et une reconduction serait affichée comme un arrêt
        // suivi d'un ajout — une divergence inventée.
        let base = vec![
            Known {
                name: "Triatec",
                dci: "ramipril",
                class: "IEC",
            },
            Known {
                name: "Doliprane",
                dci: "paracétamol",
                class: "antalgique",
            },
        ];
        let held = vec![Held {
            name: "Triatec",
            dci: "",
            class: "",
            posology: "",
        }];
        let out = compare(&held, "Triatec 10 mg : 1 cp le soir", &base);
        assert_eq!(out.len(), 1, "un traitement, pas un arrêt et un ajout");
        assert_eq!(out[0].kind, Change::Kept);
        assert_eq!(out[0].after, "10 mg : 1 cp le soir");
        // Ce qui n'est pas au dossier passe par la base, comme avant.
        let out = compare(&held, "paracétamol 1 g", &base);
        assert!(out
            .iter()
            .any(|d| d.label == "Doliprane" && d.kind == Change::Added));
    }

    #[test]
    fn the_same_answer_twice_running() {
        // Le tableau s'affiche à chaque image tant que la vue est
        // ouverte : deux appels identiques doivent rendre exactement la
        // même chose, dans le même ordre.
        let base = base();
        let held = vec![
            Held {
                name: "Triatec",
                dci: "ramipril",
                class: "IEC",
                posology: "5 mg",
            },
            Held {
                name: "Kardegic 75",
                dci: "acétylsalicylate de lysine",
                class: "antiagrégant",
                posology: "",
            },
        ];
        let list = "Coversyl 5 mg\nEliquis 5 mg x2\nMachin 3 gouttes\nKardegic 75 mg";
        let first = compare(&held, list, &base);
        for _ in 0..20 {
            assert_eq!(compare(&held, list, &base), first);
        }
    }
}
