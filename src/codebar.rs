//! Ce qu'une douchette a tapé, une fois lu.
//!
//! Une douchette USB est un clavier : elle tape des caractères et
//! valide. Il n'y a donc ici ni pilote, ni image, ni décodage optique —
//! seulement la lecture d'une chaîne. Tout ce qui parlerait de décoder
//! un Code128 depuis une photo est un autre programme.
//!
//! Deux choses arrivent au comptoir : le **CIP13** imprimé en clair sous
//! le code-barres, et le **DataMatrix GS1** de la boîte, qui porte le
//! GTIN (AI 01), le lot (AI 10), la péremption (AI 17) et le numéro de
//! série (AI 21).
//!
//! # La clé décide, jamais la longueur
//!
//! C'est le moment `vitale.rs` de ce module : là-bas le NIR se prouve
//! par sa clé de contrôle, ici le GTIN aussi. **Treize chiffres dont la
//! clé ne tombe pas sont treize chiffres**, pas une coquille à
//! rattraper. Rien n'est lu à un décalage que quelqu'un a supposé.
//!
//! # Le séparateur, et pourquoi son absence n'atteint pas le produit
//!
//! Les AI 10 et 21 sont de longueur variable et se terminent au
//! caractère FNC1 (`0x1D`). Beaucoup de douchettes ne l'émettent pas.
//! Quand un champ variable court jusqu'au bout de la chaîne et que la
//! suite ressemble à un AI, **le module ne coupe pas** : la donnée est
//! réellement ambiguë, il prend le reste entier pour le lot et le dit
//! (`lot_certain: false`).
//!
//! Cela se supporte à cause d'un fait de structure qui mérite d'être
//! écrit : **l'AI 01 est de longueur fixe et vient par convention en
//! tête, donc l'identification du produit ne dépend jamais du
//! séparateur.** Seuls le lot et la péremption en dépendent. C'est un
//! test.
//!
//! Un AI que cette version ne connaît pas **arrête** la lecture : ce qui
//! précède est certain, ce qui suit ne l'est pas, et comprendre à moitié
//! une chaîne est pire que s'arrêter.
//!
//! # Ce module ne livre aucun catalogue
//!
//! L'application n'embarque aucune table CIP — la base publique des
//! médicaments est encore à faire — et c'est un heureux hasard qu'il
//! faut verrouiller : elle n'a **rien avec quoi deviner**. `fuzzy.rs`
//! est juste à côté, et rapprocher un scan d'un libellé serait facile et
//! faux. **Un GTIN ne porte aucun nom.** Les codes que l'officine
//! attache à ses produits sont son contenu à elle, comme ses pièces
//! numérisées et son registre.
//!
//! Statique, pur, testé. Aucune horloge : le jour est donné.

// Comme `vigilance`, le module arrive avant l'écran qui s'en sert. La
// permission tombe avec le formulaire qui lira la douchette.
#![allow(dead_code)]

/// La forme sous laquelle le code identifiant a été lu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeKind {
    /// Treize chiffres que leur clé de contrôle prouve — le CIP13
    /// imprimé sous le code-barres.
    Gtin13,
    /// Quatorze, lus dans l'AI 01 du DataMatrix.
    Gtin14,
}

/// Ce qu'une douchette a tapé, une fois lu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scanned {
    /// Le code identifiant, chiffres seuls, sous sa forme canonique.
    pub code: String,
    pub kind: CodeKind,
    /// Le lot (AI 10), vide s'il n'y en avait pas.
    pub lot: String,
    /// Le lot a-t-il été **fermé** par un séparateur ? Faux quand il
    /// courait jusqu'à la fin de la chaîne : il est alors lu au plus
    /// large, ce qui n'atteint jamais le code.
    pub lot_certain: bool,
    /// La péremption (AI 17), en ISO. Vide si absente ou illisible.
    pub expiry: String,
    /// Le numéro de série (AI 21). Lu, gardé, et ici sans emploi.
    pub serial: String,
    /// La lecture est-elle allée au bout ? Faux quand un AI inconnu l'a
    /// arrêtée.
    pub read_to_end: bool,
    /// Ce que la douchette a tapé, entier. Rien ne se garde sans ce qui
    /// l'a produit.
    pub raw: String,
}

impl Scanned {
    /// Les formes sous lesquelles ce code peut être rangé.
    ///
    /// Les treize chiffres et, quand l'indicateur d'emballage est zéro,
    /// la forme à quatorze — la même boîte scannée en clair ou par son
    /// DataMatrix. De l'arithmétique, jamais une supposition : la clé
    /// est recalculée et non recopiée.
    pub fn keys(&self) -> Vec<String> {
        let mut out = vec![self.code.clone()];
        match self.kind {
            CodeKind::Gtin13 => {
                let padded = format!("0{}", &self.code[..self.code.len() - 1]);
                if let Some(k) = check_digit(&padded) {
                    out.push(format!("{padded}{k}"));
                }
            }
            CodeKind::Gtin14 => {
                if self.code.starts_with('0') {
                    let body = &self.code[1..self.code.len() - 1];
                    if let Some(k) = check_digit(body) {
                        out.push(format!("{body}{k}"));
                    }
                }
            }
        }
        out.dedup();
        out
    }

    /// Le préfixe français du médicament (340).
    ///
    /// Un **indice** qu'on affiche, jamais une condition de lecture : un
    /// préfixe est une politique d'attribution, et un module qui
    /// refuserait un code valide parce qu'il ne connaît pas son préfixe
    /// refuserait une boîte en 2031.
    pub fn french_drug_prefix(&self) -> bool {
        let digits = match self.kind {
            CodeKind::Gtin13 => self.code.as_str(),
            CodeKind::Gtin14 => &self.code[1..],
        };
        digits.starts_with("340")
    }
}

/// La clé de contrôle d'un GTIN, calculée sur les chiffres **sans**
/// elle.
///
/// De droite à gauche, en pesant trois puis un ; la clé complète la
/// somme à la dizaine. Vaut pour toutes les longueurs de GTIN, ce qui
/// est ce qui permet de passer du treize au quatorze sans recopier une
/// clé qu'on n'aurait pas vérifiée.
pub fn check_digit(digits: &str) -> Option<u8> {
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let sum: u32 = digits
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| c.to_digit(10).unwrap_or(0) * if i % 2 == 0 { 3 } else { 1 })
        .sum();
    Some(u8::try_from((10 - sum % 10) % 10).unwrap_or(0))
}

/// Le code entier tient-il sa propre clé ?
fn key_holds(full: &str) -> bool {
    if full.len() < 2 || !full.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let (body, last) = full.split_at(full.len() - 1);
    check_digit(body).is_some_and(|k| last.as_bytes()[0] - b'0' == k)
}

/// Lire ce qu'une douchette a tapé, avec le seul séparateur normalisé.
pub fn read(typed: &str, today: &str) -> Option<Scanned> {
    read_with(typed, today, &[])
}

/// La même chose, en acceptant en plus les caractères que *cette*
/// douchette-là émet à la place du FNC1 quand elle n'est pas réglée
/// dessus.
pub fn read_with(typed: &str, today: &str, separators: &[char]) -> Option<Scanned> {
    let raw = typed.trim_matches(|c: char| c.is_whitespace() || c == '\r' || c == '\n');
    // Certaines douchettes préfixent l'identifiant de symbologie AIM.
    // Il est reconnu et **jeté**, jamais lu comme une donnée.
    let body = ["]d2", "]C1", "]e0", "]Q3"]
        .iter()
        .find_map(|p| raw.strip_prefix(p))
        .unwrap_or(raw);
    if body.is_empty() {
        return None;
    }
    let base = Scanned {
        code: String::new(),
        kind: CodeKind::Gtin13,
        lot: String::new(),
        lot_certain: true,
        expiry: String::new(),
        serial: String::new(),
        read_to_end: true,
        raw: raw.to_owned(),
    };
    // Le cas courant : les chiffres du CIP13 tapés tels quels.
    if body.chars().all(|c| c.is_ascii_digit()) && key_holds(body) {
        return match body.len() {
            13 => Some(Scanned {
                code: body.to_owned(),
                kind: CodeKind::Gtin13,
                ..base
            }),
            14 => Some(Scanned {
                code: body.to_owned(),
                kind: CodeKind::Gtin14,
                ..base
            }),
            _ => None,
        };
    }
    read_element_string(body, today, separators, base)
}

/// Parcourir une chaîne d'éléments GS1.
fn read_element_string(
    body: &str,
    today: &str,
    separators: &[char],
    mut out: Scanned,
) -> Option<Scanned> {
    let is_sep = |c: char| c == '\u{1d}' || separators.contains(&c);
    let mut rest = body;
    let mut seen_code = false;
    while !rest.is_empty() {
        rest = rest.trim_start_matches(is_sep);
        if rest.is_empty() {
            break;
        }
        let (ai, after) = rest.split_at_checked(2)?;
        match ai {
            // Longueur fixe : aucun séparateur n'est nécessaire, et
            // c'est ce qui met l'identification du produit hors de
            // portée du problème du FNC1.
            "01" => {
                let (digits, tail) = after.split_at_checked(14)?;
                if !key_holds(digits) {
                    return None;
                }
                out.code = digits.to_owned();
                out.kind = CodeKind::Gtin14;
                seen_code = true;
                rest = tail;
            }
            "17" => {
                let (digits, tail) = after.split_at_checked(6)?;
                out.expiry = read_expiry(digits, today).unwrap_or_default();
                rest = tail;
            }
            "10" | "21" => {
                let (value, tail, closed) = match after.find(is_sep) {
                    Some(i) => (&after[..i], &after[i..], true),
                    None => (after, "", false),
                };
                if ai == "10" {
                    out.lot = value.to_owned();
                    out.lot_certain = closed;
                } else {
                    out.serial = value.to_owned();
                }
                rest = tail;
            }
            // Un AI que cette version ne connaît pas arrête la lecture.
            // Ce qui précède est certain ; ce qui suit ne l'est pas.
            _ => {
                out.read_to_end = false;
                break;
            }
        }
    }
    seen_code.then_some(out)
}

/// La péremption d'un AI 17, `AAMMJJ`, rendue en ISO.
///
/// Deux pièges, et le second est celui qu'on ne voit pas venir :
///
/// - le siècle d'un millésime à deux chiffres se décide par rapport au
///   **jour qui est donné**, jamais par une horloge ;
/// - **le jour peut valoir `00`**, ce qui veut dire « fin du mois » :
///   `260500` est le 31 mai 2026. Le lecteur naïf en fait une date
///   invalide ou, pire, le premier du mois.
///
/// Un mois ou un jour impossibles rendent `None`, et l'appelant garde
/// tout le reste : on ne devine jamais une date sur une boîte.
fn read_expiry(digits: &str, today: &str) -> Option<String> {
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let yy: i64 = digits[..2].parse().ok()?;
    let m: i64 = digits[2..4].parse().ok()?;
    let d: i64 = digits[4..6].parse().ok()?;
    if !(1..=12).contains(&m) || !(0..=31).contains(&d) {
        return None;
    }
    let (this_year, _, _) = crate::date::parse_iso(today)?;
    // La fenêtre glissante : le millésime le plus proche du jour donné.
    let mut year = this_year - this_year % 100 + yy;
    if year - this_year > 50 {
        year -= 100;
    } else if this_year - year > 49 {
        year += 100;
    }
    let last = crate::date::end_of_month(year, m)?;
    let day = if d == 0 { last } else { d };
    if day > last {
        return None;
    }
    Some(format!("{year:04}-{m:02}-{day:02}"))
}

/// Ce qu'un scan désigne, contre les codes que l'officine a appris.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    Known {
        stup_id: i64,
    },
    /// Personne ne le porte. Le scan est une **proposition
    /// d'apprendre**, et le choix est celui d'un humain.
    Unknown,
    /// Deux produits le portent — impossible tant que la base tient sa
    /// clé primaire, et lu comme inconnu plutôt que comme l'un des deux.
    Ambiguous,
}

/// À quel produit suivi ce scan correspond, parmi ceux à qui l'officine
/// a appris un code.
///
/// Jamais de voisin, jamais de rapprochement sur un libellé : un GTIN ne
/// porte aucun nom, et le seul lien qui existe est celui qu'un humain a
/// posé en présentant une boîte.
pub fn resolve(s: &Scanned, taught: &[(i64, &str)]) -> Resolved {
    let keys = s.keys();
    let mut hits: Vec<i64> = taught
        .iter()
        .filter(|(_, code)| keys.iter().any(|k| k == code))
        .map(|(id, _)| *id)
        .collect();
    hits.sort_unstable();
    hits.dedup();
    match hits.len() {
        0 => Resolved::Unknown,
        1 => Resolved::Known { stup_id: hits[0] },
        _ => Resolved::Ambiguous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un CIP13 valide, et le même sous sa forme à quatorze.
    ///
    /// Calculé à la main : la charge « 340093000000 », lue de droite à
    /// gauche en pesant 3 puis 1, donne 3·3 + 9·1 + 4·3 + 3·1 = 33 ; la
    /// clé complète à la dizaine, soit 7.
    const CIP: &str = "3400930000007";
    const GTIN14: &str = "03400930000007";

    #[test]
    fn a_thirteen_digit_code_proves_itself_by_its_key() {
        assert_eq!(check_digit("340093000000"), Some(7));
        let s = read(CIP, "2026-09-01").expect("un code valide doit se lire");
        assert_eq!(s.code, CIP);
        assert_eq!(s.kind, CodeKind::Gtin13);
        assert!(s.french_drug_prefix());

        // Un chiffre changé : ce ne sont plus que treize chiffres.
        assert_eq!(read("3400930000008", "2026-09-01"), None);
        assert_eq!(read("3400930000107", "2026-09-01"), None);
    }

    #[test]
    fn a_fourteen_digit_gtin_and_its_thirteen_digit_cip_are_the_same_box() {
        let short = read(CIP, "2026-09-01").unwrap();
        let long = read(GTIN14, "2026-09-01").unwrap();
        assert_eq!(long.kind, CodeKind::Gtin14);
        // Chacun connaît les deux formes, et la clé est **recalculée**
        // et non recopiée d'une longueur à l'autre.
        assert!(short.keys().contains(&GTIN14.to_owned()));
        assert!(long.keys().contains(&CIP.to_owned()));
        // Donc les deux répondent au même produit appris sous l'une ou
        // l'autre graphie.
        assert_eq!(
            resolve(&short, &[(7, GTIN14)]),
            Resolved::Known { stup_id: 7 }
        );
        assert_eq!(resolve(&long, &[(7, CIP)]), Resolved::Known { stup_id: 7 });
    }

    #[test]
    fn a_datamatrix_gives_up_its_gtin_its_lot_and_its_expiry() {
        let typed = format!("01{GTIN14}1726053110LOT42\u{1d}21SER9");
        let s = read(&typed, "2026-09-01").expect("le DataMatrix doit se lire");
        assert_eq!(s.code, GTIN14);
        assert_eq!(s.kind, CodeKind::Gtin14);
        assert_eq!(s.expiry, "2026-05-31");
        assert_eq!(s.lot, "LOT42");
        assert!(s.lot_certain, "le séparateur a fermé le lot");
        assert_eq!(s.serial, "SER9");
        assert!(s.read_to_end);
    }

    /// Un lot que rien n'a fermé le dit — et le code, lui, ne bouge pas.
    ///
    /// C'est l'invariant qui rend l'ambiguïté du FNC1 supportable :
    /// l'AI 01 est de longueur fixe et vient en tête, donc identifier le
    /// produit ne dépend jamais du séparateur. Seuls le lot et la
    /// péremption en dépendent.
    #[test]
    fn a_lot_that_no_separator_closed_says_so() {
        let closed = read(
            &format!("01{GTIN14}1726053110ABC\u{1d}21SER9"),
            "2026-09-01",
        )
        .unwrap();
        let open = read(&format!("01{GTIN14}1726053110ABC21SER9"), "2026-09-01").unwrap();
        assert_eq!(open.code, closed.code, "le code est intact des deux côtés");
        assert_eq!(open.expiry, closed.expiry);
        assert!(closed.lot_certain);
        assert!(!open.lot_certain, "rien ne l'a fermé, et il le dit");
        assert_eq!(open.lot, "ABC21SER9", "lu au plus large plutôt que coupé");
    }

    /// Une douchette réglée sur un autre séparateur se déclare.
    #[test]
    fn a_wedge_that_types_its_own_separator_is_understood_when_it_is_named() {
        let typed = format!("01{GTIN14}1726053110ABC|21SER9");
        let blind = read(&typed, "2026-09-01").unwrap();
        assert!(!blind.lot_certain);
        let told = read_with(&typed, "2026-09-01", &['|']).unwrap();
        assert!(told.lot_certain);
        assert_eq!(told.lot, "ABC");
        assert_eq!(told.serial, "SER9");
    }

    #[test]
    fn an_ai_this_version_does_not_know_stops_the_reading_and_keeps_what_it_read() {
        let s = read(&format!("01{GTIN14}1726053130012"), "2026-09-01").unwrap();
        assert_eq!(s.code, GTIN14, "ce qui précède est certain");
        assert_eq!(s.expiry, "2026-05-31");
        assert!(!s.read_to_end, "et la lecture le dit");
        assert!(s.lot.is_empty());
    }

    /// Le jour `00` d'une péremption est la fin du mois.
    #[test]
    fn an_expiry_day_of_zero_is_the_end_of_the_month() {
        let at = |exp: &str, today: &str| {
            read(&format!("01{GTIN14}17{exp}"), today)
                .expect("doit se lire")
                .expiry
        };
        assert_eq!(at("260500", "2026-09-01"), "2026-05-31");
        assert_eq!(at("260200", "2026-09-01"), "2026-02-28");
        assert_eq!(at("240200", "2024-09-01"), "2024-02-29", "bissextile");
        assert_eq!(at("260531", "2026-09-01"), "2026-05-31");
        // Une date impossible laisse la péremption vide et garde tout le
        // reste : on ne devine jamais une date sur une boîte.
        let bad = read(&format!("01{GTIN14}17261331"), "2026-09-01").unwrap();
        assert_eq!(bad.code, GTIN14);
        assert!(bad.expiry.is_empty());
        let overflow = read(&format!("01{GTIN14}17260231"), "2026-09-01").unwrap();
        assert!(overflow.expiry.is_empty(), "le 31 février n'existe pas");
    }

    /// Le siècle d'un millésime à deux chiffres vient du jour donné, et
    /// d'aucune horloge.
    #[test]
    fn the_century_of_a_two_digit_year_is_decided_by_the_day_that_is_passed_in() {
        let at = |exp: &str, today: &str| {
            read(&format!("01{GTIN14}17{exp}"), today)
                .expect("doit se lire")
                .expiry
        };
        assert_eq!(at("270131", "2026-09-01"), "2027-01-31");
        // Lu depuis 2098, « 03 » est 2103 et non 2003 : la fenêtre
        // glisse avec le jour, elle n'est pas écrite en dur.
        assert_eq!(at("030131", "2098-09-01"), "2103-01-31");
        assert_eq!(at("030131", "2026-09-01"), "2003-01-31");
    }

    #[test]
    fn a_symbology_identifier_is_stripped_and_never_read_as_data() {
        for prefix in ["]d2", "]C1", "]e0", "]Q3"] {
            let s = read(&format!("{prefix}01{GTIN14}"), "2026-09-01")
                .unwrap_or_else(|| panic!("« {prefix} » doit être jeté"));
            assert_eq!(s.code, GTIN14);
        }
        // Et le préfixe seul n'est pas une donnée.
        assert_eq!(read("]d2", "2026-09-01"), None);
    }

    #[test]
    fn nothing_typed_is_nothing_read() {
        for junk in [
            "",
            "   ",
            "\n",
            "\r\n",
            "Skenan LP 30 mg",
            "340093000000",
            "0134009300000",
            "abcdefghijklm",
        ] {
            assert_eq!(
                read(junk, "2026-09-01"),
                None,
                "« {junk} » ne doit rien rendre"
            );
        }
    }

    /// Un code que personne n'a appris ne désigne rien — et surtout pas
    /// son voisin.
    #[test]
    fn a_code_nobody_taught_resolves_to_nothing_and_never_to_a_neighbour() {
        let s = read(CIP, "2026-09-01").unwrap();
        assert_eq!(resolve(&s, &[]), Resolved::Unknown);
        // Un code à un chiffre près : inconnu, jamais rapproché.
        assert_eq!(resolve(&s, &[(7, "3400930000106")]), Resolved::Unknown);
        assert_eq!(resolve(&s, &[(7, CIP)]), Resolved::Known { stup_id: 7 });
    }

    /// Deux produits ne peuvent pas répondre pour un code.
    ///
    /// La base le refuse par sa clé primaire ; ici, si cela arrivait
    /// quand même, le scan est lu comme inconnu plutôt que comme l'un
    /// des deux.
    #[test]
    fn two_products_can_never_answer_for_one_code() {
        let s = read(CIP, "2026-09-01").unwrap();
        assert_eq!(resolve(&s, &[(7, CIP), (9, CIP)]), Resolved::Ambiguous);
        // Le même produit appris sous les deux graphies reste lui-même.
        assert_eq!(
            resolve(&s, &[(7, CIP), (7, GTIN14)]),
            Resolved::Known { stup_id: 7 }
        );
    }

    /// Le préfixe français est un indice et jamais une condition.
    #[test]
    fn a_prefix_is_shown_and_never_gates_the_reading() {
        // Une charge qui ne commence pas par 340, avec sa vraie clé.
        let body = "500000000000";
        let key = check_digit(body).unwrap();
        let other = format!("{body}{key}");
        let s = read(&other, "2026-09-01").expect("un code valide se lit, d'où qu'il vienne");
        assert!(!s.french_drug_prefix());
        assert_eq!(
            resolve(&s, &[(3, other.as_str())]),
            Resolved::Known { stup_id: 3 }
        );
    }
}
