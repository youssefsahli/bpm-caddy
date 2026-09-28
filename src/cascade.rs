//! Récepteurs et cascades : un mécanisme d'action décrit en quelques
//! lignes, dessiné, et **joué** — ce qu'une molécule fait à la chaîne
//! qu'elle touche, et ce que la chaîne en fait avec le temps.
//!
//! La monographie dit « antagoniste des récepteurs bêta-1 » en une
//! phrase. Ce qu'elle ne peut pas montrer, c'est **le reste de la
//! chaîne** : que bloquer le récepteur baisse l'AMPc, donc l'entrée de
//! calcium, donc la fréquence *et* la contractilité ; qu'un agoniste
//! partiel donné sur un agoniste plein **baisse** l'effet ; qu'une
//! benzodiazépine ne fait rien sans GABA ; et qu'un récepteur longtemps
//! bloqué se multiplie, si bien que l'arrêt brutal donne un rebond. Ce
//! sont les questions du comptoir — « pourquoi ne pas arrêter d'un coup »
//! — et elles se lisent sur un dessin qui bouge bien mieux que dans une
//! phrase.
//!
//! Trois choses, pures et testées, sans egui et sans horloge :
//!
//! * [`parse`] lit la **description**, un texte que l'équipe écrit et
//!   réécrit (voir le langage plus bas). Le texte est la seule vérité :
//!   la base garde le texte, et rien d'autre.
//! * [`settle`] et [`run`] calculent l'**état** de la chaîne — chaque
//!   nœud rapporté à son état de repos, qui vaut 1 — pour un jeu de
//!   molécules données, puis au fil du temps quand un récepteur
//!   s'adapte.
//! * [`layout`] range les nœuds en **étages**, du ligand vers l'effet ;
//!   la vue met à l'échelle et peint.
//!
//! **Un modèle qualitatif, et il le dit.** Les constantes ci-dessous ne
//! sont pas des données cliniques : aucune ne sort d'une monographie et
//! aucune ne se lit comme un chiffre. Elles sont choisies pour que les
//! **sens de variation** soient justes — ce qui monte, ce qui baisse, ce
//! qui revient — et la vue ne montre jamais une valeur, seulement une
//! direction et une ampleur relative. Ni délai, ni dose : le temps est
//! en pas, sans unité.
//!
//! # Le langage
//!
//! Une instruction par ligne ; `#` commence un commentaire. Les mots-clés
//! se lisent sans casse ni accents (« recepteur » vaut « Récepteur »), et
//! un nom de nœud aussi.
//!
//! ```text
//! titre : Récepteur bêta-1 (cœur)
//! sujet : Bêtabloquants, dobutamine
//!
//! ligand Noradrénaline
//! récepteur Bêta-1 : couplé à la protéine Gs
//! relais Protéine Gs
//! enzyme Adénylate cyclase
//! messager AMPc
//! effet Fréquence cardiaque
//!
//! Noradrénaline -> Bêta-1 -> Protéine Gs -> Adénylate cyclase -> AMPc
//! AMPc -> Fréquence cardiaque, Contractilité
//! Acétylcholine -| Adénylate cyclase
//!
//! molécule bisoprolol : antagoniste Bêta-1
//! molécule dobutamine : agoniste Bêta-1
//! adaptation Bêta-1
//! ```
//!
//! * `titre :` et `sujet :` — ce que la liste montre ; `source :`, une
//!   référence par ligne, citée sous la figure.
//! * une **déclaration** : `ligand`, `récepteur`, `relais`, `enzyme`,
//!   `messager`, `canal`, `transporteur` ou `effet`, puis le nom, puis
//!   facultativement `: une note` que le survol montre.
//! * une **flèche** : `->` active, `-|` inhibe. Elles s'enchaînent sur
//!   une ligne, et une virgule en fait partir ou arriver plusieurs. Un
//!   nœud qu'on n'a pas déclaré est un relais.
//! * une **molécule** : `molécule <nom> : <action> <nœud>[, <nœud>…]`,
//!   plusieurs actions séparées par `;`. Les actions : `agoniste`,
//!   `agoniste partiel`, `agoniste partiel faible`, `antagoniste`,
//!   `inhibiteur`, `activateur`, `potentialisateur`.
//! * `adaptation <nœud>[, <nœud>…]` — le nœud règle sa densité contre
//!   ce qu'il reçoit : bloqué longtemps il se multiplie, stimulé
//!   longtemps il se raréfie. C'est ce qui fait la tolérance et le
//!   rebond.
//! * `scénario <nom> : <molécule> <début>-<fin> ; <molécule> <début>-` —
//!   une histoire toute prête, que la vue joue d'un clic : les pas où
//!   chaque molécule commence, et où elle s'arrête (rien : jusqu'au
//!   bout).
//! * `tonus faible <nœud>[, <nœud>…]` — le nœud est presque au repos à
//!   l'état de base : le bloquer ne retire presque rien, le stimuler
//!   ajoute tout. C'est le récepteur opioïde sans opioïde : la naloxone
//!   seule n'y fait rien, la naloxone sur la morphine la renverse.

use crate::fuzzy;

/// Ce qu'un nœud est. Ne change rien au calcul — un récepteur et un
/// enzyme se propagent de la même façon — mais tout au dessin : la
/// forme dit la nature avant qu'on lise le nom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Ligand,
    Recepteur,
    Relais,
    Enzyme,
    Messager,
    Canal,
    Transporteur,
    Effet,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::Ligand,
        Kind::Recepteur,
        Kind::Relais,
        Kind::Enzyme,
        Kind::Messager,
        Kind::Canal,
        Kind::Transporteur,
        Kind::Effet,
    ];

    /// Le mot qui le déclare, tel qu'on l'écrit.
    pub fn keyword(self) -> &'static str {
        match self {
            Kind::Ligand => "ligand",
            Kind::Recepteur => "récepteur",
            Kind::Relais => "relais",
            Kind::Enzyme => "enzyme",
            Kind::Messager => "messager",
            Kind::Canal => "canal",
            Kind::Transporteur => "transporteur",
            Kind::Effet => "effet",
        }
    }
}

/// Le sens d'une flèche.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sign {
    /// `->` : plus en amont, plus en aval.
    Active,
    /// `-|` : plus en amont, moins en aval.
    Inhibe,
}

/// Ce qu'une molécule fait au nœud qu'elle touche.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    /// Occupe le récepteur et le stimule au maximum.
    Agoniste,
    /// Occupe le récepteur et le stimule à moitié : plus que le repos
    /// seul, **moins** qu'un agoniste plein qu'il déplace.
    AgonistePartiel,
    /// Un agoniste partiel dont l'efficacité est **sous** le tonus de
    /// repos : il baisse l'effet, moins qu'un antagoniste — ce qu'on
    /// appelle chez un bêtabloquant une activité sympathomimétique
    /// intrinsèque.
    AgonistePartielFaible,
    /// Occupe le récepteur sans le stimuler : ce qui arrive d'amont ne
    /// passe plus.
    Antagoniste,
    /// Baisse l'activité du nœud, quoi qu'il reçoive — un enzyme, un
    /// canal, un transporteur.
    Inhibiteur,
    /// L'inverse : la monte, quoi qu'il reçoive.
    Activateur,
    /// Amplifie ce que le nœud reçoit **d'amont**, et rien d'autre :
    /// sans signal, il n'y a rien à amplifier.
    Potentialisateur,
}

impl Action {
    pub const ALL: [Action; 7] = [
        Action::Agoniste,
        Action::AgonistePartiel,
        Action::AgonistePartielFaible,
        Action::Antagoniste,
        Action::Inhibiteur,
        Action::Activateur,
        Action::Potentialisateur,
    ];

    /// Le mot qui l'écrit dans la description.
    pub fn keyword(self) -> &'static str {
        match self {
            Action::Agoniste => "agoniste",
            Action::AgonistePartiel => "agoniste partiel",
            Action::AgonistePartielFaible => "agoniste partiel faible",
            Action::Antagoniste => "antagoniste",
            Action::Inhibiteur => "inhibiteur",
            Action::Activateur => "activateur",
            Action::Potentialisateur => "potentialisateur",
        }
    }

    /// Occupe-t-elle le site du ligand ? Celles-là se **partagent** le
    /// récepteur entre elles et avec le ligand ; les autres agissent
    /// par-dessus.
    pub fn occupies(self) -> bool {
        matches!(
            self,
            Action::Agoniste
                | Action::AgonistePartiel
                | Action::AgonistePartielFaible
                | Action::Antagoniste
        )
    }

    /// Ce qui la fait monter l'activité du nœud, ou la baisser — le sens
    /// de la marque qu'on peint sur la flèche de la molécule.
    pub fn raises(self) -> bool {
        matches!(
            self,
            Action::Agoniste
                | Action::AgonistePartiel
                | Action::Activateur
                | Action::Potentialisateur
        )
    }
}

/// Un nœud de la chaîne.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub kind: Kind,
    /// Ce qu'on a écrit après le `:` de la déclaration.
    pub note: String,
    /// Règle-t-il sa densité ? Voir `adaptation`.
    pub adapts: bool,
    /// Presque rien au repos : voir `tonus faible`.
    pub low_tone: bool,
    /// Déclaré explicitement, ou seulement nommé par une flèche.
    pub declared: bool,
}

/// Une flèche, d'un nœud à l'autre (leurs indices dans `nodes`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub sign: Sign,
}

/// Une molécule et ce qu'elle fait.
#[derive(Clone, Debug, PartialEq)]
pub struct Molecule {
    pub name: String,
    pub acts: Vec<(Action, usize)>,
}

/// Ce que le texte n'a pas pu dire. Le numéro est celui de la ligne,
/// en partant de 1 — celui que l'éditeur montre dans sa marge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// Une ligne qui ne commence par aucun mot connu et ne porte pas de
    /// flèche.
    Unreadable { line: usize },
    /// Une déclaration, une molécule ou une adaptation sans nom.
    MissingName { line: usize },
    /// Un nœud déclaré deux fois sous deux natures.
    Redeclared { line: usize, name: String },
    /// Une flèche d'un nœud vers lui-même.
    SelfLoop { line: usize, name: String },
    /// Une flèche dont un bout est vide : « A -> ».
    DanglingArrow { line: usize },
    /// Une molécule sans `:`, ou une action que la liste ne connaît pas.
    UnknownAction { line: usize, text: String },
    /// Une molécule ou une adaptation qui nomme un nœud qu'aucune ligne
    /// ne déclare ni ne relie.
    UnknownNode { line: usize, name: String },
    /// Un scénario dont une prise ne se lit pas : ni `molécule 10-70` ni
    /// `molécule 10-`.
    BadScenario { line: usize, text: String },
    /// Un scénario qui nomme une molécule que la cascade ne déclare pas.
    UnknownMolecule { line: usize, name: String },
}

impl Fault {
    pub fn line(&self) -> usize {
        match self {
            Fault::Unreadable { line }
            | Fault::MissingName { line }
            | Fault::Redeclared { line, .. }
            | Fault::SelfLoop { line, .. }
            | Fault::DanglingArrow { line }
            | Fault::UnknownAction { line, .. }
            | Fault::UnknownNode { line, .. }
            | Fault::BadScenario { line, .. }
            | Fault::UnknownMolecule { line, .. } => *line,
        }
    }
}

/// Un scénario : un nom, et des prises — une molécule, d'un pas à un
/// autre ou jusqu'au bout. « Arrêt brutal du bisoprolol : bisoprolol
/// 10-70 » se joue d'un clic, et c'est ce que la vue existe pour
/// montrer.
#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
    pub name: String,
    pub doses: Vec<Dose>,
}

/// Une description lue.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cascade {
    pub title: String,
    pub subject: String,
    /// Les références, une par ligne `source :` — ce que la vue cite
    /// sous la figure.
    pub sources: Vec<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub molecules: Vec<Molecule>,
    /// Les histoires toutes prêtes : des prises, qu'un clic joue.
    pub scenarios: Vec<Scenario>,
    pub faults: Vec<Fault>,
}

impl Cascade {
    /// L'indice du nœud de ce nom, sans casse ni accents.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.nodes
            .iter()
            .position(|n| fuzzy::eq_folded(&n.name, name))
    }

    /// Les nœuds dont on suit l'évolution : les effets déclarés, et à
    /// défaut les nœuds où la chaîne s'arrête. Une description sans
    /// `effet` se lit quand même, par ses bouts.
    pub fn outcomes(&self) -> Vec<usize> {
        let effects: Vec<usize> = (0..self.nodes.len())
            .filter(|&i| self.nodes[i].kind == Kind::Effet)
            .collect();
        if !effects.is_empty() {
            return effects;
        }
        (0..self.nodes.len())
            .filter(|&i| self.edges.iter().all(|e| e.from != i))
            .filter(|&i| self.edges.iter().any(|e| e.to == i))
            .collect()
    }

    /// Les molécules qui portent ce nom de fiche — la porte depuis une
    /// monographie. Sans casse ni accents, et sur le nom entier : un
    /// morceau de nom en attraperait un autre (« ipp » est dans
    /// « grippe »).
    pub fn names_molecule(&self, dci: &str) -> bool {
        self.molecules
            .iter()
            .any(|m| fuzzy::eq_folded(&m.name, dci))
    }
}

/// Le mot `word` ouvre-t-il `line` — suivi d'un blanc, d'un `:` ou de
/// rien ? Rend le reste, sans ses blancs de tête.
fn keyword<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    let n = word.chars().count();
    let mut end = line.len();
    let mut taken = 0;
    for (i, _) in line.char_indices() {
        if taken == n {
            end = i;
            break;
        }
        taken += 1;
    }
    if taken < n {
        return None;
    }
    let head = &line[..end];
    if !fuzzy::eq_folded(head, word) {
        return None;
    }
    let rest = &line[end..];
    match rest.chars().next() {
        None => Some(""),
        Some(c) if c.is_whitespace() || c == ':' => Some(rest.trim_start()),
        Some(_) => None,
    }
}

/// Le mot `word` ouvre-t-il cette ligne **en instruction** ? Sur une
/// ligne qui porte une flèche, seulement s'il est suivi de son `:` —
/// sans quoi c'est le début d'un nom : « Tonus vagal -| Fréquence
/// cardiaque », « Source de calcium -> Contraction » sont des flèches,
/// et les lire comme des instructions les perdait.
fn heads<'a>(body: &'a str, word: &str) -> Option<&'a str> {
    let rest = keyword(body, word)?;
    if arrows(body).is_some() && !rest.starts_with(':') {
        return None;
    }
    Some(rest)
}

/// Le nom d'une déclaration et sa note.
fn name_and_note(rest: &str) -> (String, String) {
    match rest.split_once(':') {
        Some((name, note)) => (tidy(name), note.trim().to_owned()),
        None => (tidy(rest), String::new()),
    }
}

/// Un nom tel qu'on le garde : sans blancs autour, et un seul blanc
/// entre les mots — « Protéine  Gs » est « Protéine Gs ».
fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Les morceaux d'une ligne de flèches : les groupes de noms, et le
/// signe entre chacun. `None` quand la ligne n'en porte pas.
fn arrows(line: &str) -> Option<(Vec<&str>, Vec<Sign>)> {
    let mut groups = Vec::new();
    let mut signs = Vec::new();
    let mut rest = line;
    loop {
        let act = rest.find("->");
        let inh = rest.find("-|");
        let (at, sign) = match (act, inh) {
            (Some(a), Some(i)) if a < i => (a, Sign::Active),
            (Some(_), Some(i)) => (i, Sign::Inhibe),
            (Some(a), None) => (a, Sign::Active),
            (None, Some(i)) => (i, Sign::Inhibe),
            (None, None) => break,
        };
        groups.push(&rest[..at]);
        signs.push(sign);
        rest = &rest[at + 2..];
    }
    if signs.is_empty() {
        return None;
    }
    groups.push(rest);
    Some((groups, signs))
}

/// L'action que `text` commence par nommer, et ce qui suit. La plus
/// longue d'abord : « agoniste partiel » n'est pas « agoniste » suivi
/// d'un nœud nommé « partiel ».
fn action_of(text: &str) -> Option<(Action, &str)> {
    // Rangés une fois, du plus long au plus court.
    static WORDS: std::sync::OnceLock<Vec<(&'static str, Action)>> = std::sync::OnceLock::new();
    let words = WORDS.get_or_init(|| {
        let mut words: Vec<(&str, Action)> =
            Action::ALL.iter().map(|a| (a.keyword(), *a)).collect();
        // Les manières usuelles de l'écrire, lues comme leur action.
        words.extend([
            ("agonistes partiels", Action::AgonistePartiel),
            ("agoniste partielle", Action::AgonistePartiel),
            ("agonistes", Action::Agoniste),
            ("antagonistes", Action::Antagoniste),
            ("inhibitrice", Action::Inhibiteur),
            ("inhibe", Action::Inhibiteur),
            ("bloque", Action::Antagoniste),
            ("active", Action::Activateur),
            ("activatrice", Action::Activateur),
            ("potentialise", Action::Potentialisateur),
            ("modulateur positif", Action::Potentialisateur),
        ]);
        words.sort_by_key(|(w, _)| std::cmp::Reverse(w.chars().count()));
        words
    });
    let text = text.trim_start();
    words
        .iter()
        .find_map(|(w, a)| keyword(text, w).map(|rest| (*a, rest)))
}

/// Une prise de scénario lue, sa molécule nommée : le nom, le premier pas
/// et le dernier.
type Take = (String, u32, Option<u32>);

/// Une molécule lue, dont les nœuds se résolvent à la fin : sa ligne,
/// son nom, et ce qu'elle fait à quel nom de nœud.
type Pending = (usize, String, Vec<(Action, String)>);

/// Lire une description. Ne refuse jamais : ce qui se lit est gardé, ce
/// qui ne se lit pas est rapporté avec sa ligne, et la figure montre le
/// reste — une faute de frappe à la ligne douze n'efface pas les onze
/// d'avant.
pub fn parse(text: &str) -> Cascade {
    let mut c = Cascade::default();
    // Les molécules et les adaptations nomment des nœuds qu'une flèche
    // plus bas peut encore introduire : elles se résolvent à la fin.
    let mut pending_mols: Vec<Pending> = Vec::new();
    let mut pending_adapt: Vec<(usize, String)> = Vec::new();
    let mut pending_tone: Vec<(usize, String)> = Vec::new();
    // Un scénario : sa ligne, son nom, et ses prises par nom de molécule.
    let mut pending_scenarios: Vec<(usize, String, Vec<Take>)> = Vec::new();
    fn node(c: &mut Cascade, name: &str, kind: Option<Kind>, note: &str, line: usize) -> usize {
        let name = tidy(name);
        match c.find(&name) {
            Some(i) => {
                if let Some(k) = kind {
                    let n = &mut c.nodes[i];
                    if n.declared && n.kind != k {
                        c.faults.push(Fault::Redeclared { line, name });
                    } else {
                        n.kind = k;
                        n.declared = true;
                        if !note.is_empty() {
                            n.note = note.to_owned();
                        }
                    }
                }
                i
            }
            None => {
                c.nodes.push(Node {
                    name,
                    kind: kind.unwrap_or(Kind::Relais),
                    note: note.to_owned(),
                    adapts: false,
                    low_tone: false,
                    declared: kind.is_some(),
                });
                c.nodes.len() - 1
            }
        }
    }
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        // Un commentaire ouvre la ligne, ou suit un blanc : « # » collé à
        // un mot fait partie du nom.
        let body = match raw.find(" #") {
            Some(at) => &raw[..at],
            None => raw,
        };
        let body = body.trim();
        if body.is_empty() || body.starts_with('#') {
            continue;
        }
        let head = |word: &str| heads(body, word);
        if let Some(rest) = head("titre") {
            c.title = rest.trim_start_matches(':').trim().to_owned();
            continue;
        }
        if let Some(rest) = head("sujet") {
            c.subject = rest.trim_start_matches(':').trim().to_owned();
            continue;
        }
        if let Some(rest) = head("source").or_else(|| head("sources")) {
            let source = rest.trim_start_matches(':').trim();
            if source.is_empty() {
                c.faults.push(Fault::MissingName { line });
            } else {
                c.sources.push(source.to_owned());
            }
            continue;
        }
        if let Some(rest) = head("molécule").or_else(|| head("molecules")) {
            let Some((name, acts)) = rest.split_once(':') else {
                c.faults.push(Fault::UnknownAction {
                    line,
                    text: rest.trim().to_owned(),
                });
                continue;
            };
            let name = tidy(name);
            if name.is_empty() {
                c.faults.push(Fault::MissingName { line });
                continue;
            }
            let mut list = Vec::new();
            for part in acts.split(';').filter(|p| !p.trim().is_empty()) {
                match action_of(part) {
                    Some((action, targets)) => {
                        let targets: Vec<String> = targets
                            .split(',')
                            .map(tidy)
                            .filter(|t| !t.is_empty())
                            .collect();
                        if targets.is_empty() {
                            c.faults.push(Fault::MissingName { line });
                        }
                        list.extend(targets.into_iter().map(|t| (action, t)));
                    }
                    None => c.faults.push(Fault::UnknownAction {
                        line,
                        text: part.trim().to_owned(),
                    }),
                }
            }
            pending_mols.push((line, name, list));
            continue;
        }
        if let Some(rest) = head("scénario").or_else(|| head("scenario")) {
            let rest = rest.trim_start_matches(':').trim_start();
            let Some((name, spec)) = rest.split_once(':') else {
                c.faults.push(Fault::BadScenario {
                    line,
                    text: rest.trim().to_owned(),
                });
                continue;
            };
            let name = tidy(name);
            if name.is_empty() {
                c.faults.push(Fault::MissingName { line });
                continue;
            }
            let mut takes = Vec::new();
            let mut readable = true;
            for part in spec.split(';').map(str::trim).filter(|p| !p.is_empty()) {
                match dose_of(part) {
                    Some(take) => takes.push(take),
                    None => {
                        readable = false;
                        c.faults.push(Fault::BadScenario {
                            line,
                            text: part.to_owned(),
                        });
                    }
                }
            }
            if readable && !takes.is_empty() {
                pending_scenarios.push((line, name, takes));
            } else if takes.is_empty() && readable {
                c.faults.push(Fault::BadScenario {
                    line,
                    text: spec.trim().to_owned(),
                });
            }
            continue;
        }
        if let Some(rest) = head("tonus") {
            let rest = rest.trim_start_matches(':').trim_start();
            match keyword(rest, "faible") {
                Some(names) => {
                    let names: Vec<String> = names
                        .trim_start_matches(':')
                        .split(',')
                        .map(tidy)
                        .filter(|t| !t.is_empty())
                        .collect();
                    if names.is_empty() {
                        c.faults.push(Fault::MissingName { line });
                    }
                    pending_tone.extend(names.into_iter().map(|n| (line, n)));
                }
                None => c.faults.push(Fault::UnknownAction {
                    line,
                    text: rest.trim().to_owned(),
                }),
            }
            continue;
        }
        if let Some(rest) = head("adaptation") {
            let names: Vec<String> = rest
                .trim_start_matches(':')
                .split(',')
                .map(tidy)
                .filter(|t| !t.is_empty())
                .collect();
            if names.is_empty() {
                c.faults.push(Fault::MissingName { line });
            }
            pending_adapt.extend(names.into_iter().map(|n| (line, n)));
            continue;
        }
        // Une flèche avant une déclaration : « effet -> x » est une
        // flèche partant d'un nœud nommé « effet », pas un effet nommé
        // « -> x ».
        if let Some((groups, signs)) = arrows(body) {
            let named: Vec<Vec<String>> = groups
                .iter()
                .map(|g| g.split(',').map(tidy).filter(|t| !t.is_empty()).collect())
                .collect();
            if named.iter().any(|g| g.is_empty()) {
                c.faults.push(Fault::DanglingArrow { line });
                continue;
            }
            let ids: Vec<Vec<usize>> = named
                .iter()
                .map(|g| g.iter().map(|n| node(&mut c, n, None, "", line)).collect())
                .collect();
            for (k, sign) in signs.iter().enumerate() {
                for &from in &ids[k] {
                    for &to in &ids[k + 1] {
                        if from == to {
                            c.faults.push(Fault::SelfLoop {
                                line,
                                name: c.nodes[from].name.clone(),
                            });
                            continue;
                        }
                        let edge = Edge {
                            from,
                            to,
                            sign: *sign,
                        };
                        // Écrite deux fois, une flèche compterait deux
                        // fois dans la moyenne de ce que le nœud reçoit.
                        if !c.edges.contains(&edge) {
                            c.edges.push(edge);
                        }
                    }
                }
            }
            continue;
        }
        let declared = Kind::ALL
            .iter()
            .find_map(|k| keyword(body, k.keyword()).map(|rest| (*k, rest)))
            .or_else(|| keyword(body, "recepteurs").map(|rest| (Kind::Recepteur, rest)))
            .or_else(|| keyword(body, "effets").map(|rest| (Kind::Effet, rest)));
        if let Some((kind, rest)) = declared {
            let (name, note) = name_and_note(rest);
            if name.is_empty() {
                c.faults.push(Fault::MissingName { line });
            } else {
                node(&mut c, &name, Some(kind), &note, line);
            }
            continue;
        }
        c.faults.push(Fault::Unreadable { line });
    }
    for (line, name) in pending_adapt {
        match c.find(&name) {
            Some(i) => c.nodes[i].adapts = true,
            None => c.faults.push(Fault::UnknownNode { line, name }),
        }
    }
    for (line, name) in pending_tone {
        match c.find(&name) {
            Some(i) => c.nodes[i].low_tone = true,
            None => c.faults.push(Fault::UnknownNode { line, name }),
        }
    }
    for (line, name, list) in pending_mols {
        let mut acts = Vec::new();
        for (action, target) in list {
            match c.find(&target) {
                Some(i) => {
                    if !acts.contains(&(action, i)) {
                        acts.push((action, i));
                    }
                }
                None => c.faults.push(Fault::UnknownNode { line, name: target }),
            }
        }
        match c
            .molecules
            .iter_mut()
            .find(|m| fuzzy::eq_folded(&m.name, &name))
        {
            // La même molécule sur deux lignes : une molécule, deux
            // actions — c'est une seule case à cocher.
            Some(m) => {
                for a in acts {
                    if !m.acts.contains(&a) {
                        m.acts.push(a);
                    }
                }
            }
            None => c.molecules.push(Molecule { name, acts }),
        }
    }
    for (line, name, takes) in pending_scenarios {
        let mut doses = Vec::new();
        let mut whole = true;
        for (molecule, from, until) in takes {
            match c
                .molecules
                .iter()
                .position(|m| fuzzy::eq_folded(&m.name, &molecule))
            {
                Some(m) => doses.push(Dose {
                    molecule: m,
                    from,
                    until,
                }),
                None => {
                    whole = false;
                    c.faults.push(Fault::UnknownMolecule {
                        line,
                        name: molecule,
                    });
                }
            }
        }
        if whole {
            stitch(&mut doses);
            c.scenarios.push(Scenario { name, doses });
        }
    }
    c.faults.sort_by_key(Fault::line);
    c
}

/// Une prise de scénario : « bisoprolol 10-70 », « morphine 5- ». Le
/// dernier mot porte les pas ; tout ce qui le précède est le nom.
fn dose_of(part: &str) -> Option<Take> {
    let (name, range) = part.trim().rsplit_once(char::is_whitespace)?;
    let (from, until) = range.split_once('-')?;
    let from: u32 = from.trim().parse().ok()?;
    let until = match until.trim() {
        "" => None,
        u => Some(u.parse::<u32>().ok()?),
    };
    if until.is_some_and(|u| u <= from) || from >= HORIZON {
        return None;
    }
    let name = tidy(name);
    (!name.is_empty()).then_some((name, from, until))
}

/// La lignée d'un nœud : lui, tout ce qui le nourrit et tout ce qu'il
/// nourrit, de proche en proche. C'est ce qu'un clic sur un nœud garde
/// en clair — une cascade de vingt nœuds se lit chemin par chemin.
pub fn lineage(c: &Cascade, of: usize) -> Vec<bool> {
    let n = c.nodes.len();
    let mut keep = vec![false; n];
    if of >= n {
        return keep;
    }
    keep[of] = true;
    for downstream in [true, false] {
        let mut seen = vec![false; n];
        let mut stack = vec![of];
        while let Some(v) = stack.pop() {
            if std::mem::replace(&mut seen[v], true) {
                continue;
            }
            keep[v] = true;
            for e in &c.edges {
                let (from, to) = if downstream {
                    (e.from, e.to)
                } else {
                    (e.to, e.from)
                };
                if from == v && !seen[to] {
                    stack.push(to);
                }
            }
        }
    }
    keep
}

/// Le texte d'une copie : le même, le titre suivi de `suffix` — ou un
/// titre posé en tête quand le texte n'en avait pas. Une cascade livrée
/// se retravaille sur sa copie : l'originale réécrite ne reviendrait
/// jamais.
pub fn copy_text(text: &str, suffix: &str) -> String {
    // Le titre que `parse` retient est le dernier, et il s'arrête au
    // commentaire : c'est celui-là qui reçoit le suffixe, avant « # ».
    let lines: Vec<&str> = text.lines().collect();
    let last = lines.iter().rposition(|line| {
        let body = line.find(" #").map_or(*line, |at| &line[..at]);
        heads(body.trim(), "titre").is_some()
    });
    let mut out: Vec<String> = lines.iter().map(|l| (*l).to_owned()).collect();
    match last {
        Some(i) => {
            let line = lines[i];
            let (body, comment) = match line.find(" #") {
                Some(at) => (&line[..at], &line[at..]),
                None => (line, ""),
            };
            let title = heads(body.trim(), "titre")
                .unwrap_or("")
                .trim_start_matches(':')
                .trim();
            out[i] = format!("titre : {title} {suffix}{comment}");
        }
        None => out.insert(0, format!("titre : {}", suffix.trim())),
    }
    let mut joined = out.join("\n");
    if text.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// Ce qui se lit sans faute mais ressemble à une erreur d'écriture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lint {
    /// Une molécule dont aucune fiche ne porte le nom : la cascade se
    /// joue, mais aucune fiche n'y mène.
    NoCard(String),
    /// Un nœud qu'aucune flèche ne relie — le plus souvent, un nom qui
    /// diffère d'une lettre de celui de la flèche.
    Isolated(String),
}

/// Relever ces deux cas. `has_card` dit si un nom de molécule est celui
/// d'une fiche de la base.
pub fn lint(c: &Cascade, has_card: impl Fn(&str) -> bool) -> Vec<Lint> {
    let mut out: Vec<Lint> = c
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| c.edges.iter().all(|e| e.from != *i && e.to != *i))
        .map(|(_, n)| Lint::Isolated(n.name.clone()))
        .collect();
    out.extend(
        c.molecules
            .iter()
            .filter(|m| !has_card(&m.name))
            .map(|m| Lint::NoCard(m.name.clone())),
    );
    out
}

/// L'encre d'un morceau de texte dans l'éditeur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Comment,
    /// Le mot qui ouvre une instruction : `récepteur`, `molécule`…
    Keyword,
    /// `->` et `-|`.
    Arrow,
    /// Ce qu'une molécule fait : `antagoniste`, `inhibiteur`…
    Action,
    Plain,
}

/// Le texte découpé en morceaux d'une encre : `(début, fin, encre)` en
/// octets, bout à bout, couvrant tout le texte. Lu ligne par ligne avec
/// les mêmes règles que [`parse`], pour que ce qui se colore comme un
/// mot-clé soit ce qui se lit comme un mot-clé.
pub fn colour(text: &str) -> Vec<(usize, usize, Ink)> {
    let mut out: Vec<(usize, usize, Ink)> = Vec::new();
    let push = |out: &mut Vec<(usize, usize, Ink)>, from: usize, to: usize, ink: Ink| {
        if from >= to {
            return;
        }
        match out.last_mut() {
            Some(last) if last.1 == from && last.2 == ink => last.1 = to,
            _ => out.push((from, to, ink)),
        }
    };
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let end = start + line.len();
        let trimmed = line.trim_start();
        let lead = line.len() - trimmed.len();
        // Le commentaire : toute la ligne, ou ce qui suit « # ».
        let body_end = if trimmed.starts_with('#') {
            lead
        } else {
            line.find(" #").unwrap_or(line.len())
        };
        let body = &line[..body_end];
        let mut words: Vec<&str> = Kind::ALL.iter().map(|k| k.keyword()).collect();
        words.extend([
            "titre",
            "sujet",
            "source",
            "sources",
            "molécule",
            "molecules",
            "scénario",
            "scenario",
            "adaptation",
            "tonus",
            "recepteurs",
            "effets",
        ]);
        let head = {
            words
                .iter()
                .find(|w| {
                    let t = body.trim_start();
                    // Les mêmes règles que `parse` : une déclaration ne
                    // porte pas de flèche, une instruction sur une ligne
                    // fléchée porte son `:`.
                    let declares = Kind::ALL.iter().any(|k| k.keyword() == **w)
                        || **w == "recepteurs"
                        || **w == "effets";
                    if declares {
                        arrows(t).is_none() && keyword(t, w).is_some()
                    } else {
                        heads(t.trim_end(), w).is_some()
                    }
                })
                .map(|w| {
                    lead + body
                        .trim_start()
                        .char_indices()
                        .nth(w.chars().count())
                        .map_or(body.trim_start().len(), |(i, _)| i)
                })
        };
        let mut at = 0;
        if let Some(h) = head {
            push(&mut out, start, start + h, Ink::Keyword);
            at = h;
        }
        // Après le « : » d'une molécule, l'action.
        if head.is_some()
            && (heads(body.trim(), "molécule").is_some()
                || heads(body.trim(), "molecules").is_some())
        {
            if let Some(colon) = body[at..].find(':') {
                let colon = at + colon + 1;
                push(&mut out, start + at, start + colon, Ink::Plain);
                at = colon;
                for part in body[at..].split_inclusive(';') {
                    let lead_ws = part.len() - part.trim_start().len();
                    let action_len = action_of(part).map_or(0, |(_, rest)| {
                        let rest_at = part.len() - rest.len();
                        part[..rest_at].trim_end().len()
                    });
                    push(
                        &mut out,
                        start + at,
                        start + at + lead_ws.min(action_len),
                        Ink::Plain,
                    );
                    push(
                        &mut out,
                        start + at + lead_ws.min(action_len),
                        start + at + action_len,
                        Ink::Action,
                    );
                    push(
                        &mut out,
                        start + at + action_len,
                        start + at + part.len(),
                        Ink::Plain,
                    );
                    at += part.len();
                }
            }
        }
        // Les flèches, dans le reste du corps.
        let mut i = at;
        while i < body.len() {
            let rest = &body[i..];
            let next = [rest.find("->"), rest.find("-|")]
                .into_iter()
                .flatten()
                .min();
            match next {
                Some(k) => {
                    push(&mut out, start + i, start + i + k, Ink::Plain);
                    push(&mut out, start + i + k, start + i + k + 2, Ink::Arrow);
                    i += k + 2;
                }
                None => {
                    push(&mut out, start + i, start + body.len(), Ink::Plain);
                    i = body.len();
                }
            }
        }
        push(&mut out, start + body_end, end, Ink::Comment);
        start = end;
    }
    out
}

// --- Le modèle --------------------------------------------------------
//
// Chaque nœud vaut 1 au repos. Ce qu'il reçoit d'amont est la moyenne de
// ce qui l'active, divisée par ce qui l'inhibe (sous la forme bornée
// 2 / (1 + x) : un inhibiteur au repos ne change rien, absent il double,
// au plafond il divise par deux) ; les molécules agissent par-dessus, et
// la densité multiplie le tout. Les constantes sont celles d'un modèle
// qualitatif — voir l'en-tête du module.

/// La part du récepteur que les molécules qui l'occupent prennent au
/// ligand, ensemble.
pub const OCCUPIED: f32 = 0.9;
/// Ce que vaut un récepteur tenu par un agoniste plein.
pub const FULL: f32 = 2.0;
/// Et par un agoniste partiel : au-dessus du repos, au-dessous du plein.
pub const PARTIAL: f32 = 1.3;
/// Et par un agoniste partiel faible : au-dessous du repos, au-dessus de
/// rien.
pub const WEAK: f32 = 0.5;
/// Ce qu'un inhibiteur laisse passer.
pub const BLOCK: f32 = 0.15;
/// Ce qu'un activateur ou un potentialisateur multiplie.
pub const BOOST: f32 = 2.0;
/// Le plafond d'un nœud, trois fois son repos : au-delà, le dessin ne
/// distingue plus rien et le calcul d'une boucle pourrait s'emballer.
pub const CEILING: f32 = 3.0;
/// Ce qu'un nœud au tonus faible perd de ce qu'on lui retire : le
/// dixième, parce qu'il n'avait presque rien à perdre.
pub const LOW_TONE: f32 = 0.1;
/// Les bornes de la densité d'un nœud qui s'adapte.
pub const DENSITY_MIN: f32 = 0.4;
pub const DENSITY_MAX: f32 = 2.5;
/// La part de l'écart que la densité rattrape à chaque pas.
pub const RATE: f32 = 0.1;

/// L'état d'une chaîne à un instant : l'activité de chaque nœud, et la
/// densité de ceux qui s'adaptent (1 pour les autres).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub level: Vec<f32>,
    /// Ce que le nœud reçoit **avant** sa densité : ce contre quoi il
    /// s'adapte.
    pub drive: Vec<f32>,
    pub density: Vec<f32>,
}

/// Quel ordre suivre pour propager : les flèches qui remontent — une
/// boucle de rétrocontrôle — sont lues à l'image d'avant.
fn order(c: &Cascade) -> (Vec<usize>, Vec<bool>) {
    let back = back_edges(c);
    let n = c.nodes.len();
    let mut indeg = vec![0usize; n];
    for (k, e) in c.edges.iter().enumerate() {
        if !back[k] {
            indeg[e.to] += 1;
        }
    }
    // Le plus petit indice d'abord parmi les prêts : l'ordre ne dépend
    // que du texte, jamais d'un hasard de parcours.
    let mut ready: std::collections::BinaryHeap<std::cmp::Reverse<usize>> = (0..n)
        .filter(|&i| indeg[i] == 0)
        .map(std::cmp::Reverse)
        .collect();
    let mut out = Vec::with_capacity(n);
    while let Some(std::cmp::Reverse(i)) = ready.pop() {
        out.push(i);
        for (k, e) in c.edges.iter().enumerate() {
            if !back[k] && e.from == i {
                indeg[e.to] -= 1;
                if indeg[e.to] == 0 {
                    ready.push(std::cmp::Reverse(e.to));
                }
            }
        }
    }
    (out, back)
}

/// Les flèches qui ferment une boucle — celles que le dessin fait
/// remonter sur le côté et que la propagation lit à l'image d'avant.
///
/// **L'ordre du texte décide laquelle.** Une boucle se coupe n'importe
/// où, et le choix fait tout le dessin : dans le système
/// rénine-angiotensine, couper « rénine -> angiotensine I » plutôt que
/// « pression -| rénine » pose la rénine sous la pression artérielle,
/// c'est-à-dire la figure à l'envers. Ce que l'auteur écrit d'abord est
/// ce qu'il lit en haut : une flèche qui va d'un nœud nommé plus tôt
/// vers un nœud nommé plus tard descend toujours — ces flèches-là ne
/// peuvent pas faire de boucle entre elles —, et une flèche qui remonte
/// l'ordre du texte n'est coupée **que si** elle ferme vraiment une
/// boucle. Une description sans boucle n'a donc jamais de flèche
/// coupée, quel que soit l'ordre dans lequel on l'a écrite.
pub fn back_edges(c: &Cascade) -> Vec<bool> {
    let n = c.nodes.len();
    let mut back = vec![false; c.edges.len()];
    let mut kept: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in c.edges.iter().filter(|e| e.from < e.to) {
        kept[e.from].push(e.to);
    }
    // Du plus proche au plus lointain dans le texte : une flèche qui
    // remonte d'une ligne se garde avant une qui remonte de dix.
    let mut up: Vec<usize> = (0..c.edges.len())
        .filter(|&k| c.edges[k].from > c.edges[k].to)
        .collect();
    up.sort_by_key(|&k| (c.edges[k].from - c.edges[k].to, k));
    for k in up {
        let Edge { from, to, .. } = c.edges[k];
        // La garder ferme-t-il une boucle ? Oui si `from` s'atteint
        // déjà depuis `to`.
        let mut seen = vec![false; n];
        let mut stack = vec![to];
        let mut loops = false;
        while let Some(v) = stack.pop() {
            if v == from {
                loops = true;
                break;
            }
            if std::mem::replace(&mut seen[v], true) {
                continue;
            }
            stack.extend(kept[v].iter().copied());
        }
        if loops {
            back[k] = true;
        } else {
            kept[from].push(to);
        }
    }
    back
}

/// Ce que la propagation lit de la chaîne, rangé une fois : l'ordre, et
/// pour chaque nœud ce qui l'active, ce qui l'inhibe et les molécules
/// qui le touchent. Relire les flèches et les molécules à chaque nœud,
/// à chaque passage et à chaque pas faisait d'une description collée de
/// quelques centaines de nœuds un clic qui fige l'écran.
struct Wiring {
    order: Vec<usize>,
    up: Vec<Vec<usize>>,
    down: Vec<Vec<usize>>,
    acts: Vec<Vec<(usize, Action)>>,
    low: Vec<bool>,
}

impl Wiring {
    fn of(c: &Cascade) -> Wiring {
        let n = c.nodes.len();
        let (order, _) = order(c);
        let mut up = vec![Vec::new(); n];
        let mut down = vec![Vec::new(); n];
        for e in &c.edges {
            match e.sign {
                Sign::Active => up[e.to].push(e.from),
                Sign::Inhibe => down[e.to].push(e.from),
            }
        }
        let mut acts = vec![Vec::new(); n];
        for (m, mol) in c.molecules.iter().enumerate() {
            for (a, node) in &mol.acts {
                if let Some(list) = acts.get_mut(*node) {
                    list.push((m, *a));
                }
            }
        }
        Wiring {
            order,
            up,
            down,
            acts,
            low: c.nodes.iter().map(|n| n.low_tone).collect(),
        }
    }

    /// Ce qu'un nœud devient pour ce qu'il reçoit et les molécules qui
    /// le touchent, **avant** sa densité.
    fn drive(&self, i: usize, level: &[f32], given: &[bool]) -> f32 {
        let up = &self.up[i];
        // Sans amont, un nœud est à son tonus de repos.
        let mut x = if up.is_empty() {
            1.0
        } else {
            up.iter().map(|&p| level[p]).sum::<f32>() / up.len() as f32
        };
        for &p in &self.down[i] {
            x *= 2.0 / (1.0 + level[p].max(0.0));
        }
        let (mut occupants, mut efficacy) = (0usize, 0.0_f32);
        let mut factor = 1.0_f32;
        for &(m, a) in &self.acts[i] {
            if !given.get(m).copied().unwrap_or(false) {
                continue;
            }
            match a {
                Action::Agoniste => {
                    occupants += 1;
                    efficacy += FULL;
                }
                Action::AgonistePartiel => {
                    occupants += 1;
                    efficacy += PARTIAL;
                }
                Action::AgonistePartielFaible => {
                    occupants += 1;
                    efficacy += WEAK;
                }
                Action::Antagoniste => occupants += 1,
                Action::Inhibiteur => factor *= BLOCK,
                Action::Activateur | Action::Potentialisateur => factor *= BOOST,
            }
        }
        if occupants > 0 {
            // Le site se partage : le ligand garde ce que les molécules
            // ne prennent pas, et elles se le partagent à parts égales.
            x = x * (1.0 - OCCUPIED) + OCCUPIED * efficacy / occupants as f32;
        }
        let mut x = x * factor;
        // Au tonus faible, ce qui descend sous le repos n'en retire que
        // le dixième : il n'y avait presque rien à retirer.
        if self.low[i] && x < 1.0 {
            x = 1.0 - (1.0 - x) * LOW_TONE;
        }
        x.clamp(0.0, CEILING)
    }

    fn settle(&self, given: &[bool], density: &[f32]) -> Frame {
        let n = self.up.len();
        let mut level = vec![1.0_f32; n];
        let mut drive = vec![1.0_f32; n];
        let k = |i: usize| density.get(i).copied().unwrap_or(1.0);
        // Une chaîne sans boucle s'établit en un passage ; une boucle de
        // rétrocontrôle se rapproche de son point fixe à chaque passage.
        // Un nombre fixe : le résultat ne dépend pas d'un seuil de
        // convergence que personne ne choisirait.
        for _ in 0..24 {
            for &i in &self.order {
                let d = self.drive(i, &level, given);
                drive[i] = d;
                // Moyenné avec l'image d'avant : une boucle qui s'inverse
                // à chaque passage finit sur son milieu au lieu d'osciller.
                let next = (d * k(i)).clamp(0.0, CEILING);
                level[i] = 0.5 * level[i] + 0.5 * next;
            }
        }
        for &i in &self.order {
            let d = self.drive(i, &level, given);
            drive[i] = d;
            level[i] = (d * k(i)).clamp(0.0, CEILING);
        }
        Frame {
            level,
            drive,
            density: (0..n).map(k).collect(),
        }
    }
}

/// L'état établi d'une chaîne pour ces molécules et ces densités — ce
/// qu'on voit une fois le signal passé, sans que le temps ait joué.
pub fn settle(c: &Cascade, given: &[bool], density: &[f32]) -> Frame {
    Wiring::of(c).settle(given, density)
}

/// Jusqu'où le temps d'une partie court, en pas : assez pour qu'une
/// adaptation s'installe, puis qu'un arrêt montre son rebond et son
/// retour.
pub const HORIZON: u32 = 120;

/// Une prise : une molécule donnée à partir d'un pas, jusqu'à un autre
/// ou jusqu'au bout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Dose {
    pub molecule: usize,
    pub from: u32,
    pub until: Option<u32>,
}

impl Dose {
    pub fn given_at(&self, t: u32) -> bool {
        t >= self.from && self.until.is_none_or(|u| t < u)
    }
}

/// Cocher ou décocher une molécule **au pas `t`** — le geste de la
/// vue : on se place dans le temps, et on donne ou on arrête.
///
/// Donnée à `t`, elle s'arrête là (et disparaît si elle commençait
/// là : cocher puis décocher au même instant ne laisse rien). Pas
/// donnée, elle commence à `t` et court jusqu'à sa prochaine prise
/// écrite plus loin, ou jusqu'au bout. Les prises d'une molécule sont
/// ensuite recousues : deux prises qui se touchent n'en font qu'une.
pub fn toggle(doses: &mut Vec<Dose>, molecule: usize, t: u32) {
    if let Some(k) = doses
        .iter()
        .position(|d| d.molecule == molecule && d.given_at(t))
    {
        if doses[k].from == t {
            doses.remove(k);
        } else {
            doses[k].until = Some(t);
        }
    } else {
        let next = doses
            .iter()
            .filter(|d| d.molecule == molecule && d.from > t)
            .map(|d| d.from)
            .min();
        doses.push(Dose {
            molecule,
            from: t,
            until: next,
        });
    }
    stitch(doses);
}

/// Recoudre les prises : triées par molécule puis par début, et deux
/// prises de la même molécule qui se touchent ou se chevauchent n'en
/// font qu'une.
fn stitch(doses: &mut Vec<Dose>) {
    doses.sort_by_key(|d| (d.molecule, d.from));
    let mut out: Vec<Dose> = Vec::with_capacity(doses.len());
    for d in doses.drain(..) {
        if let Some(last) = out.last_mut() {
            let touches = last.molecule == d.molecule && last.until.is_none_or(|u| u >= d.from);
            if touches {
                last.until = match (last.until, d.until) {
                    (None, _) | (_, None) => None,
                    (Some(a), Some(b)) => Some(a.max(b)),
                };
                continue;
            }
        }
        out.push(d);
    }
    *doses = out;
}

/// Les molécules données au pas `t`.
pub fn given_at(c: &Cascade, doses: &[Dose], t: u32) -> Vec<bool> {
    (0..c.molecules.len())
        .map(|m| doses.iter().any(|d| d.molecule == m && d.given_at(t)))
        .collect()
}

/// Jouer `steps` pas : l'état à chaque pas, de 0 à `steps` compris. À
/// chaque pas les nœuds qui s'adaptent rapprochent leur densité de ce
/// qui ramènerait leur activité au repos — c'est tout le mécanisme de la
/// tolérance, et du rebond quand on arrête.
pub fn run(c: &Cascade, doses: &[Dose], steps: u32) -> Vec<Frame> {
    let n = c.nodes.len();
    let wiring = Wiring::of(c);
    let mut density = vec![1.0_f32; n];
    let mut out = Vec::with_capacity(steps as usize + 1);
    for t in 0..=steps {
        let frame = wiring.settle(&given_at(c, doses, t), &density);
        for ((node, k), d) in c.nodes.iter().zip(density.iter_mut()).zip(&frame.drive) {
            if !node.adapts {
                continue;
            }
            let target = if *d > 1e-3 {
                (1.0 / d).clamp(DENSITY_MIN, DENSITY_MAX)
            } else {
                DENSITY_MAX
            };
            *k += RATE * (target - *k);
        }
        out.push(frame);
    }
    out
}

/// Comment se lit une activité : le sens, et s'il est franc. Le seuil
/// est celui en dessous duquel le dessin ne montre pas de différence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trend {
    Down,
    Rest,
    Up,
}

pub fn trend(level: f32) -> Trend {
    if level < 0.9 {
        Trend::Down
    } else if level > 1.1 {
        Trend::Up
    } else {
        Trend::Rest
    }
}

// --- Deux lignes d'une ordonnance sur une même cascade -----------------

/// Ce que deux molécules font **ensemble** à un effet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Together {
    /// L'effet va plus loin à deux qu'avec chacune : le nitré sur un
    /// inhibiteur de la PDE5, deux sérotoninergiques.
    Adds,
    /// L'une défait ce que fait l'autre : le propranolol sur le
    /// salbutamol, l'oxybutynine sur le donépézil, la spironolactone sur
    /// la kaliémie d'un diurétique de l'anse.
    Opposes,
}

/// L'écart minimal, en niveau, pour qu'une différence entre « seule » et
/// « à deux » soit dite. Même ordre que le seuil de [`trend`] : en deçà,
/// le dessin ne montre rien, et la lecture ne doit pas en dire plus que
/// le dessin.
const APART: f32 = 0.05;

/// Vers le bas, la part de ce qui reste qu'une seconde molécule doit
/// retirer pour qu'on dise qu'elles s'additionnent.
const SHARE: f32 = 0.2;

/// Pour une paire de molécules de la cascade, les effets qui bougent
/// autrement à deux que séparément : `(effet, ensemble)`.
///
/// **C'est une lecture du modèle, pas une table d'interactions** : elle
/// ne dit que ce que la figure montre quand on coche les deux cases, et
/// la figure est qualitative. Aucun effet n'est dit s'il ne bouge pas —
/// deux molécules qui se rencontrent sur un nœud sans changer ce qu'on
/// lit en bout de chaîne ne font pas une rencontre.
pub fn together(c: &Cascade, a: usize, b: usize) -> Vec<(usize, Together)> {
    let n = c.molecules.len();
    if a == b || a >= n || b >= n {
        return Vec::new();
    }
    let alone = |m: usize| {
        let mut g = vec![false; n];
        g[m] = true;
        settle(c, &g, &[])
    };
    let (la, lb) = (alone(a), alone(b));
    let mut both = vec![false; n];
    both[a] = true;
    both[b] = true;
    let lab = settle(c, &both, &[]);
    let mut out = Vec::new();
    for o in c.outcomes() {
        let (x, y, z) = (la.level[o] - 1.0, lb.level[o] - 1.0, lab.level[o] - 1.0);
        let (tx, ty, tz) = (trend(la.level[o]), trend(lb.level[o]), trend(lab.level[o]));
        let opposite = |p: Trend, q: Trend| {
            matches!((p, q), (Trend::Up, Trend::Down) | (Trend::Down, Trend::Up))
        };
        // S'opposer : deux sens contraires, ou l'une qui ramène vers le
        // repos ce que l'autre déplaçait seule — la naloxone ne fait
        // rien seule et défait la morphine.
        let undoes = |own: f32, t_own: Trend| {
            t_own != Trend::Rest && z.abs() < own.abs() - APART && !opposite(tz, t_own)
                || t_own != Trend::Rest && opposite(tz, t_own)
        };
        if opposite(tx, ty) || undoes(x, tx) || undoes(y, ty) {
            out.push((o, Together::Opposes));
            continue;
        }
        // S'additionner : plus loin à deux qu'avec la plus forte des
        // deux, dans le sens où elles vont. **Vers le bas, en
        // proportion** : un niveau ne descend pas sous zéro, et l'aspirine
        // seule éteint déjà presque l'agrégation — le clopidogrel en
        // retire encore un tiers de ce qui reste, ce qu'un écart absolu
        // ne verrait pas.
        let further = match tz {
            Trend::Down => lab.level[o] < la.level[o].min(lb.level[o]) * (1.0 - SHARE),
            Trend::Up => z > x.max(y) + APART,
            Trend::Rest => false,
        };
        if further && !opposite(tz, tx) && !opposite(tz, ty) {
            out.push((o, Together::Adds));
        }
    }
    out
}

/// Une ligne de l'ordonnance, telle que cette lecture la reçoit.
pub struct Line<'a> {
    pub name: &'a str,
    pub dci: &'a str,
}

/// Deux lignes de l'ordonnance qui agissent sur une même cascade, et ce
/// que la cascade montre d'elles ensemble.
#[derive(Clone, Debug, PartialEq)]
pub struct Meeting {
    /// Le rang de la cascade dans la liste reçue.
    pub cascade: usize,
    /// Les deux lignes, telles qu'écrites, dans l'ordre de la liste.
    pub lines: (String, String),
    /// Les deux molécules de la cascade qu'elles portent.
    pub molecules: (String, String),
    /// Les effets, par leur nom, et ce que la paire en fait.
    pub effects: Vec<(String, Together)>,
}

/// La molécule de la cascade qu'une ligne porte : sa DCI entière, ou
/// l'un des composants d'une association (« vérapamil + trandolapril »
/// porte le vérapamil). **Jamais une sous-chaîne** — la règle des
/// portes de fiche, et pour la même raison.
fn carried(c: &Cascade, dci: &str) -> Vec<usize> {
    let parts: Vec<&str> = std::iter::once(dci)
        .chain(dci.split(" + ").filter(|p| p.len() < dci.len()))
        .collect();
    c.molecules
        .iter()
        .enumerate()
        .filter(|(_, m)| parts.iter().any(|p| fuzzy::eq_folded(p, &m.name)))
        .map(|(i, _)| i)
        .collect()
}

/// Ce que les cascades disent des paires d'une ordonnance.
///
/// Une paire n'est retenue que si la cascade montre **au moins un
/// effet** qui bouge autrement à deux — voir [`together`]. Deux lignes
/// qui portent la même molécule n'en font pas une : le doublon est
/// l'affaire de la revue d'ordonnance, et une molécule donnée deux fois
/// sur la figure est une molécule donnée une fois.
pub fn meetings(cascades: &[&Cascade], lines: &[Line]) -> Vec<Meeting> {
    let mut out = Vec::new();
    for (ci, c) in cascades.iter().enumerate() {
        let carried: Vec<Vec<usize>> = lines.iter().map(|l| carried(c, l.dci)).collect();
        for i in 0..lines.len() {
            for j in i + 1..lines.len() {
                for &a in &carried[i] {
                    for &b in &carried[j] {
                        if a == b {
                            continue;
                        }
                        let effects: Vec<(String, Together)> = together(c, a, b)
                            .into_iter()
                            .map(|(o, t)| (c.nodes[o].name.clone(), t))
                            .collect();
                        if effects.is_empty() {
                            continue;
                        }
                        out.push(Meeting {
                            cascade: ci,
                            lines: (lines[i].name.to_owned(), lines[j].name.to_owned()),
                            molecules: (c.molecules[a].name.clone(), c.molecules[b].name.clone()),
                            effects,
                        });
                    }
                }
            }
        }
    }
    // Ce qui s'oppose d'abord : c'est ce qu'on n'attend pas d'une
    // ordonnance, là où une addition est souvent voulue.
    out.sort_by_key(|m| !m.effects.iter().any(|(_, t)| *t == Together::Opposes));
    out
}

// --- La disposition ---------------------------------------------------

/// Où chaque nœud se pose : un étage (0 en haut, les ligands) et une
/// abscisse en unités de case, la plus petite valant 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub layer: Vec<usize>,
    pub x: Vec<f32>,
    pub layers: usize,
    /// La largeur de la figure, en cases : de la plus petite abscisse à
    /// la plus grande, plus une case.
    pub span: f32,
    /// Les flèches qui remontent — dessinées sur le côté.
    pub back: Vec<bool>,
}

/// Ranger les nœuds en étages : chacun sous le plus bas de ceux qui le
/// nourrissent, et dans son étage près de ses voisins. Le classique de
/// Sugiyama, en petit : les étages par le plus long chemin, l'ordre par
/// barycentres, puis les abscisses tirées vers les voisins sans jamais
/// se chevaucher.
pub fn layout(c: &Cascade) -> Layout {
    let n = c.nodes.len();
    let (topo, back) = order(c);
    let mut layer = vec![0usize; n];
    for &i in &topo {
        for (k, e) in c.edges.iter().enumerate() {
            if !back[k] && e.from == i {
                layer[e.to] = layer[e.to].max(layer[i] + 1);
            }
        }
    }
    // **Une source descend jusqu'au-dessus de ce qu'elle nourrit.** Posée
    // en haut par principe, l'antithrombine de la coagulation tirait deux
    // flèches à travers toute la figure pour rejoindre le Xa et la
    // thrombine, quatre étages plus bas ; juste au-dessus d'eux, elle se
    // lit à côté de ce qu'elle freine. De la plus basse à la plus haute,
    // pour qu'une source qui en nourrit une autre la suive.
    let mut sources: Vec<usize> = (0..n)
        .filter(|&i| c.edges.iter().zip(&back).all(|(e, b)| *b || e.to != i))
        .collect();
    sources.sort_by_key(|&i| std::cmp::Reverse(layer[i]));
    for i in sources {
        let below = c
            .edges
            .iter()
            .zip(&back)
            .filter(|(e, b)| !**b && e.from == i)
            .map(|(e, _)| layer[e.to])
            .min();
        if let Some(below) = below {
            layer[i] = layer[i].max(below.saturating_sub(1));
        }
    }
    // Un nœud isolé — nommé, déclaré, mais que rien ne relie encore —
    // se pose en haut ; c'est là qu'on le voit en écrivant.
    let layers = layer.iter().copied().max().map_or(0, |m| m + 1);
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); layers];
    for i in 0..n {
        rows[layer[i]].push(i);
    }
    let fwd: Vec<(usize, usize)> = c
        .edges
        .iter()
        .zip(&back)
        .filter(|(_, b)| !**b)
        .map(|(e, _)| (e.from, e.to))
        .collect();
    let mut pos = vec![0.0_f32; n];
    let place = |rows: &Vec<Vec<usize>>, pos: &mut Vec<f32>| {
        for row in rows {
            for (k, &i) in row.iter().enumerate() {
                pos[i] = k as f32;
            }
        }
    };
    place(&rows, &mut pos);
    // Les barycentres : descendre en triant chaque étage sur ses
    // parents, remonter en le triant sur ses enfants.
    for sweep in 0..6 {
        let down = sweep % 2 == 0;
        let range: Vec<usize> = if down {
            (1..layers).collect()
        } else {
            (0..layers.saturating_sub(1)).rev().collect()
        };
        for l in range {
            let mut keyed: Vec<(f32, usize)> = rows[l]
                .iter()
                .map(|&i| {
                    let near: Vec<f32> = fwd
                        .iter()
                        .filter_map(|&(a, b)| {
                            if down && b == i {
                                Some(pos[a])
                            } else if !down && a == i {
                                Some(pos[b])
                            } else {
                                None
                            }
                        })
                        .collect();
                    let key = if near.is_empty() {
                        pos[i]
                    } else {
                        near.iter().sum::<f32>() / near.len() as f32
                    };
                    (key, i)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            rows[l] = keyed.into_iter().map(|(_, i)| i).collect();
            for (k, &i) in rows[l].iter().enumerate() {
                pos[i] = k as f32;
            }
        }
    }
    // Les abscisses : chaque étage centré, puis chaque nœud tiré vers
    // la moyenne de ses voisins, l'ordre gardé et une case d'écart au
    // moins.
    let widest = rows.iter().map(Vec::len).max().unwrap_or(0) as f32;
    for row in &rows {
        let off = (widest - row.len() as f32) / 2.0;
        for (k, &i) in row.iter().enumerate() {
            pos[i] = off + k as f32;
        }
    }
    for _ in 0..8 {
        for row in &rows {
            let want: Vec<f32> = row
                .iter()
                .map(|&i| {
                    let near: Vec<f32> = fwd
                        .iter()
                        .filter_map(|&(a, b)| {
                            if b == i {
                                Some(pos[a])
                            } else if a == i {
                                Some(pos[b])
                            } else {
                                None
                            }
                        })
                        .collect();
                    if near.is_empty() {
                        pos[i]
                    } else {
                        0.5 * pos[i] + 0.5 * near.iter().sum::<f32>() / near.len() as f32
                    }
                })
                .collect();
            // De gauche à droite, chacun au moins une case après le
            // précédent ; puis de droite à gauche, pour ne pas tout
            // pousser d'un seul côté.
            let mut xs = want.clone();
            for k in 1..xs.len() {
                xs[k] = xs[k].max(xs[k - 1] + 1.0);
            }
            let mut back_pass = want;
            for k in (0..back_pass.len().saturating_sub(1)).rev() {
                back_pass[k] = back_pass[k].min(back_pass[k + 1] - 1.0);
            }
            for k in 0..xs.len() {
                pos[row[k]] = 0.5 * (xs[k] + back_pass[k]);
            }
            // La moyenne des deux passes peut rapprocher deux voisins :
            // un dernier passage rétablit l'écart.
            for k in 1..row.len() {
                let (a, b) = (row[k - 1], row[k]);
                if pos[b] < pos[a] + 1.0 {
                    pos[b] = pos[a] + 1.0;
                }
            }
        }
    }
    let min = pos.iter().copied().fold(f32::INFINITY, f32::min);
    let min = if min.is_finite() { min } else { 0.0 };
    for p in &mut pos {
        *p -= min;
    }
    let span = pos.iter().copied().fold(0.0_f32, f32::max) + 1.0;
    Layout {
        layer,
        x: pos,
        layers,
        span: if n == 0 { 0.0 } else { span },
        back,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BETA: &str = "\
titre : Récepteur bêta-1
sujet : Bêtabloquants
source : Un manuel
# un commentaire
ligand Noradrénaline
récepteur Bêta-1 : couplé à Gs
enzyme Adénylate cyclase
messager AMPc
effet Fréquence cardiaque
effet Contractilité
Noradrénaline -> Bêta-1 -> Protéine Gs -> Adénylate cyclase -> AMPc
AMPc -> Fréquence cardiaque, Contractilité
molécule bisoprolol : antagoniste Bêta-1
molécule dobutamine : agoniste Bêta-1
molécule pindolol : agoniste partiel Bêta-1
adaptation Bêta-1
";

    fn idx(c: &Cascade, name: &str) -> usize {
        c.find(name).unwrap_or_else(|| panic!("{name} absent"))
    }

    fn with(c: &Cascade, names: &[&str]) -> Vec<bool> {
        c.molecules
            .iter()
            .map(|m| names.iter().any(|n| fuzzy::eq_folded(n, &m.name)))
            .collect()
    }

    #[test]
    fn a_description_reads_into_nodes_arrows_and_molecules() {
        let c = parse(BETA);
        assert!(c.faults.is_empty(), "{:?}", c.faults);
        assert_eq!(c.title, "Récepteur bêta-1");
        assert_eq!(c.subject, "Bêtabloquants");
        assert_eq!(c.sources, vec!["Un manuel".to_owned()]);
        assert_eq!(c.nodes.len(), 7);
        let gs = &c.nodes[idx(&c, "protéine gs")];
        assert_eq!(gs.kind, Kind::Relais);
        assert!(!gs.declared);
        let b1 = &c.nodes[idx(&c, "BETA-1")];
        assert_eq!(b1.kind, Kind::Recepteur);
        assert_eq!(b1.note, "couplé à Gs");
        assert!(b1.adapts);
        assert_eq!(c.edges.len(), 6);
        assert_eq!(c.molecules.len(), 3);
        assert_eq!(
            c.molecules[2].acts,
            vec![(Action::AgonistePartiel, b1_of(&c))]
        );
        assert_eq!(c.outcomes().len(), 2);
    }

    fn b1_of(c: &Cascade) -> usize {
        idx(c, "Bêta-1")
    }

    #[test]
    fn keywords_and_names_read_without_case_or_accents() {
        let c = parse("RECEPTEUR  Mu \nMolecule morphine : Agoniste mu\nligand x -> MU");
        assert!(c.faults.is_empty(), "{:?}", c.faults);
        assert_eq!(c.nodes[0].name, "Mu");
        // « ligand x -> MU » est une flèche d'un nœud nommé « ligand x »
        // : la flèche passe avant la déclaration.
        assert_eq!(c.nodes.len(), 2);
        assert_eq!(c.molecules[0].acts, vec![(Action::Agoniste, 0)]);
    }

    #[test]
    fn what_does_not_read_is_reported_with_its_line_and_the_rest_is_kept() {
        let c = parse(
            "A -> B\nn'importe quoi\nmolécule x : agit sur B\nmolécule y : antagoniste Z\n\
             A -> A\nrécepteur A\nenzyme A\nC ->\nadaptation Q\nmolécule sans deux points\n\
             effet",
        );
        let lines: Vec<usize> = c.faults.iter().map(Fault::line).collect();
        assert_eq!(lines, vec![2, 3, 4, 5, 7, 8, 9, 10, 11]);
        assert!(matches!(c.faults[0], Fault::Unreadable { .. }));
        assert!(matches!(c.faults[1], Fault::UnknownAction { .. }));
        assert_eq!(
            c.faults[2],
            Fault::UnknownNode {
                line: 4,
                name: "Z".into()
            }
        );
        assert!(matches!(c.faults[3], Fault::SelfLoop { .. }));
        assert!(matches!(c.faults[4], Fault::Redeclared { .. }));
        assert!(matches!(c.faults[5], Fault::DanglingArrow { .. }));
        assert!(matches!(c.faults[6], Fault::UnknownNode { .. }));
        assert!(matches!(c.faults[7], Fault::UnknownAction { .. }));
        assert!(matches!(c.faults[8], Fault::MissingName { .. }));
        // Ce qui se lisait est là.
        assert_eq!(c.edges.len(), 1);
        assert_eq!(c.nodes[0].kind, Kind::Recepteur);
    }

    #[test]
    fn arrows_chain_fan_out_and_are_kept_once() {
        let c = parse("A, B -> C -| D, E\nA -> C");
        assert!(c.faults.is_empty());
        assert_eq!(c.edges.len(), 4);
        let d = idx(&c, "D");
        assert!(c
            .edges
            .iter()
            .any(|e| e.to == d && e.sign == Sign::Inhibe && e.from == idx(&c, "C")));
    }

    #[test]
    fn a_molecule_written_twice_is_one_molecule_with_both_actions() {
        let c =
            parse("A -> B\nmolécule x : inhibiteur A\nMolécule X : activateur B ; antagoniste A");
        assert!(c.faults.is_empty(), "{:?}", c.faults);
        assert_eq!(c.molecules.len(), 1);
        assert_eq!(c.molecules[0].acts.len(), 3);
        assert!(c.names_molecule("X"));
        assert!(!c.names_molecule("xa"));
    }

    #[test]
    fn a_hash_inside_a_word_is_part_of_the_name() {
        let c = parse("A#1 -> B # note");
        assert_eq!(c.nodes[0].name, "A#1");
        assert_eq!(c.nodes[1].name, "B");
    }

    #[test]
    fn at_rest_every_node_is_at_one() {
        let c = parse(BETA);
        let f = settle(&c, &with(&c, &[]), &[]);
        for (i, l) in f.level.iter().enumerate() {
            assert!((l - 1.0).abs() < 1e-4, "{} = {l}", c.nodes[i].name);
        }
    }

    #[test]
    fn an_antagonist_lowers_the_whole_chain_and_an_agonist_raises_it() {
        let c = parse(BETA);
        let hr = idx(&c, "Fréquence cardiaque");
        let blocked = settle(&c, &with(&c, &["bisoprolol"]), &[]);
        assert_eq!(trend(blocked.level[hr]), Trend::Down);
        assert_eq!(trend(blocked.level[idx(&c, "AMPc")]), Trend::Down);
        // L'amont n'en sait rien : le ligand ne bouge pas.
        assert_eq!(trend(blocked.level[idx(&c, "Noradrénaline")]), Trend::Rest);
        let pushed = settle(&c, &with(&c, &["dobutamine"]), &[]);
        assert_eq!(trend(pushed.level[hr]), Trend::Up);
        // Et l'antagoniste donné avec l'agoniste le ramène vers le repos.
        let both = settle(&c, &with(&c, &["dobutamine", "bisoprolol"]), &[]);
        assert!(both.level[hr] < pushed.level[hr]);
        assert!(both.level[hr] > blocked.level[hr]);
    }

    #[test]
    fn a_partial_agonist_raises_rest_but_lowers_a_full_agonist() {
        let c = parse(BETA);
        let hr = idx(&c, "Fréquence cardiaque");
        let alone = settle(&c, &with(&c, &["pindolol"]), &[]);
        assert!(alone.level[hr] > 1.0);
        let full = settle(&c, &with(&c, &["dobutamine"]), &[]);
        let mixed = settle(&c, &with(&c, &["dobutamine", "pindolol"]), &[]);
        assert!(mixed.level[hr] < full.level[hr]);
    }

    #[test]
    fn a_potentiator_needs_a_signal_to_potentiate() {
        let text = "ligand GABA\nrécepteur GABA-A\nGABA -> GABA-A -> Sédation\n\
                    enzyme Stop -| GABA\nmolécule bzd : potentialisateur GABA-A\n\
                    molécule ko : inhibiteur GABA";
        let c = parse(text);
        assert!(c.faults.is_empty(), "{:?}", c.faults);
        let sed = idx(&c, "Sédation");
        assert_eq!(
            trend(settle(&c, &with(&c, &["bzd"]), &[]).level[sed]),
            Trend::Up
        );
        // Sans GABA, la benzodiazépine n'a rien à amplifier : bien en
        // dessous de ce qu'elle fait sur un GABA au repos.
        let none = settle(&c, &with(&c, &["ko"]), &[]);
        let none_bzd = settle(&c, &with(&c, &["ko", "bzd"]), &[]);
        assert!(none.level[sed] < 0.5);
        assert!(none_bzd.level[sed] < 1.0);
    }

    #[test]
    fn an_inhibiting_arrow_turns_a_fall_upstream_into_a_rise_downstream() {
        let c = parse("A -| B -> C\nmolécule x : inhibiteur A");
        let f = settle(&c, &with(&c, &["x"]), &[]);
        assert_eq!(trend(f.level[idx(&c, "A")]), Trend::Down);
        assert_eq!(trend(f.level[idx(&c, "C")]), Trend::Up);
    }

    #[test]
    fn a_feedback_loop_settles_and_stays_within_bounds() {
        let c = parse("A -> B -> C\nC -| B\nmolécule x : agoniste A");
        let back = back_edges(&c);
        assert_eq!(back, vec![false, false, true]);
        let f = settle(&c, &with(&c, &["x"]), &[]);
        let again = settle(&c, &with(&c, &["x"]), &[]);
        assert_eq!(f, again);
        for l in &f.level {
            assert!(l.is_finite() && (0.0..=CEILING).contains(l));
        }
        // Le rétrocontrôle freine : C monte moins qu'il ne monterait
        // sans la boucle.
        let open = parse("A -> B -> C\nmolécule x : agoniste A");
        let g = settle(&open, &with(&open, &["x"]), &[]);
        assert!(f.level[2] < g.level[2]);
    }

    #[test]
    fn a_long_blocked_receptor_multiplies_and_rebounds_when_stopped() {
        let c = parse(BETA);
        let hr = idx(&c, "Fréquence cardiaque");
        let b1 = b1_of(&c);
        let m = c
            .molecules
            .iter()
            .position(|m| m.name == "bisoprolol")
            .unwrap();
        let doses = [Dose {
            molecule: m,
            from: 5,
            until: Some(60),
        }];
        let frames = run(&c, &doses, 100);
        assert_eq!(frames.len(), 101);
        // Au repos avant, bloqué dès la prise.
        assert_eq!(trend(frames[4].level[hr]), Trend::Rest);
        assert_eq!(trend(frames[6].level[hr]), Trend::Down);
        // Pendant le traitement les récepteurs se multiplient, et
        // l'effet s'érode un peu — sans revenir au repos.
        assert!(frames[59].density[b1] > 1.5);
        assert!(frames[59].level[hr] > frames[6].level[hr]);
        assert_eq!(trend(frames[59].level[hr]), Trend::Down);
        // L'arrêt : le rebond, puis le retour.
        assert_eq!(trend(frames[60].level[hr]), Trend::Up);
        assert!(frames[100].level[hr] < frames[60].level[hr]);
        // Et un nœud qui ne s'adapte pas garde sa densité.
        assert!(frames.iter().all(|f| f.density[idx(&c, "AMPc")] == 1.0));
    }

    #[test]
    fn a_long_stimulated_receptor_grows_tolerant_and_falls_when_stopped() {
        let c = parse(BETA);
        let hr = idx(&c, "Fréquence cardiaque");
        let m = c
            .molecules
            .iter()
            .position(|m| m.name == "dobutamine")
            .unwrap();
        let frames = run(
            &c,
            &[Dose {
                molecule: m,
                from: 0,
                until: Some(50),
            }],
            80,
        );
        assert!(frames[49].level[hr] < frames[0].level[hr]);
        assert_eq!(trend(frames[50].level[hr]), Trend::Down);
    }

    #[test]
    fn the_run_never_leaves_its_bounds() {
        let c = parse(BETA);
        let doses: Vec<Dose> = (0..c.molecules.len())
            .map(|m| Dose {
                molecule: m,
                from: m as u32 * 7,
                until: Some(40 + m as u32 * 3),
            })
            .collect();
        for f in run(&c, &doses, 120) {
            assert!(f
                .level
                .iter()
                .all(|l| l.is_finite() && (0.0..=CEILING).contains(l)));
            assert!(f
                .density
                .iter()
                .all(|d| (DENSITY_MIN..=DENSITY_MAX).contains(d)));
        }
    }

    #[test]
    fn layers_go_down_along_every_forward_arrow_and_no_two_nodes_overlap() {
        for text in [
            BETA,
            "A -> B -> C\nA -> C\nC -| B",
            "A, B, C, D -> E\nE -> F, G, H\nX\nrécepteur Seul",
            "",
        ] {
            let c = parse(text);
            let l = layout(&c);
            assert_eq!(l.x.len(), c.nodes.len());
            for (k, e) in c.edges.iter().enumerate() {
                if !l.back[k] {
                    assert!(l.layer[e.to] > l.layer[e.from], "{text}");
                }
            }
            for a in 0..c.nodes.len() {
                assert!(l.x[a] >= 0.0 && l.x[a] + 1.0 <= l.span + 1e-4);
                for b in 0..a {
                    if l.layer[a] == l.layer[b] {
                        assert!((l.x[a] - l.x[b]).abs() >= 1.0 - 1e-4, "{text}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_loop_is_cut_where_the_text_goes_back_up() {
        // La rénine est nommée avant la pression : c'est la flèche de la
        // pression vers la rénine qui remonte, pas l'inverse.
        let c = parse("Rénine -> Angiotensine -> Pression\nPression -| Rénine");
        assert_eq!(back_edges(&c), vec![false, false, true]);
        assert_eq!(layout(&c).layer, vec![0, 1, 2]);
        // Écrite de bas en haut mais sans boucle : rien n'est coupé.
        let c = parse("B -> C\nA -> B");
        assert_eq!(back_edges(&c), vec![false, false]);
        assert_eq!(layout(&c).layer[c.find("A").unwrap()], 0);
    }

    #[test]
    fn toggling_gives_from_now_stops_now_and_leaves_nothing_when_undone() {
        let mut d = Vec::new();
        toggle(&mut d, 0, 5);
        assert_eq!(
            d,
            vec![Dose {
                molecule: 0,
                from: 5,
                until: None
            }]
        );
        toggle(&mut d, 0, 20);
        assert_eq!(d[0].until, Some(20));
        // Redonnée à l'instant où elle s'arrêtait : une seule prise.
        toggle(&mut d, 0, 20);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].until, None);
        // Cochée puis décochée au même instant : rien.
        toggle(&mut d, 1, 7);
        toggle(&mut d, 1, 7);
        assert!(d.iter().all(|x| x.molecule == 0));
        // Donnée avant une prise écrite plus loin : elle court jusqu'à
        // elle, et les deux se recousent.
        let mut d = vec![Dose {
            molecule: 2,
            from: 30,
            until: Some(40),
        }];
        toggle(&mut d, 2, 10);
        assert_eq!(
            d,
            vec![Dose {
                molecule: 2,
                from: 10,
                until: Some(40)
            }]
        );
        assert!(d[0].given_at(10) && d[0].given_at(39) && !d[0].given_at(40));
        assert!(!d[0].given_at(9));
    }

    #[test]
    fn the_colours_cover_the_text_end_to_end_and_mark_what_parse_reads() {
        let text = "# titre\nrécepteur Bêta-1 : note\nA -> B -| C # fin\n\
                    molécule x : agoniste partiel A ; inhibiteur B\nwhatever";
        let spans = colour(text);
        let mut at = 0;
        for (from, to, _) in &spans {
            assert_eq!(*from, at, "{spans:?}");
            assert!(to > from);
            assert!(text.is_char_boundary(*from) && text.is_char_boundary(*to));
            at = *to;
        }
        assert_eq!(at, text.len());
        let of = |ink: Ink| -> Vec<&str> {
            spans
                .iter()
                .filter(|s| s.2 == ink)
                .map(|s| &text[s.0..s.1])
                .collect()
        };
        assert_eq!(of(Ink::Keyword), vec!["récepteur", "molécule"]);
        assert_eq!(of(Ink::Arrow), vec!["->", "-|"]);
        assert_eq!(of(Ink::Action), vec!["agoniste partiel", "inhibiteur"]);
        assert!(of(Ink::Comment).iter().any(|c| c.starts_with("# titre")));
        assert!(of(Ink::Comment).iter().any(|c| c.starts_with(" # fin")));
        for text in STARTER_CASCADES {
            let spans = colour(text);
            assert_eq!(spans.last().map_or(0, |s| s.1), text.len());
        }
    }

    #[test]
    fn a_low_tone_node_loses_little_when_blocked_and_gains_everything_when_stimulated() {
        let c = parse(
            "ligand Endorphines\nrécepteur Mu\nEndorphines -> Mu -> Effet\n\
             tonus faible Mu\nmolécule morphine : agoniste Mu\n\
             molécule naloxone : antagoniste Mu\ntonus fort Mu",
        );
        assert_eq!(c.faults.len(), 1, "{:?}", c.faults);
        assert!(matches!(c.faults[0], Fault::UnknownAction { line: 7, .. }));
        assert!(c.nodes[c.find("Mu").unwrap()].low_tone);
        let e = idx(&c, "Effet");
        assert_eq!(
            trend(settle(&c, &with(&c, &["naloxone"]), &[]).level[e]),
            Trend::Rest
        );
        assert_eq!(
            trend(settle(&c, &with(&c, &["morphine"]), &[]).level[e]),
            Trend::Up
        );
        // Et sur la morphine, elle la renverse.
        let both = settle(&c, &with(&c, &["morphine", "naloxone"]), &[]).level[e];
        assert_eq!(trend(both), Trend::Rest);
    }

    #[test]
    fn a_source_sits_just_above_what_it_feeds() {
        let c = parse("A -> B -> C -> D\nX -| D");
        let l = layout(&c);
        assert_eq!(l.layer[c.find("X").unwrap()], 2);
        assert_eq!(l.layer[c.find("A").unwrap()], 0);
    }

    #[test]
    fn the_lint_names_a_molecule_without_card_and_a_node_left_alone() {
        let c =
            parse("A -> B\nrécepteur Bêta1\nmolécule x : antagoniste A\nmolécule y : agoniste B");
        let found = lint(&c, |n| n == "y");
        assert_eq!(
            found,
            vec![Lint::Isolated("Bêta1".into()), Lint::NoCard("x".into())]
        );
        for text in STARTER_CASCADES {
            let c = parse(text);
            assert!(lint(&c, |_| true).is_empty(), "{}", c.title);
        }
    }

    #[test]
    fn a_pasted_text_of_hundreds_of_nodes_plays_in_a_blink() {
        // Une chaîne de trois cents nœuds, une boucle, une molécule.
        let mut text = String::new();
        for i in 0..300 {
            text.push_str(&format!("N{i} -> N{}\n", i + 1));
        }
        text.push_str("N300 -| N150\nmolécule x : agoniste N0\nadaptation N10\n");
        let c = parse(&text);
        assert!(c.faults.is_empty());
        let started = std::time::Instant::now();
        let frames = run(
            &c,
            &[Dose {
                molecule: 0,
                from: 0,
                until: Some(60),
            }],
            120,
        );
        assert_eq!(frames.len(), 121);
        let _ = layout(&c);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(3),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_name_that_starts_like_a_keyword_is_still_a_name_on_an_arrow_line() {
        let text = "Tonus vagal -| Fréquence cardiaque\nSource de calcium -> Contraction\n\
                    Adaptation cellulaire -> Survie\ntitre : A -> B\nsource : X -> Y";
        let c = parse(text);
        assert!(c.faults.is_empty(), "{:?}", c.faults);
        assert_eq!(c.edges.len(), 3);
        assert!(c.find("Tonus vagal").is_some());
        assert_eq!(c.title, "A -> B");
        assert_eq!(c.sources, vec!["X -> Y".to_owned()]);
        let spans = colour(text);
        let keywords: Vec<&str> = spans
            .iter()
            .filter(|s| s.2 == Ink::Keyword)
            .map(|s| &text[s.0..s.1])
            .collect();
        assert_eq!(keywords, vec!["titre", "source"]);
    }

    #[test]
    fn domperidone_stays_outside_the_brain_and_metoclopramide_does_not() {
        let c = shipped("Récepteurs dopaminergiques D2");
        let (motor, prl, vom) = (
            "Contrôle moteur",
            "Prolactinémie",
            "Nausées et vomissements",
        );
        assert_eq!(trend(level(&c, &["dompéridone"], motor)), Trend::Rest);
        assert_eq!(trend(level(&c, &["dompéridone"], prl)), Trend::Up);
        // Seule, rien à retirer : l'area postrema est au repos sans
        // stimulation.
        assert_eq!(trend(level(&c, &["dompéridone"], vom)), Trend::Rest);
        assert_eq!(trend(level(&c, &["métoclopramide"], motor)), Trend::Down);
        // Le métoclopramide défait la lévodopa ; la dompéridone lui ôte
        // les nausées sans toucher au mouvement.
        let l = "lévodopa + carbidopa";
        assert!(level(&c, &[l, "métoclopramide"], motor) < level(&c, &[l], motor));
        assert_eq!(
            level(&c, &[l, "dompéridone"], motor),
            level(&c, &[l], motor)
        );
        assert!(level(&c, &[l, "dompéridone"], vom) < level(&c, &[l], vom));
        assert_eq!(trend(level(&c, &["bromocriptine"], prl)), Trend::Down);
    }

    #[test]
    fn oxybutynin_undoes_donepezil_in_the_brain_and_trospium_does_not() {
        let c = shipped("Récepteurs muscariniques");
        let (mind, hr, bladder) = (
            "Mémoire et vigilance",
            "Fréquence cardiaque",
            "Contraction vésicale",
        );
        assert_eq!(trend(level(&c, &["donépézil"], mind)), Trend::Up);
        assert_eq!(trend(level(&c, &["donépézil"], hr)), Trend::Down);
        assert_eq!(trend(level(&c, &["donépézil"], bladder)), Trend::Up);
        assert_eq!(trend(level(&c, &["oxybutynine"], mind)), Trend::Down);
        assert_eq!(trend(level(&c, &["trospium"], mind)), Trend::Rest);
        assert!(level(&c, &["donépézil", "oxybutynine"], mind) < level(&c, &["donépézil"], mind));
        assert_eq!(
            level(&c, &["donépézil", "trospium"], mind),
            level(&c, &["donépézil"], mind)
        );
        assert_eq!(trend(level(&c, &["amitriptyline"], hr)), Trend::Up);
        assert_eq!(trend(level(&c, &["trospium"], hr)), Trend::Up);
        assert_eq!(trend(level(&c, &["solifénacine"], hr)), Trend::Rest);
        assert_eq!(
            trend(level(&c, &["tiotropium"], "Bronchoconstriction")),
            Trend::Down
        );
        assert_eq!(trend(level(&c, &["tiotropium"], "Salivation")), Trend::Rest);
    }

    #[test]
    fn a_scenario_reads_its_doses_and_reports_what_it_cannot() {
        let c = parse(
            "A -> B\nmolécule lévodopa + carbidopa : activateur A\nmolécule x : antagoniste B\n\
             scénario Arrêt : x 10-70 ; lévodopa + carbidopa 5-\n\
             scénario Faux : x 70-10 ; x 130-\nscénario Inconnu : y 1-2\nscénario sans deux points\n\
             Scenario Vide :",
        );
        assert_eq!(c.scenarios.len(), 1);
        let s = &c.scenarios[0];
        assert_eq!(s.name, "Arrêt");
        assert_eq!(
            s.doses,
            vec![
                Dose {
                    molecule: 0,
                    from: 5,
                    until: None
                },
                Dose {
                    molecule: 1,
                    from: 10,
                    until: Some(70)
                },
            ]
        );
        let lines: Vec<usize> = c.faults.iter().map(Fault::line).collect();
        // Deux prises refusées ligne 5 : à l'envers, et hors du temps.
        assert_eq!(lines, vec![5, 5, 6, 7, 8], "{:?}", c.faults);
        assert!(matches!(c.faults[2], Fault::UnknownMolecule { .. }));
    }

    #[test]
    fn every_shipped_scenario_tells_something() {
        for text in STARTER_CASCADES {
            let c = parse(text);
            assert!(!c.scenarios.is_empty(), "{} sans scénario", c.title);
            for sc in &c.scenarios {
                assert!(sc.doses.iter().all(|d| d.until.unwrap_or(120) <= 120));
                let frames = run(&c, &sc.doses, 120);
                let moves = c
                    .outcomes()
                    .iter()
                    .any(|&o| frames.iter().any(|f| trend(f.level[o]) != Trend::Rest));
                assert!(moves, "{} : « {} » ne montre rien", c.title, sc.name);
            }
        }
    }

    #[test]
    fn a_lineage_keeps_what_feeds_a_node_and_what_it_feeds() {
        let c = parse("A -> B -> C\nX -> B\nB -> D\nY -> Z\nC -| A");
        let keep = lineage(&c, c.find("C").unwrap());
        let kept: Vec<&str> = c
            .nodes
            .iter()
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(n, _)| n.name.as_str())
            .collect();
        // La boucle ramène tout ce qui touche à B ; Y et Z restent dehors.
        assert_eq!(kept, vec!["A", "B", "C", "X", "D"]);
        assert!(lineage(&c, 99).iter().all(|k| !k));
    }

    #[test]
    fn a_copy_keeps_everything_but_its_title() {
        let text = "# note\ntitre : Bêta\nA -> B\n";
        let copy = copy_text(text, "(copie)");
        assert_eq!(copy, "# note\ntitre : Bêta (copie)\nA -> B\n");
        assert_eq!(parse(&copy).title, "Bêta (copie)");
        assert_eq!(parse(&copy).edges, parse(text).edges);
        assert_eq!(copy_text("A -> B", "(copie)"), "titre : (copie)\nA -> B");
        // Le titre retenu est le dernier, coupé à son commentaire.
        let copy = copy_text("titre : Un\ntitre : Bêta # v2\n", "(copie)");
        assert_eq!(copy, "titre : Un\ntitre : Bêta (copie) # v2\n");
        assert_eq!(parse(&copy).title, "Bêta (copie)");
    }

    #[test]
    fn a_straight_chain_is_laid_straight() {
        let c = parse("A -> B -> C -> D");
        let l = layout(&c);
        assert_eq!(l.layer, vec![0, 1, 2, 3]);
        assert!(l.x.iter().all(|x| *x == 0.0));
        assert_eq!(l.span, 1.0);
    }

    // --- Le contenu livré ---------------------------------------------

    use crate::db::STARTER_CASCADES;

    /// Le plancher : une cascade retirée est une question à laquelle le
    /// comptoir ne sait plus répondre. Il ne peut que monter.
    const SHIPPED: usize = 51;

    /// Les molécules qu'un mécanisme ne peut pas taire et que la base
    /// n'a pas en fiche. Tenues ici par leur nom : ailleurs, une faute de
    /// frappe dans une DCI se lirait comme une molécule sans fiche.
    const WITHOUT_CARD: &[&str] = &[];

    fn shipped(title: &str) -> Cascade {
        STARTER_CASCADES
            .iter()
            .map(|t| parse(t))
            .find(|c| c.title == title)
            .unwrap_or_else(|| panic!("« {title} » n'est pas livrée"))
    }

    fn level(c: &Cascade, given: &[&str], node: &str) -> f32 {
        settle(c, &with(c, given), &[]).level[idx(c, node)]
    }

    /// Le sens d'un effet sous une liste de molécules : ce que les tests
    /// des cascades de 0.358 viennent y lire.
    fn reads(c: &Cascade, given: &[&str], node: &str) -> Trend {
        trend(level(c, given, node))
    }

    /// Une histoire toute prête, jouée : l'état à chaque pas.
    fn played(c: &Cascade, name: &str) -> (Vec<Frame>, u32) {
        let sc = c
            .scenarios
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{} : pas de scénario « {name} »", c.title));
        let stop = sc.doses.iter().filter_map(|d| d.until).max().unwrap_or(0);
        (run(c, &sc.doses, 120), stop)
    }

    #[test]
    fn an_anti_tnf_opens_tuberculosis_and_etanercept_spares_the_gut() {
        let c = shipped("TNF-alpha");
        for m in [
            "adalimumab",
            "étanercept",
            "infliximab",
            "certolizumab pégol",
        ] {
            assert_eq!(
                reads(&c, &[m], "Défense contre la tuberculose"),
                Trend::Down,
                "{m}"
            );
            assert_eq!(
                reads(&c, &[m], "Inflammation articulaire"),
                Trend::Down,
                "{m}"
            );
        }
        assert_eq!(
            reads(&c, &["infliximab"], "Inflammation intestinale"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["étanercept"], "Inflammation intestinale"),
            Trend::Rest
        );
        assert_eq!(reads(&c, &["adalimumab"], "CRP"), Trend::Down);
    }

    #[test]
    fn an_anti_il6_hides_the_crp_and_gives_the_cytochromes_back() {
        let c = shipped("Interleukine 6");
        for m in ["tocilizumab", "sarilumab"] {
            assert_eq!(reads(&c, &[m], "CRP"), Trend::Down, "{m}");
            assert_eq!(reads(&c, &[m], "Fièvre"), Trend::Down, "{m}");
            assert_eq!(
                reads(&c, &[m], "Activité des cytochromes hépatiques"),
                Trend::Up,
                "{m}"
            );
            assert_eq!(
                reads(&c, &[m], "Exposition aux substrats des cytochromes"),
                Trend::Down,
                "{m}"
            );
        }
        assert_eq!(reads(&c, &["tocilizumab"], "Hémoglobine"), Trend::Up);
    }

    #[test]
    fn a_jak_inhibitor_lowers_the_antiviral_defence_and_jak2_the_haemoglobin() {
        let c = shipped("Voie JAK-STAT");
        assert_eq!(
            reads(&c, &["upadacitinib"], "Défense antivirale"),
            Trend::Down
        );
        assert_eq!(reads(&c, &["tofacitinib"], "Lymphocytes"), Trend::Down);
        assert_eq!(reads(&c, &["ruxolitinib"], "Hémoglobine"), Trend::Down);
        assert!(
            level(&c, &["ruxolitinib"], "Hémoglobine")
                < level(&c, &["upadacitinib"], "Hémoglobine")
        );
        assert_eq!(
            reads(&c, &["deucravacitinib"], "Inflammation cutanée"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["baricitinib"], "Inflammation articulaire"),
            Trend::Down
        );
    }

    #[test]
    fn an_anti_il17_can_worsen_the_gut_and_an_anti_il23_treats_it() {
        let c = shipped("Axe IL-23 et IL-17");
        for m in ["sécukinumab", "guselkumab", "brodalumab"] {
            assert_eq!(reads(&c, &[m], "Plaques de psoriasis"), Trend::Down, "{m}");
        }
        assert_eq!(
            reads(&c, &["sécukinumab"], "Inflammation intestinale"),
            Trend::Up
        );
        assert_eq!(
            reads(&c, &["guselkumab"], "Inflammation intestinale"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["sécukinumab"], "Défense contre Candida"),
            Trend::Down
        );
        assert!(
            level(&c, &["guselkumab"], "Défense contre Candida")
                > level(&c, &["sécukinumab"], "Défense contre Candida")
        );
    }

    #[test]
    fn dupilumab_raises_the_blood_eosinophils_and_anti_il5_lowers_them() {
        let c = shipped("Inflammation de type 2");
        assert_eq!(
            reads(&c, &["dupilumab"], "Éosinophiles sanguins"),
            Trend::Up
        );
        for m in ["mépolizumab", "benralizumab", "tézépélumab"] {
            assert_eq!(reads(&c, &[m], "Éosinophiles sanguins"), Trend::Down, "{m}");
        }
        assert!(
            level(&c, &["benralizumab"], "Éosinophiles sanguins")
                < level(&c, &["mépolizumab"], "Éosinophiles sanguins")
        );
        assert_eq!(reads(&c, &["dupilumab"], "Eczéma atopique"), Trend::Down);
        assert_eq!(
            reads(&c, &["omalizumab"], "Exacerbations d'asthme"),
            Trend::Down
        );
    }

    #[test]
    fn a_nitrate_on_a_pde5_inhibitor_falls_further_than_either() {
        let c = shipped("Monoxyde d'azote et GMPc");
        let pa = |g: &[&str]| level(&c, g, "Pression artérielle");
        assert!(pa(&["sildénafil", "trinitrine"]) < pa(&["trinitrine"]));
        assert!(pa(&["sildénafil", "trinitrine"]) < pa(&["sildénafil"]));
        assert!(pa(&["riociguat", "sildénafil"]) < pa(&["riociguat"]));
        assert_eq!(
            reads(&c, &["trinitrine"], "Besoin en oxygène du myocarde"),
            Trend::Down
        );
        assert_eq!(reads(&c, &["trinitrine"], "Érection"), Trend::Rest);
        // La tolérance loge dans la bioactivation : l'effet s'érode.
        let (f, _) = played(&c, "Trinitrine en continu, puis arrêt");
        let o2 = idx(&c, "Besoin en oxygène du myocarde");
        assert!(f[70].level[o2] > f[11].level[o2]);
    }

    #[test]
    fn the_loop_and_the_thiazide_lose_potassium_and_the_mr_blockers_keep_it() {
        let c = shipped("Néphron et potassium");
        for m in [
            "furosémide",
            "bumétanide",
            "hydrochlorothiazide",
            "indapamide",
        ] {
            assert_eq!(reads(&c, &[m], "Kaliémie"), Trend::Down, "{m}");
        }
        for m in ["spironolactone", "éplérénone", "amiloride"] {
            assert_eq!(reads(&c, &[m], "Kaliémie"), Trend::Up, "{m}");
        }
        assert_eq!(reads(&c, &["furosémide"], "Calciurie"), Trend::Up);
        assert_eq!(
            reads(&c, &["hydrochlorothiazide"], "Calciurie"),
            Trend::Down
        );
        assert!(
            level(&c, &["furosémide", "spironolactone"], "Kaliémie")
                > level(&c, &["furosémide"], "Kaliémie")
        );
        assert!(
            level(&c, &["furosémide", "hydrochlorothiazide"], "Natriurèse")
                > level(&c, &["furosémide"], "Natriurèse")
        );
        assert_eq!(reads(&c, &["furosémide"], "Aldostérone"), Trend::Up);
    }

    #[test]
    fn a_sulfonylurea_forces_insulin_and_metformin_does_not() {
        let c = shipped("Glycémie, insuline et incrétines");
        for m in [
            "gliclazide",
            "metformine",
            "dapagliflozine",
            "sémaglutide",
            "insuline glargine",
        ] {
            assert_eq!(reads(&c, &[m], "Glycémie"), Trend::Down, "{m}");
        }
        assert_eq!(reads(&c, &["gliclazide"], "Insuline sécrétée"), Trend::Up);
        assert_eq!(reads(&c, &["metformine"], "Insuline sécrétée"), Trend::Rest);
        assert_eq!(
            reads(&c, &["dapagliflozine"], "Insuline sécrétée"),
            Trend::Rest
        );
        assert!(
            level(&c, &["sémaglutide"], "Insuline sécrétée")
                < level(&c, &["gliclazide"], "Insuline sécrétée")
        );
        assert!(
            level(&c, &["gliclazide", "sémaglutide"], "Glycémie")
                < level(&c, &["gliclazide"], "Glycémie")
        );
        assert_eq!(reads(&c, &["sémaglutide"], "Appétit"), Trend::Down);
        assert_eq!(reads(&c, &["sitagliptine"], "Appétit"), Trend::Rest);
    }

    #[test]
    fn a_statin_raises_pcsk9_and_ezetimibe_adds_to_it() {
        let c = shipped("LDL-cholestérol et récepteur LDL");
        assert_eq!(
            reads(&c, &["atorvastatine"], "LDL-cholestérol"),
            Trend::Down
        );
        assert_eq!(reads(&c, &["atorvastatine"], "PCSK9"), Trend::Up);
        assert_eq!(reads(&c, &["ézétimibe"], "HMG-CoA réductase"), Trend::Up);
        let ldl = |g: &[&str]| level(&c, g, "LDL-cholestérol");
        assert!(ldl(&["atorvastatine", "ézétimibe"]) < ldl(&["atorvastatine"]));
        assert!(ldl(&["atorvastatine", "évolocumab"]) < ldl(&["atorvastatine"]));
        assert_eq!(
            reads(&c, &["acide bempédoïque"], "LDL-cholestérol"),
            Trend::Down
        );
    }

    #[test]
    fn a_uroselective_alpha_blocker_still_lowers_the_pressure_less() {
        let c = shipped("Récepteurs alpha-1 adrénergiques");
        assert_eq!(reads(&c, &["tamsulosine"], "Débit urinaire"), Trend::Up);
        assert_eq!(
            reads(&c, &["doxazosine"], "Pression artérielle"),
            Trend::Down
        );
        assert!(
            level(&c, &["tamsulosine"], "Pression artérielle")
                > level(&c, &["doxazosine"], "Pression artérielle")
        );
        assert_eq!(reads(&c, &["doxazosine"], "Fréquence cardiaque"), Trend::Up);
        assert_eq!(
            reads(&c, &["tamsulosine"], "Dilatation de l'iris"),
            Trend::Down
        );
    }

    #[test]
    fn an_ssri_waits_for_its_autoreceptor_and_tramadol_adds_to_it() {
        let c = shipped("Synapse sérotoninergique");
        assert_eq!(
            reads(&c, &["sertraline"], "Transmission sérotoninergique"),
            Trend::Up
        );
        assert_eq!(
            reads(&c, &["sertraline"], "Agrégation plaquettaire"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["sertraline"], "Pression artérielle"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["venlafaxine"], "Pression artérielle"),
            Trend::Up
        );
        let t = |g: &[&str]| level(&c, g, "Transmission sérotoninergique");
        assert!(t(&["sertraline", "tramadol"]) > t(&["sertraline"]));
        assert!(t(&["sertraline", "moclobémide"]) > t(&["sertraline"]));
        for m in ["tramadol", "moclobémide"] {
            assert_eq!(
                reads(&c, &[m], "Agrégation plaquettaire"),
                Trend::Rest,
                "{m}"
            );
        }
        // Le délai d'action : l'autorécepteur se désensibilise.
        let (f, _) = played(&c, "ISRS au long cours");
        let tr = idx(&c, "Transmission sérotoninergique");
        assert!(f[120].level[tr] > f[10].level[tr]);
    }

    #[test]
    fn a_setron_takes_the_acute_phase_and_the_nk1_antagonist_the_delayed_one() {
        let c = shipped("Centre du vomissement");
        let chemo = "cyclophosphamide";
        assert_eq!(reads(&c, &[chemo], "Vomissements aigus"), Trend::Up);
        assert_eq!(reads(&c, &[chemo], "Vomissements retardés"), Trend::Up);
        assert_eq!(
            reads(&c, &[chemo, "ondansétron"], "Vomissements aigus"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &[chemo, "ondansétron"], "Vomissements retardés"),
            Trend::Up
        );
        assert_eq!(
            reads(&c, &[chemo, "aprépitant"], "Vomissements retardés"),
            Trend::Rest
        );
        assert_eq!(reads(&c, &["apomorphine"], "Vomissements aigus"), Trend::Up);
        assert_eq!(
            reads(&c, &["apomorphine", "dompéridone"], "Vomissements aigus"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["scopolamine"], "Mal des transports"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["ondansétron"], "Mal des transports"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &[chemo, "scopolamine"], "Vomissements aigus"),
            Trend::Up
        );
    }

    #[test]
    fn levothyroxine_lowers_the_tsh_and_an_antithyroid_raises_it() {
        let c = shipped("Axe thyréotrope");
        assert_eq!(reads(&c, &["lévothyroxine"], "TSH"), Trend::Down);
        assert_eq!(
            reads(&c, &["lévothyroxine"], "Fréquence cardiaque"),
            Trend::Up
        );
        for m in ["carbimazole", "thiamazole", "lithium"] {
            assert_eq!(reads(&c, &[m], "TSH"), Trend::Up, "{m}");
        }
        assert_eq!(reads(&c, &["carbimazole"], "T4 libre"), Trend::Down);
        assert_eq!(reads(&c, &["amiodarone"], "T4 libre"), Trend::Up);
        assert_eq!(reads(&c, &["amiodarone"], "T3 libre"), Trend::Down);
        assert_eq!(reads(&c, &["liothyronine"], "T4 libre"), Trend::Down);
    }

    #[test]
    fn a_corticoid_silences_the_adrenal_and_fludrocortisone_only_the_kidney() {
        let c = shipped("Axe corticotrope");
        assert_eq!(reads(&c, &["prednisone"], "Cortisol endogène"), Trend::Down);
        assert_eq!(reads(&c, &["prednisone"], "Glycémie"), Trend::Up);
        assert_eq!(reads(&c, &["prednisone"], "Inflammation"), Trend::Down);
        assert_eq!(reads(&c, &["bétaméthasone"], "Kaliémie"), Trend::Rest);
        assert_eq!(reads(&c, &["fludrocortisone"], "Kaliémie"), Trend::Down);
        assert_eq!(reads(&c, &["fludrocortisone"], "Glycémie"), Trend::Rest);
        assert_eq!(
            reads(&c, &["fludrocortisone"], "Cortisol endogène"),
            Trend::Rest
        );
        // Aucune adaptation : le modèle dirait l'inverse de l'atrophie.
        assert!(c.nodes.iter().all(|n| !n.adapts));
    }

    #[test]
    fn stopping_denosumab_rebounds_and_a_bisphosphonate_relay_does_not() {
        let c = shipped("Remodelage osseux");
        assert_eq!(reads(&c, &["dénosumab"], "Résorption osseuse"), Trend::Down);
        assert_eq!(reads(&c, &["dénosumab"], "Calcémie"), Trend::Down);
        assert_eq!(reads(&c, &["tériparatide"], "Formation osseuse"), Trend::Up);
        assert_eq!(
            reads(&c, &["alendronate"], "Formation osseuse"),
            Trend::Rest
        );
        let res = idx(&c, "Résorption osseuse");
        let (f, stop) = played(&c, "Arrêt du dénosumab sans relais");
        assert!(f[stop as usize..]
            .iter()
            .any(|f| trend(f.level[res]) == Trend::Up));
        let (f, stop) = played(&c, "Arrêt du dénosumab, relais par l'acide zolédronique");
        assert!(f[stop as usize..]
            .iter()
            .all(|f| trend(f.level[res]) != Trend::Up));
    }

    #[test]
    fn a_first_generation_antihistamine_sedates_and_bilastine_does_not() {
        let c = shipped("Récepteurs histaminiques H1");
        assert_eq!(reads(&c, &["hydroxyzine"], "Vigilance"), Trend::Down);
        assert_eq!(reads(&c, &["hydroxyzine"], "Salivation"), Trend::Down);
        for m in ["bilastine", "desloratadine", "cétirizine"] {
            assert_eq!(reads(&c, &[m], "Vigilance"), Trend::Rest, "{m}");
            assert_eq!(reads(&c, &[m], "Prurit et urticaire"), Trend::Down, "{m}");
        }
    }

    #[test]
    fn a_gnrh_agonist_or_antagonist_castrates_and_clomifene_lifts_the_feedback() {
        let c = shipped("Axe gonadotrope");
        assert_eq!(reads(&c, &["triptoréline"], "Testostérone"), Trend::Down);
        assert_eq!(reads(&c, &["dégarélix"], "Testostérone"), Trend::Down);
        assert_eq!(reads(&c, &["leuproréline"], "Œstradiol"), Trend::Down);
        assert_eq!(reads(&c, &["citrate de clomifène"], "FSH"), Trend::Up);
        assert_eq!(
            reads(&c, &["citrate de clomifène"], "Croissance folliculaire"),
            Trend::Up
        );
        assert_eq!(
            reads(&c, &["testostérone énanthate"], "Spermatogenèse"),
            Trend::Down
        );
    }

    #[test]
    fn abiraterone_loses_potassium_that_prednisone_gives_back() {
        let c = shipped("Récepteur des androgènes");
        assert_eq!(reads(&c, &["abiratérone"], "Kaliémie"), Trend::Down);
        assert_eq!(
            reads(&c, &["abiratérone"], "Pression artérielle"),
            Trend::Up
        );
        assert!(
            level(&c, &["abiratérone", "prednisone"], "Kaliémie")
                > level(&c, &["abiratérone"], "Kaliémie")
        );
        assert_eq!(reads(&c, &["abiratérone"], "PSA"), Trend::Down);
        assert_eq!(reads(&c, &["enzalutamide"], "PSA"), Trend::Down);
        assert_eq!(reads(&c, &["finastéride"], "PSA"), Trend::Down);
    }

    #[test]
    fn tamoxifen_thickens_the_endometrium_and_an_aromatase_inhibitor_thins_the_bone() {
        let c = shipped("Récepteurs des œstrogènes et aromatase");
        assert_eq!(
            reads(&c, &["tamoxifène"], "Prolifération tumorale mammaire"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["tamoxifène"], "Épaisseur de l'endomètre"),
            Trend::Up
        );
        assert!(
            level(&c, &["raloxifène"], "Épaisseur de l'endomètre")
                < level(&c, &["tamoxifène"], "Épaisseur de l'endomètre")
        );
        assert_eq!(
            reads(&c, &["létrozole"], "Risque thromboembolique veineux"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["létrozole"], "Densité minérale osseuse"),
            Trend::Down
        );
        assert!(
            level(&c, &["létrozole"], "Risque thromboembolique veineux")
                < level(&c, &["tamoxifène"], "Risque thromboembolique veineux")
        );
        assert_eq!(
            reads(&c, &["fulvestrant"], "Prolifération tumorale mammaire"),
            Trend::Down
        );
    }

    #[test]
    fn desmopressin_lowers_the_sodium_and_a_thiazide_adds_to_it() {
        let c = shipped("Vasopressine et eau libre");
        assert_eq!(reads(&c, &["desmopressine"], "Natrémie"), Trend::Down);
        assert_eq!(reads(&c, &["tolvaptan"], "Natrémie"), Trend::Up);
        assert_eq!(reads(&c, &["tolvaptan"], "Soif"), Trend::Up);
        assert_eq!(reads(&c, &["hydrochlorothiazide"], "Natrémie"), Trend::Down);
        assert!(
            level(&c, &["desmopressine", "hydrochlorothiazide"], "Natrémie")
                < level(&c, &["desmopressine"], "Natrémie")
        );
        assert_eq!(reads(&c, &["lithium"], "Excrétion d'eau libre"), Trend::Up);
    }

    #[test]
    fn carbidopa_keeps_levodopa_off_the_stomach_and_entacapone_stretches_it() {
        let c = shipped("Lévodopa et dégradation de la dopamine");
        assert_eq!(
            reads(&c, &["lévodopa + carbidopa"], "Contrôle moteur"),
            Trend::Up
        );
        assert_eq!(
            reads(&c, &["lévodopa + carbidopa"], "Nausées et vomissements"),
            Trend::Rest
        );
        assert!(
            level(
                &c,
                &["lévodopa + carbidopa + entacapone"],
                "Contrôle moteur"
            ) > level(&c, &["lévodopa + carbidopa"], "Contrôle moteur")
        );
        assert_eq!(reads(&c, &["rasagiline"], "Contrôle moteur"), Trend::Up);
        assert_eq!(
            reads(&c, &["pramipexole"], "Pression artérielle"),
            Trend::Down
        );
        assert_eq!(
            reads(
                &c,
                &["ropinirole"],
                "Troubles du contrôle des impulsions et hallucinations"
            ),
            Trend::Up
        );
    }

    #[test]
    fn two_sodium_channel_blockers_add_their_dizziness_and_ethosuximide_takes_only_absences() {
        let c = shipped("Cibles des antiépileptiques");
        assert_eq!(reads(&c, &["carbamazépine"], "Seuil convulsif"), Trend::Up);
        assert_eq!(
            reads(&c, &["carbamazépine"], "Vertiges et ataxie"),
            Trend::Up
        );
        assert!(
            level(&c, &["carbamazépine", "lacosamide"], "Vertiges et ataxie")
                > level(&c, &["carbamazépine"], "Vertiges et ataxie")
        );
        assert_eq!(reads(&c, &["éthosuximide"], "Seuil convulsif"), Trend::Rest);
        assert_eq!(reads(&c, &["éthosuximide"], "Absences"), Trend::Down);
        assert_eq!(reads(&c, &["valproate de sodium"], "Absences"), Trend::Down);
        assert_eq!(
            reads(&c, &["vigabatrine"], "Vertiges et ataxie"),
            Trend::Rest
        );
    }

    #[test]
    fn a_triptan_narrows_the_coronaries_and_an_anti_cgrp_does_not() {
        let c = shipped("Migraine, sérotonine et CGRP");
        assert_eq!(
            reads(&c, &["sumatriptan"], "Céphalée migraineuse"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["sumatriptan"], "Calibre des coronaires"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["érénumab"], "Céphalée migraineuse"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["érénumab"], "Calibre des coronaires"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["galcanézumab"], "Calibre des coronaires"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["rimégépant"], "Céphalée migraineuse"),
            Trend::Down
        );
    }

    #[test]
    fn two_herg_blockers_lengthen_the_qt_more_than_one_and_hypokalaemia_adds() {
        let c = shipped("Canal hERG et QT long");
        assert_eq!(reads(&c, &["citalopram"], "Durée du QT"), Trend::Up);
        assert!(
            level(&c, &["citalopram", "clarithromycine"], "Durée du QT")
                > level(&c, &["citalopram"], "Durée du QT")
        );
        assert!(
            level(
                &c,
                &["hydroxyzine", "méthadone"],
                "Risque de torsades de pointes"
            ) > level(&c, &["méthadone"], "Risque de torsades de pointes")
        );
        assert_eq!(reads(&c, &["furosémide"], "Kaliémie"), Trend::Down);
        assert!(
            level(&c, &["dompéridone", "furosémide"], "Durée du QT")
                > level(&c, &["dompéridone"], "Durée du QT")
        );
        assert_eq!(reads(&c, &["sotalol"], "Fréquence cardiaque"), Trend::Down);
    }

    #[test]
    fn hypokalaemia_raises_digoxin_arrhythmias() {
        let c = shipped("Digoxine et pompe Na/K-ATPase");
        assert_eq!(reads(&c, &["digoxine"], "Contractilité"), Trend::Up);
        assert_eq!(
            reads(&c, &["digoxine"], "Conduction auriculo-ventriculaire"),
            Trend::Down
        );
        assert!(
            level(&c, &["digoxine", "furosémide"], "Troubles du rythme")
                > level(&c, &["digoxine"], "Troubles du rythme")
        );
        assert!(
            level(&c, &["digoxine", "bisoprolol"], "Fréquence cardiaque")
                < level(&c, &["digoxine"], "Fréquence cardiaque")
        );
        assert!(
            level(
                &c,
                &["digoxine", "vérapamil"],
                "Conduction auriculo-ventriculaire"
            ) < level(&c, &["digoxine"], "Conduction auriculo-ventriculaire")
        );
        assert_eq!(reads(&c, &["furosémide"], "Kaliémie"), Trend::Down);
    }

    #[test]
    fn allopurinol_raises_thiopurine_toxicity_without_a_thiopurine_it_does_nothing() {
        let c = shipped("Purines, urate et goutte");
        assert_eq!(reads(&c, &["allopurinol"], "Uricémie"), Trend::Down);
        assert_eq!(
            reads(&c, &["allopurinol"], "Toxicité médullaire des thiopurines"),
            Trend::Rest
        );
        assert!(
            level(
                &c,
                &["azathioprine", "allopurinol"],
                "Toxicité médullaire des thiopurines"
            ) > level(&c, &["azathioprine"], "Toxicité médullaire des thiopurines")
        );
        assert!(
            level(
                &c,
                &["mercaptopurine", "fébuxostat"],
                "Toxicité médullaire des thiopurines"
            ) > level(
                &c,
                &["mercaptopurine"],
                "Toxicité médullaire des thiopurines"
            )
        );
        assert_eq!(
            reads(&c, &["colchicine"], "Inflammation articulaire"),
            Trend::Down
        );
        assert_eq!(reads(&c, &["colchicine"], "Uricémie"), Trend::Rest);
        assert_eq!(reads(&c, &["hydrochlorothiazide"], "Uricémie"), Trend::Up);
        assert!(
            level(&c, &["losartan", "hydrochlorothiazide"], "Uricémie")
                < level(&c, &["hydrochlorothiazide"], "Uricémie")
        );
    }

    #[test]
    fn folinic_acid_rescues_methotrexate_and_folic_acid_does_not() {
        let c = shipped("Érythropoïèse, fer et folates");
        assert_eq!(reads(&c, &["époétine alfa"], "Hémoglobine"), Trend::Up);
        assert!(
            level(
                &c,
                &["époétine alfa", "carboxymaltose ferrique"],
                "Hémoglobine"
            ) > level(&c, &["époétine alfa"], "Hémoglobine")
        );
        assert_eq!(reads(&c, &["méthotrexate"], "Hémoglobine"), Trend::Down);
        assert!(
            level(&c, &["méthotrexate", "acide folinique"], "Hémoglobine")
                > level(&c, &["méthotrexate", "acide folique"], "Hémoglobine")
        );
        assert!(
            level(&c, &["méthotrexate", "acide folinique"], "Hémoglobine")
                > level(&c, &["méthotrexate"], "Hémoglobine")
        );
        assert_eq!(
            reads(&c, &["méthotrexate", "acide folinique"], "Hémoglobine"),
            Trend::Rest
        );
    }

    #[test]
    fn iron_or_calcium_taken_with_eltrombopag_cancel_its_rise() {
        let c = shipped("Thrombopoïèse");
        assert_eq!(reads(&c, &["eltrombopag"], "Plaquettes"), Trend::Up);
        assert_eq!(reads(&c, &["sulfate ferreux"], "Plaquettes"), Trend::Rest);
        assert!(
            level(&c, &["eltrombopag", "sulfate ferreux"], "Plaquettes")
                < level(&c, &["eltrombopag"], "Plaquettes")
        );
        assert_eq!(
            reads(&c, &["eltrombopag", "carbonate de calcium"], "Plaquettes"),
            Trend::Rest
        );
        assert_eq!(reads(&c, &["anagrélide"], "Plaquettes"), Trend::Down);
        assert_eq!(reads(&c, &["hydroxycarbamide"], "Plaquettes"), Trend::Down);
        assert_eq!(
            reads(&c, &["eltrombopag"], "Risque thrombotique"),
            Trend::Up
        );
    }

    #[test]
    fn a_laba_alone_leaves_the_asthma_exacerbations_and_an_ics_lowers_them() {
        let c = shipped("Bronchodilatation et inflammation bronchique");
        assert_eq!(reads(&c, &["salmétérol"], "Bronchodilatation"), Trend::Up);
        assert_eq!(
            reads(&c, &["salmétérol"], "Exacerbations d'asthme"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["budésonide"], "Exacerbations d'asthme"),
            Trend::Down
        );
        assert!(
            level(&c, &["budésonide + formotérol"], "Gêne respiratoire")
                < level(&c, &["formotérol"], "Gêne respiratoire")
        );
        assert_eq!(
            reads(&c, &["montélukast"], "Inflammation bronchique"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["tiotropium"], "Inflammation bronchique"),
            Trend::Rest
        );
        assert!(
            level(&c, &["salmétérol", "budésonide"], "Exacerbations d'asthme")
                < level(&c, &["salmétérol"], "Exacerbations d'asthme")
        );
        assert!(
            level(&c, &["budésonide"], "Inflammation bronchique")
                < level(&c, &["montélukast"], "Inflammation bronchique")
        );
    }

    #[test]
    fn the_cascade_canaux_calciques_l_vaisseaux_et_coeur_reads_as_drawn() {
        let c = shipped("Canaux calciques L, vaisseaux et cœur");
        assert_eq!(reads(&c, &["amlodipine"], "Fréquence cardiaque"), Trend::Up);
        assert_eq!(
            reads(&c, &["amlodipine"], "Œdèmes des chevilles"),
            Trend::Up
        );
        assert!(
            level(&c, &["amlodipine", "bisoprolol"], "Fréquence cardiaque")
                < level(&c, &["amlodipine"], "Fréquence cardiaque")
        );
        assert!(
            level(&c, &["amlodipine", "bisoprolol"], "Fréquence cardiaque")
                > level(&c, &["vérapamil", "bisoprolol"], "Fréquence cardiaque")
        );
        assert_eq!(
            reads(&c, &["vérapamil"], "Conduction auriculo-ventriculaire"),
            Trend::Down
        );
        assert!(
            level(&c, &["vérapamil", "bisoprolol"], "Fréquence cardiaque")
                < level(&c, &["vérapamil"], "Fréquence cardiaque")
        );
        assert!(
            level(
                &c,
                &["diltiazem", "bisoprolol"],
                "Conduction auriculo-ventriculaire"
            ) < level(&c, &["bisoprolol"], "Conduction auriculo-ventriculaire")
        );
        assert!(
            level(
                &c,
                &["amlodipine", "bisoprolol"],
                "Conduction auriculo-ventriculaire"
            ) > level(
                &c,
                &["vérapamil", "bisoprolol"],
                "Conduction auriculo-ventriculaire"
            )
        );
    }

    #[test]
    fn the_cascade_calcium_parathormone_et_vitamine_d_reads_as_drawn() {
        let c = shipped("Calcium, parathormone et vitamine D");
        assert_eq!(reads(&c, &["calcitriol"], "Calcémie"), Trend::Up);
        assert_eq!(reads(&c, &["calcitriol"], "Parathormone"), Trend::Down);
        assert_eq!(reads(&c, &["cinacalcet"], "Parathormone"), Trend::Down);
        assert_eq!(reads(&c, &["cinacalcet"], "Calcémie"), Trend::Down);
        assert!(
            level(&c, &["calcitriol", "sévélamer"], "Phosphatémie")
                < level(&c, &["calcitriol"], "Phosphatémie")
        );
        assert!(
            level(&c, &["cinacalcet", "calcitriol"], "Parathormone")
                < level(&c, &["cinacalcet"], "Parathormone")
        );
        assert!(
            level(&c, &["cinacalcet", "calcitriol"], "Calcémie")
                > level(&c, &["cinacalcet"], "Calcémie")
        );
    }

    #[test]
    fn the_cascade_hyperkaliemie_et_chelateurs_du_potassium_reads_as_drawn() {
        let c = shipped("Hyperkaliémie et chélateurs du potassium");
        assert_eq!(reads(&c, &["ramipril"], "Kaliémie"), Trend::Up);
        assert!(
            level(&c, &["ramipril", "spironolactone"], "Kaliémie")
                > level(&c, &["ramipril"], "Kaliémie")
        );
        assert!(
            level(&c, &["losartan", "triméthoprime"], "Kaliémie")
                > level(&c, &["losartan"], "Kaliémie")
        );
        assert_eq!(reads(&c, &["patiromère"], "Kaliémie"), Trend::Down);
        assert!(
            level(
                &c,
                &["ramipril", "spironolactone", "patiromère"],
                "Kaliémie"
            ) < level(&c, &["ramipril", "spironolactone"], "Kaliémie")
        );
        assert_eq!(reads(&c, &["héparine sodique"], "Kaliémie"), Trend::Up);
    }

    #[test]
    fn the_cascade_cycle_de_la_vitamine_k_reads_as_drawn() {
        let c = shipped("Cycle de la vitamine K");
        assert_eq!(reads(&c, &["warfarine"], "INR"), Trend::Up);
        assert_eq!(
            reads(&c, &["warfarine"], "Formation du caillot"),
            Trend::Down
        );
        assert!(
            level(&c, &["warfarine", "phytoménadione"], "Formation du caillot")
                > level(&c, &["warfarine"], "Formation du caillot")
        );
        assert_eq!(
            reads(&c, &["phytoménadione"], "Formation du caillot"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["apixaban", "phytoménadione"], "Formation du caillot"),
            Trend::Down
        );
        assert_eq!(
            reads(
                &c,
                &["dabigatran", "phytoménadione"],
                "Formation du caillot"
            ),
            Trend::Down
        );
    }

    #[test]
    fn the_cascade_alcool_acetaldehyde_et_recompense_reads_as_drawn() {
        let c = shipped("Alcool, acétaldéhyde et récompense");
        assert_eq!(reads(&c, &["disulfirame"], "Réaction antabuse"), Trend::Up);
        assert_eq!(
            reads(&c, &["métronidazole"], "Réaction antabuse"),
            Trend::Up
        );
        assert_eq!(reads(&c, &["disulfirame"], "Envie de boire"), Trend::Rest);
        assert_eq!(reads(&c, &["naltrexone"], "Plaisir de boire"), Trend::Down);
        assert_eq!(reads(&c, &["acamprosate"], "Envie de boire"), Trend::Down);
        assert_eq!(reads(&c, &["baclofène"], "Envie de boire"), Trend::Down);
        assert!(
            level(&c, &["naltrexone", "acamprosate"], "Envie de boire")
                < level(&c, &["naltrexone"], "Envie de boire")
        );
    }

    #[test]
    fn the_cascade_nicotine_et_arret_du_tabac_reads_as_drawn() {
        let c = shipped("Nicotine et arrêt du tabac");
        assert_eq!(
            reads(&c, &["nicotine"], "Manque et envie de fumer"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["varénicline"], "Manque et envie de fumer"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["bupropion"], "Manque et envie de fumer"),
            Trend::Down
        );
        assert!(
            level(&c, &["nicotine", "varénicline"], "Récompense")
                < level(&c, &["nicotine"], "Récompense")
        );
        assert!(
            level(&c, &["nicotine", "bupropion"], "Manque et envie de fumer")
                < level(&c, &["nicotine"], "Manque et envie de fumer")
        );
    }

    #[test]
    fn the_cascade_veille_et_sommeil_reads_as_drawn() {
        let c = shipped("Veille et sommeil");
        assert_eq!(reads(&c, &["daridorexant"], "Vigilance"), Trend::Down);
        assert_eq!(reads(&c, &["daridorexant"], "Endormissement"), Trend::Up);
        assert_eq!(reads(&c, &["mélatonine"], "Endormissement"), Trend::Up);
        assert_eq!(reads(&c, &["mélatonine"], "Vigilance"), Trend::Rest);
        assert_eq!(reads(&c, &["pitolisant"], "Vigilance"), Trend::Up);
        assert_eq!(reads(&c, &["modafinil"], "Vigilance"), Trend::Up);
        assert!(
            level(&c, &["oxybate de sodium", "daridorexant"], "Vigilance")
                < level(&c, &["oxybate de sodium"], "Vigilance")
        );
    }

    #[test]
    fn the_cascade_glutamate_et_recepteur_nmda_reads_as_drawn() {
        let c = shipped("Glutamate et récepteur NMDA");
        assert_eq!(reads(&c, &["mémantine"], "Excitotoxicité"), Trend::Down);
        assert_eq!(reads(&c, &["mémantine"], "Dissociation"), Trend::Rest);
        assert_eq!(reads(&c, &["mémantine"], "Humeur"), Trend::Rest);
        assert_eq!(reads(&c, &["eskétamine"], "Dissociation"), Trend::Up);
        assert_eq!(reads(&c, &["eskétamine"], "Humeur"), Trend::Up);
        assert_eq!(reads(&c, &["eskétamine"], "Pression artérielle"), Trend::Up);
        assert_eq!(reads(&c, &["riluzole"], "Excitotoxicité"), Trend::Down);
    }

    #[test]
    fn the_cascade_activation_du_lymphocyte_t_reads_as_drawn() {
        let c = shipped("Activation du lymphocyte T");
        assert_eq!(reads(&c, &["tacrolimus"], "Rejet du greffon"), Trend::Down);
        assert!(
            level(
                &c,
                &["tacrolimus", "mycophénolate mofétil"],
                "Rejet du greffon"
            ) < level(&c, &["tacrolimus"], "Rejet du greffon")
        );
        assert!(
            level(&c, &["ciclosporine", "sirolimus"], "Rejet du greffon")
                < level(&c, &["ciclosporine"], "Rejet du greffon")
        );
        assert_eq!(
            reads(
                &c,
                &["mycophénolate mofétil"],
                "Défense contre les infections"
            ),
            Trend::Down
        );
        assert_eq!(reads(&c, &["abatacept"], "Rejet du greffon"), Trend::Down);
        assert!(
            level(
                &c,
                &["tacrolimus", "mycophénolate mofétil", "prednisone"],
                "Défense contre les infections"
            ) < level(
                &c,
                &["tacrolimus", "mycophénolate mofétil"],
                "Défense contre les infections"
            )
        );
    }

    #[test]
    fn the_cascade_lymphocyte_b_et_anticorps_reads_as_drawn() {
        let c = shipped("Lymphocyte B et anticorps");
        assert_eq!(
            reads(
                &c,
                &["rituximab"],
                "Réponse à un vaccin fait sous traitement"
            ),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["rituximab"], "Protection des vaccins faits avant"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["ocrélizumab"], "Activité de la maladie"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["bélimumab"], "Activité de la maladie"),
            Trend::Down
        );
        assert_eq!(
            reads(
                &c,
                &["bélimumab"],
                "Réponse à un vaccin fait sous traitement"
            ),
            Trend::Rest
        );
        assert_eq!(reads(&c, &["rituximab"], "IgG sériques"), Trend::Down);
    }

    #[test]
    fn the_cascade_contraception_hormonale_reads_as_drawn() {
        let c = shipped("Contraception hormonale");
        assert_eq!(
            reads(&c, &["lévonorgestrel + éthinylestradiol"], "Ovulation"),
            Trend::Down
        );
        assert_eq!(
            reads(
                &c,
                &["lévonorgestrel + éthinylestradiol"],
                "Risque thromboembolique veineux"
            ),
            Trend::Up
        );
        assert_eq!(reads(&c, &["désogestrel"], "Ovulation"), Trend::Down);
        assert_eq!(
            reads(&c, &["désogestrel"], "Risque thromboembolique veineux"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["désogestrel"], "Glaire cervicale perméable"),
            Trend::Down
        );
    }

    #[test]
    fn the_cascade_mastocyte_et_degranulation_reads_as_drawn() {
        let c = shipped("Mastocyte et dégranulation");
        assert_eq!(
            reads(&c, &["cétirizine"], "Urticaire et prurit"),
            Trend::Down
        );
        assert_eq!(
            reads(&c, &["cétirizine"], "Pression artérielle"),
            Trend::Rest
        );
        assert_eq!(
            reads(&c, &["cétirizine"], "Calibre bronchique"),
            Trend::Rest
        );
        assert_eq!(reads(&c, &["adrénaline"], "Pression artérielle"), Trend::Up);
        assert_eq!(reads(&c, &["adrénaline"], "Calibre bronchique"), Trend::Up);
        assert!(
            level(&c, &["kétotifène"], "Conjonctivite allergique")
                < level(&c, &["lévocabastine"], "Conjonctivite allergique")
        );
        assert_eq!(
            reads(&c, &["cromoglicate de sodium"], "Conjonctivite allergique"),
            Trend::Down
        );
    }

    /// **La cyprotérone baisse la testostérone** : progestatif, elle
    /// freine la LH en plus de bloquer le récepteur. Écrite en simple
    /// antagoniste, elle levait le rétrocontrôle et la montrait monter,
    /// comme le bicalutamide — l'inverse de ce qu'elle fait.
    #[test]
    fn cyproterone_lowers_the_testosterone_that_bicalutamide_raises() {
        let c = shipped("Récepteur des androgènes");
        let t = c
            .nodes
            .iter()
            .find(|n| fuzzy::sort_key(&n.name).contains("testosterone"))
            .map(|n| n.name.clone())
            .expect("pas de nœud testostérone");
        assert!(level(&c, &["cyprotérone acétate"], &t) < level(&c, &["bicalutamide"], &t));
        assert_ne!(reads(&c, &["cyprotérone acétate"], &t), Trend::Up);
    }

    fn meet(given: &[(&str, &str)]) -> Vec<Meeting> {
        let cascades: Vec<Cascade> = STARTER_CASCADES.iter().map(|t| parse(t)).collect();
        let lines: Vec<Line> = given
            .iter()
            .map(|(n, d)| Line { name: n, dci: d })
            .collect();
        let refs: Vec<&Cascade> = cascades.iter().collect();
        meetings(&refs, &lines)
    }

    fn effect(m: &[Meeting], title: &str, effect: &str) -> Option<Together> {
        let cascades: Vec<Cascade> = STARTER_CASCADES.iter().map(|t| parse(t)).collect();
        m.iter()
            .filter(|m| cascades[m.cascade].title == title)
            .flat_map(|m| m.effects.iter())
            .find(|(e, _)| e == effect)
            .map(|(_, t)| *t)
    }

    /// **Ce qu'une ordonnance croise sur les cascades**, tel que la
    /// figure le montrerait si l'on cochait les deux cases : le nitré
    /// sur l'inhibiteur de la PDE5, le propranolol sur le salbutamol,
    /// l'oxybutynine sur le donépézil, la naloxone sur la morphine.
    #[test]
    fn two_lines_on_one_cascade_add_up_or_undo_each_other() {
        let m = meet(&[("Viagra", "sildénafil"), ("Natispray", "trinitrine")]);
        assert_eq!(
            effect(&m, "Monoxyde d'azote et GMPc", "Pression artérielle"),
            Some(Together::Adds)
        );
        let m = meet(&[("Ventoline", "salbutamol"), ("Avlocardyl", "propranolol")]);
        assert_eq!(
            effect(&m, "Récepteurs bêta-adrénergiques", "Bronchodilatation"),
            Some(Together::Opposes)
        );
        let m = meet(&[("Aricept", "donépézil"), ("Ditropan", "oxybutynine")]);
        assert_eq!(
            effect(&m, "Récepteurs muscariniques", "Mémoire et vigilance"),
            Some(Together::Opposes)
        );
        let m = meet(&[("Skenan", "morphine"), ("Narcan", "naloxone")]);
        assert_eq!(
            effect(&m, "Récepteur opioïde mu", "Transmission de la douleur"),
            Some(Together::Opposes)
        );
        let m = meet(&[
            ("Kardégic", "acide acétylsalicylique"),
            ("Plavix", "clopidogrel"),
        ]);
        assert_eq!(
            effect(&m, "Activation plaquettaire", "Agrégation plaquettaire"),
            Some(Together::Adds)
        );
        let m = meet(&[("Lasilix", "furosémide"), ("Aldactone", "spironolactone")]);
        assert_eq!(
            effect(&m, "Néphron et potassium", "Kaliémie"),
            Some(Together::Opposes)
        );
        // Ce qui s'oppose passe devant ce qui s'additionne.
        let m = meet(&[
            ("Viagra", "sildénafil"),
            ("Natispray", "trinitrine"),
            ("Ventoline", "salbutamol"),
            ("Avlocardyl", "propranolol"),
        ]);
        assert!(m[0].effects.iter().any(|(_, t)| *t == Together::Opposes));
    }

    /// Une association porte chacun de ses composants ; une sous-chaîne
    /// ne porte rien ; deux lignes de la même molécule ne se croisent
    /// pas ; deux molécules sans cascade commune non plus.
    #[test]
    fn a_meeting_needs_two_molecules_named_whole() {
        let m = meet(&[
            ("Tarka", "vérapamil + trandolapril"),
            ("Natispray", "trinitrine"),
        ]);
        assert!(
            m.is_empty(),
            "le vérapamil n'est dans aucune cascade : {m:?}"
        );
        let m = meet(&[("Aricept", "donépézil"), ("Ditropan", "oxybutyn")]);
        assert!(m.is_empty());
        let m = meet(&[("Zoloft", "sertraline"), ("Zoloft 50", "sertraline")]);
        assert!(m.is_empty());
        let m = meet(&[("Doliprane", "paracétamol"), ("Zoloft", "sertraline")]);
        assert!(m.is_empty());
        let c = parse("A -> B\nmolécule x : agoniste A\nmolécule y : agoniste A");
        let lines = [
            Line {
                name: "X+Y",
                dci: "x + y",
            },
            Line {
                name: "Autre",
                dci: "z",
            },
        ];
        assert!(
            meetings(&[&c], &lines).is_empty(),
            "une ligne ne se croise pas elle-même"
        );
    }

    #[test]
    fn the_shipped_cascades_only_ever_grow() {
        assert!(
            STARTER_CASCADES.len() >= SHIPPED,
            "{} cascades livrées, le plancher est {SHIPPED}",
            STARTER_CASCADES.len()
        );
    }

    #[test]
    fn every_shipped_cascade_reads_whole_and_says_where_it_comes_from() {
        let mut titles = std::collections::HashSet::new();
        for text in STARTER_CASCADES {
            let c = parse(text);
            assert!(c.faults.is_empty(), "{} : {:?}", c.title, c.faults);
            assert!(!c.title.is_empty() && !c.subject.is_empty(), "{text}");
            assert!(titles.insert(c.title.clone()), "{} en double", c.title);
            assert!(!c.sources.is_empty(), "{} sans source", c.title);
            assert!(!c.outcomes().is_empty(), "{} sans effet", c.title);
            // Un nœud déclaré que rien ne relie est une faute d'écriture :
            // son nom diffère d'une lettre de celui de la flèche.
            for (i, n) in c.nodes.iter().enumerate() {
                assert!(
                    c.edges.iter().any(|e| e.from == i || e.to == i),
                    "{} : « {} » n'est relié à rien",
                    c.title,
                    n.name
                );
            }
        }
    }

    #[test]
    fn every_shipped_molecule_names_a_card_and_moves_an_effect() {
        let cards: Vec<&str> = crate::db::STARTER_DRUGS.iter().map(|d| d.1).collect();
        for text in STARTER_CASCADES {
            let c = parse(text);
            for (m, mol) in c.molecules.iter().enumerate() {
                assert!(
                    cards.iter().any(|d| fuzzy::eq_folded(d, &mol.name))
                        || WITHOUT_CARD.contains(&mol.name.as_str()),
                    "{} : « {} » ne nomme aucune fiche",
                    c.title,
                    mol.name
                );
                // Une molécule qui ne change aucun effet ne montre rien :
                // sa case se cocherait sur une figure immobile. Seule, ou
                // donnée sur une autre — la naloxone ne fait rien sans
                // morphinique, et tout sur lui.
                let moves = |base: Option<usize>| {
                    let mut without = vec![false; c.molecules.len()];
                    if let Some(b) = base {
                        without[b] = true;
                    }
                    let before = settle(&c, &without, &[]);
                    let mut with = without.clone();
                    with[m] = true;
                    let after = settle(&c, &with, &[]);
                    c.outcomes()
                        .iter()
                        .any(|&o| trend(after.level[o]) != trend(before.level[o]))
                };
                assert!(
                    moves(None) || (0..c.molecules.len()).any(|b| b != m && moves(Some(b))),
                    "{} : « {} » ne change aucun effet",
                    c.title,
                    mol.name
                );
            }
        }
        for name in WITHOUT_CARD {
            assert!(
                !cards.iter().any(|d| fuzzy::eq_folded(d, name)),
                "{name} a maintenant sa fiche : la retirer de la liste"
            );
        }
    }

    #[test]
    fn a_cardioselective_blocker_spares_the_bronchi_and_propranolol_does_not() {
        let c = shipped("Récepteurs bêta-adrénergiques");
        for m in ["bisoprolol", "propranolol", "acébutolol"] {
            assert_eq!(
                trend(level(&c, &[m], "Fréquence cardiaque")),
                Trend::Down,
                "{m}"
            );
        }
        assert_eq!(
            trend(level(&c, &["bisoprolol"], "Bronchodilatation")),
            Trend::Rest
        );
        assert_eq!(
            trend(level(&c, &["propranolol"], "Bronchodilatation")),
            Trend::Down
        );
        // Et le propranolol éteint le salbutamol.
        assert!(
            level(&c, &["salbutamol", "propranolol"], "Bronchodilatation")
                < level(&c, &["salbutamol"], "Bronchodilatation")
        );
        // L'activité sympathomimétique intrinsèque : moins de bradycardie.
        assert!(
            level(&c, &["acébutolol"], "Fréquence cardiaque")
                > level(&c, &["bisoprolol"], "Fréquence cardiaque")
        );
    }

    #[test]
    fn an_iec_makes_the_cough_and_a_sartan_does_not_both_raise_potassium() {
        let c = shipped("Système rénine-angiotensine-aldostérone");
        assert_eq!(trend(level(&c, &["ramipril"], "Toux sèche")), Trend::Up);
        assert_eq!(trend(level(&c, &["losartan"], "Toux sèche")), Trend::Rest);
        for m in ["ramipril", "losartan", "spironolactone"] {
            assert_eq!(trend(level(&c, &[m], "Kaliémie")), Trend::Up, "{m}");
        }
        for m in ["ramipril", "losartan"] {
            assert_eq!(
                trend(level(&c, &[m], "Pression artérielle")),
                Trend::Down,
                "{m}"
            );
            // Le rétrocontrôle : la rénine monte sous blocage.
            assert_eq!(trend(level(&c, &[m], "Rénine")), Trend::Up, "{m}");
        }
        // Le double blocage monte encore la kaliémie.
        assert!(
            level(&c, &["ramipril", "spironolactone"], "Kaliémie")
                > level(&c, &["ramipril"], "Kaliémie")
        );
    }

    #[test]
    fn a_coxib_spares_the_stomach_and_the_platelets_but_not_the_kidney() {
        let c = shipped("Cyclo-oxygénases");
        let stomach = "Protection de la muqueuse gastrique";
        assert_eq!(trend(level(&c, &["ibuprofène"], stomach)), Trend::Down);
        assert_eq!(trend(level(&c, &["célécoxib"], stomach)), Trend::Rest);
        // La prostacycline baisse seule : l'agrégation monte — le risque
        // thrombotique des coxibs. Un AINS non sélectif la baisse.
        assert_eq!(
            trend(level(&c, &["célécoxib"], "Agrégation plaquettaire")),
            Trend::Up
        );
        assert_eq!(
            trend(level(&c, &["ibuprofène"], "Agrégation plaquettaire")),
            Trend::Down
        );
        assert_eq!(
            trend(level(&c, &["célécoxib"], "Inflammation")),
            Trend::Down
        );
        assert_eq!(
            trend(level(&c, &["célécoxib"], "Débit sanguin rénal")),
            Trend::Down
        );
    }

    #[test]
    fn a_benzodiazepine_needs_gaba_and_its_abrupt_stop_lowers_the_seizure_threshold() {
        let c = shipped("Récepteur GABA-A");
        assert_eq!(trend(level(&c, &["diazépam"], "Vigilance")), Trend::Down);
        assert_eq!(
            trend(level(&c, &["diazépam"], "Seuil convulsif")),
            Trend::Up
        );
        let m = c
            .molecules
            .iter()
            .position(|m| m.name == "diazépam")
            .unwrap();
        let frames = run(
            &c,
            &[Dose {
                molecule: m,
                from: 0,
                until: Some(60),
            }],
            90,
        );
        let seuil = idx(&c, "Seuil convulsif");
        assert_eq!(trend(frames[60].level[seuil]), Trend::Down);
        assert_eq!(trend(frames[60].level[idx(&c, "Anxiété")]), Trend::Up);
    }

    #[test]
    fn buprenorphine_on_morphine_lowers_the_effect_and_constipation_outlasts_tolerance() {
        let c = shipped("Récepteur opioïde mu");
        let pain = "Transmission de la douleur";
        assert!(level(&c, &["morphine", "buprénorphine"], pain) > level(&c, &["morphine"], pain));
        assert!(level(&c, &["morphine", "naloxone"], pain) > level(&c, &["morphine"], pain));
        // Seule, la naloxone ne fait rien : le récepteur est presque au
        // repos sans opioïde.
        for effect in [pain, "Commande respiratoire", "Motricité intestinale"] {
            assert_eq!(
                trend(level(&c, &["naloxone"], effect)),
                Trend::Rest,
                "{effect}"
            );
        }
        let m = c
            .molecules
            .iter()
            .position(|m| m.name == "morphine")
            .unwrap();
        let frames = run(
            &c,
            &[Dose {
                molecule: m,
                from: 0,
                until: None,
            }],
            60,
        );
        let (p, gut) = (idx(&c, pain), idx(&c, "Motricité intestinale"));
        // L'analgésie s'érode ; la constipation reste entière.
        assert!(frames[60].level[p] > frames[0].level[p]);
        assert_eq!(frames[60].level[gut], frames[0].level[gut]);
        assert_eq!(trend(frames[60].level[gut]), Trend::Down);
        // Le manque précipité : la naloxone donnée après une longue
        // morphine fait déborder l'AMPc — la douleur au-dessus du repos.
        let n = c
            .molecules
            .iter()
            .position(|m| m.name == "naloxone")
            .unwrap();
        let frames = run(
            &c,
            &[
                Dose {
                    molecule: m,
                    from: 0,
                    until: None,
                },
                Dose {
                    molecule: n,
                    from: 60,
                    until: None,
                },
            ],
            61,
        );
        assert_eq!(trend(frames[61].level[p]), Trend::Up);
        assert_eq!(trend(frames[61].level[idx(&c, "AMPc")]), Trend::Up);
    }

    #[test]
    fn zolpidem_puts_to_sleep_without_the_anxiolysis_of_a_benzodiazepine() {
        let c = shipped("Récepteur GABA-A");
        assert_eq!(trend(level(&c, &["zolpidem"], "Vigilance")), Trend::Down);
        for effect in ["Anxiété", "Tonus musculaire", "Seuil convulsif"] {
            assert_eq!(
                trend(level(&c, &["zolpidem"], effect)),
                Trend::Rest,
                "{effect}"
            );
            assert_ne!(
                trend(level(&c, &["diazépam"], effect)),
                Trend::Rest,
                "{effect}"
            );
        }
    }

    #[test]
    fn enoxaparin_and_fondaparinux_act_on_xa_and_heparin_on_both() {
        let c = shipped("Coagulation");
        let (xa, iia) = ("Facteur Xa", "Thrombine");
        for m in ["héparine sodique", "énoxaparine", "fondaparinux"] {
            assert_eq!(trend(level(&c, &[m], xa)), Trend::Down, "{m}");
        }
        assert!(level(&c, &["héparine sodique"], iia) < level(&c, &["fondaparinux"], iia));
    }

    #[test]
    fn a_ppi_lowers_acid_and_raises_gastrin_and_an_avk_lowers_the_clot() {
        let c = shipped("Sécrétion acide gastrique");
        assert_eq!(
            trend(level(&c, &["oméprazole"], "Sécrétion acide")),
            Trend::Down
        );
        assert_eq!(trend(level(&c, &["oméprazole"], "Gastrine")), Trend::Up);
        let c = shipped("Coagulation");
        for m in ["warfarine", "apixaban", "dabigatran", "énoxaparine"] {
            assert_eq!(
                trend(level(&c, &[m], "Formation du caillot")),
                Trend::Down,
                "{m}"
            );
        }
        let c = shipped("Activation plaquettaire");
        for m in ["acide acétylsalicylique", "clopidogrel"] {
            assert_eq!(
                trend(level(&c, &[m], "Agrégation plaquettaire")),
                Trend::Down,
                "{m}"
            );
        }
    }
}
