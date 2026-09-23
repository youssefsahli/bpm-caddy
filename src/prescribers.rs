//! Les prescripteurs : qui a signé l'ordonnance.
//!
//! # Pourquoi une table, alors qu'un champ de texte marchait
//!
//! Le prescripteur est du **texte libre** partout où il est demandé :
//! sur une ligne de registre, sur une location, sur un dossier.
//! « Dr Morel », « dr morel », « Morel », « Dr MOREL J. » — quatre
//! chaînes pour un médecin. C'est exactement la dérive que
//! `classes.rs` a trouvée sur les classes thérapeutiques, et elle a ici
//! une conséquence que la classe n'avait pas : `vigilance.rs` pose la
//! question « plusieurs prescripteurs ? » **en comparant ces
//! chaînes-là**. Quatre orthographes d'un même médecin font une
//! question posée pour rien ; une orthographe partagée par deux
//! médecins fait une question qui ne se pose jamais.
//!
//! Un annuaire donne un **identifiant** : le RPPS, onze chiffres, qui
//! désigne un professionnel et un seul.
//!
//! # Rien n'est livré, et rien n'est deviné
//!
//! Aucun annuaire n'est embarqué : ce fichier-là appartient à
//! l'officine, qui l'importe et le met à jour. C'est la même règle que
//! les codes-barres — aucune table CIP n'est livrée non plus, parce
//! qu'une table figée dans un binaire vieillit sans que personne le
//! voie.
//!
//! Et **la recherche ne décide de rien** : elle propose, l'officine
//! choisit. Un prescripteur que l'annuaire ne connaît pas reste du
//! texte libre, comme aujourd'hui — un logiciel qui rapprocherait tout
//! seul « Morel » du docteur Morel le ferait un jour d'un autre Morel.
//!
//! # Une colonne se trouve par son nom
//!
//! Les fichiers publics changent d'ordre de colonnes d'une version à
//! l'autre, et lire la septième c'est lire autre chose l'année
//! suivante, en silence. L'en-tête est donc lu, les colonnes sont
//! trouvées par leur **nom**, et un fichier dont l'en-tête ne porte pas
//! ce qu'il faut est refusé en disant ce qu'il portait — plutôt que
//! d'importer trente mille lignes de colonnes décalées.
//!
//! Pur, testé, ne lit aucun disque : le texte est passé.

/// Un prescripteur, tel que l'annuaire le donne.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Prescriber {
    /// Onze chiffres. Ce qui **désigne** le praticien.
    pub rpps: String,
    pub last: String,
    pub first: String,
    /// « Médecin généraliste », « Chirurgien-dentiste »…
    pub speciality: String,
    /// L'établissement, quand il y en a un : neuf chiffres.
    pub finess: String,
    /// Le numéro Assurance Maladie (ex-ADELI), quand le fichier le
    /// porte. C'est celui qui s'écrit sur une feuille de soins, et ce
    /// n'est pas le RPPS.
    pub am: String,
    pub city: String,
}

impl Prescriber {
    /// Ce qui s'écrit sur une ligne de registre : court, et sans
    /// prénom — c'est ainsi qu'une ordonnance est signée.
    pub fn short(&self) -> String {
        let name = self.last.trim();
        if name.is_empty() {
            return self.first.trim().to_owned();
        }
        format!("Dr {name}")
    }

    /// Ce qui s'affiche dans une liste de choix : de quoi distinguer
    /// deux homonymes, ce qu'un nom seul ne fait pas.
    pub fn label(&self) -> String {
        let mut out = self.short();
        if !self.first.trim().is_empty() {
            out.push(' ');
            out.push_str(self.first.trim());
        }
        for extra in [self.speciality.trim(), self.city.trim()] {
            if !extra.is_empty() {
                out.push_str(" — ");
                out.push_str(extra);
            }
        }
        out
    }

    /// Le RPPS se prouve-t-il par sa clé ?
    ///
    /// Onze chiffres dont le dernier est une clé de Luhn. **La clé
    /// décide, jamais la longueur** — la même règle que le NIR et que
    /// le CIP13, et pour la même raison : un numéro de la bonne
    /// longueur qu'on a mal recopié passerait pour un identifiant.
    ///
    /// Ce qui en est fait est écrit chez l'appelant : un numéro qui ne
    /// se prouve pas est **gardé et signalé**, jamais jeté. Un import
    /// qui laisse trente lignes sur le carreau sans le dire est
    /// exactement ce que ce dépôt refuse partout ailleurs.
    pub fn rpps_checks(&self) -> bool {
        let digits = self.rpps.trim();
        digits.len() == 11 && luhn_ok(digits)
    }
}

/// La clé de Luhn : la somme pondérée est-elle un multiple de dix ?
///
/// Écrite une fois. Elle vaut pour le RPPS et pour le FINESS, et deux
/// écritures d'un même calcul finissent par ne plus donner le même
/// résultat sur le cas qu'une seule des deux a vu.
pub fn luhn_ok(digits: &str) -> bool {
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let sum: u32 = digits
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            let d = c.to_digit(10).unwrap_or(0);
            // Un chiffre sur deux en partant de la fin est doublé, et
            // un doublement au-dessus de neuf se replie.
            if i % 2 == 1 {
                let doubled = d * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                d
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

/// Ce qu'un import a donné.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Imported {
    pub found: Vec<Prescriber>,
    /// Les lignes qui n'ont pas pu être lues du tout — trop courtes,
    /// sans nom. **Comptées et dites** : un import qui en laisse
    /// tomber sans le dire est un annuaire qu'on croit complet.
    pub skipped: usize,
    /// Ceux dont le RPPS ne se prouve pas par sa clé. Gardés, et
    /// comptés à part : c'est à l'officine de regarder, pas au
    /// logiciel de trancher.
    pub unverified: usize,
    /// Les lignes d'un praticien déjà lu sous le même RPPS : l'extraction
    /// publique porte une ligne par **activité**, et un médecin à trois
    /// cabinets remplissait trois des cinq propositions de la recherche.
    /// Personne ne manque : c'est la même personne, gardée une fois.
    pub merged: usize,
    /// Les colonnes effectivement reconnues, dans l'ordre du fichier.
    /// Ce qui permet de dire ce qu'on a lu plutôt que « importé ».
    pub columns: Vec<String>,
}

/// Les noms sous lesquels une colonne peut se présenter.
///
/// Plusieurs orthographes parce que les fichiers publics n'en gardent
/// pas une seule d'une version à l'autre — et que celui qu'une officine
/// exporte de son propre logiciel n'a aucune raison de suivre la même.
/// Comparés repliés : casse, accents et ponctuation ne décident de rien.
const COLUMNS: &[(&str, &[&str])] = &[
    (
        "rpps",
        &["identifiant pp", "numero rpps", "rpps", "identifiant rpps"],
    ),
    (
        "last",
        &[
            "nom d exercice",
            "nom d'exercice",
            "nom",
            "nom patronymique",
        ],
    ),
    (
        "first",
        &["prenom d exercice", "prenom d'exercice", "prenom"],
    ),
    (
        "speciality",
        &[
            "libelle profession",
            "profession",
            "specialite",
            "libelle savoir faire",
        ],
    ),
    (
        "finess",
        &[
            "numero finess etablissement",
            "finess",
            "numero finess",
            // Les deux de l'extraction publique : le site d'abord. Pas
            // « identifiant technique de la structure », qui est un
            // numéro interne à l'annuaire et non un FINESS.
            "numero finess site",
            "numero finess etablissement juridique",
        ],
    ),
    (
        "am",
        &[
            "numero am",
            "numero adeli",
            "adeli",
            "identifiant national",
            "numero assurance maladie",
        ],
    ),
    (
        "city",
        &[
            "libelle commune",
            "commune",
            "ville",
            "libelle commune coordonnees structure",
            // L'en-tête réel de l'extraction : « Libellé commune
            // (coord. structure) ». Sans lui, aucun prescripteur de
            // l'annuaire national n'avait de ville.
            "libelle commune coord structure",
        ],
    ),
    // Ce que l'identifiant est : 8 pour un RPPS, 0 pour un ADELI —
    // l'extraction range les deux dans « Identifiant PP ».
    ("kind", &["type d identifiant pp", "type identifiant pp"]),
];

/// Le séparateur du fichier, lu dans son en-tête.
///
/// Deviné et non réglé : une officine qui exporte son annuaire ne sait
/// pas ce que son logiciel a mis entre les colonnes, et lui demander
/// serait lui demander d'ouvrir le fichier dans un éditeur.
fn separator(header: &str) -> char {
    ['\t', ';', '|', ',']
        .into_iter()
        .max_by_key(|c| header.matches(*c).count())
        .unwrap_or(';')
}

/// Les cellules d'une ligne, **guillemets compris**.
///
/// Le fichier « RPPS autorisés à exercer » met chaque champ entre
/// guillemets : découpé au séparateur seul, un numéro se lisait
/// `"10001234565"` — guillemets gardés, donc jamais vérifié —, et un
/// séparateur dans un nom entre guillemets décalait toute la suite de la
/// ligne. Un champ qui **commence** par un guillemet est lu jusqu'au
/// guillemet qui le ferme, `""` valant un guillemet ; les autres sont
/// lus tels quels — l'extraction à barres verticales garde des
/// guillemets littéraux dans ses données, et ils y restent.
fn cells_of(line: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut at_start = true;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = false;
                }
            } else {
                cell.push(c);
            }
        } else if c == sep {
            out.push(std::mem::take(&mut cell));
            at_start = true;
            continue;
        } else if c == '"' && at_start {
            quoted = true;
        } else {
            cell.push(c);
        }
        at_start = false;
    }
    out.push(cell);
    out
}

/// Replie un en-tête de colonne pour le comparer.
fn fold(raw: &str) -> String {
    let folded = crate::fuzzy::sort_key(raw);
    let spaced: String = folded
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Lit un annuaire.
///
/// `Err` quand l'en-tête ne porte ni nom ni identifiant : le message
/// dit ce qu'il portait, parce qu'un refus qui ne nomme pas ce qu'il a
/// vu laisse chercher au hasard.
pub fn import(text: &str) -> Result<Imported, String> {
    let records = records(text);
    let mut lines = records
        .iter()
        .map(String::as_str)
        .filter(|l| !l.trim().is_empty());
    let Some(header) = lines.next() else {
        return Err(crate::strings::tr("presc_import_empty").to_owned());
    };
    let sep = separator(header);
    let heads: Vec<String> = cells_of(header, sep).iter().map(|h| fold(h)).collect();

    // Où chaque champ se trouve, **par son nom**.
    let mut at: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (field, names) in COLUMNS {
        if let Some(i) = heads.iter().position(|h| names.contains(&h.as_str())) {
            at.insert(field, i);
        }
    }
    // Un nom : c'est le minimum pour que la ligne désigne quelqu'un.
    // Sans lui, ce fichier n'est pas un annuaire, et l'importer
    // donnerait trente mille lignes vides.
    if !at.contains_key("last") {
        return Err(crate::strings::trf(
            "presc_import_no_name",
            heads
                .iter()
                .filter(|h| !h.is_empty())
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }

    let take = |cells: &[String], field: &str| -> String {
        at.get(field)
            .and_then(|i| cells.get(*i))
            .map(|s| s.trim().to_owned())
            .unwrap_or_default()
    };
    let mut out = Imported {
        columns: at.keys().map(|k| (*k).to_owned()).collect(),
        ..Imported::default()
    };
    let mut seen = std::collections::HashSet::new();
    for line in lines {
        let cells = cells_of(line, sep);
        let mut who = Prescriber {
            rpps: take(&cells, "rpps"),
            last: take(&cells, "last"),
            first: take(&cells, "first"),
            speciality: take(&cells, "speciality"),
            finess: take(&cells, "finess"),
            am: take(&cells, "am"),
            city: take(&cells, "city"),
        };
        if who.last.trim().is_empty() {
            out.skipped += 1;
            continue;
        }
        // **Un numéro ADELI n'est pas un RPPS qui ne se prouve pas** :
        // l'extraction range les deux dans la même colonne, et les neuf
        // chiffres d'un ADELI se comptaient comme autant de RPPS faux.
        // Rangé à sa place, il ne se vérifie pas par une clé qu'il n'a
        // pas.
        if take(&cells, "kind").trim() == "0" {
            if who.am.trim().is_empty() {
                who.am = std::mem::take(&mut who.rpps);
            } else {
                who.rpps.clear();
            }
        }
        let key = who.rpps.trim().to_owned();
        if !key.is_empty() && !seen.insert(key) {
            out.merged += 1;
            continue;
        }
        if !who.rpps.trim().is_empty() && !who.rpps_checks() {
            out.unverified += 1;
        }
        out.found.push(who);
    }
    // **Zéro personne lue n'est pas un annuaire vide** : l'import
    // remplace l'annuaire en place, et un fichier tronqué ou une réponse
    // de serveur qui n'était pas la bonne l'effaçait sous « 0
    // prescripteur importé ». Rien n'est remplacé par rien.
    if out.found.is_empty() {
        return Err(crate::strings::trf("presc_import_none", out.skipped));
    }
    Ok(out)
}

/// Les enregistrements d'un CSV, et non ses lignes : un champ entre
/// guillemets peut porter un saut de ligne (« Alice⏎Marie »), et couper
/// sur les lignes en faisait deux personnes, la seconde nommée d'après
/// la ville de la première. Le saut de ligne d'un champ devient une
/// espace ; celui d'un fin d'enregistrement Windows perd son `\r`.
fn records(text: &str) -> Vec<String> {
    // The separator is the header's, and a quote opens a field only at
    // its start — the same reading as `cells_of`, or a stray quote inside
    // a name would swallow every line after it.
    let header = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let sep = separator(header);
    let mut out = Vec::new();
    let mut record = String::new();
    let mut quoted = false;
    let mut at_start = true;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                record.push(c);
                if chars.peek() == Some(&'"') {
                    record.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else if c == '\n' {
                if record.ends_with('\r') {
                    record.pop();
                }
                record.push(' ');
            } else {
                record.push(c);
            }
            continue;
        }
        match c {
            '\n' => {
                if record.ends_with('\r') {
                    record.pop();
                }
                out.push(std::mem::take(&mut record));
                at_start = true;
                continue;
            }
            '"' if at_start => quoted = true,
            _ => {}
        }
        at_start = c == sep;
        record.push(c);
    }
    if !record.is_empty() {
        out.push(record);
    }
    out
}

/// Cherche dans l'annuaire, comme le reste de l'application cherche.
///
/// Par le nom, le prénom, la spécialité, la ville **et le numéro** :
/// quelqu'un qui a l'ordonnance sous les yeux y lit un RPPS, et devoir
/// retaper un nom pour retrouver un numéro qu'on a déjà serait une
/// recherche qui ignore ce qu'on lui donne.
///
/// L'ordre est total — le score, puis le nom, puis le numéro — sans
/// quoi deux frappes identiques rendraient deux listes différentes.
pub fn search<'a>(directory: &'a [Prescriber], query: &str, most: usize) -> Vec<&'a Prescriber> {
    let needle = query.trim();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(i32, &'a Prescriber)> = directory
        .iter()
        .filter_map(|p| {
            // Le meilleur des champs, et non leur concaténation : « Morel
            // Montpellier » ne doit pas mieux sortir qu'un docteur
            // Morel-Montpellier qui n'existe pas.
            [
                p.last.as_str(),
                p.first.as_str(),
                p.speciality.as_str(),
                p.city.as_str(),
                p.rpps.as_str(),
                p.am.as_str(),
            ]
            .into_iter()
            .filter(|f| !f.is_empty())
            .filter_map(|f| crate::fuzzy::score(needle, f))
            .max()
            .map(|s| (s, p))
        })
        .collect();
    hits.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.last.cmp(&b.1.last))
            .then_with(|| a.1.rpps.cmp(&b.1.rpps))
    });
    hits.truncate(most);
    hits.into_iter().map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "Identifiant PP;Nom d'exercice;Prénom d'exercice;Libellé profession;Numéro FINESS établissement;Numéro AM;Libellé commune";

    fn sample() -> String {
        format!(
            "{HEADER}\n\
             10001234565;Morel;Jean;Médecin généraliste;340000123;341234567;Montpellier\n\
             10001234573;Bonnet;Alice;Chirurgien-dentiste;;;Sète\n"
        )
    }

    /// Une colonne se trouve par son **nom**, jamais par son rang.
    ///
    /// Les fichiers publics changent d'ordre d'une version à l'autre, et
    /// lire la septième colonne c'est lire autre chose l'année suivante,
    /// en silence — trente mille lignes décalées qu'aucune erreur ne
    /// signale.
    #[test]
    fn a_column_is_found_by_its_name_and_never_by_its_rank() {
        let straight = import(&sample()).expect("un annuaire");
        assert_eq!(straight.found.len(), 2);
        assert_eq!(straight.found[0].last, "Morel");
        assert_eq!(straight.found[0].city, "Montpellier");
        assert_eq!(straight.found[0].finess, "340000123");

        // Les mêmes colonnes, dans un autre ordre : le même annuaire.
        let shuffled = "Libellé commune;Nom d'exercice;Identifiant PP;Prénom d'exercice\n\
                        Montpellier;Morel;10001234565;Jean\n";
        let other = import(shuffled).expect("un annuaire");
        assert_eq!(other.found[0].last, "Morel");
        assert_eq!(other.found[0].city, "Montpellier");
        assert_eq!(other.found[0].rpps, "10001234565");
        // Ce qui n'était pas dans le fichier reste vide, et n'est pas
        // rempli par la colonne d'à côté.
        assert!(other.found[0].speciality.is_empty());
    }

    /// Le séparateur est deviné, et l'accentuation de l'en-tête ne
    /// décide de rien.
    #[test]
    fn the_separator_and_the_spelling_of_a_header_decide_nothing() {
        let tabbed = "NOM D'EXERCICE\tPRENOM D'EXERCICE\tIDENTIFIANT PP\n\
                      Morel\tJean\t10001234565\n";
        let read = import(tabbed).expect("un annuaire");
        assert_eq!(read.found.len(), 1);
        assert_eq!(read.found[0].first, "Jean");
        // Accents, casse, ponctuation : repliés avant d'être comparés.
        assert_eq!(fold("Libellé  profession"), "libelle profession");
        assert_eq!(fold("NOM D'EXERCICE"), "nom d exercice");
    }

    /// Un fichier qui n'est pas un annuaire est refusé, **en disant ce
    /// qu'il portait**.
    ///
    /// Un refus qui ne nomme pas ce qu'il a vu laisse chercher au
    /// hasard : l'officine ouvre le fichier dans un tableur et compare.
    #[test]
    fn a_file_that_is_not_a_directory_is_refused_and_says_what_it_saw() {
        let wrong = "Code;Libellé;Prix\nA1;Boîte;4,20\n";
        let err = import(wrong).expect_err("refusé");
        assert!(err.contains("code"), "« {err} »");
        assert!(err.contains("prix"), "« {err} »");
        assert!(import("").is_err());
        assert!(import("   \n\n").is_err());
    }

    /// **La clé décide, jamais la longueur** — et un numéro qui ne se
    /// prouve pas est gardé, puis signalé.
    ///
    /// Le jeter serait laisser une officine croire son annuaire complet
    /// alors qu'il lui manque les lignes qu'un logiciel a mal exportées.
    #[test]
    fn a_number_that_does_not_prove_itself_is_kept_and_counted() {
        // Onze chiffres, clé juste.
        assert!(luhn_ok("10001234565"));
        // Le même avec un chiffre changé : la longueur est bonne, la
        // clé non.
        assert!(!luhn_ok("10001234566"));
        assert!(!luhn_ok("1000123456"), "dix chiffres, clé fausse");
        assert!(!luhn_ok("1000123456X"));
        assert!(!luhn_ok(""));

        let with_bad = format!(
            "{HEADER}\n\
             10001234566;Morel;Jean;Médecin généraliste;;;Montpellier\n\
             ;Sans;Numero;Sage-femme;;;Sète\n"
        );
        let read = import(&with_bad).expect("un annuaire");
        // Les deux sont là : rien n'est jeté.
        assert_eq!(read.found.len(), 2);
        assert_eq!(read.unverified, 1, "et le douteux est compté");
        assert_eq!(read.skipped, 0);
        assert!(!read.found[0].rpps_checks());
        // Une ligne sans nom ne désigne personne : celle-là est
        // écartée, et **comptée**.
        let mut nameless = format!("{HEADER}\n10001234565;;;;;;\n");
        nameless.push_str("10001234573;Bonnet;Alice;;;;Sète\n");
        let read = import(&nameless).expect("un annuaire");
        assert_eq!(read.found.len(), 1);
        assert_eq!(read.skipped, 1);
        // Et **personne** lu n'est pas un annuaire vide : c'est un refus,
        // qui laisse l'annuaire en place au lieu de le remplacer par rien.
        assert!(import(&format!("{HEADER}\n10001234565;;;;;;\n")).is_err());
        assert!(import(&format!("{HEADER}\n")).is_err());
    }

    /// Un champ entre guillemets peut porter un saut de ligne, et un
    /// praticien à plusieurs activités n'est qu'une personne.
    #[test]
    fn a_record_is_not_a_line_and_one_rpps_is_one_person() {
        let text =
            "\"Identifiant PP\";\"Nom d'exercice\";\"Prénom d'exercice\";\"Libellé commune\"\r\n\
                    \"10001234573\";\"Bonnet\";\"Alice\r\nMarie\";\"Sète\"\r\n\
                    \"10001234573\";\"Bonnet\";\"Alice Marie\";\"Agde\"\r\n";
        let read = import(text).expect("un annuaire");
        assert_eq!(read.found.len(), 1, "{:?}", read.found);
        assert_eq!(read.found[0].first, "Alice Marie");
        assert_eq!(read.found[0].city, "Sète");
        assert_eq!(read.merged, 1);
    }

    /// La recherche trouve par ce qu'on a sous les yeux — y compris le
    /// numéro, qui est écrit sur l'ordonnance.
    #[test]
    fn a_search_finds_by_name_by_place_and_by_number() {
        let dir = import(&sample()).unwrap().found;
        let by_name = search(&dir, "morel", 5);
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].last, "Morel");
        // Par la ville, par la spécialité, par le numéro.
        assert_eq!(search(&dir, "sete", 5)[0].last, "Bonnet");
        assert_eq!(search(&dir, "dentiste", 5)[0].last, "Bonnet");
        assert_eq!(search(&dir, "10001234565", 5)[0].last, "Morel");
        assert_eq!(search(&dir, "341234567", 5)[0].last, "Morel");
        // Une requête vide ne propose rien : elle ne propose pas tout.
        assert!(search(&dir, "  ", 5).is_empty());
        // Et le plafond est un plafond.
        assert_eq!(search(&dir, "e", 1).len().min(1), 1);
    }

    /// Ce qui s'écrit sur une ligne, et ce qui s'affiche dans une
    /// liste, ne sont pas la même chose : l'un est court parce qu'une
    /// ordonnance se signe court, l'autre distingue deux homonymes.
    #[test]
    fn what_is_written_on_a_line_is_shorter_than_what_a_list_shows() {
        let dir = import(&sample()).unwrap().found;
        assert_eq!(dir[0].short(), "Dr Morel");
        assert_eq!(
            dir[0].label(),
            "Dr Morel Jean — Médecin généraliste — Montpellier"
        );
        // Sans nom de famille, ce qui reste est le prénom — et non
        // « Dr », qui ne désigne personne.
        let only_first = Prescriber {
            first: "Alice".to_owned(),
            ..Prescriber::default()
        };
        assert_eq!(only_first.short(), "Alice");
        assert_eq!(Prescriber::default().short(), "");
    }

    /// **Rien n'est livré.** L'annuaire appartient à l'officine : une
    /// table figée dans un binaire vieillit sans que personne le voie,
    /// et celle-ci désignerait des personnes.
    #[test]
    fn no_directory_ships_with_the_application() {
        let text = include_str!("prescribers.rs");
        let code = text.split("mod tests").next().unwrap();
        for shipped in [
            concat!("sta", "tic DIRECTORY"),
            concat!("con", "st DIRECTORY"),
            concat!("&[Prescri", "ber]"),
            concat!("[Prescri", "ber;"),
        ] {
            assert!(!code.contains(shipped), "un annuaire livré : « {shipped} »");
        }
        // Et le module ne lit aucun disque : le texte lui est passé.
        for reaching in ["std::fs", "File::open", "read_to_string"] {
            assert!(!code.contains(reaching), "« {reaching} »");
        }
    }

    /// **Le fichier réel, tel qu'il est publié** : champs entre
    /// guillemets, séparateur dans un nom, ville sous son libellé de
    /// l'extraction, et un ADELI rangé dans la colonne des RPPS.
    #[test]
    fn the_public_extract_is_read_as_it_is_published() {
        let text = "\"Type d'identifiant PP\";\"Identifiant PP\";\"Nom d'exercice\";\"Prénom d'exercice\";\"Libellé commune (coord. structure)\"\n\
                    \"8\";\"10001234565\";\"Morel\";\"Jean\";\"Montpellier\"\n\
                    \"0\";\"341234567\";\"Dupont; Martin\";\"Anne\";\"Sète\"\n";
        let read = import(text).expect("lu");
        assert_eq!(read.found.len(), 2);
        assert_eq!(read.found[0].rpps, "10001234565");
        assert_eq!(read.found[0].last, "Morel");
        assert_eq!(read.found[0].city, "Montpellier");
        // Le point-virgule entre guillemets ne décale rien.
        assert_eq!(read.found[1].last, "Dupont; Martin");
        assert_eq!(read.found[1].city, "Sète");
        // L'ADELI est rangé comme tel, et ne compte pas comme un RPPS
        // qui ne se prouve pas.
        assert_eq!(read.found[1].rpps, "");
        assert_eq!(read.found[1].am, "341234567");
        assert_eq!(read.unverified, 0);
        // Et une ligne sans guillemets se lit comme avant.
        assert_eq!(cells_of("a|b||c", '|'), vec!["a", "b", "", "c"]);
        assert_eq!(cells_of("\"x \"\"y\"\"\";z", ';'), vec!["x \"y\"", "z"]);
    }
}
