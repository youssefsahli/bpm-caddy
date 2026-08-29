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
    /// Every group matches, **and** nothing on the ordonnance carries
    /// any of the last words.
    ///
    /// What is *missing* is half of what a bilan finds. An opioid with
    /// no laxative beside it, a corticothérapie with nothing for the
    /// bone: neither is an interaction, and both are the reason the
    /// patient comes back. The point names the treatments that are
    /// there — naming an absence is impossible, and the sentence says
    /// what is not.
    Without(&'static [&'static [&'static str]], &'static [&'static str]),
}

struct Rule {
    kind: Kind,
    severity: Severity,
    title: &'static str,
    detail: &'static str,
}

/// Read an ordonnance against itself. Loudest first; inside one
/// severity, the order of the rules — which is the order a pharmacist
/// checks them in.
pub fn review(treatments: &[Treatment]) -> Vec<Point> {
    let folded: Vec<(String, String)> = treatments
        .iter()
        .map(|t| (t.name.trim().to_owned(), t.haystack()))
        .collect();
    let matches = |words: &[&str]| -> Vec<String> {
        folded
            .iter()
            .filter(|(_, hay)| words.iter().any(|w| crate::fuzzy::contains_folded(hay, w)))
            .map(|(name, _)| name.clone())
            .collect()
    };
    let mut out = Vec::new();
    for rule in RULES {
        let drugs = match &rule.kind {
            Kind::Combination(groups) => {
                let mut named: Vec<String> = Vec::new();
                let mut complete = true;
                for group in groups.iter() {
                    let hit = matches(group);
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
            Kind::Duplicate(words, min) => {
                let hit = matches(words);
                if hit.len() >= *min {
                    hit
                } else {
                    Vec::new()
                }
            }
            Kind::Without(groups, absent) => {
                if !matches(absent).is_empty() {
                    Vec::new()
                } else {
                    let mut named: Vec<String> = Vec::new();
                    let mut complete = true;
                    for group in groups.iter() {
                        let hit = matches(group);
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

/// The classic readings of a French ordonnance, in the order they are
/// checked. The words are matched inside a card's name, DCI, class and
/// tags, accent- and case-insensitively.
const RULES: &[Rule] = &[
    Rule {
        kind: Kind::Combination(&[
            &["IEC", "sartan", "ARA II"],
            &["diurétique", "furosémide", "hydrochlorothiazide", "indapamide"],
            &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène", "coxib"],
        ]),
        severity: Severity::Alert,
        title: "Triade néfaste",
        detail: "Bloqueur du système rénine-angiotensine, diurétique et AINS ensemble : c'est l'association qui fait l'insuffisance rénale aiguë, d'autant plus vite qu'il fait chaud ou que le patient se déshydrate. L'AINS est celui des trois qui se retire.",
    },
    Rule {
        kind: Kind::Combination(&[&["IEC"], &["sartan", "ARA II"]]),
        severity: Severity::Alert,
        title: "Double blocage",
        detail: "IEC et sartan ensemble : le double blocage du système rénine-angiotensine n'apporte rien et multiplie l'insuffisance rénale et l'hyperkaliémie. À signaler au prescripteur.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["anticoagulant", "AOD", "AVK", "héparine", "apixaban", "rivaroxaban", "dabigatran", "édoxaban", "warfarine", "fluindione"],
            &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène", "coxib"],
        ]),
        severity: Severity::Alert,
        title: "Anticoagulant + AINS",
        detail: "Le risque hémorragique digestif est multiplié, et ce n'est pas une question de dose : l'AINS se remplace par du paracétamol ou un topique, jamais délivré en conseil.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["benzodiazépine", "zolpidem", "zopiclone", "hypnotique"],
            &["opioïde", "morphine", "oxycodone", "tramadol", "codéine", "fentanyl"],
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
        detail: "Un anticholinestérasique et un anticholinergique s'annulent : l'un est donné pour la mémoire, l'autre l'aggrave. C'est le critère STOPP le plus souvent retrouvé sur une ordonnance de sujet âgé.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["lithium", "thymorégulateur"],
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
            &["statine", "atorvastatine", "simvastatine", "rosuvastatine", "pravastatine"],
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
        detail: "L'oméprazole et l'ésoméprazole inhibent le CYP2C19 qui active le clopidogrel. Le pantoprazole ou le rabéprazole font le même travail sans cette réserve.",
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
        detail: "Agitation, sueurs, tremblement, fièvre et diarrhée dans les heures qui suivent une introduction : c'est le syndrome sérotoninergique. Le tramadol et les triptans comptent, le millepertuis aussi.",
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
            &["macrolide", "clarithromycine", "érythromycine", "statine", "vérapamil", "antifongique azolé", "ciclosporine"],
        ]),
        severity: Severity::Alert,
        title: "Colchicine exposée",
        detail: "La colchicine a une marge très étroite et un antidote inexistant : macrolides, azolés, vérapamil et ciclosporine font grimper ses concentrations. Diarrhée précoce sous colchicine : arrêt immédiat, c'est le premier signe de surdosage.",
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
            &["AOD", "AVK", "contraception", "immunosuppresseur", "anticancéreux", "antirétroviral", "statine"],
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
        detail: "Confusion, chutes, rétention urinaire, constipation et sécheresse : les effets s'additionnent d'une molécule à l'autre. C'est l'ordonnance entière qu'il faut compter, pas chaque ligne.",
    },
    Rule {
        kind: Kind::Duplicate(
            &["benzodiazépine", "zolpidem", "zopiclone", "hypnotique", "opioïde", "antipsychotique", "antihistaminique H1 sédatif"],
            3,
        ),
        severity: Severity::Warn,
        title: "Trois sédatifs",
        detail: "Trois molécules sédatives ou plus : chutes et confusion, surtout après 75 ans. Chacune est justifiable, l'addition ne l'est pas — on hiérarchise et on retire dans l'ordre.",
    },
    Rule {
        kind: Kind::Duplicate(&["IPP", "oméprazole", "ésoméprazole", "pantoprazole", "lansoprazole", "rabéprazole"], 2),
        severity: Severity::Warn,
        title: "Deux IPP",
        detail: "Deux inhibiteurs de la pompe à protons : le plus souvent un reliquat d'ordonnance hospitalière. Un seul suffit, et son indication se réévalue.",
    },
    Rule {
        kind: Kind::Duplicate(&["benzodiazépine", "zolpidem", "zopiclone"], 2),
        severity: Severity::Warn,
        title: "Deux benzodiazépines",
        detail: "Deux benzodiazépines ou apparentés ensemble : rien n'est gagné en efficacité, tout l'est en dépendance et en chutes. Le relais vers une seule molécule se prépare.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["inhibiteur calcique", "amlodipine", "nifédipine", "félodipine", "lercanidipine"],
            &["diurétique", "furosémide", "hydrochlorothiazide", "indapamide"],
        ]),
        severity: Severity::Info,
        title: "Œdème et diurétique",
        detail: "L'œdème des chevilles des dihydropyridines n'est pas une rétention d'eau : un diurétique ne le corrige pas. Si le diurétique a été ajouté pour cela, c'est une cascade — la baisse de dose ou le changement de classe est la réponse.",
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
            &["lévothyroxine", "hormone thyroïdienne"],
            &["fer", "calcium", "IPP", "oméprazole", "pantoprazole", "magnésium"],
        ]),
        severity: Severity::Info,
        title: "Lévothyroxine à distance",
        detail: "Fer, calcium, magnésium et IPP réduisent l'absorption de la lévothyroxine. Deux heures d'écart au moins, et la TSH se contrôle après tout changement de rythme.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["biphosphonate", "bisphosphonate", "alendronate", "risédronate", "ibandronate"],
            &["calcium", "fer", "magnésium", "IPP"],
        ]),
        severity: Severity::Info,
        title: "Bisphosphonate à distance",
        detail: "Le calcium et les cations divalents annulent l'absorption du bisphosphonate : la prise se fait à jeun, seule, et le calcium au moins deux heures plus tard.",
    },
    Rule {
        kind: Kind::Duplicate(&["paracétamol"], 2),
        severity: Severity::Alert,
        title: "Deux sources de paracétamol",
        detail: "Deux spécialités contenant du paracétamol sur la même ordonnance : c'est ainsi que se fait le surdosage, sans que personne l'ait voulu, parce que l'un des deux noms ne dit pas ce qu'il contient. Faire la somme des grammes par jour devant le patient, et ne garder qu'une source.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["statine", "simvastatine", "atorvastatine", "rosuvastatine", "pravastatine"],
            &["macrolide", "clarithromycine", "érythromycine", "azolé", "kétoconazole", "itraconazole", "fluconazole", "vérapamil", "diltiazem"],
        ]),
        severity: Severity::Alert,
        title: "Statine + inhibiteur enzymatique",
        detail: "Macrolide, azolé ou inhibiteur calcique bradycardisant : la concentration de la statine grimpe et c'est la rhabdomyolyse. Pour une antibiothérapie courte, la statine se suspend le temps du traitement — l'arrêt de quelques jours ne coûte rien, l'association coûte un muscle.",
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
        detail: "L'allopurinol bloque la voie qui dégrade l'azathioprine : l'exposition est multipliée et l'aplasie médullaire est le risque, pas une éventualité théorique. L'association se refuse en délivrance et se discute avec le prescripteur ; si elle est maintenue, la dose d'azathioprine est divisée par quatre et la NFS surveillée.",
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
        detail: "Metformine, diurétique et bloqueur du système rénine-angiotensine : le rein tient tant que le patient boit. Fièvre, diarrhée, vomissements ou forte chaleur, et les trois se suspendent le temps de l'épisode — c'est la règle des jours de maladie, et elle s'explique une fois pour toutes, par écrit.",
    },
    Rule {
        kind: Kind::Duplicate(
            &["antiagrégant", "clopidogrel", "ticagrélor", "prasugrel", "acide acétylsalicylique", "aspirine"],
            2,
        ),
        severity: Severity::Warn,
        title: "Double antiagrégation",
        detail: "Deux antiagrégants : après un stent c'est le traitement, mais il a une durée — souvent six à douze mois — au terme de laquelle il n'en reste qu'un. Chercher la date de pose sur le dossier, et si elle est ancienne, poser la question au prescripteur.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["bêtabloquant", "bêta-bloquant", "bisoprolol", "aténolol", "métoprolol", "propranolol", "nébivolol"],
            &["insuline", "sulfamide hypoglycémiant", "glinide", "gliclazide", "glimépiride", "répaglinide"],
        ]),
        severity: Severity::Warn,
        title: "Bêtabloquant + hypoglycémiant",
        detail: "Le bêtabloquant masque les signes de l'hypoglycémie — tremblements, palpitations — et n'en laisse que les sueurs. Le patient doit le savoir : la sueur seule devient le signal, et la glycémie se contrôle au moindre doute plutôt que de se fier aux sensations.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["AINS", "ibuprofène", "diclofénac", "kétoprofène", "naproxène", "coxib"],
            &["antihypertenseur", "IEC", "sartan", "ARA II", "bêtabloquant", "diurétique", "inhibiteur calcique"],
        ]),
        severity: Severity::Warn,
        title: "AINS et tension",
        detail: "Un AINS fait remonter la tension et annule une partie de l'effet du traitement, y compris pris quelques jours en automédication. Une tension qui se dérègle sans raison se cherche d'abord dans l'armoire à pharmacie du patient.",
    },
    Rule {
        kind: Kind::Without(
            &[&[
                "opioïde",
                "morphine",
                "oxycodone",
                "tramadol",
                "codéine",
                "fentanyl",
                "buprénorphine",
            ]],
            &["laxatif", "macrogol", "lactulose", "bisacodyl", "sterculia", "docusate"],
        ),
        severity: Severity::Warn,
        title: "Opioïde sans laxatif",
        detail: "La constipation sous opioïde est constante, ne s'épuise pas avec le temps et se prévient dès la première prise. Aucun laxatif sur cette ordonnance : le proposer maintenant coûte une phrase, l'occlusion coûte une hospitalisation.",
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
            &[
                "vitamine D",
                "cholécalciférol",
                "calcium",
                "bisphosphonate",
                "biphosphonate",
                "alendronate",
                "risédronate",
                "acide zolédronique",
                "dénosumab",
            ],
        ),
        severity: Severity::Warn,
        title: "Corticoïde sans protection osseuse",
        detail: "Une corticothérapie orale prolongée fait perdre de l'os dès les premiers mois, et rien sur cette ordonnance ne s'y oppose. Si la cure dépasse trois mois, la question du calcium, de la vitamine D et d'un bisphosphonate se pose au prescripteur — s'il s'agit d'une cure courte, il n'y a rien à faire et cette ligne se referme.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["méthotrexate"],
            &["cotrimoxazole", "triméthoprime", "sulfaméthoxazole", "bactrim"],
        ]),
        severity: Severity::Alert,
        title: "Méthotrexate + cotrimoxazole",
        detail: "Deux antifoliques ensemble : l'aplasie médullaire est le risque, et elle survient même sous méthotrexate hebdomadaire à faible dose. L'association est à proscrire — appeler le prescripteur pour un autre antibiotique avant de délivrer.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["lévothyroxine", "hormone thyroïdienne"],
            &["fer", "calcium", "IPP", "oméprazole", "pantoprazole", "ésoméprazole", "lansoprazole", "colestyramine"],
        ]),
        severity: Severity::Warn,
        title: "Lévothyroxine et chélation",
        detail: "Fer, calcium, IPP et résines abaissent l'absorption de la lévothyroxine, et une TSH qui dérive vient plus souvent de là que de la dose. Deux heures d'écart au moins, à heure fixe, et la TSH se recontrôle six semaines après tout changement.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["digoxine", "digitalique"],
            &["amiodarone", "vérapamil", "diltiazem", "macrolide", "clarithromycine", "itraconazole"],
        ]),
        severity: Severity::Alert,
        title: "Digoxine potentialisée",
        detail: "Ces molécules font grimper la digoxinémie, parfois du double : nausées, vision jaune, pouls lent ou irrégulier et confusion en sont les premiers signes. La dose de digoxine se réduit à l'introduction et la digoxinémie se contrôle — cela ne s'improvise pas au comptoir.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["ISRS", "IRSNa", "fluoxétine", "paroxétine", "sertraline", "citalopram", "escitalopram", "venlafaxine", "duloxétine"],
            &["tramadol", "triptan", "sumatriptan", "linézolide", "millepertuis", "lithium"],
        ]),
        severity: Severity::Alert,
        title: "Risque de syndrome sérotoninergique",
        detail: "Agitation, tremblements, sueurs, diarrhée, fièvre et rigidité dans les heures qui suivent l'ajout : c'est un syndrome sérotoninergique, et il peut être grave. Le tramadol est le plus souvent en cause parce qu'il passe pour un simple antalgique. Signaler avant de délivrer.",
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
        detail: "Le risque de rupture du tendon d'Achille est multiplié, et il est le plus élevé après 60 ans. Toute douleur tendineuse fait arrêter la quinolone et cesser tout appui sur le tendon — la rupture survient souvent sans effort particulier, parfois après l'arrêt du traitement.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["statine", "simvastatine", "atorvastatine", "rosuvastatine", "pravastatine"],
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
        ),
        severity: Severity::Warn,
        title: "Corticoïde inhalé sans traitement de crise",
        detail: "Un traitement de fond de l'asthme sans bronchodilatateur de secours sur l'ordonnance : soit le patient en a un chez lui et il faut vérifier sa date de péremption et sa technique, soit il n'en a pas, et c'est la crise qui le découvrira. La question se pose maintenant.",
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
        detail: "Un collyre bêtabloquant du glaucome passe dans la circulation par la muqueuse nasale et échappe au premier passage hépatique : il s'ajoute au bêtabloquant oral et la bradycardie, l'asthénie ou le bronchospasme qui suivent ne sont attribués ni à l'un ni à l'autre. Occlure le point lacrymal une minute après l'instillation divise ce passage. Un asthme est une contre-indication qui vaut aussi pour la goutte dans l'œil.",
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
        detail: "L'inducteur enzymatique accélère la dégradation des hormones et fait échouer la contraception, pilule comme implant et anneau — le stérilet au cuivre et le dispositif au lévonorgestrel sont les deux méthodes qui n'en dépendent pas. L'échec est silencieux jusqu'au test de grossesse. Cela vaut pendant tout le traitement et encore quatre semaines après son arrêt, et le millepertuis compte, même acheté sans ordonnance.",
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
            ],
        ]),
        severity: Severity::Warn,
        title: "AOD potentialisé",
        detail: "Ces molécules inhibent la P-glycoprotéine, et pour certaines le CYP3A4 : l'exposition à l'anticoagulant direct monte sans que rien ne se voie, puisqu'il n'y a pas d'INR pour le dire. Selon la molécule et l'association, la dose se réduit ou l'association se contre-indique — la dronédarone et le kétoconazole sont contre-indiqués avec le dabigatran. Vérifier la dose prescrite contre l'âge, le poids et la clairance, et signaler tout saignement.",
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
        detail: "Une corticothérapie fait monter la glycémie dès les premiers jours, surtout en fin de journée avec une prise matinale, et un diabète équilibré ne l'est plus. Prévenir le patient d'augmenter l'autosurveillance pendant la cure et de ne pas s'inquiéter d'une baisse à l'arrêt : c'est l'adaptation qui suit la corticothérapie, et elle se fait avec le prescripteur. Chez un patient non diabétique connu, une cure prolongée justifie de vérifier la glycémie.",
    },
    Rule {
        kind: Kind::Duplicate(&["corticoïde", "cortico"], 3),
        severity: Severity::Info,
        title: "Charge corticoïde cumulée",
        detail: "Trois corticoïdes ou plus sur une même ordonnance — inhalé, nasal, cutané, collyre, oral — s'additionnent : chacun pris isolément est faible, la somme ne l'est pas. La freination surrénalienne, la fragilité cutanée, la cataracte et l'ostéoporose se jugent sur le total et non sur une ligne. Vérifier que chacun garde une indication actuelle et une durée, en particulier le dermocorticoïde renouvelé sans limite.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["warfarine", "fluindione", "acénocoumarol"],
            &["amiodarone"],
        ]),
        severity: Severity::Alert,
        title: "AVK + amiodarone",
        detail: "L'amiodarone augmente fortement l'effet de l'antivitamine K, et elle le fait lentement : l'INR s'emballe au bout d'une à trois semaines, longtemps après l'introduction, puis reste perturbé des mois après l'arrêt tant la molécule est stockée. Un INR est nécessaire dans la semaine qui suit l'introduction, puis rapproché, et la dose d'AVK se réduit le plus souvent d'un tiers.",
    },
    Rule {
        kind: Kind::Combination(&[
            &["hydrochlorothiazide", "indapamide", "thiazidique"],
            &["calcium"],
            &["vitamine d", "cholécalciférol", "calcifédiol"],
        ]),
        severity: Severity::Warn,
        title: "Calcémie cumulée",
        detail: "Le thiazidique réduit l'élimination urinaire du calcium pendant que la supplémentation en apporte : l'hypercalcémie s'installe lentement et se manifeste par de la soif, des urines abondantes, une constipation, des nausées et une confusion — signes qu'on met volontiers sur le compte de l'âge. Une calcémie suffit à trancher, et la supplémentation se réévalue : elle est souvent prescrite pour une durée que personne n'a rediscutée.",
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
            t("Tahor", "atorvastatine", "statine"),
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
                Kind::Without(groups, absent) => {
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

    /// What is *missing* only counts as a finding when the thing that
    /// should be there is not: the same ordonnance with a laxative on
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

    #[test]
    fn every_rule_says_what_to_do_about_it() {
        // The catalogue only ever grows: a rule removed is a reading
        // nobody does any more.
        assert!(
            RULES.len() >= 55,
            "{} règles de revue, il y en avait cinquante-cinq",
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
}
