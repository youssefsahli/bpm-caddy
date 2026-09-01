//! Les questions que le registre pose, et qu'il ne posait à personne.
//!
//! Chaque délivrance de stupéfiant porte depuis toujours son dossier,
//! son prescripteur, son jour et sa quantité. Rien ne les a jamais
//! interrogés ensemble — alors que les monographies de la base nomment
//! trois signaux, page après page, comme ce que le comptoir surveille :
//! « renouvellements anticipés », « pluralité de prescripteurs »,
//! « escalade de doses ». Ce module les lit.
//!
//! # Ce que le registre ne sait pas, et qu'on n'inventera pas
//!
//! Une ligne `SORTIE` porte un jour, une quantité **en unités de
//! comptage**, un dossier, un prescripteur en texte libre, et le plafond
//! de la famille du produit. Elle ne porte **ni la dose quotidienne, ni
//! la durée prescrite, ni à quelle ordonnance elle se rattache** — un
//! sirop de méthadone fractionné par sept jours sous une seule
//! ordonnance ne se distingue pas de quatre ordonnances.
//!
//! Donc « ce traitement aurait dû durer jusqu'au » ne se calcule pas, et
//! l'approximation qui vient à l'esprit — `quantité / max_days` — est
//! fausse dans le sens dangereux : `max_days` est un **plafond légal**,
//! si bien qu'une ordonnance de sept jours délivrée sous un plafond de
//! vingt-huit donne un débit quatre fois trop faible, et **chaque
//! délivrance légitime devient un signalement**. Une règle qui crie au
//! loup est une règle qu'on désactive, et celle-ci ne doit pas l'être.
//!
//! # Ce qui se sait : le dossier contre lui-même
//!
//! Deux silences indépendants, et une question n'est posée que si les
//! **deux** sont franchis :
//!
//! 1. **Le plafond.** Au-delà de `max_days`, deux ordonnances ne peuvent
//!    pas se chevaucher — c'est de l'arithmétique et non une inférence.
//!    Donc `jours >= max_days` fait taire, toujours. `max_days` ne sert
//!    **qu'à se taire**, jamais à déduire un débit. C'est la phrase la
//!    plus importante du module.
//! 2. **La cadence du dossier.** La médiane des intervalles de ses
//!    délivrances *précédentes* de ce produit. Un dossier venu trois
//!    fois à vingt-huit jours et qui revient au neuvième a changé son
//!    propre rythme. Cela n'affirme rien sur la consommation ; cela
//!    constate un fait du registre, que quiconque relit les mêmes lignes
//!    peut vérifier.
//!
//! Moins de `min_history` délivrances antérieures : pas de médiane, donc
//! **pas de question**. Le module refuse de parler d'un dossier qu'il
//! vient de rencontrer.
//!
//! Un produit dont la famille n'est pas renseignée (`max_days == 0`) est
//! hors vigilance et le module n'en dit **rien** — c'est [`unwatched`]
//! qui le compte, pour que la vue l'écrive sur la porte. Un trou
//! silencieux se lirait « rien à signaler ».
//!
//! # Une question, jamais un verdict
//!
//! Il n'y a nulle part où écrire une conclusion, et c'est voulu :
//! [`Finding`] ne porte ni phrase, ni gravité, ni score. Les mots
//! viennent de `strings.fr.toml` par [`Signal::question_key`] — dont un
//! test exige que chacun **finisse par un point d'interrogation** —, les
//! nombres viennent de [`Evidence`], et une question ne peut pas exister
//! sans **les lignes du registre qui la posent**. Le dossier est désigné
//! par son numéro, jamais par un nom, comme partout dans le registre.
//!
//! Statique, pur, testé. Aucune horloge (le jour est donné), aucune
//! base, aucun egui — et **aucun catalogue** : tout ce que ce module
//! sait vient du registre de l'officine.

/// Une délivrance, telle que la vigilance la lit.
///
/// Assemblée par la base en une requête : `stup_moves` joint à
/// `stupefiants` pour le libellé, l'unité et le plafond de la famille.
#[derive(Clone, Copy, Debug)]
pub struct Dispensing<'a> {
    /// L'identifiant de la ligne du registre. C'est ce qu'une question
    /// cite pour qu'on aille la relire.
    pub seq: i64,
    pub stup_id: i64,
    pub label: &'a str,
    pub unit: &'a str,
    /// Le plafond de prescription de la famille, en jours. Zéro quand la
    /// famille n'est pas renseignée — et le module se tait alors.
    pub max_days: i64,
    /// Le dossier, jamais le nom. Zéro = aucun dossier saisi.
    pub patient_id: i64,
    /// Le prescripteur **tel qu'il a été tapé**. Jamais réécrit.
    pub prescriber: &'a str,
    pub day: &'a str,
    pub quantity: f64,
    /// Une délivrance annulée n'a pas eu lieu, et ne fait pas un motif.
    pub cancelled: bool,
}

/// La question qu'une série de lignes pose.
///
/// Trois, et pas une de plus que les monographies n'en nomment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Signal {
    /// Revenu bien plus tôt que d'habitude, et pour autant.
    Rapprochement,
    /// Plusieurs prescripteurs pour le même produit sur la fenêtre.
    Prescripteurs,
    /// Le débit délivré monte, délivrance après délivrance.
    Escalade,
}

impl Signal {
    /// L'énumération complète des signaux.
    ///
    /// Elle n'a pas d'appelant dans la vue — un tableau les mêle plutôt
    /// que de les séparer — et elle existe pour le test qui tient le
    /// contrat du module : chaque signal **demande** au lieu d'énoncer.
    /// Un jour où quelqu'un ajouterait un quatrième signal sans sa
    /// question, c'est cette liste qui le ferait tomber.
    #[allow(dead_code)]
    pub const ALL: [Signal; 3] = [
        Signal::Rapprochement,
        Signal::Prescripteurs,
        Signal::Escalade,
    ];

    /// L'ordre d'affichage : ce sur quoi on peut encore agir aujourd'hui
    /// d'abord, ce qui se lit sur des mois en dernier.
    fn rank(self) -> u8 {
        match self {
            Signal::Rapprochement => 0,
            Signal::Prescripteurs => 1,
            Signal::Escalade => 2,
        }
    }

    /// La clé de la **question**.
    ///
    /// Pas `label_key` : le nom du verbe est le contrat. Un test relit ce
    /// que la clé rend et exige un point d'interrogation, ce qui est le
    /// cliquet contre le jour où quelqu'un écrira « mésusage probable »
    /// dans cette table.
    pub fn question_key(self) -> &'static str {
        match self {
            Signal::Rapprochement => "vigilance_q_rapprochement",
            Signal::Prescripteurs => "vigilance_q_prescripteurs",
            Signal::Escalade => "vigilance_q_escalade",
        }
    }

    /// Sa couleur dans la palette des séries, fixe comme celle des
    /// natures de ligne.
    pub fn series(self) -> usize {
        match self {
            Signal::Rapprochement => 0,
            Signal::Prescripteurs => 4,
            Signal::Escalade => 3,
        }
    }
}

/// Ce qui a été **mesuré**.
///
/// Aucune variante ne porte de phrase, de note, de gravité ni de score :
/// il n'y a pas de champ où une conclusion pourrait s'écrire.
#[derive(Clone, Debug, PartialEq)]
pub enum Evidence {
    /// Deux délivrances à `days` jours, là où ce dossier en compte
    /// habituellement `usual`, sous un plafond de `ceiling` jours.
    Interval {
        days: i64,
        usual: i64,
        ceiling: i64,
        quantity: f64,
    },
    /// Les prescripteurs rencontrés, **dans leur graphie d'origine** et
    /// dans l'ordre où le registre les porte.
    Prescribers {
        spellings: Vec<String>,
        window_days: i64,
    },
    /// Le débit délivré, en unités de comptage par jour, sur les
    /// dernières délivrances — et la médiane de celles d'avant.
    Rate { rates: Vec<f64>, usual: f64 },
}

/// Une ligne du tableau : un dossier, un produit, une question.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub patient_id: i64,
    pub stup_id: i64,
    pub label: String,
    pub unit: String,
    pub signal: Signal,
    /// Les identifiants des lignes du registre qui l'ont produite, dans
    /// l'ordre du registre.
    ///
    /// **Jamais moins de deux** : une question sans les lignes qui la
    /// posent est une accusation.
    pub lines: Vec<i64>,
    pub evidence: Evidence,
    /// Le jour de la dernière ligne en cause. Le tableau trie dessus.
    pub last_day: String,
}

/// Les seuils.
///
/// Livrés dans le code parce qu'ils **sont** la règle. Seul
/// `window_days` est une préférence et vit dans config.toml : une règle
/// avec une glissière est une règle qu'on tourne jusqu'à ce que plus
/// rien ne sorte.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub window_days: i64,
    /// Combien de délivrances antérieures avant de pouvoir parler.
    pub min_history: usize,
    /// En deçà de quelle part de sa propre médiane un retour est
    /// rapproché.
    pub interval_share: f64,
    /// En deçà de quelle part de la quantité précédente c'est un
    /// dépannage et non une question.
    pub min_share: f64,
    pub prescribers_min: usize,
    /// Combien de hausses consécutives font une pente.
    pub escalation_steps: usize,
    pub escalation_ratio: f64,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            window_days: 90,
            min_history: 3,
            interval_share: 0.60,
            min_share: 0.90,
            prescribers_min: 3,
            escalation_steps: 3,
            escalation_ratio: 1.50,
        }
    }
}

/// La clé sous laquelle deux graphies d'un même prescripteur se
/// rejoignent.
///
/// Ne réécrit rien — c'est l'esprit de `crate::classes` : un
/// référentiel **lit** les fiches et ne les corrige pas. Ici il n'y a
/// même pas de table, parce que les médecins d'une officine ne sont pas
/// connus du binaire.
///
/// La civilité ne tombe qu'en **tête** et qu'en **entier** : ailleurs
/// elle mangerait « Drion ».
///
/// # Sa limite, délibérée
///
/// « Dr Martin » et « Dr Martin Dupont » restent deux prescripteurs. Il
/// n'y a aucun moyen de les replier sans deviner que l'un est l'autre,
/// et c'est la seule chose qu'un nom ne doit jamais faire. Ce n'est pas
/// du code qui rattrape cela : la question porte les graphies **telles
/// que tapées**, et l'œil qui les voit côte à côte tranche en une
/// seconde.
pub fn prescriber_key(raw: &str) -> String {
    const CIVILITIES: &[&str] = &[
        "dr",
        "dr.",
        "docteur",
        "dre",
        "pr",
        "pr.",
        "professeur",
        "m",
        "m.",
        "mme",
    ];
    let folded = crate::fuzzy::sort_key(raw);
    let cleaned: String = folded
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let mut words = cleaned.split_whitespace().peekable();
    if let Some(first) = words.peek() {
        if CIVILITIES.contains(first) {
            words.next();
        }
    }
    words.collect::<Vec<_>>().join(" ")
}

/// Combien de délivrances ne portent pas de dossier.
///
/// Une vigilance aveugle doit dire de combien elle est aveugle.
pub fn unfiled(moves: &[Dispensing]) -> usize {
    moves
        .iter()
        .filter(|d| !d.cancelled && d.patient_id == 0)
        .count()
}

/// Combien de produits sont hors vigilance faute de plafond.
///
/// Un produit inscrit à la main n'a pas de famille, donc pas de
/// `max_days`, donc aucun des silences ne peut jouer et le module se
/// tait. La vue écrit ce nombre sur la porte, comme l'explorateur écrit
/// l'effectif de chaque axe : sans lui, un trou se lit « rien à
/// signaler ».
pub fn unwatched(moves: &[Dispensing]) -> usize {
    let mut seen: Vec<i64> = moves
        .iter()
        .filter(|d| !d.cancelled && d.max_days <= 0)
        .map(|d| d.stup_id)
        .collect();
    seen.sort_unstable();
    seen.dedup();
    seen.len()
}

/// Les questions que le registre pose, une par dossier et par produit,
/// sous le motif le plus grave.
///
/// Même discipline que `crate::ordonnancier::to_check` : un sujet ne
/// paraît qu'une fois, et c'est la question la plus urgente qui sort.
pub fn findings(moves: &[Dispensing], today: &str, rules: &Rules) -> Vec<Finding> {
    // Une ligne annulée n'a pas eu lieu ; une ligne sans dossier ne
    // s'attribue à personne et se compte dans `unfiled`.
    let mut kept: Vec<&Dispensing> = moves
        .iter()
        .filter(|d| !d.cancelled && d.patient_id != 0)
        .collect();
    // L'ordre du registre : les jours, puis la saisie.
    kept.sort_by(|a, b| a.day.cmp(b.day).then(a.seq.cmp(&b.seq)));

    let mut subjects: Vec<(i64, i64)> = kept.iter().map(|d| (d.patient_id, d.stup_id)).collect();
    subjects.sort_unstable();
    subjects.dedup();

    let mut out: Vec<Finding> = Vec::new();
    for (patient_id, stup_id) in subjects {
        let group: Vec<&Dispensing> = kept
            .iter()
            .copied()
            .filter(|d| d.patient_id == patient_id && d.stup_id == stup_id)
            .collect();
        if let Some(f) = judge(&group, today, rules) {
            out.push(f);
        }
    }
    // Le plus urgent d'abord, puis le plus récent, puis le dossier —
    // pour que deux lectures du même registre donnent le même tableau.
    out.sort_by(|a, b| {
        a.signal
            .rank()
            .cmp(&b.signal.rank())
            .then(b.last_day.cmp(&a.last_day))
            .then(a.patient_id.cmp(&b.patient_id))
            .then(a.stup_id.cmp(&b.stup_id))
    });
    out
}

/// Ce qu'une série de délivrances d'un dossier pour un produit dit
/// d'elle-même : la question la plus grave, ou rien.
fn judge(group: &[&Dispensing], today: &str, rules: &Rules) -> Option<Finding> {
    let mut found: Vec<Finding> = Vec::new();
    if let Some(f) = rapprochement(group, today, rules) {
        found.push(f);
    }
    if let Some(f) = prescripteurs(group, today, rules) {
        found.push(f);
    }
    if let Some(f) = escalade(group, today, rules) {
        found.push(f);
    }
    found.sort_by_key(|f| f.signal.rank());
    found.into_iter().next()
}

/// La dernière délivrance est-elle dans la fenêtre ?
///
/// La fenêtre décide de ce qui peut **se déclencher** ; l'historique
/// entier, lui, nourrit la référence. Un dossier clos l'an dernier ne
/// pose plus de question aujourd'hui, mais ses intervalles d'alors
/// comptent encore pour dire quelle était sa cadence.
fn within(day: &str, today: &str, window_days: i64) -> bool {
    crate::date::days_between(day, today).is_some_and(|d| (0..=window_days).contains(&d))
}

fn median(mut values: Vec<i64>) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let n = values.len();
    Some(if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2
    })
}

fn median_f(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    Some(if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    })
}

fn head(d: &Dispensing, signal: Signal, lines: Vec<i64>, evidence: Evidence) -> Finding {
    Finding {
        patient_id: d.patient_id,
        stup_id: d.stup_id,
        label: d.label.to_owned(),
        unit: d.unit.to_owned(),
        signal,
        lines,
        evidence,
        last_day: d.day.to_owned(),
    }
}

/// Revenu bien plus tôt que d'habitude, et pour autant.
fn rapprochement(group: &[&Dispensing], today: &str, rules: &Rules) -> Option<Finding> {
    let last = *group.last()?;
    // Le plafond : sans lui, un des deux silences manque, et le module
    // ne parle pas.
    if last.max_days <= 0 || !within(last.day, today, rules.window_days) {
        return None;
    }
    // Assez d'histoire pour avoir une cadence à soi.
    if group.len() < rules.min_history + 1 {
        return None;
    }
    let prev = *group.get(group.len() - 2)?;
    let days = crate::date::days_between(prev.day, last.day)?;
    if days < 0 || days >= last.max_days {
        return None;
    }
    // Un dépannage n'est pas une question : c'est le retour *pour
    // autant* qui en est une.
    if last.quantity < prev.quantity * rules.min_share {
        return None;
    }
    // La médiane des intervalles **d'avant** — celui qu'on juge n'entre
    // pas dans sa propre référence.
    let earlier: Vec<i64> = group[..group.len() - 1]
        .windows(2)
        .filter_map(|w| crate::date::days_between(w[0].day, w[1].day))
        .filter(|d| *d >= 0)
        .collect();
    let usual = median(earlier)?;
    // Deux délivrances le même jour donnent `days == 0` : c'est une
    // question, et jamais un débit — rien n'est divisé ici.
    if (days as f64) > usual as f64 * rules.interval_share {
        return None;
    }
    Some(head(
        last,
        Signal::Rapprochement,
        vec![prev.seq, last.seq],
        Evidence::Interval {
            days,
            usual,
            ceiling: last.max_days,
            quantity: last.quantity,
        },
    ))
}

/// Plusieurs prescripteurs pour le même produit sur la fenêtre.
///
/// Trois, et non deux : deux s'explique couramment — un remplaçant l'été,
/// une sortie d'hôpital. Trois est le plus petit nombre qui ne s'explique
/// pas de lui-même.
fn prescripteurs(group: &[&Dispensing], today: &str, rules: &Rules) -> Option<Finding> {
    let inside: Vec<&&Dispensing> = group
        .iter()
        .filter(|d| within(d.day, today, rules.window_days))
        .collect();
    let mut keys: Vec<String> = Vec::new();
    let mut spellings: Vec<String> = Vec::new();
    let mut lines: Vec<i64> = Vec::new();
    for d in &inside {
        let key = prescriber_key(d.prescriber);
        if key.is_empty() {
            continue;
        }
        if !keys.contains(&key) {
            keys.push(key);
            spellings.push(d.prescriber.trim().to_owned());
        }
        lines.push(d.seq);
    }
    if keys.len() < rules.prescribers_min {
        return None;
    }
    let last = **inside.last()?;
    Some(head(
        last,
        Signal::Prescripteurs,
        lines,
        Evidence::Prescribers {
            spellings,
            window_days: rules.window_days,
        },
    ))
}

/// Le débit délivré monte, délivrance après délivrance.
///
/// # Le trou de cette règle, et pourquoi on ne le bouche pas
///
/// Une gélule par jour n'est pas un milligramme par jour. Un patient
/// passé de Skenan LP 30 à Skenan LP 60 est **deux produits**, deux
/// `stup_id`, et cette règle y voit deux séries plates.
///
/// **On ne corrige pas cela en lisant les milligrammes du libellé pour
/// les additionner.** À l'intérieur d'une famille ce serait déjà
/// discutable ; entre familles c'est de l'équianalgésie, et les notes du
/// catalogue existent précisément parce que c'est là qu'on se trompe.
/// La réponse est un rapprochement **de vue et non de règle** : quand on
/// ouvre une question, on montre les autres produits suivis de la même
/// famille pour ce dossier, et l'œil voit le changement. Le module n'en
/// calcule rien.
fn escalade(group: &[&Dispensing], today: &str, rules: &Rules) -> Option<Finding> {
    let last = *group.last()?;
    if !within(last.day, today, rules.window_days) {
        return None;
    }
    // Un débit par paire consécutive : la quantité de l'arrivée sur les
    // jours écoulés depuis la précédente. Les paires du même jour n'ont
    // pas de débit — on ne divise jamais par zéro.
    let mut rates: Vec<(f64, i64, i64)> = Vec::new();
    for w in group.windows(2) {
        let Some(days) = crate::date::days_between(w[0].day, w[1].day) else {
            continue;
        };
        if days < 1 {
            continue;
        }
        rates.push((w[1].quantity / days as f64, w[0].seq, w[1].seq));
    }
    if rates.len() <= rules.escalation_steps {
        return None;
    }
    let tail = &rates[rates.len() - rules.escalation_steps..];
    // Une pente, et non une bosse : strictement croissante.
    if !tail.windows(2).all(|p| p[1].0 > p[0].0) {
        return None;
    }
    let usual = median_f(
        rates[..rates.len() - rules.escalation_steps]
            .iter()
            .map(|r| r.0)
            .collect(),
    )?;
    let top = tail.last()?.0;
    if usual <= 0.0 || top < usual * rules.escalation_ratio {
        return None;
    }
    let mut lines: Vec<i64> = Vec::new();
    for (_, from, to) in tail {
        if !lines.contains(from) {
            lines.push(*from);
        }
        if !lines.contains(to) {
            lines.push(*to);
        }
    }
    Some(head(
        last,
        Signal::Escalade,
        lines,
        Evidence::Rate {
            rates: tail.iter().map(|r| r.0).collect(),
            usual,
        },
    ))
}

/// Ce que cette officine a délivré sous chaque nom, une année.
#[derive(Clone, Debug, PartialEq)]
pub struct PrescriberYear {
    pub key: String,
    /// La graphie la plus fréquente. Ce qui s'affiche — jamais une
    /// réécriture de ce que le registre porte.
    pub shown: String,
    /// Toutes les graphies repliées ici, pour que l'œil reconnaisse.
    pub spellings: Vec<String>,
    pub year: i64,
    pub lines: usize,
    /// Combien de dossiers distincts. Ni un classement, ni un indice.
    pub patients: usize,
    /// Par produit : identifiant, libellé, quantité totale.
    pub by_product: Vec<(i64, String, f64)>,
}

/// Les volumes de l'année, par prescripteur.
///
/// # Ce que ce tableau dit honnêtement : rien sur le prescripteur
///
/// Il dit combien de chaque produit est sorti **de cette officine** sur
/// des ordonnances portant ce nom, tel qu'il a été tapé. Il ignore
/// l'activité de ce médecin ailleurs — les patients vont dans d'autres
/// pharmacies —, il ignore sa patientèle : un médecin de soins
/// palliatifs ou un CSAPA sera en tête, correctement et innocemment, et
/// le nom est une chaîne qu'un opérateur a saisie, qui peut désigner
/// plusieurs personnes.
///
/// Il est donc rendu **par ordre alphabétique et non par volume**, et il
/// ne porte aucun rang ni aucun poids : un tableau trié par volume est
/// un classement de suspects. La vue peut offrir le tri ; le module ne
/// le choisit pas.
pub fn prescriber_year(moves: &[Dispensing], year: i64) -> Vec<PrescriberYear> {
    let mut out: Vec<PrescriberYear> = Vec::new();
    for d in moves.iter().filter(|d| !d.cancelled) {
        let Some((y, _, _)) = crate::date::parse_iso(d.day) else {
            continue;
        };
        if y != year {
            continue;
        }
        let key = prescriber_key(d.prescriber);
        if key.is_empty() {
            continue;
        }
        let raw = d.prescriber.trim().to_owned();
        let slot = match out.iter_mut().find(|p| p.key == key) {
            Some(p) => p,
            None => {
                out.push(PrescriberYear {
                    key: key.clone(),
                    shown: raw.clone(),
                    spellings: Vec::new(),
                    year,
                    lines: 0,
                    patients: 0,
                    by_product: Vec::new(),
                });
                out.last_mut().expect("on vient de l'ajouter")
            }
        };
        slot.lines += 1;
        if !slot.spellings.contains(&raw) {
            slot.spellings.push(raw);
        }
        match slot
            .by_product
            .iter_mut()
            .find(|(id, _, _)| *id == d.stup_id)
        {
            Some(p) => p.2 += d.quantity,
            None => slot
                .by_product
                .push((d.stup_id, d.label.to_owned(), d.quantity)),
        }
    }
    // Les dossiers distincts, et la graphie la plus fréquente.
    for p in &mut out {
        let mut files: Vec<i64> = moves
            .iter()
            .filter(|d| {
                !d.cancelled
                    && d.patient_id != 0
                    && prescriber_key(d.prescriber) == p.key
                    && crate::date::parse_iso(d.day).is_some_and(|(y, _, _)| y == year)
            })
            .map(|d| d.patient_id)
            .collect();
        files.sort_unstable();
        files.dedup();
        p.patients = files.len();
        p.shown = p
            .spellings
            .iter()
            .max_by_key(|s| {
                moves
                    .iter()
                    .filter(|d| !d.cancelled && d.prescriber.trim() == s.as_str())
                    .count()
            })
            .cloned()
            .unwrap_or_default();
        p.by_product.sort_by(|a, b| a.1.cmp(&b.1));
    }
    out.sort_by(|a, b| a.shown.cmp(&b.shown).then(a.key.cmp(&b.key)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une délivrance de test : le reste est constant, seul varie ce que
    /// la règle regarde.
    fn d<'a>(seq: i64, day: &'a str, qty: f64, who: &'a str, patient: i64) -> Dispensing<'a> {
        Dispensing {
            seq,
            stup_id: 1,
            label: "Skenan LP 30 mg",
            unit: "gélule",
            max_days: 28,
            patient_id: patient,
            prescriber: who,
            day,
            quantity: qty,
            cancelled: false,
        }
    }

    /// Le module refuse de parler d'un dossier qu'il vient de
    /// rencontrer, et sait dire ce qu'est la cadence d'un dossier
    /// qu'il connaît.
    #[test]
    fn a_question_is_never_asked_of_a_file_the_register_has_only_just_met() {
        let today = "2026-04-05";
        let rules = Rules::default();

        // Deux délivrances : aucune cadence, donc rien à dire, même
        // rapprochées.
        let two = [
            d(1, "2026-03-28", 14.0, "Dr Martin", 7),
            d(2, "2026-04-02", 14.0, "Dr Martin", 7),
        ];
        assert!(findings(&two, today, &rules).is_empty());

        // Quatre à vingt-huit jours, puis un retour au neuvième.
        let steady = [
            d(1, "2026-01-01", 14.0, "Dr Martin", 7),
            d(2, "2026-01-29", 14.0, "Dr Martin", 7),
            d(3, "2026-02-26", 14.0, "Dr Martin", 7),
            d(4, "2026-03-26", 14.0, "Dr Martin", 7),
            d(5, "2026-04-04", 14.0, "Dr Martin", 7),
        ];
        let out = findings(&steady, today, &rules);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].signal, Signal::Rapprochement);
        assert_eq!(out[0].patient_id, 7);
        match &out[0].evidence {
            Evidence::Interval {
                days,
                usual,
                ceiling,
                ..
            } => {
                assert_eq!(*days, 9);
                assert_eq!(
                    *usual, 28,
                    "la cadence du dossier, pas une moyenne du monde"
                );
                assert_eq!(*ceiling, 28);
            }
            other => panic!("mauvaise preuve : {other:?}"),
        }
    }

    /// Le plafond de la famille fait taire la question qu'il ne peut pas
    /// trancher — et un produit sans plafond n'en pose aucune.
    #[test]
    fn the_ceiling_of_the_family_silences_the_question_it_cannot_answer() {
        let today = "2026-07-05";
        let rules = Rules::default();

        // Un dossier dont l'habitude est de soixante jours et qui revient
        // au trentième : rapproché *pour lui*, mais au-delà du plafond de
        // vingt-huit, où deux ordonnances ne peuvent pas se chevaucher.
        // Le module se tait : il ne sait pas ce qu'il en est.
        let slow = [
            d(1, "2026-01-02", 14.0, "Dr Martin", 7),
            d(2, "2026-03-03", 14.0, "Dr Martin", 7),
            d(3, "2026-05-02", 14.0, "Dr Martin", 7),
            d(4, "2026-07-01", 14.0, "Dr Martin", 7),
            d(5, "2026-07-31", 14.0, "Dr Martin", 7),
        ];
        assert!(findings(&slow, "2026-08-01", &rules).is_empty());

        // Et un produit dont la famille n'est pas renseignée est hors
        // vigilance : aucune question, jamais, et il se compte.
        let mut loose = [
            d(1, "2026-04-01", 14.0, "Dr Martin", 7),
            d(2, "2026-04-29", 14.0, "Dr Martin", 7),
            d(3, "2026-05-27", 14.0, "Dr Martin", 7),
            d(4, "2026-06-24", 14.0, "Dr Martin", 7),
            d(5, "2026-07-03", 14.0, "Dr Martin", 7),
        ];
        for m in &mut loose {
            m.max_days = 0;
        }
        assert!(findings(&loose, today, &rules).is_empty());
        assert_eq!(unwatched(&loose), 1, "et la vue doit pouvoir le dire");
    }

    /// Un dépannage entre deux délivrances pleines n'est pas une
    /// question : c'est le retour *pour autant* qui en est une.
    #[test]
    fn a_small_top_up_between_two_full_deliveries_is_not_a_question() {
        let rules = Rules::default();
        let mut rows = [
            d(1, "2026-01-01", 14.0, "Dr Martin", 7),
            d(2, "2026-01-29", 14.0, "Dr Martin", 7),
            d(3, "2026-02-26", 14.0, "Dr Martin", 7),
            d(4, "2026-03-26", 14.0, "Dr Martin", 7),
            d(5, "2026-04-04", 3.0, "Dr Martin", 7),
        ];
        assert!(
            findings(&rows, "2026-04-05", &rules).is_empty(),
            "trois gélules ne sont pas une ordonnance de plus"
        );
        // La même chose pour autant : la question revient.
        rows[4].quantity = 14.0;
        assert_eq!(findings(&rows, "2026-04-05", &rules).len(), 1);
    }

    /// Deux délivrances le même jour sont une question, et jamais un
    /// débit : rien n'est divisé par zéro.
    #[test]
    fn two_deliveries_on_one_day_are_a_question_and_never_a_rate() {
        let rules = Rules::default();
        let rows = [
            d(1, "2026-01-01", 14.0, "Dr Martin", 7),
            d(2, "2026-01-29", 14.0, "Dr Martin", 7),
            d(3, "2026-02-26", 14.0, "Dr Martin", 7),
            d(4, "2026-03-26", 14.0, "Dr Martin", 7),
            d(5, "2026-03-26", 14.0, "Dr Martin", 7),
        ];
        let out = findings(&rows, "2026-03-27", &rules);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].signal, Signal::Rapprochement);
        match &out[0].evidence {
            Evidence::Interval { days, .. } => assert_eq!(*days, 0),
            other => panic!("mauvaise preuve : {other:?}"),
        }
    }

    /// Deux graphies d'un même médecin sont un prescripteur ; trois
    /// médecins sont une question. Et la limite du repliement est dite.
    #[test]
    fn two_spellings_of_one_doctor_are_one_prescriber_and_three_doctors_are_a_question() {
        assert_eq!(prescriber_key("Dr Martin"), "martin");
        assert_eq!(prescriber_key("MARTIN"), "martin");
        assert_eq!(prescriber_key("Dr. Martin"), "martin");
        assert_eq!(prescriber_key("docteur martin"), "martin");
        // La civilité ne tombe qu'en tête et qu'en entier.
        assert_eq!(prescriber_key("Drion"), "drion");
        assert_eq!(prescriber_key("Dr Drion"), "drion");
        // **La limite, assumée** : on ne devine pas qu'un nom est
        // l'autre. Les deux graphies voyagent avec la question, et c'est
        // l'œil qui tranche.
        assert_ne!(
            prescriber_key("Dr Martin"),
            prescriber_key("Dr Martin Dupont")
        );

        let rules = Rules::default();
        let two = [
            d(1, "2026-03-02", 14.0, "Dr Martin", 7),
            d(2, "2026-03-30", 14.0, "MARTIN", 7),
            d(3, "2026-04-27", 14.0, "Dr Lemoine", 7),
        ];
        assert!(
            findings(&two, "2026-04-28", &rules).is_empty(),
            "deux prescripteurs s'expliquent : un remplaçant, une sortie d'hôpital"
        );

        let three = [
            d(1, "2026-03-02", 14.0, "Dr Martin", 7),
            d(2, "2026-03-30", 14.0, "Dr Lemoine", 7),
            d(3, "2026-04-27", 14.0, "Dr Sow", 7),
        ];
        let out = findings(&three, "2026-04-28", &rules);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].signal, Signal::Prescripteurs);
        match &out[0].evidence {
            Evidence::Prescribers { spellings, .. } => {
                assert_eq!(spellings.len(), 3);
                assert!(
                    spellings.contains(&"Dr Martin".to_owned()),
                    "les graphies sont rendues telles que tapées : {spellings:?}"
                );
            }
            other => panic!("mauvaise preuve : {other:?}"),
        }
    }

    /// Une escalade demande une pente, et non une bosse.
    #[test]
    fn an_escalation_needs_a_slope_and_not_a_bump() {
        let rules = Rules::default();
        // Série plate avec une seule bosse : rien.
        let bump = [
            d(1, "2026-01-01", 14.0, "Dr Martin", 7),
            d(2, "2026-01-29", 14.0, "Dr Martin", 7),
            d(3, "2026-02-26", 28.0, "Dr Martin", 7),
            d(4, "2026-03-26", 14.0, "Dr Martin", 7),
            d(5, "2026-04-23", 14.0, "Dr Martin", 7),
        ];
        let out = findings(&bump, "2026-04-24", &rules);
        assert!(out.iter().all(|f| f.signal != Signal::Escalade), "{out:?}");

        // Trois hausses consécutives, au-delà d'une fois et demie la
        // médiane d'avant.
        let slope = [
            d(1, "2026-01-01", 14.0, "Dr Martin", 7),
            d(2, "2026-01-29", 14.0, "Dr Martin", 7),
            d(3, "2026-02-26", 14.0, "Dr Martin", 7),
            d(4, "2026-03-26", 22.0, "Dr Martin", 7),
            d(5, "2026-04-23", 30.0, "Dr Martin", 7),
            d(6, "2026-05-21", 45.0, "Dr Martin", 7),
        ];
        let out = findings(&slope, "2026-05-22", &rules);
        let esc = out
            .iter()
            .find(|f| f.signal == Signal::Escalade)
            .expect("la pente doit se voir");
        match &esc.evidence {
            Evidence::Rate { rates, usual } => {
                assert_eq!(rates.len(), 3);
                assert!(
                    rates.windows(2).all(|p| p[1] > p[0]),
                    "les débits montent : {rates:?}"
                );
                assert!(*usual > 0.0);
            }
            other => panic!("mauvaise preuve : {other:?}"),
        }
    }

    /// Une délivrance annulée n'a pas eu lieu, et ne fait pas un motif.
    #[test]
    fn a_cancelled_delivery_is_not_a_pattern() {
        let rules = Rules::default();
        let mut rows = [
            d(1, "2026-01-01", 14.0, "Dr Martin", 7),
            d(2, "2026-01-29", 14.0, "Dr Martin", 7),
            d(3, "2026-02-26", 14.0, "Dr Martin", 7),
            d(4, "2026-03-26", 14.0, "Dr Martin", 7),
            d(5, "2026-04-04", 14.0, "Dr Martin", 7),
        ];
        assert_eq!(findings(&rows, "2026-04-05", &rules).len(), 1);
        // La ligne de trop est annulée : la question disparaît avec
        // elle, sans que rien d'autre ne bouge.
        rows[4].cancelled = true;
        assert!(findings(&rows, "2026-04-05", &rules).is_empty());
    }

    /// Un dossier ne paraît qu'une fois, sous le motif le plus grave.
    #[test]
    fn a_file_never_appears_twice_in_the_list() {
        let rules = Rules::default();
        // De quoi déclencher le rapprochement *et* les prescripteurs.
        let rows = [
            d(1, "2026-01-01", 14.0, "Dr Martin", 7),
            d(2, "2026-01-29", 14.0, "Dr Lemoine", 7),
            d(3, "2026-02-26", 14.0, "Dr Sow", 7),
            d(4, "2026-03-26", 14.0, "Dr Martin", 7),
            d(5, "2026-04-04", 14.0, "Dr Martin", 7),
        ];
        let out = findings(&rows, "2026-04-05", &rules);
        assert_eq!(out.len(), 1, "un sujet, une ligne : {out:?}");
        assert_eq!(
            out[0].signal,
            Signal::Rapprochement,
            "et c'est le plus urgent qui sort"
        );
    }

    /// La fenêtre décide de ce qui peut se déclencher ; l'historique
    /// entier nourrit la référence.
    #[test]
    fn a_file_closed_last_year_is_not_a_question_today() {
        let rules = Rules::default();
        let rows = [
            d(1, "2025-01-01", 14.0, "Dr Martin", 7),
            d(2, "2025-01-29", 14.0, "Dr Martin", 7),
            d(3, "2025-02-26", 14.0, "Dr Martin", 7),
            d(4, "2025-03-26", 14.0, "Dr Martin", 7),
            d(5, "2025-04-04", 14.0, "Dr Martin", 7),
        ];
        // La même série, un an plus tard : plus rien à signaler.
        assert!(findings(&rows, "2026-04-05", &rules).is_empty());
        // Mais lue en son temps, elle parlait.
        assert_eq!(findings(&rows, "2025-04-05", &rules).len(), 1);
    }

    /// Chaque signal pose une question plutôt que d'énoncer un fait.
    ///
    /// C'est le cliquet du module : le jour où quelqu'un écrira
    /// « mésusage probable » dans cette table, ce test tombera.
    #[test]
    fn every_signal_asks_a_question_rather_than_stating_one() {
        for s in Signal::ALL {
            let text = crate::strings::tr(s.question_key());
            assert!(!text.is_empty(), "{} ne rend rien", s.question_key());
            assert!(
                text.trim_end().ends_with('?'),
                "« {text} » énonce au lieu de demander"
            );
        }
        // Et les trois rangs sont distincts, sans quoi l'ordre du
        // tableau dépendrait de l'ordre d'écriture des règles.
        let mut ranks: Vec<u8> = Signal::ALL.iter().map(|s| s.rank()).collect();
        ranks.sort_unstable();
        ranks.dedup();
        assert_eq!(ranks.len(), Signal::ALL.len());
    }

    /// Une question porte les lignes qui la posent, et jamais moins de
    /// deux.
    #[test]
    fn a_finding_carries_the_lines_that_produced_it_and_never_fewer_than_two() {
        let rules = Rules::default();
        let rows = [
            d(1, "2026-01-01", 14.0, "Dr Martin", 7),
            d(2, "2026-01-29", 14.0, "Dr Martin", 7),
            d(3, "2026-02-26", 14.0, "Dr Martin", 7),
            d(4, "2026-03-26", 14.0, "Dr Martin", 7),
            d(5, "2026-04-04", 14.0, "Dr Martin", 7),
            d(6, "2026-03-02", 14.0, "Dr Lemoine", 9),
            d(7, "2026-03-30", 14.0, "Dr Sow", 9),
            d(8, "2026-04-01", 14.0, "Dr Martin", 9),
        ];
        let out = findings(&rows, "2026-04-05", &rules);
        assert!(!out.is_empty());
        for f in &out {
            assert!(
                f.lines.len() >= 2,
                "une question sans ses lignes est une accusation : {f:?}"
            );
            for seq in &f.lines {
                let row = rows
                    .iter()
                    .find(|r| r.seq == *seq)
                    .expect("la ligne citée doit exister");
                assert_eq!(row.patient_id, f.patient_id);
                assert_eq!(row.stup_id, f.stup_id);
            }
        }
    }

    /// Le tableau des prescripteurs est rendu par nom et non par volume.
    #[test]
    fn the_prescriber_table_is_ordered_by_name_and_not_by_volume() {
        let rows = [
            d(1, "2026-01-01", 90.0, "Dr Zola", 7),
            d(2, "2026-01-29", 90.0, "ZOLA", 8),
            d(3, "2026-02-26", 5.0, "Dr Abadie", 9),
            d(4, "2025-12-01", 40.0, "Dr Abadie", 9),
        ];
        let table = prescriber_year(&rows, 2026);
        assert_eq!(table.len(), 2);
        assert_eq!(
            table[0].shown, "Dr Abadie",
            "l'ordre est alphabétique, pas celui du volume"
        );
        assert_eq!(table[1].key, "zola");
        assert_eq!(table[1].lines, 2);
        assert_eq!(table[1].patients, 2, "deux dossiers distincts");
        assert_eq!(
            table[1].spellings.len(),
            2,
            "les deux graphies restent lisibles : {:?}",
            table[1].spellings
        );
        // L'année précédente n'entre pas dans celle-ci.
        assert_eq!(table[0].lines, 1);
    }

    /// Deux lectures du même registre donnent le même tableau.
    ///
    /// La vue le repeint soixante fois par seconde ; un ordre qui
    /// dépendrait d'un parcours de table sauterait sous les yeux.
    #[test]
    fn the_same_register_read_twice_says_the_same_thing() {
        let rules = Rules::default();
        let rows = [
            d(1, "2026-01-01", 14.0, "Dr Martin", 7),
            d(2, "2026-01-29", 14.0, "Dr Martin", 7),
            d(3, "2026-02-26", 14.0, "Dr Martin", 7),
            d(4, "2026-03-26", 14.0, "Dr Martin", 7),
            d(5, "2026-04-04", 14.0, "Dr Martin", 7),
            d(6, "2026-03-02", 14.0, "Dr Lemoine", 9),
            d(7, "2026-03-30", 14.0, "Dr Sow", 9),
            d(8, "2026-04-01", 14.0, "Dr Martin", 9),
        ];
        let once = findings(&rows, "2026-04-05", &rules);
        let twice = findings(&rows, "2026-04-05", &rules);
        assert_eq!(once, twice);
        // Et l'ordre des lignes en entrée ne le change pas.
        let mut shuffled = rows;
        shuffled.reverse();
        assert_eq!(findings(&shuffled, "2026-04-05", &rules), once);
    }

    /// Une vigilance aveugle dit de combien elle est aveugle.
    #[test]
    fn a_blind_watch_says_how_blind_it_is() {
        let rows = [
            d(1, "2026-04-01", 14.0, "Dr Martin", 7),
            d(2, "2026-04-02", 14.0, "Dr Martin", 0),
            d(3, "2026-04-03", 14.0, "Dr Martin", 0),
        ];
        assert_eq!(unfiled(&rows), 2);
        // Une ligne annulée n'a pas eu lieu : elle ne manque pas non
        // plus.
        let mut with_cancel = rows;
        with_cancel[2].cancelled = true;
        assert_eq!(unfiled(&with_cancel), 1);
    }
}
