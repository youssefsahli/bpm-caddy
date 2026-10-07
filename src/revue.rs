//! What a set of treatments says about itself.
//!
//! The interactions the app quotes from the monographs answer « A et B
//! ensemble ? ». This module answers the other half of a bilan partagé
//! de médication: the doublons, the associations that add up, the
//! cascades where one médicament treats the effect of another. None of
//! it needs a new database — it reads the classes and the tags the drug
//! cards already carry.
//!
//! Static, pure and tested, like the calendrier vaccinal and the
//! biology rules. It says what is worth looking at; it decides nothing.
//!
//! **What a rule here cannot know, and must not pretend to.** A
//! `Treatment` is a name, a DCI, a class and some tags — and nothing
//! else. No age, no dose, no duration, no diagnosis. That boundary is
//! not a gap to be filled: it decides what belongs here and what
//! belongs in a reference table of `tables.rs`.
//!
//! Confronted with « Sujet âgé — médicaments à réévaluer » (Laroche,
//! STOPP/START) on 13/09/2026, everything the module *can* express was
//! already there — la charge anticholinergique, trois sédatifs, les
//! AINS, la digoxine, l'opioïde sans laxatif. Everything missing was
//! missing for one of those four reasons: « benzodiazépine à demi-vie
//! longue **après 75 ans** » needs an age, « digoxine **au-delà de
//! 0,125 mg/j** » a dose, « IPP **sans indication réévaluée** » a
//! duration, « anticoagulant **dans la fibrillation atriale** » a
//! diagnosis. Writing them anyway would fire on every diazépam and
//! every Previscan, which is how a bilan stops being read.
//!
//! A rule that names a dose does so as a **ceiling to check on the
//! ordonnance** — « Simvastatine au-dessus de son plafond » says so in
//! its own sentence — never as a verdict on a dose it has not seen.

use crate::biology::Severity;

/// One treatment, reduced to the words the rules match on.
pub struct Treatment<'a> {
    pub name: &'a str,
    pub dci: &'a str,
    pub class: &'a str,
    pub tags: &'a str,
}

impl Treatment<'_> {
    /// Everything that describes this treatment, folded once.
    fn haystack(&self) -> String {
        crate::fuzzy::sort_key(&format!(
            "{} {} {} {}",
            self.name, self.dci, self.class, self.tags
        ))
    }
}

/// One thing worth looking at on this ordonnance.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub severity: Severity,
    /// Three or four words, for a chip.
    pub title: &'static str,
    /// The sentence that says why, and what to do about it.
    pub detail: &'static str,
    /// The treatments it is about, by name.
    pub drugs: Vec<String>,
}

/// How a rule matches the ordonnance.
enum Kind {
    /// Every group must match a treatment. A fixed combination that
    /// matches two groups at once counts for both — an IEC-diurétique
    /// in one tablet plus an AINS is the same triad as three boxes.
    Combination(&'static [&'static [&'static str]]),
    /// At least `1` distinct treatments carrying one of these words.
    Duplicate(&'static [&'static str], usize),
    /// Every group matches, **and** no *other* line of the ordonnance
    /// carries any of the last words.
    ///
    /// « Other » is the whole point: searched over the ordonnance
    /// entière, the absence was silenced by the very line that triggers
    /// the rule, whose free tags belong to the officine. A pharmacist
    /// noting « pyridoxine à associer » on the Rimifon card — the exact
    /// reminder this rule exists to give — switched the rule off.
    ///
    /// What is *missing* is half of what a bilan finds. An opioid with
    /// no laxative beside it, a corticothérapie with nothing for the
    /// bone: neither is an interaction, and both are the reason the
    /// patient comes back. The point names the treatments that are
    /// there — naming an absence is impossible, and the sentence says
    /// what is not.
    ///
    /// **Ce que cette forme ne sait pas dire**, et qu'il vaut mieux
    /// écrire que redécouvrir : une règle dont la ligne qui la
    /// *déclenche* peut aussi être celle qui la *comble*. L'absence est
    /// cherchée sur les lignes que le groupe n'a pas nommées — c'est
    /// la correction du Rimifon ci-dessus — si bien qu'une ligne
    /// nommée ne peut pas fournir ce qui manque. « Méthotrexate sans
    /// acide folique » est le cas : la Lederfoline porte « antidote du
    /// méthotrexate » dans sa classe, elle est donc *nommée* par le
    /// groupe, et une ordonnance qui ne porterait qu'elle recevrait
    /// « méthotrexate sans acide folique » — c'est-à-dire un reproche
    /// adressé au sauvetage lui-même.
    ///
    /// D'où le **troisième membre : le veto**, celui que `renal` et
    /// `gravidity` portent déjà et pour la même raison. Une ligne qu'il
    /// attrape n'est pas nommée par le groupe — donc elle n'est pas
    /// *déclenchante* — et redevient par là même une ligne comme les
    /// autres, capable de fournir ce qui manque. La Lederfoline sort du
    /// groupe « méthotrexate » par le mot « antidote », et c'est alors
    /// son acide folinique qui répond.
    ///
    /// Le veto est vide pour quatre des cinq règles, et il le restera :
    /// il ne sert qu'aux règles dont la ligne déclenchante peut aussi
    /// être celle qui comble.
    Without(
        &'static [&'static [&'static str]],
        &'static [&'static str],
        &'static [&'static str],
    ),
}

struct Rule {
    kind: Kind,
    severity: Severity,
    title: &'static str,
    detail: &'static str,
}

/// Les règles **écrites pour une forme locale** : le collyre
/// bêtabloquant qui passe dans le sang, le gel buccal de miconazole dont
/// le passage systémique suffit, et la charge corticoïde qui additionne
/// l'inhalé, le nasal et le cutané. Ailleurs, une forme locale ne
/// déclenche rien.
const LOCAL_RULES: &[&str] = &[
    "Bêtabloquant caché",
    "Charge corticoïde cumulée",
    "Miconazole + AVK",
    "Miconazole + sulfamide hypoglycémiant",
];

/// Read an ordonnance against itself. Loudest first; inside one
/// severity, the order of the rules — which is the order a pharmacist
/// checks them in.
pub fn review(treatments: &[Treatment]) -> Vec<Point> {
    let lines: Vec<Folded> = treatments.iter().map(Folded::of).collect();
    review_folded(&lines.iter().collect::<Vec<_>>())
}

/// Une ligne telle que la revue la lit : son nom, **repliée sans ce que
/// son libellé nie** (voir `classes::strip_unsaid`), les deux questions
/// posées une fois — est-ce une forme locale, est-ce un antidote — et,
/// règle par règle, **quels mots de la règle elle porte**.
///
/// À part pour qui lit beaucoup de paires : la carte du voisinage
/// demande à la revue ce qu'une fiche fait avec chacune des huit cents
/// autres, et relire le centre contre chaque règle huit cents fois
/// coûtait plus que tout le reste. Lue une fois, une ligne se relit
/// autant de fois qu'on veut — **par la même lecture** ([`review_folded`])
/// : il n'y a qu'une écriture des règles, et c'est elle qui dit quels
/// mots chercher.
#[derive(Clone, Debug)]
pub struct Folded {
    name: String,
    local: bool,
    antidote: bool,
    /// Pour chaque règle, dans l'ordre de `RULES`.
    hits: Vec<Hits>,
}

/// Ce qu'une ligne porte des mots d'une règle : chaque groupe, les mots
/// de l'absence, les mots du veto.
#[derive(Clone, Debug, Default)]
struct Hits {
    groups: Vec<bool>,
    absent: bool,
    never: bool,
}

impl Folded {
    pub fn of(t: &Treatment) -> Folded {
        let hay = crate::classes::strip_unsaid(&t.haystack());
        let any = |words: &[&str]| words.iter().any(|w| crate::fuzzy::contains_folded(&hay, w));
        let hits = RULES
            .iter()
            .map(|rule| match &rule.kind {
                Kind::Combination(groups) => Hits {
                    groups: groups.iter().map(|g| any(g)).collect(),
                    ..Hits::default()
                },
                Kind::Duplicate(words, _) => Hits {
                    groups: vec![any(words)],
                    ..Hits::default()
                },
                Kind::Without(groups, absent, never) => Hits {
                    groups: groups.iter().map(|g| any(g)).collect(),
                    absent: any(absent),
                    never: any(never),
                },
            })
            .collect();
        Folded {
            name: t.name.trim().to_owned(),
            // Le miconazole buccal passe dans le sang et compte comme la
            // voie générale — voir `classes::local_but_absorbed`.
            local: crate::classes::stays_local(t.dci, t.class),
            antidote: crate::classes::is_antidote(t.class),
            hits,
        }
    }
}

/// [`review`] sur des lignes déjà lues.
pub fn review_folded(lines: &[&Folded]) -> Vec<Point> {
    let mut out = Vec::new();
    for (r, rule) in RULES.iter().enumerate() {
        // **Ce qui déclenche une règle** : ni un antidote — la vitamine K1
        // recevait les règles des AVK, et « deux anticoagulants » avec le
        // Previscan qu'elle corrige —, ni une forme locale, sauf pour les
        // règles écrites pour elle. Les tables sont indexées sur la
        // molécule, et l'Indocollyre, le Tantum ou le Kétoderm tombaient
        // dans les règles de leur molécule générale : « Deux AINS » pour
        // un bain de bouche, et le Kétoderm face à la simvastatine, que
        // ce dépôt cite comme l'exemple même à ne pas faire.
        //
        // Un antidote reste une ligne de l'ordonnance : il peut toujours
        // **combler** une absence (la Lederfoline répond à « méthotrexate
        // sans acide folique »).
        let local_ok = LOCAL_RULES.contains(&rule.title);
        let matches = |group: usize| -> Vec<String> {
            lines
                .iter()
                .filter(|l| !l.antidote && (local_ok || !l.local))
                .filter(|l| l.hits[r].groups.get(group).copied().unwrap_or(false))
                .map(|l| l.name.clone())
                .collect()
        };
        let drugs = match &rule.kind {
            Kind::Combination(groups) => {
                let mut named: Vec<String> = Vec::new();
                let mut complete = true;
                for g in 0..groups.len() {
                    let hit = matches(g);
                    if hit.is_empty() {
                        complete = false;
                        break;
                    }
                    for name in hit {
                        if !named.contains(&name) {
                            named.push(name);
                        }
                    }
                }
                if complete {
                    named
                } else {
                    Vec::new()
                }
            }
            Kind::Duplicate(_, min) => {
                let hit = matches(0);
                if hit.len() >= *min {
                    hit
                } else {
                    Vec::new()
                }
            }
            Kind::Without(groups, _, _) => {
                let mut named: Vec<String> = Vec::new();
                let mut complete = true;
                for g in 0..groups.len() {
                    // **Le veto écarte la ligne du groupe**, il ne la
                    // retire pas de l'ordonnance : elle cesse d'être
                    // déclenchante et redevient capable de fournir.
                    let hit: Vec<String> = matches(g)
                        .into_iter()
                        .filter(|name| lines.iter().any(|l| l.name == *name && !l.hits[r].never))
                        .collect();
                    if hit.is_empty() {
                        complete = false;
                        break;
                    }
                    for name in hit {
                        if !named.contains(&name) {
                            named.push(name);
                        }
                    }
                }
                // **L'absence se lit sur les *autres* lignes.** Cherchée
                // sur toute l'ordonnance, elle se laissait éteindre par
                // la ligne qui déclenche la règle, dont les étiquettes
                // libres appartiennent à l'officine : un pharmacien qui
                // note « pyridoxine à associer » sur la fiche du
                // Rimifon — c'est-à-dire exactement le rappel que cette
                // règle existe pour donner — faisait taire la règle.
                // C'est d'ailleurs ce que la phrase veut dire :
                // « isoniazide sans vitamine B6 » parle d'une seconde
                // ligne qui n'y est pas.
                let provided = lines
                    .iter()
                    .any(|l| !named.contains(&l.name) && l.hits[r].absent);
                if complete && !provided {
                    named
                } else {
                    Vec::new()
                }
            }
        };
        if drugs.is_empty() {
            continue;
        }
        out.push(Point {
            severity: rule.severity,
            title: rule.title,
            detail: rule.detail,
            drugs,
        });
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.severity));
    out
}

/// Le document sous lequel les phrases de la revue sont adressées.
pub const DOC: &str = "revue";

/// Toutes les phrases de la revue, avec leur adresse — voir
/// `content.rs`.
///
/// Le repère est le **titre** de la règle et non son rang : les
/// quatre-vingt-sept titres sont distincts, un test le tient, et une
/// règle insérée au milieu du tableau ne périme donc aucune réécriture.
pub fn phrases() -> Vec<(String, &'static str, &'static str)> {
    let mut out = Vec::with_capacity(RULES.len() * 2);
    for rule in RULES {
        let id = crate::content::slug(rule.title);
        out.push((crate::content::key(DOC, &id, "titre"), "titre", rule.title));
        out.push((
            crate::content::key(DOC, &id, "detail"),
            "detail",
            rule.detail,
        ));
    }
    out
}

/// Appliquer les réécritures de l'officine à ce que la revue a trouvé.
///
/// Sur les points rendus et non sur le tableau : la revue reste pure —
/// ses règles, ses appariements et ses tests portent sur ce qui est
/// livré — et les mots de l'officine arrivent au moment d'afficher ou
/// d'imprimer. Même frontière que les carnets.
pub fn resolve(points: Vec<Point>, over: &crate::content::Overrides) -> Vec<Resolved> {
    points
        .into_iter()
        .map(|p| {
            let id = crate::content::slug(p.title);
            Resolved {
                severity: p.severity,
                title: over
                    .get(&crate::content::key(DOC, &id, "titre"), p.title)
                    .to_owned(),
                detail: over
                    .get(&crate::content::key(DOC, &id, "detail"), p.detail)
                    .to_owned(),
                drugs: p.drugs,
            }
        })
        .collect()
}

/// Un point de revue tel qu'il sera montré et imprimé : les mots de
/// l'officine quand elle en a écrit.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub drugs: Vec<String>,
}

/// The classic readings of a French ordonnance, in the order they are
/// checked. The words are matched inside a card's name, DCI, class and
/// tags, accent- and case-insensitively.
const RULES: &[Rule] = &[
    Rule {
        kind: Kind::Combination(&[
            &["bêtabloquant", "bêta-bloquant", "bisoprolol", "métoprolol", "aténolol", "propranolol", "nébivolol", "sotalol"],
            &["donépézil", "rivastigmine", "galantamine", "anticholinestérasique"],
        ]),
        severity: Severity::Alert,
        title: "Bêtabloquant + anticholinestérasique",
        detail: "Les deux ralentissent le cœur par des voies différentes et l'effet s'additionne : bradycardie, syncope et chute, chez un patient déjà traité pour des troubles cognitifs. Prendre le pouls, demander s'il y a eu des malaises, et signaler l'association.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["bêtabloquant", "bêta-bloquant", "bisoprolol", "métoprolol", "aténolol", "propranolol", "nébivolol"],
            &["amiodarone", "dronédarone"],
        ]),
        severity: Severity::Alert,
        title: "Bêtabloquant + amiodarone",
        detail: "Bradycardie et troubles de la conduction, d'autant que l'amiodarone allonge aussi le QT et s'élimine sur des mois. L'association est possible sous surveillance étroite : pouls, tolérance à l'effort, et un ECG si le patient dit se sentir ralenti.",
    },
    Rule {
        kind: Kind::Duplicate(&["codéine", "tramadol", "lamaline", "izalgi", "poudre d'opium", "dihydrocodéine"], 2),
        severity: Severity::Alert,
        title: "Deux opioïdes faibles",
        detail: "Deux sources d'opioïde faible sur la même ordonnance : les effets s'additionnent (somnolence, constipation, dépression respiratoire) et l'une des deux est souvent cachée dans une association au paracétamol. Faire la somme devant le patient et n'en garder qu'une.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["sulfamide hypoglycémiant", "gliclazide", "glimépiride", "glibenclamide", "répaglinide"],
            &["insuline"],
        ]),
        severity: Severity::Alert,
        title: "Sulfamide + insuline",
        detail: "Association des deux seuls antidiabétiques hypoglycémiants : risque de malaise nocturne et de chute chez le sujet âgé. Vérifier que le patient a du sucre sur lui, qu'il sait reconnaître les signes, et que l'entourage sait quoi faire.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["alfuzosine", "tamsulosine", "silodosine", "doxazosine", "alpha-bloquant"],
            &["antihypertenseur", "IEC", "sartan", "bêtabloquant", "diurétique", "inhibiteur calcique", "amlodipine"],
        ]),
        severity: Severity::Warn,
        title: "Alpha-bloquant + antihypertenseur",
        detail: "Hypotension orthostatique, surtout à l'instauration et à la première dose du soir : ce mécanisme de chute est rarement rattaché au traitement de la prostate. Se lever en deux temps, prendre la dose au coucher, et signaler tout vertige au lever.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["oméprazole", "ésoméprazole", "pantoprazole", "lansoprazole", "rabéprazole", "oméprazole", "pantoprazole", "ésoméprazole", "lansoprazole", "rabéprazole"],
            &["ferreux", " ferrique", "fer saccharose", "sulfate ferreux", "fumarate ferreux", "ascorbate ferreux"],
        ]),
        severity: Severity::Warn,
        title: "IPP + fer oral",
        detail: "Le fer a besoin de l'acidité de l'estomac pour être absorbé, et l'IPP la supprime : le traitement martial échoue sans cause apparente, et la dose est parfois augmentée sans que l'ordonnance soit relue. Fer à distance, avec de la vitamine C, et réévaluer l'IPP.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["IEC", "sartan", "ARA II"],
            &["diurétique", "furosémide", "hydrochlorothiazide", "indapamide"],
            &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène", "coxib"],
        ]),
        severity: Severity::Alert,
        title: "Triade néfaste",
        detail: "Bloqueur du système rénine-angiotensine, diurétique et AINS ensemble : cette association expose à l'insuffisance rénale aiguë, d'autant plus vite qu'il fait chaud ou que le patient se déshydrate. L'AINS est le médicament à retirer.",
    },
    Rule {
        kind: Kind::Combination(&[&["IEC"], &["sartan", "ARA II"]]),
        severity: Severity::Alert,
        title: "Double blocage",
        detail: "IEC et sartan ensemble : le double blocage du système rénine-angiotensine n'apporte pas de bénéfice et majore le risque d'insuffisance rénale et d'hyperkaliémie. À signaler au prescripteur.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["anticoagulant", "AOD", "AVK", "héparine", "apixaban", "rivaroxaban", "dabigatran", "édoxaban", "warfarine", "fluindione"],
            &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène", "coxib"],
        ]),
        severity: Severity::Alert,
        title: "Anticoagulant + AINS",
        detail: "Le risque hémorragique digestif est multiplié, quelle que soit la dose : l'AINS se remplace par du paracétamol ou un topique, jamais délivré en conseil.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["benzodiazépine", "zolpidem", "zopiclone", "hypnotique"],
            &["opioïde", "skenan", "sevredol", "actiskenan", "oramorph", "moscontin", "oxycodone", "tramadol", "codéine", "fentanyl"],
        ]),
        severity: Severity::Alert,
        title: "Benzodiazépine + opioïde",
        detail: "Dépression respiratoire : l'association est la première cause de décès par surdose médicamenteuse. Si elle est justifiée, les doses sont les plus faibles possibles et l'entourage est informé.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["anticholinestérasique", "donépézil", "rivastigmine", "galantamine"],
            &["anticholinergique", "oxybutynine", "solifénacine", "toltérodine"],
        ]),
        severity: Severity::Alert,
        title: "Cascade anticholinergique",
        detail: "Un anticholinestérasique et un anticholinergique ont des effets opposés : l'un est prescrit pour la mémoire, l'autre l'altère. C'est le critère STOPP le plus souvent retrouvé sur une ordonnance de sujet âgé.",
    },
    Rule {
        kind: Kind::Combination(&[
            // Le lithium, et non « thymorégulateur » : la classe nomme
            // aussi le Dépakote et le Dépamide, qui recevaient l'alerte
            // d'une lithémie qu'ils n'ont pas.
            &["lithium", "téralithe"],
            &["AINS", "IEC", "sartan", "diurétique"],
        ]),
        severity: Severity::Alert,
        title: "Lithium exposé",
        detail: "AINS, IEC, sartans et diurétiques font monter la lithémie à dose inchangée. Toute introduction impose un contrôle de la lithémie, et toute déshydratation devient une urgence.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["méthotrexate"],
            &["triméthoprime", "cotrimoxazole", "AINS", "sulfamide antibactérien"],
        ]),
        severity: Severity::Alert,
        title: "Méthotrexate exposé",
        detail: "Cotrimoxazole et AINS augmentent la toxicité hématologique du méthotrexate. L'association au cotrimoxazole est à éviter formellement ; la NFS se contrôle.",
    },
    Rule {
        kind: Kind::Duplicate(&["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène", "coxib"], 2),
        severity: Severity::Alert,
        title: "Deux AINS",
        detail: "Deux anti-inflammatoires ensemble n'ajoutent pas d'efficacité, seulement le risque digestif et rénal. Un seul, à la dose la plus faible et le moins longtemps possible.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["digoxine", "digitalique"],
            &["furosémide", "hydrochlorothiazide", "indapamide", "diurétique de l'anse", "diurétique thiazidique"],
        ]),
        severity: Severity::Warn,
        title: "Digoxine et diurétique",
        detail: "Le diurétique fait baisser le potassium, et l'hypokaliémie rend la digoxine toxique à concentration inchangée. Kaliémie et digoxinémie se surveillent ensemble.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["vastatine", "atorvastatine", "simvastatine", "rosuvastatine", "pravastatine"],
            &["fibrate", "gemfibrozil", "fénofibrate"],
        ]),
        severity: Severity::Warn,
        title: "Statine + fibrate",
        detail: "Le risque musculaire est majoré, et il est maximal avec le gemfibrozil, qui ne s'associe pas à une statine. Toute douleur musculaire diffuse impose un dosage des CPK.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["clopidogrel"],
            &["oméprazole", "ésoméprazole"],
        ]),
        severity: Severity::Warn,
        title: "Clopidogrel et IPP",
        detail: "L'oméprazole et l'ésoméprazole inhibent le CYP2C19 qui active le clopidogrel. Le pantoprazole ou le rabéprazole n'exposent pas à cette interaction.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["anticoagulant", "AOD", "AVK", "héparine", "apixaban", "rivaroxaban", "dabigatran", "édoxaban"],
            &["antiagrégant", "aspirine", "clopidogrel", "prasugrel", "ticagrélor"],
        ]),
        severity: Severity::Warn,
        title: "Anticoagulant + antiagrégant",
        detail: "L'association existe après un stent, mais elle a une durée prévue : au-delà, elle se réévalue. Vérifier que la date de fin est connue du patient.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["ISRS", "sertraline", "paroxétine", "citalopram", "fluoxétine", "IRSNa", "venlafaxine", "duloxétine"],
            &["anticoagulant", "AOD", "AVK", "antiagrégant", "aspirine", "clopidogrel", "AINS"],
        ]),
        severity: Severity::Warn,
        title: "Sérotoninergique et saignement",
        detail: "Les ISRS et les IRSNa bloquent la recapture plaquettaire de la sérotonine : associés à un antithrombotique ou à un AINS, ils majorent le risque hémorragique digestif. Un IPP se discute.",
    },
    Rule {
        kind: Kind::Duplicate(
            &["ISRS", "IRSNa", "sertraline", "paroxétine", "citalopram", "fluoxétine", "venlafaxine", "duloxétine", "tramadol", "triptan", "millepertuis", "linézolide"],
            2,
        ),
        severity: Severity::Alert,
        title: "Deux sérotoninergiques",
        detail: "Agitation, sueurs, tremblement, fièvre et diarrhée dans les heures qui suivent une introduction : évoquer un syndrome sérotoninergique. Le tramadol, les triptans et le millepertuis sont à prendre en compte.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["bêtabloquant", "bêta-bloquant", "bisoprolol", "métoprolol", "aténolol", "propranolol"],
            &["vérapamil", "diltiazem"],
        ]),
        severity: Severity::Alert,
        title: "Bêtabloquant + vérapamil",
        detail: "Bradycardie sévère et bloc auriculo-ventriculaire : l'association d'un bêtabloquant au vérapamil ou au diltiazem est à éviter, et jamais sans surveillance du pouls et un avis cardiologique.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["colchicine"],
            // Sans les statines : elles n'inhibent rien et ne font pas
            // monter la colchicine — la phrase de cette règle ne les
            // nomme d'ailleurs pas. Ce qu'elles partagent avec elle est
            // la myotoxicité, et « Statine + colchicine » le dit.
            &["clarithromycine", "érythromycine", "josamycine", "télithromycine", "vérapamil", "antifongique azolé", "ciclosporine"],
        ]),
        severity: Severity::Alert,
        title: "Colchicine exposée",
        detail: "La colchicine a une marge très étroite et pas d'antidote : macrolides, azolés, vérapamil et ciclosporine augmentent ses concentrations. Une diarrhée précoce sous colchicine, premier signe de surdosage, impose l'arrêt immédiat.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["digoxine", "digitalique"],
            &["amiodarone", "vérapamil", "propafénone", "quinidine"],
        ]),
        severity: Severity::Warn,
        title: "Digoxine majorée",
        detail: "Amiodarone, vérapamil et propafénone augmentent la digoxinémie sans que la dose change : elle se réduit souvent de moitié à l'introduction, et la digoxinémie se contrôle.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène", "coxib"],
            &["corticoïde", "prednisone", "prednisolone", "méthylprednisolone"],
        ]),
        severity: Severity::Warn,
        title: "AINS + corticoïde",
        detail: "Les deux ensemble multiplient le risque d'ulcère et d'hémorragie digestive, sans gain anti-inflammatoire proportionné. Si l'association est maintenue, un IPP l'accompagne.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["millepertuis"],
            &["AOD", "AVK", "contraception", "immunosuppresseur", "anticancéreux", "antirétroviral", "vastatine"],
        ]),
        severity: Severity::Alert,
        title: "Millepertuis inducteur",
        detail: "Le millepertuis est un inducteur enzymatique puissant : il fait échouer une contraception, un anticoagulant, un immunosuppresseur ou un anticancéreux oral. « Naturel » ne veut pas dire sans interaction.",
    },
    Rule {
        kind: Kind::Duplicate(
            &["macrolide", "fluoroquinolone", "antipsychotique", "citalopram", "escitalopram", "amiodarone", "antifongique azolé", "dompéridone", "hydroxyzine", "méthadone"],
            2,
        ),
        severity: Severity::Warn,
        title: "Deux allongeurs du QT",
        detail: "Deux molécules qui allongent l'intervalle QT sur la même ordonnance : le risque de torsade de pointes s'additionne, surtout si la kaliémie est basse. Un ECG et un contrôle du potassium se discutent.",
    },
    Rule {
        kind: Kind::Duplicate(
            &["anticholinergique", "oxybutynine", "solifénacine", "toltérodine", "hydroxyzine", "antidépresseur tricyclique", "antihistaminique H1 sédatif", "butylscopolamine", "antiparkinsonien anticholinergique"],
            2,
        ),
        severity: Severity::Warn,
        title: "Charge anticholinergique",
        detail: "Confusion, chutes, rétention urinaire, constipation et sécheresse : les effets s'additionnent d'une molécule à l'autre. La charge anticholinergique s'évalue sur l'ensemble de l'ordonnance, et non ligne par ligne.",
    },
    Rule {
        kind: Kind::Duplicate(
            &["benzodiazépine", "zolpidem", "zopiclone", "hypnotique", "opioïde", "antipsychotique", "antihistaminique H1 sédatif"],
            3,
        ),
        severity: Severity::Warn,
        title: "Trois sédatifs",
        detail: "Trois molécules sédatives ou plus : chutes et confusion, surtout après 75 ans. Chacune peut se justifier isolément, mais leur cumul expose au risque : hiérarchiser les molécules et les retirer dans l'ordre.",
    },
    Rule {
        kind: Kind::Duplicate(&["oméprazole", "ésoméprazole", "pantoprazole", "lansoprazole", "rabéprazole", "oméprazole", "ésoméprazole", "pantoprazole", "lansoprazole", "rabéprazole"], 2),
        severity: Severity::Warn,
        title: "Deux IPP",
        detail: "Deux inhibiteurs de la pompe à protons : le plus souvent un reliquat d'ordonnance hospitalière. Un seul suffit, et son indication se réévalue.",
    },
    Rule {
        kind: Kind::Duplicate(&["benzodiazépine", "zolpidem", "zopiclone"], 2),
        severity: Severity::Warn,
        title: "Deux benzodiazépines",
        detail: "Deux benzodiazépines ou apparentés ensemble : aucun gain d'efficacité, mais un risque accru de dépendance et de chutes. Le relais vers une seule molécule se prépare.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["inhibiteur calcique", "amlodipine", "nifédipine", "félodipine", "lercanidipine"],
            &["diurétique", "furosémide", "hydrochlorothiazide", "indapamide"],
        ]),
        severity: Severity::Info,
        title: "Œdème et diurétique",
        detail: "L'œdème des chevilles des dihydropyridines n'est pas une rétention d'eau : un diurétique ne le corrige pas. Si le diurétique a été ajouté pour cela, il s'agit d'une cascade de prescription : la baisse de dose ou le changement de classe est à proposer.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["IEC", "sartan", "ARA II"],
            &["spironolactone", "éplérénone", "antialdostérone", "diurétique épargneur"],
        ]),
        severity: Severity::Info,
        title: "Kaliémie à surveiller",
        detail: "Bloqueur du système rénine-angiotensine et anti-aldostérone : l'association est légitime dans l'insuffisance cardiaque, mais la kaliémie et la créatinine se contrôlent une à deux semaines après chaque changement, puis régulièrement.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["biphosphonate", "bisphosphonate", "alendronate", "risédronate", "ibandronate"],
            &["calcium", "ferreux", "ferrique", "fer saccharose", "magnésium", "oméprazole", "ésoméprazole", "pantoprazole", "lansoprazole", "rabéprazole"],
        ]),
        severity: Severity::Info,
        title: "Bisphosphonate à distance",
        detail: "Le calcium et les cations divalents annulent l'absorption du bisphosphonate : la prise se fait à jeun, seule, et le calcium au moins deux heures plus tard.",
    },
    Rule {
        kind: Kind::Duplicate(&["paracétamol"], 2),
        severity: Severity::Alert,
        title: "Deux sources de paracétamol",
        detail: "Deux spécialités contenant du paracétamol sur la même ordonnance : situation classique de surdosage involontaire, le nom de l'une des spécialités ne mentionnant pas le paracétamol. Faire la somme des grammes par jour devant le patient, et ne garder qu'une source.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["vastatine", "simvastatine", "atorvastatine", "rosuvastatine", "pravastatine"],
            &["clarithromycine", "érythromycine", "josamycine", "télithromycine", "kétoconazole", "itraconazole", "fluconazole", "voriconazole", "posaconazole", "miconazole", "vérapamil", "diltiazem"],
        ]),
        severity: Severity::Alert,
        title: "Statine + inhibiteur enzymatique",
        detail: "Macrolide, azolé ou inhibiteur calcique bradycardisant : la concentration de la statine augmente, avec un risque de rhabdomyolyse. Pour une antibiothérapie courte, la statine se suspend le temps du traitement : un arrêt de quelques jours est sans conséquence.",
    },
    // L'amlodipine et l'amiodarone ne sont **pas** dans la règle
    // ci-dessus, et c'est voulu : ce qu'elles imposent est un plafond de
    // dose sur une ordonnance au long cours, pas une suspension le temps
    // d'une cure. La conduite n'est pas la même, et elle ne vaut que
    // pour la simvastatine — l'atorvastatine n'a pas ce plafond. Le
    // vérapamil et le diltiazem restent là-haut : ce sont des
    // inhibiteurs puissants, et la fiche du Zocor les contre-indique
    // au-delà du même plafond.
    Rule {
        kind: Kind::Combination(&[
            &["simvastatine", "zocor", "lodales"],
            &["amlodipine", "amiodarone"],
        ]),
        severity: Severity::Warn,
        title: "Simvastatine au-dessus de son plafond",
        detail: "L'amlodipine et l'amiodarone limitent la simvastatine à 20 mg par jour. C'est un plafond et non une contre-indication, et le logiciel ne connaît pas la dose : elle se lit sur l'ordonnance, et une amlodipine avec de la simvastatine 40 mg est une ordonnance à corriger. L'atorvastatine et la rosuvastatine n'ont pas ce plafond, et le passage à l'une d'elles est la réponse habituelle du prescripteur.",
    },
    Rule {
        kind: Kind::Duplicate(
            &["anticoagulant", "AOD", "AVK", "héparine", "apixaban", "rivaroxaban", "dabigatran", "édoxaban", "warfarine", "fluindione", "acénocoumarol", "énoxaparine", "tinzaparine", "fondaparinux"],
            2,
        ),
        severity: Severity::Alert,
        title: "Deux anticoagulants",
        detail: "Deux anticoagulants ensemble hors relais organisé : le risque hémorragique s'additionne sans bénéfice. Un relais héparine-AVK se chevauche quelques jours et c'est écrit sur l'ordonnance ; en dehors de ce cas, appeler le prescripteur avant de délivrer.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["allopurinol", "fébuxostat"],
            &["azathioprine", "mercaptopurine"],
        ]),
        severity: Severity::Alert,
        title: "Allopurinol + azathioprine",
        detail: "L'allopurinol bloque la voie qui dégrade l'azathioprine : l'exposition est multipliée, avec un risque réel d'aplasie médullaire. L'association se refuse en délivrance et se discute avec le prescripteur ; si elle est maintenue, la dose d'azathioprine est divisée par quatre et la NFS surveillée.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["IEC", "sartan", "ARA II"],
            &["antialdostérone", "spironolactone", "éplérénone"],
            &["potassium", "kaléorid", "diffu-k"],
        ]),
        severity: Severity::Alert,
        title: "Triple risque hyperkaliémique",
        detail: "Bloqueur du système rénine-angiotensine, antialdostérone et supplément potassique : trois sources de potassium sur la même ordonnance. La kaliémie se contrôle avant de délivrer, et le supplément est presque toujours celui qui se retire.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["AVK", "warfarine", "fluindione", "acénocoumarol"],
            &["antibiotique", "amoxicilline", "macrolide", "fluoroquinolone", "cotrimoxazole", "métronidazole", "cycline", "céphalosporine"],
        ]),
        severity: Severity::Warn,
        title: "AVK + antibiotique",
        detail: "Toute antibiothérapie déséquilibre un AVK, dans un sens ou dans l'autre, et le cotrimoxazole et le métronidazole le font fortement. Prévoir un INR trois à cinq jours après le début du traitement, et le dire au patient avant qu'il sorte.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["metformine", "biguanide"],
            &["diurétique", "furosémide", "hydrochlorothiazide", "indapamide"],
            &["IEC", "sartan", "ARA II"],
        ]),
        severity: Severity::Warn,
        title: "Metformine et jours de maladie",
        detail: "Metformine, diurétique et bloqueur du système rénine-angiotensine : la fonction rénale dépend de l'hydratation. En cas de fièvre, de diarrhée, de vomissements ou de forte chaleur, les trois se suspendent le temps de l'épisode. C'est la règle des jours de maladie, à expliquer au patient par écrit.",
    },
    Rule {
        kind: Kind::Duplicate(
            &["antiagrégant", "clopidogrel", "ticagrélor", "prasugrel", "acide acétylsalicylique", "aspirine"],
            2,
        ),
        severity: Severity::Warn,
        title: "Double antiagrégation",
        detail: "Deux antiagrégants : après un stent, c'est le traitement, pour une durée limitée (souvent six à douze mois) au terme de laquelle un seul est conservé. Chercher la date de pose sur le dossier, et si elle est ancienne, poser la question au prescripteur.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["bêtabloquant", "bêta-bloquant", "bisoprolol", "aténolol", "métoprolol", "propranolol", "nébivolol"],
            &["insuline", "sulfamide hypoglycémiant", "glinide", "gliclazide", "glimépiride", "répaglinide"],
        ]),
        severity: Severity::Warn,
        title: "Bêtabloquant + hypoglycémiant",
        detail: "Le bêtabloquant masque les signes de l'hypoglycémie (tremblements, palpitations) et n'en laisse que les sueurs. En informer le patient : la sueur seule devient le signal, et la glycémie se contrôle au moindre doute plutôt que de se fier aux sensations.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène", "coxib"],
            &["antihypertenseur", "IEC", "sartan", "ARA II", "bêtabloquant", "diurétique", "inhibiteur calcique"],
        ]),
        severity: Severity::Warn,
        title: "AINS et tension",
        detail: "Un AINS fait remonter la tension et annule une partie de l'effet du traitement, y compris pris quelques jours en automédication. Devant une tension qui se dérègle sans raison, rechercher d'abord une automédication par AINS.",
    },
    Rule {
        kind: Kind::Without(
            &[&[
                "opioïde",
                "skenan", "sevredol", "actiskenan", "oramorph", "moscontin",
                "oxycodone",
                "tramadol",
                "codéine",
                "fentanyl",
                "buprénorphine",
            ]],
            &["laxatif", "macrogol", "lactulose", "bisacodyl", "sterculia", "docusate"],
            &[],
        ),
        severity: Severity::Warn,
        title: "Opioïde sans laxatif",
        detail: "La constipation sous opioïde est constante, ne s'épuise pas avec le temps et se prévient dès la première prise. Aucun laxatif sur cette ordonnance : en proposer un dès maintenant pour prévenir l'occlusion.",
    },
    Rule {
        kind: Kind::Without(
            &[&[
                "corticoïde",
                "prednisone",
                "prednisolone",
                "méthylprednisolone",
                "bétaméthasone",
                "dexaméthasone",
            ]],
            // Pas « calcium » : le mot nomme aussi le Rennie (un
            // antiacide) et le Resikali (une résine à potassium), qui
            // faisaient taire la règle — voir
            // `an_absence_is_not_filled_by_a_namesake`.
            &[
                "vitamine D",
                "cholécalciférol",
                "bisphosphonate",
                "biphosphonate",
                "alendronate",
                "risédronate",
                "acide zolédronique",
                "dénosumab",
            ],
            &[],
        ),
        severity: Severity::Warn,
        title: "Corticoïde sans protection osseuse",
        detail: "Une corticothérapie orale prolongée fait perdre de l'os dès les premiers mois, et rien sur cette ordonnance ne s'y oppose. Si la cure dépasse trois mois, la question du calcium, de la vitamine D et d'un bisphosphonate se pose au prescripteur. S'il s'agit d'une cure courte, aucune mesure n'est nécessaire.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["méthotrexate"],
            &["cotrimoxazole", "triméthoprime", "sulfaméthoxazole", "bactrim"],
        ]),
        severity: Severity::Alert,
        title: "Méthotrexate + cotrimoxazole",
        detail: "Deux antifoliques ensemble : l'aplasie médullaire est le risque, et elle survient même sous méthotrexate hebdomadaire à faible dose. L'association est à proscrire : appeler le prescripteur pour un autre antibiotique avant de délivrer.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["lévothyroxine", "hormone thyroïdienne"],
            &["ferreux", "ferrique", "fer saccharose", "calcium", "magnésium", "oméprazole", "ésoméprazole", "pantoprazole", "lansoprazole", "rabéprazole", "colestyramine"],
        ]),
        severity: Severity::Warn,
        title: "Lévothyroxine et chélation",
        detail: "Fer, calcium, IPP et résines abaissent l'absorption de la lévothyroxine, et une TSH qui dérive vient plus souvent de là que de la dose. Deux heures d'écart au moins, à heure fixe, et la TSH se recontrôle six semaines après tout changement.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["digoxine", "digitalique"],
            &["amiodarone", "vérapamil", "diltiazem", "clarithromycine", "érythromycine", "josamycine", "itraconazole"],
        ]),
        severity: Severity::Alert,
        title: "Digoxine potentialisée",
        detail: "Ces molécules augmentent la digoxinémie, parfois du double : nausées, vision jaune, pouls lent ou irrégulier et confusion en sont les premiers signes. La dose de digoxine se réduit à l'introduction et la digoxinémie se contrôle, en lien avec le prescripteur.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["ISRS", "IRSNa", "fluoxétine", "paroxétine", "sertraline", "citalopram", "escitalopram", "venlafaxine", "duloxétine"],
            &["tramadol", "triptan", "sumatriptan", "linézolide", "millepertuis", "lithium"],
        ]),
        severity: Severity::Alert,
        title: "Risque de syndrome sérotoninergique",
        detail: "Agitation, tremblements, sueurs, diarrhée, fièvre et rigidité dans les heures qui suivent l'ajout : évoquer un syndrome sérotoninergique, potentiellement grave. Le tramadol est le plus souvent en cause parce qu'il passe pour un simple antalgique. Signaler avant de délivrer.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["inhibiteur PDE5", "sildénafil", "tadalafil", "vardénafil", "avanafil"],
            &["dérivé nitré", "trinitrine", "molsidomine", "isosorbide", "nicorandil"],
        ]),
        severity: Severity::Alert,
        title: "PDE5 + donneur de NO",
        detail: "Association formellement contre-indiquée : la chute de tension est brutale et peut être fatale. Elle se cherche activement, parce que le patient n'annonce pas spontanément qu'il prend un traitement de l'érection, et parce que la trinitrine sublinguale se prend sans y penser.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["quinolone", "fluoroquinolone", "ciprofloxacine", "lévofloxacine", "ofloxacine", "norfloxacine", "moxifloxacine"],
            &["corticoïde", "prednisone", "prednisolone", "méthylprednisolone"],
        ]),
        severity: Severity::Warn,
        title: "Fluoroquinolone + corticoïde",
        detail: "Le risque de rupture du tendon d'Achille est multiplié, et il est le plus élevé après 60 ans. Toute douleur tendineuse fait arrêter la quinolone et cesser tout appui sur le tendon ; la rupture survient souvent sans effort particulier, parfois après l'arrêt du traitement.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["vastatine", "simvastatine", "atorvastatine", "rosuvastatine", "pravastatine"],
            &["colchicine"],
        ]),
        severity: Severity::Warn,
        title: "Statine + colchicine",
        detail: "Les deux sont myotoxiques et l'association multiplie le risque de rhabdomyolyse, d'autant plus que la colchicine est prescrite dans la goutte, chez des patients souvent insuffisants rénaux. Toute douleur musculaire diffuse avec urines foncées fait arrêter les deux et consulter.",
    },
    Rule {
        kind: Kind::Without(
            &[&[
                "corticoïde inhalé",
                "CSI",
                "budésonide",
                "fluticasone",
                "béclométasone",
                "ciclésonide",
            ]],
            &["bêta-2", "salbutamol", "terbutaline", "bronchodilatateur"],
            // **Le budésonide avalé n'est pas un traitement de l'asthme** :
            // l'Entocort et le Cortiment soignent un Crohn ou une
            // rectocolite, et « budésonide » les attrapait dès qu'ils ont
            // cessé d'être comptés parmi les formes locales.
            &["budésonide oral", "MMX", "MICI", "action locale"],
        ),
        severity: Severity::Warn,
        title: "Corticoïde inhalé sans traitement de crise",
        detail: "Un traitement de fond de l'asthme sans bronchodilatateur de secours sur l'ordonnance : soit le patient en a un chez lui et il faut vérifier sa date de péremption et sa technique, soit il n'en a pas, et il en sera dépourvu lors de la prochaine crise. Poser la question dès maintenant.",
    },
    Rule {
        // The oral group deliberately lists molecules and not the class:
        // « bêtabloquant » would match the collyre too, and a single
        // Timoptol would then satisfy both halves of the pair.
        kind: Kind::Combination(&[
            &[
                "bisoprolol",
                "aténolol",
                "métoprolol",
                "propranolol",
                "nébivolol",
                "carvédilol",
                "sotalol",
                "acébutolol",
                "céliprolol",
                "labétalol",
            ],
            &["timolol", "cartéolol", "bétaxolol"],
        ]),
        severity: Severity::Warn,
        title: "Bêtabloquant caché",
        detail: "Un collyre bêtabloquant du glaucome passe dans la circulation par la muqueuse nasale et échappe au premier passage hépatique : il s'ajoute au bêtabloquant oral et la bradycardie, l'asthénie ou le bronchospasme qui suivent ne sont attribués ni à l'un ni à l'autre. Occlure le point lacrymal une minute après l'instillation divise ce passage. L'asthme contre-indique aussi la forme collyre.",
    },
    Rule {
        kind: Kind::Combination(&[
            &[
                "carbamazépine",
                "oxcarbazépine",
                "phénytoïne",
                "phénobarbital",
                "primidone",
                "rifampicine",
                "millepertuis",
                "éfavirenz",
            ],
            &[
                "contraception",
                "éthinylestradiol",
                "estroprogestatif",
                "désogestrel",
                "lévonorgestrel",
                "drospirénone",
                "gestodène",
                "norgestimate",
            ],
        ]),
        severity: Severity::Alert,
        title: "Contraception sous inducteur",
        detail: "L'inducteur enzymatique accélère la dégradation des hormones et fait échouer la contraception, pilule comme implant et anneau ; le stérilet au cuivre et le dispositif au lévonorgestrel sont les deux méthodes qui n'en dépendent pas. L'échec est silencieux jusqu'au test de grossesse. Cela vaut pendant tout le traitement et encore quatre semaines après son arrêt, et le millepertuis compte, même acheté sans ordonnance.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["apixaban", "rivaroxaban", "édoxaban", "dabigatran"],
            &[
                "amiodarone",
                "dronédarone",
                "vérapamil",
                "clarithromycine",
                "itraconazole",
                "kétoconazole",
                "ciclosporine",
                "ritonavir",
                "cobicistat",
            ],
        ]),
        severity: Severity::Warn,
        title: "AOD potentialisé",
        detail: "Ces molécules inhibent la P-glycoprotéine, et pour certaines le CYP3A4 : l'exposition à l'anticoagulant direct augmente sans signe visible, faute d'INR pour le mesurer. Selon la molécule et l'association, la dose se réduit ou l'association se contre-indique : la dronédarone et le kétoconazole sont contre-indiqués avec le dabigatran. Vérifier la dose prescrite contre l'âge, le poids et la clairance, et signaler tout saignement.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["prednisone", "prednisolone", "méthylprednisolone", "bétaméthasone"],
            &[
                "insuline",
                "biguanide",
                "sulfamide hypoglycémiant",
                "gliptine",
                "isglt2",
                "analogue glp-1",
                "glinide",
            ],
        ]),
        severity: Severity::Warn,
        title: "Corticoïde et glycémie",
        detail: "Une corticothérapie fait monter la glycémie dès les premiers jours, surtout en fin de journée avec une prise matinale, et peut déséquilibrer un diabète jusque-là contrôlé. Prévenir le patient d'augmenter l'autosurveillance pendant la cure et de ne pas s'inquiéter d'une baisse à l'arrêt : c'est l'adaptation qui suit la corticothérapie, et elle se fait avec le prescripteur. Chez un patient non diabétique connu, une cure prolongée justifie de vérifier la glycémie.",
    },
    Rule {
        kind: Kind::Duplicate(&["corticoïde", "prednisone", "prednisolone", "cortancyl", "solupred", "célestène", "médrol", "corticoïde substitutif"], 3),
        severity: Severity::Info,
        title: "Charge corticoïde cumulée",
        detail: "Trois corticoïdes ou plus sur une même ordonnance (inhalé, nasal, cutané, collyre, oral) s'additionnent : l'exposition de chacun est faible, mais leur somme est significative. La freination surrénalienne, la fragilité cutanée, la cataracte et l'ostéoporose se jugent sur le total et non sur une ligne. Vérifier que chacun garde une indication actuelle et une durée, en particulier le dermocorticoïde renouvelé sans limite.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["warfarine", "fluindione", "acénocoumarol"],
            &["amiodarone"],
        ]),
        severity: Severity::Alert,
        title: "AVK + amiodarone",
        detail: "L'amiodarone augmente fortement l'effet de l'antivitamine K, et elle le fait lentement : l'INR augmente au bout d'une à trois semaines, longtemps après l'introduction, puis reste perturbé des mois après l'arrêt tant la molécule est stockée. Un INR est nécessaire dans la semaine qui suit l'introduction, puis rapproché, et la dose d'AVK se réduit le plus souvent d'un tiers.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["hydrochlorothiazide", "indapamide", "thiazidique"],
            &["calcium"],
            &["vitamine d", "cholécalciférol", "calcifédiol"],
        ]),
        severity: Severity::Warn,
        title: "Calcémie cumulée",
        detail: "Le thiazidique réduit l'élimination urinaire du calcium pendant que la supplémentation en apporte : l'hypercalcémie s'installe lentement et se manifeste par de la soif, des urines abondantes, une constipation, des nausées et une confusion, signes souvent attribués à l'âge. Une calcémie suffit à trancher, et la supplémentation se réévalue : elle est souvent poursuivie sans réévaluation.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["carbapénème", "méropénème", "imipénème", "ertapénème"],
            &["valproate", "valproïque", "divalproate"],
        ]),
        severity: Severity::Alert,
        title: "Carbapénème + valproate",
        detail: "Le carbapénème diminue massivement les concentrations de valproate en quelques jours, sans signe d'alerte : des états de mal épileptiques sont survenus chez des patients jusque-là équilibrés. Augmenter la dose de valproate ne compense pas le phénomène. L'association est à éviter et impose de choisir un autre antibiotique ; si elle est maintenue, la couverture antiépileptique doit être assurée autrement et le patient surveillé.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["miconazole"],
            &["warfarine", "fluindione", "acénocoumarol", "avk"],
        ]),
        severity: Severity::Alert,
        title: "Miconazole + AVK",
        detail: "Le miconazole inhibe puissamment le CYP2C9 et augmente l'INR jusqu'au risque hémorragique, et cela vaut aussi pour le gel buccal et pour l'ovule gynécologique, dont le passage systémique suffit ; un gel buccal est rarement considéré comme un médicament à action générale. L'association est contre-indiquée. Si un traitement local a déjà été commencé, l'INR se contrôle sans attendre.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["miconazole"],
            &[
                "sulfamide hypoglycémiant",
                "glibenclamide",
                "gliclazide",
                "glimépiride",
            ],
        ]),
        severity: Severity::Alert,
        title: "Miconazole + sulfamide hypoglycémiant",
        detail: "Même mécanisme que pour les AVK : l'inhibition du CYP2C9 majore fortement l'exposition au sulfamide et provoque des hypoglycémies sévères et prolongées, gel buccal et ovule compris. L'association est contre-indiquée. Un antifongique local qui ne passe pas par cette voie est à préférer, et une hypoglycémie inexpliquée chez un patient qui vient de traiter une mycose doit faire rechercher ce gel, souvent absent de l'ordonnance.",
    },
    Rule {
        kind: Kind::Combination(&[&["gemfibrozil"], &["répaglinide", "glinide"]]),
        severity: Severity::Alert,
        title: "Gemfibrozil + répaglinide",
        detail: "Le gemfibrozil inhibe le CYP2C8 et multiplie l'exposition au répaglinide dans des proportions considérables, avec des hypoglycémies sévères et prolongées : l'association est contre-indiquée, et non simplement déconseillée. Un autre fibrate ou un autre antidiabétique doit être retenu, et la question se pose au prescripteur avant la délivrance.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["gentamicine", "amikacine"],
            &["furosémide", "bumétanide", "diurétique de l'anse"],
        ]),
        severity: Severity::Warn,
        title: "Aminoside + diurétique de l'anse",
        detail: "Les deux sont ototoxiques et leurs effets s'additionnent : l'atteinte cochléaire et vestibulaire de l'aminoside est irréversible, et le diurétique en abaisse le seuil tout en favorisant la déshydratation qui majore la néphrotoxicité. L'association se limite à la durée strictement nécessaire, sous contrôle de la fonction rénale et des concentrations résiduelles, et toute baisse d'audition, tout acouphène ou toute instabilité à la marche se signale sans attendre.",
    },
    // Ce qui suit est sorti des sections « Toxicité / marge
    // thérapeutique » des fiches, relu contre elles. C'est le gisement
    // que `docs/CONTENU.md` désigne : une association écrite comme
    // contre-indiquée sur une monographie ne sert au comptoir que si la
    // revue la voit sur l'ordonnance. Chaque règle nomme la fiche qui
    // l'écrit, et se corrige en corrigeant cette fiche.
    Rule {
        kind: Kind::Combination(&[
            &["cotrimoxazole", "triméthoprime", "sulfaméthoxazole", "bactrim"],
            &["IEC", "sartan", "ARA II", "spironolactone", "éplérénone", "antialdostérone", "amiloride", "diurétique épargneur"],
        ]),
        severity: Severity::Alert,
        title: "Cotrimoxazole + hyperkaliémiant",
        detail: "Le triméthoprime bloque le canal sodium du tube distal comme un diurétique épargneur : sur un bloqueur du système rénine-angiotensine ou une spironolactone, la kaliémie monte sans qu'aucun signe précède le trouble du rythme. Cinq jours de cure suffisent ; le sujet âgé ou insuffisant rénal est le plus exposé, avec un risque vital. Demander une kaliémie en cours de cure, écarter les sels de régime, et signaler l'association au prescripteur.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["tamoxifène"],
            &["paroxétine", "fluoxétine"],
        ]),
        severity: Severity::Alert,
        title: "Tamoxifène + paroxétine ou fluoxétine",
        detail: "Le tamoxifène est une prodrogue que le CYP2D6 active en endoxifène : la paroxétine et la fluoxétine, inhibiteurs puissants, réduisent l'efficacité d'un traitement destiné à prévenir la récidive. L'association naît le plus souvent du traitement des bouffées de chaleur dues au tamoxifène lui-même, ce qui la rend fréquente à l'ordonnance. Signaler avant délivrance ; la venlafaxine et l'escitalopram sont les alternatives usuelles.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["métoprolol", "nébivolol"],
            &["paroxétine", "fluoxétine", "bupropion", "duloxétine", "terbinafine"],
        ]),
        severity: Severity::Warn,
        title: "Métoprolol ou nébivolol + inhibiteur CYP2D6",
        detail: "Le métoprolol et le nébivolol passent par le CYP2D6 : la paroxétine, la fluoxétine, le bupropion, la duloxétine et la terbinafine multiplient leur exposition et donnent bradycardie, asthénie et hypotension chez un patient dont aucune dose n'a changé. Faire prendre le pouls et la tension les premiers jours, et rattacher toute chute ou fatigue nouvelle à cette introduction plutôt qu'à l'âge. Le bisoprolol et l'aténolol ne passent pas par cette voie.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["dolutégravir", "raltégravir", "bictégravir", "inhibiteur d'intégrase"],
            &["antiacide", "pansement gastrique", "hydroxyde d'aluminium", "calcium", "magnésium", "ferreux", "zinc"],
        ]),
        severity: Severity::Alert,
        title: "Inhibiteur d'intégrase + cations",
        detail: "Les cations chélatent l'inhibiteur d'intégrase et réduisent fortement son absorption : le traitement échoue et des résistances émergent, sans qu'aucun symptôme prévienne. Ces produits s'achètent sans ordonnance, le repérage se fait donc au comptoir. Jamais ensemble : deux heures avant ou six heures après le cation, et poser la question du pansement gastrique, que le patient cite rarement.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["méthylergométrine", "méthergin"],
            &["clarithromycine", "érythromycine", "josamycine", "télithromycine", "itraconazole", "kétoconazole", "voriconazole", "posaconazole", "ritonavir", "cobicistat"],
        ]),
        severity: Severity::Alert,
        title: "Méthylergométrine + inhibiteur CYP3A4",
        detail: "L'inhibition du CYP3A4 fait monter l'exposition à un alcaloïde de l'ergot de seigle : risque d'ergotisme, avec vasoconstriction des extrémités et ischémie parfois irréversible. L'association est contre-indiquée. En post-partum, elle survient typiquement avec un macrolide prescrit pour une mastite ; refuser la délivrance et rappeler le prescripteur.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["fluconazole", "triflucan"],
            &["AVK", "warfarine", "fluindione", "acénocoumarol"],
        ]),
        severity: Severity::Alert,
        title: "Fluconazole + AVK",
        detail: "Le fluconazole inhibe le CYP2C9 et majore fortement l'effet de l'AVK : l'INR augmente en quelques jours, avec un risque hémorragique. Une dose unique de 150 mg pour une mycose vaginale suffit à le faire. Prévoir un INR à quarante-huit heures, prévenir le patient avant qu'il sorte, et poser la question de l'antifongique local qui ne passe pas par cette voie.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["dasatinib", "erlotinib", "géfitinib", "pazopanib"],
            &["oméprazole", "ésoméprazole", "pantoprazole", "lansoprazole", "rabéprazole", "oméprazole", "ésoméprazole", "pantoprazole", "lansoprazole", "rabéprazole"],
        ]),
        severity: Severity::Warn,
        title: "Inhibiteur de tyrosine kinase + IPP",
        detail: "L'absorption de ces anticancéreux oraux dépend de l'acidité gastrique : sous IPP elle diminue, et l'efficacité du traitement anticancéreux avec elle, sans signe d'alerte. L'IPP est souvent ancien et rarement mis en cause. Signaler au prescripteur : l'antiacide décalé ou l'anti-H2 sont les recours, jamais l'IPP maintenu tel quel.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["buprénorphine", "nalbuphine"],
            &["skenan", "sevredol", "actiskenan", "oramorph", "moscontin", "oxycodone", "fentanyl", "hydromorphone", "méthadone"],
        ]),
        severity: Severity::Alert,
        title: "Buprénorphine ou nalbuphine + agoniste pur",
        detail: "L'agoniste partiel déloge l'agoniste pur de son récepteur : chez un patient sous morphinique, l'association déclenche un syndrome de sevrage aigu et une reprise brutale de la douleur. L'association n'a pas lieu d'être sur la même ordonnance. Chercher lequel des deux est le traitement de fond, et appeler le prescripteur avant de délivrer le second.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["drospirénone"],
            &["IEC", "sartan", "ARA II", "spironolactone", "éplérénone", "antialdostérone", "amiloride", "triméthoprime", "cotrimoxazole"],
        ]),
        severity: Severity::Warn,
        title: "Drospirénone + hyperkaliémiant",
        detail: "La drospirénone est un dérivé de la spironolactone et retient le potassium comme elle : une pilule est rarement identifiée comme hyperkaliémiante, ce qui expose à méconnaître l'association. Contrôler la kaliémie le premier mois, et plus étroitement en cas d'altération de la fonction rénale.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["lamotrigine", "lamictal"],
            &["valproate", "valproïque", "divalproate", "dépakine", "dépakote"],
        ]),
        severity: Severity::Alert,
        title: "Lamotrigine + valproate",
        detail: "Le valproate double la demi-vie de la lamotrigine : à titration normale, le risque est le syndrome de Lyell ou de Stevens-Johnson, qui dépend de la vitesse de montée plus que de la dose finale. Le protocole d'association divise les paliers par deux. Vérifier la titration écrite sur l'ordonnance, et dire au patient qu'une éruption dans les huit semaines impose l'arrêt et une consultation le jour même.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["naltrexone", "revia"],
            &["skenan", "sevredol", "actiskenan", "oramorph", "moscontin", "oxycodone", "fentanyl", "hydromorphone", "tramadol", "codéine", "méthadone", "buprénorphine"],
        ]),
        severity: Severity::Alert,
        title: "Naltrexone + opioïde",
        detail: "La naltrexone bloque le récepteur : l'opioïde devient inefficace, et chez un patient qui en prenait, une seule prise déclenche un sevrage aigu. À l'arrêt de la naltrexone, la tolérance est perdue et la dose antérieure expose à un surdosage. L'association ne se délivre pas sans avoir joint le prescripteur.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["ulipristal", "ellaone"],
            &["désogestrel", "lévonorgestrel", "étonogestrel", "drospirénone", "chlormadinone", "nomégestrol", "diénogest", "progestatif"],
        ]),
        severity: Severity::Warn,
        title: "Ulipristal + progestatif",
        detail: "Le progestatif prend la place de l'ulipristal sur le récepteur et lui retire son effet : reprendre la pilule le lendemain d'une contraception d'urgence à l'ulipristal fait perdre l'efficacité des deux. Attendre cinq jours avant de reprendre le progestatif, et préservatif jusqu'à la fin du cycle. Si la pilule ne peut pas être suspendue, le lévonorgestrel est la contraception d'urgence à délivrer.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["doxycycline", "minocycline", "lymécycline"],
            &["isotrétinoïne", "acitrétine"],
        ]),
        severity: Severity::Alert,
        title: "Cycline + rétinoïde oral",
        detail: "Les deux augmentent la pression intracrânienne et l'association est contre-indiquée : céphalées, vision qui se trouble, vomissements, tableau d'hypertension intracrânienne bénigne qui peut laisser une atteinte du nerf optique. L'association se rencontre dans l'acné, chez des patients jeunes. Refuser l'association et faire arrêter la cycline avant l'instauration.",
    },
    Rule {
        kind: Kind::Without(
            &[&["anti-aromatase", "anastrozole", "létrozole", "exémestane"]],
            // Pas « calcium », pour la même raison qu'au-dessus.
            &["vitamine D", "cholécalciférol", "bisphosphonate", "biphosphonate", "alendronate", "risédronate", "acide zolédronique", "dénosumab"],
            &[],
        ),
        severity: Severity::Warn,
        title: "Anti-aromatase sans protection osseuse",
        detail: "L'anti-aromatase supprime les œstrogènes restants : la perte osseuse est rapide, le traitement dure cinq ans, et rien sur l'ordonnance ne s'y oppose. Demander où en est la densitométrie, et si le calcium et la vitamine D ont été prévus. Cette question est souvent omise en consultation.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["AVK", "warfarine", "fluindione", "acénocoumarol"],
            &["rifampicine", "phénobarbital", "phénytoïne", "carbamazépine", "griséofulvine"],
        ]),
        severity: Severity::Alert,
        title: "AVK + inducteur enzymatique",
        detail: "L'inducteur accélère la dégradation de l'AVK et l'INR s'effondre : l'anticoagulation devient insuffisante, sans symptôme jusqu'à la thrombose. L'arrêt de l'inducteur a l'effet inverse et expose à l'hémorragie. INR une semaine après toute introduction et tout arrêt, et le dire au patient dans les deux sens.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["élétriptan", "relpax"],
            &["itraconazole", "kétoconazole", "clarithromycine", "ritonavir", "cobicistat", "néfazodone"],
        ]),
        severity: Severity::Alert,
        title: "Élétriptan + inhibiteur CYP3A4",
        detail: "L'inhibiteur puissant du CYP3A4 multiplie l'exposition à l'élétriptan, avec un risque de vasospasme coronarien. L'association est contre-indiquée ; respecter un délai de soixante-douze heures après la dernière prise de l'inhibiteur. Un autre triptan, non métabolisé par cette voie, est le recours si la crise ne peut pas attendre.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["fluoroquinolone", "ciprofloxacine", "ofloxacine", "lévofloxacine", "doxycycline", "minocycline", "lymécycline"],
            &["calcium", "magnésium", "zinc", "antiacide", "pansement gastrique", "ferreux"],
        ]),
        severity: Severity::Warn,
        title: "Quinolone ou cycline + cations",
        detail: "Le cation chélate l'antibiotique dans l'estomac et l'absorption tombe de moitié ou plus : la cure est inefficace, et l'échec peut être pris pour une résistance. Le lait, les pansements gastriques et les compléments en font partie, et aucun n'est sur l'ordonnance. Deux heures avant ou quatre heures après, et le demander explicitement au patient.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["sofosbuvir"],
            &["amiodarone"],
        ]),
        severity: Severity::Alert,
        title: "Sofosbuvir + amiodarone",
        detail: "Bradycardies sévères et arrêts cardiaques rapportés dès les premières heures, par un mécanisme encore inexpliqué : l'association est à éviter, et la demi-vie de l'amiodarone la rend encore possible des mois après son arrêt. Si elle est maintenue, la surveillance du rythme est hospitalière les quarante-huit premières heures.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["rizatriptan"],
            &["propranolol"],
        ]),
        severity: Severity::Warn,
        title: "Rizatriptan + propranolol",
        detail: "Le propranolol multiplie par deux l'exposition au rizatriptan : la dose se ramène à 5 mg, et l'association est fréquente puisque le propranolol est un traitement de fond de la migraine et le rizatriptan son traitement de crise. Vérifier le dosage délivré ; c'est le seul triptan concerné.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["gabapentine", "prégabaline", "gabapentinoïde"],
            &["skenan", "sevredol", "actiskenan", "oramorph", "moscontin", "oxycodone", "hydromorphone", "fentanyl", "tramadol", "codéine", "méthadone", "lamaline", "izalgi", "poudre d'opium"],
        ]),
        severity: Severity::Alert,
        title: "Gabapentinoïde + opioïde",
        detail: "Dépression respiratoire par addition, et l'association est banale sur une ordonnance de douleur chronique : elle a fait l'objet d'une alerte de pharmacovigilance. Le risque est maximal à l'instauration, à toute augmentation, et chez l'insuffisant respiratoire ou le sujet âgé. Signaler, et expliquer à l'entourage ce qu'est une somnolence anormale.",
    },
    Rule {
        kind: Kind::Without(
            // Les deux graphies : la base en porte une pour le Fosamax
            // et l'autre pour les quatre suivants, et `classes.rs` les
            // replie — mais cette table-ci compare les mots bruts.
            &[&["bisphosphonate", "biphosphonate"]],
            // **La vitamine D seule, et c'est délibéré.** « calcium »
            // nomme dans cette base trois produits qui ne font pas la
            // même chose : un supplément (Orocal, Cacit), un antiacide
            // (Rennie) et une résine échangeuse de cations (Resikali).
            // Une absence qui se laisse combler par un antiacide est une
            // règle qui se tait quand il faudrait qu'elle parle. La
            // vitamine D, elle, ne nomme que ses cinq formes. Le calcium
            // reste dans la phrase, où il ne décide de rien.
            &["vitamine d"],
            &[],
        ),
        severity: Severity::Warn,
        title: "Bisphosphonate sans vitamine D",
        detail: "Les fiches de ces produits demandent toutes que le statut en vitamine D soit corrigé avant l'instauration, et rien sur cette ordonnance n'y pourvoit. Sur une carence préexistante, l'hypocalcémie devient symptomatique, effet le plus fréquent de la perfusion annuelle. Demander où en est le dosage, et si le calcium alimentaire suffit : l'un et l'autre se règlent avant la première prise.",
    },
    Rule {
        kind: Kind::Without(
            &[&["méthotrexate"]],
            &["acide folique", "spéciafoldine", "folinique", "lederfoline"],
            // **Le veto, et la raison d'être du troisième membre.** La
            // Lederfoline porte « antidote du méthotrexate » dans sa
            // classe : sans lui, elle serait *nommée* par le groupe, et
            // une ordonnance qui ne porterait qu'elle recevrait
            // « méthotrexate sans acide folique » — un reproche adressé
            // au sauvetage lui-même. Écartée du groupe, elle redevient
            // la ligne qui comble.
            &["antidote"],
        ),
        severity: Severity::Warn,
        title: "Méthotrexate sans acide folique",
        detail: "L'acide folique se prescrit avec le méthotrexate hebdomadaire et rien sur cette ordonnance n'en porte : il divise par deux les effets qui font arrêter le traitement (aphtes, nausées, cytolyse) sans rien lui retirer de son efficacité dans le rhumatisme ou le psoriasis. Il se prend à distance de la prise, jamais le même jour. Rappeler la règle dont l'oubli peut être mortel : le méthotrexate est hebdomadaire, un jour fixe de la semaine, jamais quotidien.",
    },
    Rule {
        kind: Kind::Without(
            &[&["isoniazide"]],
            &["vitamine b6", "pyridoxine"],
            &[],
        ),
        severity: Severity::Warn,
        title: "Isoniazide sans vitamine B6",
        detail: "L'isoniazide épuise la pyridoxine et donne une neuropathie périphérique parfois irréversible ; dénutri, alcoolique, diabétique, insuffisant rénal et femme enceinte sont les plus exposés. La pyridoxine se prescrit avec, à 10 à 25 mg par jour, et elle manque ici. Poser la question au prescripteur avant que les fourmillements commencent.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn t<'a>(name: &'a str, dci: &'a str, class: &'a str) -> Treatment<'a> {
        Treatment {
            name,
            dci,
            class,
            tags: "",
        }
    }

    #[test]
    fn the_triad_needs_its_three_terms() {
        let two = [
            t("Coversyl", "périndopril", "IEC"),
            t("Lasilix", "furosémide", "diurétique de l'anse"),
        ];
        assert!(review(&two).iter().all(|p| p.title != "Triade néfaste"));
        let three = [
            t("Coversyl", "périndopril", "IEC"),
            t("Lasilix", "furosémide", "diurétique de l'anse"),
            t("Advil", "ibuprofène", "AINS"),
        ];
        let points = review(&three);
        let triad = points
            .iter()
            .find(|p| p.title == "Triade néfaste")
            .expect("la triade doit être repérée");
        assert_eq!(triad.severity, Severity::Alert);
        assert_eq!(triad.drugs.len(), 3);
        // Loudest first: an alert never sits under an information.
        assert_eq!(points[0].severity, Severity::Alert);
    }

    #[test]
    fn a_fixed_combination_counts_for_both_of_its_halves() {
        // One tablet holding a sartan and a thiazide, plus an AINS: the
        // triad is there in two boxes, not three.
        let ordonnance = [
            t(
                "Cokenzen",
                "candésartan hydrochlorothiazide",
                "ARA II et diurétique",
            ),
            t("Voltarène", "diclofénac", "AINS"),
        ];
        assert!(review(&ordonnance)
            .iter()
            .any(|p| p.title == "Triade néfaste"));
    }

    /// A local antifungal is not a local medicine: the buccal gel and
    /// the ovule pass enough miconazole into the blood to send an INR
    /// through the roof and a sulfamide into hypoglycaemia. Nobody
    /// writes a gel for the mouth on the list of general treatments,
    /// which is exactly why the rule has to read it.
    #[test]
    fn a_buccal_gel_is_read_as_a_general_treatment() {
        let avk = [
            t("Daktarin", "miconazole", "antifongique local et buccal"),
            t("Previscan", "fluindione", "AVK"),
        ];
        let point = review(&avk)
            .into_iter()
            .find(|p| p.title == "Miconazole + AVK")
            .expect("le gel buccal doit être vu face à un AVK");
        assert_eq!(point.severity, Severity::Alert);
        assert_eq!(point.drugs.len(), 2);

        let sulfamide = [
            t("Daktarin", "miconazole", "antifongique local et buccal"),
            t("Diamicron", "gliclazide", "sulfamide hypoglycémiant"),
        ];
        assert!(review(&sulfamide)
            .iter()
            .any(|p| p.title == "Miconazole + sulfamide hypoglycémiant"));

        // The gel on its own says nothing.
        let alone = [t("Daktarin", "miconazole", "antifongique local et buccal")];
        assert!(!review(&alone)
            .iter()
            .any(|p| p.title.starts_with("Miconazole")));
    }

    /// The rule that had to be written by molecule and not by class:
    /// « bêtabloquant » matches the collyre as well as the tablet, and
    /// a Combination lets one treatment satisfy both groups — a patient
    /// on Timoptol alone would have been told he takes two.
    #[test]
    fn a_collyre_alone_is_not_a_hidden_betablocker() {
        let collyre = [t("Timoptol", "timolol", "collyre bêta-bloquant")];
        assert!(review(&collyre)
            .iter()
            .all(|p| p.title != "Bêtabloquant caché"));
        let oral = [t("Cardensiel", "bisoprolol", "bêtabloquant")];
        assert!(review(&oral)
            .iter()
            .all(|p| p.title != "Bêtabloquant caché"));
        let both = [
            t("Cardensiel", "bisoprolol", "bêtabloquant"),
            t("Timoptol", "timolol", "collyre bêta-bloquant"),
        ];
        let point = review(&both)
            .into_iter()
            .find(|p| p.title == "Bêtabloquant caché")
            .expect("les deux ensemble doivent être repérés");
        assert_eq!(point.drugs.len(), 2);
    }

    /// An inducer wrecks a contraception silently — and the herbal one
    /// bought without a prescription counts like the others.
    #[test]
    fn an_inducer_beside_a_contraception_is_an_alert() {
        let ordonnance = [
            t("Tegretol", "carbamazépine", "antiépileptique"),
            t("Leeloo", "lévonorgestrel éthinylestradiol", "contraception"),
        ];
        let point = review(&ordonnance)
            .into_iter()
            .find(|p| p.title == "Contraception sous inducteur")
            .expect("l'échec de contraception doit être repéré");
        assert_eq!(point.severity, Severity::Alert);
        let herbal = [
            t("Millepertuis", "hypericum perforatum", "phytothérapie"),
            t("Optimizette", "désogestrel", "contraception"),
        ];
        assert!(review(&herbal)
            .iter()
            .any(|p| p.title == "Contraception sous inducteur"));
        // The contraception alone says nothing.
        let alone = [t(
            "Leeloo",
            "lévonorgestrel éthinylestradiol",
            "contraception",
        )];
        assert!(review(&alone)
            .iter()
            .all(|p| p.title != "Contraception sous inducteur"));
    }

    /// Three corticosteroids by three routes are one corticosteroid load.
    #[test]
    fn the_corticoid_load_is_counted_across_routes() {
        let two = [
            t("Flixotide", "fluticasone", "corticoïde inhalé"),
            t("Nasonex", "mométasone", "corticoïde nasal"),
        ];
        assert!(review(&two)
            .iter()
            .all(|p| p.title != "Charge corticoïde cumulée"));
        let three = [
            t("Flixotide", "fluticasone", "corticoïde inhalé"),
            t("Nasonex", "mométasone", "corticoïde nasal"),
            t("Diprosone", "bétaméthasone", "dermocorticoïde"),
        ];
        let point = review(&three)
            .into_iter()
            .find(|p| p.title == "Charge corticoïde cumulée")
            .expect("la charge doit être comptée");
        assert_eq!(point.drugs.len(), 3);
    }

    #[test]
    fn a_doublon_needs_two_distinct_treatments() {
        // One AINS alone says nothing.
        let one = [t("Advil", "ibuprofène", "AINS")];
        assert!(review(&one).iter().all(|p| p.title != "Deux AINS"));
        let two = [
            t("Advil", "ibuprofène", "AINS"),
            t("Voltarène", "diclofénac", "AINS"),
        ];
        let points = review(&two);
        let doublon = points
            .iter()
            .find(|p| p.title == "Deux AINS")
            .expect("le doublon doit être repéré");
        assert_eq!(doublon.drugs.len(), 2);
    }

    #[test]
    fn the_cascade_and_the_charge_are_read_on_the_whole_ordonnance() {
        let ordonnance = [
            t("Aricept", "donépézil", "anticholinestérasique"),
            t("Ditropan", "oxybutynine", "anticholinergique vésical"),
            t("Atarax", "hydroxyzine", "antihistaminique H1 sédatif"),
        ];
        let points = review(&ordonnance);
        assert!(points
            .iter()
            .any(|p| p.title == "Cascade anticholinergique"));
        // Two anticholinergics among the three: the charge adds up.
        let charge = points
            .iter()
            .find(|p| p.title == "Charge anticholinergique")
            .expect("la charge doit être comptée");
        assert!(charge.drugs.len() >= 2);
    }

    #[test]
    fn an_empty_or_quiet_ordonnance_says_nothing() {
        assert!(review(&[]).is_empty());
        let quiet = [
            t("Doliprane", "paracétamol", "antalgique"),
            t("Tahor", "atorvastatine", "vastatine"),
        ];
        assert!(review(&quiet).is_empty());
    }

    /// Same discipline as the biology rules: a rule whose every term
    /// names a card the base does not carry can never fire.

    #[test]
    fn every_rule_can_fire_on_the_base_as_shipped() {
        let matches = |words: &[&str]| {
            words.iter().any(|needle| {
                let needle = crate::fuzzy::sort_key(needle);
                crate::db::STARTER_DRUGS
                    .iter()
                    .any(|(name, dci, class, _)| {
                        crate::fuzzy::sort_key(&format!("{name} {dci} {class}")).contains(&needle)
                    })
            })
        };
        for rule in RULES {
            match &rule.kind {
                Kind::Combination(groups) => {
                    for group in groups.iter() {
                        assert!(
                            matches(group),
                            "règle « {} » : aucun médicament de la base ne correspond à {:?}",
                            rule.title,
                            group
                        );
                    }
                }
                Kind::Duplicate(words, _) => assert!(
                    matches(words),
                    "règle « {} » : aucun médicament de la base ne correspond à {:?}",
                    rule.title,
                    words
                ),
                Kind::Without(groups, absent, _) => {
                    for group in groups.iter() {
                        assert!(
                            matches(group),
                            "règle « {} » : aucun médicament de la base ne correspond à {:?}",
                            rule.title,
                            group
                        );
                    }
                    // The thing whose absence is the finding has to
                    // exist too: a rule that fires because the base has
                    // no laxative at all is a rule about the base.
                    assert!(
                        matches(absent),
                        "règle « {} » : le manque porte sur {:?}, que la base ne connaît pas",
                        rule.title,
                        absent
                    );
                }
            }
        }
    }

    /// **La spiramycine est le macrolide qui n'inhibe pas.**
    ///
    /// La table de référence « Interactions » l'écrit — « Macrolides,
    /// **sauf** spiramycine » — et `cyp.rs` la porte pour la même
    /// raison. Les règles d'inhibition enzymatique cherchaient pourtant
    /// le mot « macrolide », qui attrape la Rovamycine, le Rulid, le
    /// Zithromax et jusqu'à l'Azyter, qui est un collyre. Une statine
    /// avec une spiramycine levait donc « c'est la rhabdomyolyse ».
    ///
    /// Les règles de **classe** gardent le mot, et c'est voulu :
    /// l'allongement du QT et la déstabilisation de l'INR sous
    /// antibiotique valent pour la spiramycine comme pour les autres.
    /// Ce n'est pas la même propriété.
    #[test]
    fn spiramycin_is_the_macrolide_that_does_not_inhibit() {
        let with = |dci: &str| {
            review(&[
                t("Tahor", "atorvastatine", "statine"),
                t("X", dci, "macrolide"),
            ])
        };
        let title = "Statine + inhibiteur enzymatique";
        assert!(
            !with("spiramycine").iter().any(|p| p.title == title),
            "la spiramycine n'inhibe pas"
        );
        assert!(
            with("clarithromycine").iter().any(|p| p.title == title),
            "la clarithromycine, si"
        );
        // Et la propriété de classe reste : deux allongeurs du QT.
        let qt = review(&[
            t("Rovamycine", "spiramycine", "macrolide"),
            t("Cordarone", "amiodarone", "antiarythmique"),
        ]);
        assert!(
            qt.iter().any(|p| p.title == "Deux allongeurs du QT"),
            "l'allongement du QT vaut pour toute la classe"
        );
    }

    /// **Une statine avec un IPP ne lève pas l'alerte de
    /// rhabdomyolyse**, et une statine avec un macrolide, si.
    ///
    /// La règle cherchait « azolé », et « ésoméprazole » le contient —
    /// comme « cotrimoxazole », « aripiprazole », « mébendazole » :
    /// dix-sept fiches sur vingt-cinq. Un Tahor avec un Mopral,
    /// c'est-à-dire une ordonnance de tous les jours, annonçait donc
    /// « la concentration de la statine grimpe et c'est la
    /// rhabdomyolyse ». Une alerte qui se lève tous les jours est une
    /// alerte qu'on apprend à fermer, et c'est la seule chose que la
    /// revue ne peut pas se permettre.
    #[test]
    fn a_statin_with_a_ppi_is_not_a_rhabdomyolysis_alert() {
        let title = "Statine + inhibiteur enzymatique";
        let with_ppi = [
            t("Tahor", "atorvastatine", "statine"),
            t("Mopral", "oméprazole", "IPP"),
        ];
        assert!(
            !review(&with_ppi).iter().any(|p| p.title == title),
            "une statine et un IPP ne font pas une rhabdomyolyse"
        );
        // Et le vrai cas se lève toujours : c'est le macrolide qui
        // freine l'enzyme, et lui seul.
        let with_macrolide = [
            t("Tahor", "atorvastatine", "statine"),
            t("Zeclar", "clarithromycine", "macrolide"),
        ];
        assert!(
            review(&with_macrolide).iter().any(|p| p.title == title),
            "une statine et un macrolide, si"
        );
        // Un azolé systémique aussi.
        let with_azole = [
            t("Tahor", "atorvastatine", "statine"),
            t("Sporanox", "itraconazole", "antifongique azolé"),
        ];
        assert!(
            review(&with_azole).iter().any(|p| p.title == title),
            "une statine et un azolé systémique, si"
        );
    }

    /// What is *missing* only counts as a finding when the thing that
    /// should be there is not: the same ordonnance with a laxative on
    /// **Une absence ne se laisse pas combler par un homonyme.**
    ///
    /// « calcium » nomme dans la base livrée trois produits qui ne font
    /// pas la même chose : un supplément, un antiacide et une résine
    /// échangeuse de cations. Une règle qui accepterait l'un pour
    /// l'autre se tairait précisément quand il faudrait qu'elle parle,
    /// et c'est pourquoi « Bisphosphonate sans vitamine D » ne cherche
    /// que la vitamine D — qui, elle, ne nomme que ses cinq formes.
    ///
    /// Le test tient les deux moitiés : la vitamine D comble, sous
    /// toutes ses formes, et le carbonate de calcium d'un antiacide ne
    /// comble rien.
    #[test]
    fn an_absence_is_not_filled_by_a_namesake() {
        let has = |ordo: &[Treatment], title: &str| review(ordo).iter().any(|p| p.title == title);
        const TITLE: &str = "Bisphosphonate sans vitamine D";
        let fosamax = || t("Fosamax", "alendronate", "bisphosphonate");
        let aclasta = || {
            t(
                "Aclasta",
                "acide zolédronique",
                "biphosphonate — perfusion annuelle",
            )
        };
        // Les deux graphies déclenchent.
        assert!(has(&[fosamax()], TITLE));
        assert!(has(&[aclasta()], TITLE));
        // La vitamine D comble, native comme hydroxylée.
        assert!(!has(
            &[fosamax(), t("Uvedose", "cholécalciférol", "vitamine D")],
            TITLE
        ));
        assert!(!has(
            &[
                fosamax(),
                t(
                    "Dédrogyl",
                    "calcifédiol",
                    "dérivé hydroxylé de la vitamine D"
                ),
            ],
            TITLE
        ));
        // **L'homonyme ne comble pas** : un antiacide au carbonate de
        // calcium n'est pas une supplémentation, et la règle continue de
        // parler.
        assert!(has(
            &[
                fosamax(),
                t(
                    "Rennie",
                    "carbonate de calcium + carbonate de magnésium",
                    "antiacide",
                ),
            ],
            TITLE
        ));
    }

    /// **Le veto : une ligne qui déclenche ne doit pas pouvoir combler,
    /// et une ligne qui comble ne doit pas déclencher.**
    ///
    /// La Lederfoline porte « antidote du méthotrexate » dans sa
    /// classe. Sans veto, elle était *nommée* par le groupe de la règle
    /// — donc déclenchante — et l'absence se cherchant sur les lignes
    /// **non nommées**, elle ne pouvait plus fournir l'acide folinique
    /// qu'elle est. Une ordonnance qui ne portait qu'elle recevait donc
    /// « méthotrexate sans acide folique » : un reproche adressé au
    /// sauvetage lui-même.
    ///
    /// Les quatre cas qui définissent la règle, et le troisième est
    /// celui pour lequel le membre existe.
    #[test]
    fn a_veto_takes_a_line_out_of_the_group_and_gives_it_back_its_voice() {
        let has = |ordo: &[Treatment], title: &str| review(ordo).iter().any(|p| p.title == title);
        const TITLE: &str = "Méthotrexate sans acide folique";
        let mtx = || t("Méthotrexate", "méthotrexate", "immunosuppresseur");
        let folinique = || t("Lederfoline", "acide folinique", "antidote du méthotrexate");
        let folique = || t("Spéciafoldine", "acide folique", "vitamine B9");

        // Seul, le méthotrexate appelle l'acide folique.
        assert!(has(&[mtx()], TITLE));
        // Avec lui, la règle se tait.
        assert!(!has(&[mtx(), folique()], TITLE));
        // **Le cas du veto** : l'acide folinique comble aussi, et il ne
        // déclenche pas — seul, il ne dit rien du tout.
        assert!(!has(&[mtx(), folinique()], TITLE));
        assert!(!has(&[folinique()], TITLE));
    }

    /// it must say nothing.
    #[test]
    fn an_omission_is_a_finding_until_it_is_filled() {
        let alone = [t("Skenan", "morphine", "opioïde")];
        let point = review(&alone)
            .into_iter()
            .find(|p| p.title == "Opioïde sans laxatif")
            .expect("un opioïde seul appelle un laxatif");
        // The point names what *is* there — an absence has no name.
        assert_eq!(point.drugs, ["Skenan"]);
        assert_eq!(point.severity, Severity::Warn);

        let with = [
            t("Skenan", "morphine", "opioïde"),
            t("Forlax", "macrogol 4000", "laxatif osmotique"),
        ];
        assert!(review(&with)
            .iter()
            .all(|p| p.title != "Opioïde sans laxatif"));

        // And the same for the bone under a corticothérapie.
        let bare = [t("Cortancyl", "prednisone", "corticoïde")];
        assert!(review(&bare)
            .iter()
            .any(|p| p.title == "Corticoïde sans protection osseuse"));
        let covered = [
            t("Cortancyl", "prednisone", "corticoïde"),
            t("Uvedose", "cholécalciférol", "vitamine D"),
        ];
        assert!(review(&covered)
            .iter()
            .all(|p| p.title != "Corticoïde sans protection osseuse"));
    }

    /// Two boxes of paracétamol under two different names is the way
    /// the overdose actually happens at the counter.
    #[test]
    fn two_sources_of_the_same_molecule_are_counted() {
        let one = [t("Doliprane", "paracétamol", "antalgique")];
        assert!(review(&one)
            .iter()
            .all(|p| p.title != "Deux sources de paracétamol"));
        let two = [
            t("Doliprane", "paracétamol", "antalgique"),
            t("Lamaline", "paracétamol opium caféine", "opioïde"),
        ];
        let point = review(&two)
            .into_iter()
            .find(|p| p.title == "Deux sources de paracétamol")
            .expect("deux sources de paracétamol");
        assert_eq!(point.severity, Severity::Alert);
        assert_eq!(point.drugs.len(), 2);
    }

    /// Two rules for one reading, which `review` says twice.
    ///
    /// Like `read` in `biology.rs`, `review` runs the whole table and
    /// does not stop at the first rule that answers — so two rules
    /// claiming the same molecules both come out, often at two
    /// severities and in two wordings. « Lévothyroxine à distance » and
    /// « Lévothyroxine et chélation » did exactly that: a file carrying
    /// Levothyrox and calcium got both, and a revue that repeats itself
    /// is a revue people stop reading.
    ///
    /// Two rules are the same reading when they are the same **shape**
    /// — the same variant, the same number of groups, the same `min`
    /// for a `Duplicate` — and every group of one claims a molecule
    /// some group of the other claims. The shape is what keeps
    /// « Deux benzodiazépines » apart from « Trois sédatifs »: same
    /// words, different `min`, and that difference *is* the rule.
    ///
    /// Group membership is tested with the matcher `review` itself
    /// uses, never by comparing the lists: « IPP » and « oméprazole »
    /// share no letter and name the same box.
    #[test]
    fn two_rules_never_make_one_reading_twice() {
        // Un seul mot en commun ne fait pas deux fois la même lecture :
        // le tramadol est un opioïde faible *et* un sérotoninergique, et
        // « Deux opioïdes faibles » n'est pas « Deux sérotoninergiques ».
        // Ce qui les confond, c'est qu'une liste soit **couverte** par
        // l'autre — tous ses mots y désignant déjà quelque chose.
        let covered = |x: &[&str], y: &[&str]| {
            x.iter().all(|w| {
                let key = crate::fuzzy::sort_key(w);
                y.iter().any(|v| {
                    crate::fuzzy::contains_folded(&key, v)
                        || crate::fuzzy::contains_folded(&crate::fuzzy::sort_key(v), w)
                })
            })
        };
        let same_group = |x: &[&str], y: &[&str]| covered(x, y) || covered(y, x);
        let paired = |xs: &[&[&str]], ys: &[&[&str]]| {
            xs.len() == ys.len()
                && xs.iter().all(|x| ys.iter().any(|y| same_group(x, y)))
                && ys.iter().all(|y| xs.iter().any(|x| same_group(x, y)))
        };
        let twins = |a: &Kind, b: &Kind| match (a, b) {
            (Kind::Combination(x), Kind::Combination(y)) => paired(x, y),
            (Kind::Duplicate(x, m), Kind::Duplicate(y, n)) => m == n && same_group(x, y),
            (Kind::Without(x, xa, xn), Kind::Without(y, ya, yn)) => {
                paired(x, y) && same_group(xa, ya) && same_group(xn, yn)
            }
            _ => false,
        };
        let mut doubled: Vec<String> = Vec::new();
        for (i, a) in RULES.iter().enumerate() {
            for b in RULES.iter().skip(i + 1) {
                if twins(&a.kind, &b.kind) {
                    doubled.push(format!("« {} » et « {} »", a.title, b.title));
                }
            }
        }
        assert!(
            doubled.is_empty(),
            "la revue se répète — fondre les deux règles dans la plus complète :\n{}",
            doubled.join("\n")
        );
    }

    #[test]
    fn every_rule_says_what_to_do_about_it() {
        // The catalogue only ever grows: a rule removed is a reading
        // nobody does any more. The floor is a named constant the
        // message reads back — written twice, in figures and in words,
        // the two had already drifted by six.
        //
        // **Descendu de 87 à 86 le 13/09/2026, et c'est le seul motif
        // qui l'autorise :** « Lévothyroxine à distance » et
        // « Lévothyroxine et chélation » disaient la même chose des
        // mêmes traitements, l'une en `Info` et l'autre en `Warn`. Un
        // patient sous Levothyrox et calcium recevait les deux, en deux
        // formulations voisines — et une revue qui se répète est une
        // revue qu'on cesse de lire. Les deux ont été fondues dans la
        // plus complète, qui garde les résines, le magnésium, et la
        // phrase qui compte : une TSH qui dérive vient plus souvent de
        // là que de la dose. **Aucune question n'a été perdue** ; c'est
        // ce qu'il faut pouvoir écrire ici pour baisser ce chiffre.
        //
        // Remonté à 87 le même jour par « Simvastatine au-dessus de son
        // plafond » — une règle ajoutée, une règle de plus au plancher,
        // dans le même commit.
        const RULES_FLOOR: usize = 87;
        assert!(
            RULES.len() >= RULES_FLOOR,
            "{} règles de revue, il y en avait {RULES_FLOOR}",
            RULES.len()
        );
        let mut titles: Vec<&str> = RULES.iter().map(|r| r.title).collect();
        titles.sort_unstable();
        let seen = titles.len();
        titles.dedup();
        assert_eq!(seen, titles.len(), "deux règles portent le même titre");
        for rule in RULES {
            assert!(!rule.title.trim().is_empty());
            assert!(
                rule.detail.trim().len() > 40,
                "règle « {} » trop courte pour être utile",
                rule.title
            );
        }
    }

    /// **Le titre d'une règle est son adresse, donc il est unique.**
    ///
    /// C'est ce qui permet d'adresser une phrase de la revue par son
    /// titre plutôt que par son rang : une règle insérée au milieu du
    /// tableau ne périme alors aucune réécriture de l'officine. Deux
    /// règles au même titre casseraient cette propriété en silence — la
    /// seconde recevrait la réécriture de la première.
    #[test]
    fn a_rule_title_is_an_address_and_no_two_rules_share_one() {
        let mut ids: Vec<String> = RULES
            .iter()
            .map(|r| crate::content::slug(r.title))
            .collect();
        assert!(ids.len() >= 55, "{} règles", ids.len());
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(n, ids.len(), "deux règles partagent une adresse");
        // Et aucune adresse vide : un titre qui ne laisse que des tirets
        // ne désigne rien.
        for rule in RULES {
            assert!(
                !crate::content::slug(rule.title).is_empty(),
                "« {} » ne donne pas d'adresse",
                rule.title
            );
        }
    }

    /// **Toute phrase de la revue s'édite, et toute réécriture arrive
    /// sur le point rendu.**
    ///
    /// Les deux sens, comme pour les carnets : une phrase que `phrases`
    /// oublie ne peut pas être corrigée, une phrase que `resolve` oublie
    /// part quand même telle qu'elle est livrée.
    #[test]
    fn every_review_phrase_is_editable_and_every_rewrite_arrives() {
        let listed = phrases();
        assert_eq!(
            listed.len(),
            RULES.len() * 2,
            "un titre et un détail par règle"
        );

        // Une ordonnance qui déclenche au moins un point.
        let treatments = [
            Treatment {
                name: "Bisoprolol",
                dci: "bisoprolol",
                class: "bêtabloquant",
                tags: "",
            },
            Treatment {
                name: "Aricept",
                dci: "donépézil",
                class: "anticholinestérasique",
                tags: "",
            },
        ];
        let points = review(&treatments);
        assert!(!points.is_empty(), "l'ordonnance doit déclencher un point");

        // Sans réécriture, le point résolu est le point livré.
        let plain = resolve(points.clone(), &crate::content::Overrides::default());
        assert_eq!(plain[0].title, points[0].title);
        assert_eq!(plain[0].detail, points[0].detail);

        // Avec, il porte les mots de l'officine.
        let over = crate::content::Overrides::from_rows(
            listed
                .iter()
                .map(|(k, _, shipped)| (k.clone(), format!("réécrit:{k}"), (*shipped).to_owned()))
                .collect::<Vec<_>>(),
        );
        let mine = resolve(points, &over);
        for p in &mine {
            assert!(p.title.starts_with("réécrit:"), "titre : {}", p.title);
            assert!(p.detail.starts_with("réécrit:"), "détail : {}", p.detail);
        }
    }
    /// **Ce qui manque doit manquer sur une *autre* ligne.**
    ///
    /// Une règle `Without` cherche l'absence sur toute l'ordonnance, y
    /// compris sur la ligne qui la déclenche — et cette ligne porte les
    /// étiquettes libres que l'officine lui a écrites. Un pharmacien
    /// qui note « pyridoxine à associer » sur la fiche du Rimifon,
    /// c'est-à-dire exactement le rappel que la règle existe pour
    /// donner, **éteint la règle** : le mot est là, l'absence n'est
    /// plus constatée, et plus rien ne le dira.
    ///
    /// L'absence se lit donc sur les lignes qui n'ont pas déclenché la
    /// règle. C'est aussi ce que la phrase veut dire : « isoniazide sans
    /// vitamine B6 » parle d'une seconde ligne qui n'y est pas.
    #[test]
    fn what_is_missing_must_be_missing_from_another_line() {
        let rimifon = || treat_full("Rimifon", "isoniazide", "antituberculeux", "");
        // Telle quelle, la règle parle.
        let points = review(&[rimifon()]);
        assert!(
            points
                .iter()
                .any(|p| p.title == "Isoniazide sans vitamine B6"),
            "la règle doit parler sur une ordonnance qui ne porte que l'isoniazide"
        );
        // Une vraie seconde ligne la fait taire, et c'est ce qu'on veut.
        let points = review(&[
            rimifon(),
            treat_full("Bécilan", "pyridoxine", "vitamine B6", ""),
        ]);
        assert!(
            !points
                .iter()
                .any(|p| p.title == "Isoniazide sans vitamine B6"),
            "la vitamine B6 délivrée éteint la règle"
        );
        // Une étiquette écrite sur la fiche de l'isoniazide, non.
        let points = review(&[treat_full(
            "Rimifon",
            "isoniazide",
            "antituberculeux",
            "pyridoxine à associer",
        )]);
        assert!(
            points
                .iter()
                .any(|p| p.title == "Isoniazide sans vitamine B6"),
            "une étiquette sur la ligne qui déclenche n'est pas une délivrance"
        );
    }

    fn treat_full<'a>(name: &'a str, dci: &'a str, class: &'a str, tags: &'a str) -> Treatment<'a> {
        Treatment {
            name,
            dci,
            class,
            tags,
        }
    }

    /// **Ce qu'une ligne nie, ce qu'elle corrige et ce qu'elle ne fait
    /// qu'effleurer ne déclenchent rien** — confronté aux fiches livrées.
    #[test]
    fn a_negation_an_antidote_and_a_local_form_trigger_nothing() {
        let t = |n: &str| {
            let (name, dci, class, _) = crate::db::STARTER_DRUGS
                .iter()
                .find(|(x, ..)| *x == n)
                .unwrap_or_else(|| panic!("fiche absente : {n}"));
            Treatment {
                name,
                dci,
                class,
                tags: "",
            }
        };
        let titles = |names: &[&str]| -> Vec<&'static str> {
            let list: Vec<Treatment> = names.iter().map(|n| t(n)).collect();
            review(&list).iter().map(|p| p.title).collect()
        };
        // Le Relistor accompagne la morphine : il n'en est pas une.
        let got = titles(&["Relistor", "Xanax"]);
        assert!(!got.contains(&"Benzodiazépine + opioïde"), "{got:?}");
        assert!(!titles(&["Relistor"]).contains(&"Opioïde sans laxatif"));
        // L'antidote n'est pas ce qu'il corrige.
        assert!(!titles(&["Vitamine K1", "Previscan"]).contains(&"Deux anticoagulants"));
        // Un bain de bouche n'est pas un second AINS.
        assert!(!titles(&["Tantum", "Advil"]).contains(&"Deux AINS"));
        // Un shampooing ne croise pas la simvastatine ; le gel buccal de
        // miconazole, qui passe dans le sang, si.
        assert!(!titles(&["Kétoderm", "Zocor"]).contains(&"Statine + inhibiteur enzymatique"));
        assert!(titles(&["Daktarin", "Zocor"]).contains(&"Statine + inhibiteur enzymatique"));
        // Et ce qui doit parler parle toujours.
        assert!(titles(&["Previscan", "Advil"]).contains(&"Anticoagulant + AINS"));
        assert!(titles(&["Xanax", "Skenan"]).contains(&"Benzodiazépine + opioïde"));
    }
}
