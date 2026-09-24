//! La messagerie : des conversations entre collègues de l'officine, et
//! avec d'autres officines du réseau.
//!
//! **Deux canaux, deux clés.** Une conversation d'équipe vit dans la base
//! et voyage entre les postes de l'officine avec le reste des dossiers,
//! sous la clé des postes : elle peut nommer un patient, comme une note.
//! Une conversation avec d'autres officines passe par le journal du
//! réseau, **scellée pour ses seuls destinataires** (`bpm_sync::boxed`) :
//! les autres officines du réseau la transportent sans pouvoir la lire.
//! Y joindre un patient demande un consentement explicite, que le journal
//! des accès garde.
//!
//! **Un groupe de collègues** est un nom et des initiales
//! (« Préparateurs » : AM, JB). Une conversation adressée à un groupe en
//! suit la composition : qui y entre voit l'historique, qui en sort ne
//! le voit plus.
//!
//! Pur et testé : qui voit quoi, ce qui reste à lire, comment une
//! conversation se nomme. La base range, l'écran montre.

use std::collections::BTreeSet;

/// Sur quel canal une conversation passe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// Entre collègues, par les postes de l'officine.
    Equipe,
    /// Avec d'autres officines, par le réseau.
    Officines,
}

impl Channel {
    pub fn key(self) -> &'static str {
        match self {
            Channel::Equipe => "equipe",
            Channel::Officines => "officines",
        }
    }

    pub fn from_key(key: &str) -> Channel {
        if key == "officines" {
            Channel::Officines
        } else {
            Channel::Equipe
        }
    }
}

/// Un groupe de collègues.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub id: i64,
    pub name: String,
    /// Des initiales.
    pub members: Vec<String>,
}

/// Une conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversation {
    pub id: i64,
    /// L'identité qui voyage entre officines — la même chez chacune.
    pub uid: String,
    pub channel: Channel,
    pub title: String,
    /// Entre collègues : des initiales ; vide, toute l'équipe.
    pub members: Vec<String>,
    /// Entre collègues : le groupe dont la conversation suit la
    /// composition, quand elle est adressée à un groupe.
    pub group_id: Option<i64>,
    /// Avec d'autres officines : leurs empreintes (appareil, en hex).
    pub peers: Vec<String>,
    pub created_by: String,
    pub created_at: String,
}

/// Un message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub id: i64,
    /// L'identité qui voyage — celle que ses fichiers nomment.
    pub uid: String,
    pub conversation_id: i64,
    /// Initiales entre collègues ; nom de l'officine pour un message reçu
    /// du réseau (`source` non vide).
    pub author: String,
    pub body: String,
    /// Le dossier lié, entre collègues.
    pub patient_id: Option<i64>,
    /// Pour un message venu d'une autre officine : son nom.
    pub source: String,
    pub sent_at: String,
}

/// Un message écrit ici, à sceller pour d'autres officines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outgoing {
    pub uid: String,
    /// L'`uid` de la conversation.
    pub conversation: String,
    pub title: String,
    /// Les officines destinataires (empreintes hex).
    pub peers: Vec<String>,
    /// Les initiales de qui l'a écrit.
    pub author: String,
    pub body: String,
    pub sent_at: String,
}

/// Ce qu'une boîte scellée porte — écrit par l'officine qui envoie, lu
/// par celles qui reçoivent.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Incoming {
    pub uid: String,
    pub conversation: String,
    pub title: String,
    /// Toutes les officines de la conversation, l'expéditrice comprise.
    pub peers: Vec<String>,
    /// Le nom de l'officine qui écrit.
    pub officine: String,
    /// Les initiales de qui a écrit, chez elle.
    pub author: String,
    pub body: String,
    pub sent_at: String,
    /// Les fichiers joints ; leurs morceaux voyagent à part.
    #[serde(default)]
    pub files: Vec<FileMeta>,
}

/// Le préfixe d'une charge scellée dans le journal du réseau : les autres
/// lecteurs, qui attendent du JSON, la laissent passer.
pub const BOX_TAG: &[u8] = b"BOX1";

/// **Un fichier joint voyage en morceaux** de cette taille : écrits en
/// hexadécimal (le JSON des postes ne porte pas d'octets bruts), un
/// morceau tient dans une écriture entre postes (40 000) comme dans une
/// boîte du réseau (48 000).
pub const CHUNK: usize = 12 * 1024;

/// Au plus cette taille par fichier. Chaque morceau reste au journal de
/// chaque poste — et, entre officines, de chaque officine du réseau.
pub const MAX_FILE: usize = 5 * 1024 * 1024;

/// Un fichier joint à un message.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileMeta {
    pub uid: String,
    /// L'`uid` du message qui le porte.
    pub message: String,
    pub name: String,
    pub size: i64,
    /// BLAKE3 du contenu, en hex.
    pub hash: String,
    /// Combien de morceaux.
    pub chunks: i64,
}

/// Un morceau d'un fichier, tel qu'il voyage dans une boîte du réseau.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Chunk {
    pub file: String,
    pub n: i64,
    /// Les octets, en hex.
    pub d: String,
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// L'empreinte d'un contenu.
pub fn hash_of(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Découper un contenu en morceaux hex. Un fichier vide fait un morceau
/// vide : il existe, il ne pèse rien.
pub fn chunks_of(bytes: &[u8]) -> Vec<String> {
    if bytes.is_empty() {
        return vec![String::new()];
    }
    bytes.chunks(CHUNK).map(to_hex).collect()
}

/// Recoller un fichier. `None` tant qu'il manque un morceau, ou quand la
/// taille ou l'empreinte ne sont pas celles annoncées — un fichier faux
/// ne s'enregistre pas.
pub fn assemble(meta: &FileMeta, chunks: &[(i64, String)]) -> Option<Vec<u8>> {
    let mut sorted: Vec<&(i64, String)> = chunks.iter().collect();
    sorted.sort_by_key(|(n, _)| *n);
    sorted.dedup_by_key(|(n, _)| *n);
    if i64::try_from(sorted.len()).ok()? != meta.chunks {
        return None;
    }
    // Une taille annoncée n'alloue rien au-delà du plafond.
    let mut out = Vec::with_capacity(usize::try_from(meta.size).unwrap_or(0).min(MAX_FILE));
    for (i, (n, d)) in sorted.iter().enumerate() {
        if i64::try_from(i).ok()? != *n {
            return None;
        }
        out.extend(from_hex(d)?);
    }
    (i64::try_from(out.len()).ok()? == meta.size && hash_of(&out) == meta.hash).then_some(out)
}

/// Un nom de fichier sans chemin ni caractère qu'un système refuse : ce
/// qu'une autre officine a nommé ne choisit pas où il s'écrit.
pub fn safe_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let clean: String = base
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean
        .trim()
        .trim_start_matches('.')
        .trim_end_matches(['.', ' '])
        .to_owned();
    if clean.is_empty() {
        return "piece-jointe".to_owned();
    }
    // Les noms que Windows réserve, avec ou sans extension.
    let stem = clean.split('.').next().unwrap_or("").to_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        format!("_{clean}")
    } else {
        clean
    }
}

/// Les initiales d'une liste, rangées et sans doublon, sans les vides.
pub fn normalize(members: &[String]) -> Vec<String> {
    let set: BTreeSet<String> = members
        .iter()
        .map(|m| m.trim().to_uppercase())
        .filter(|m| !m.is_empty())
        .collect();
    set.into_iter().collect()
}

/// Les membres d'une conversation d'équipe **aujourd'hui** : ceux de son
/// groupe s'il en a un (et que le groupe existe encore), sinon ceux
/// qu'elle nomme. Vide : toute l'équipe.
pub fn members_of(c: &Conversation, groups: &[Group]) -> Vec<String> {
    if let Some(g) = c.group_id.and_then(|id| groups.iter().find(|g| g.id == id)) {
        return normalize(&g.members);
    }
    normalize(&c.members)
}

/// Est-ce que `operator` voit cette conversation ?
///
/// Une conversation avec d'autres officines se voit de toute l'équipe :
/// c'est l'officine qui parle. Une conversation d'équipe se voit de ses
/// membres, et de tous quand elle s'adresse à toute l'équipe ; son
/// auteur la voit toujours. Sans opérateur choisi, on ne voit que ce qui
/// s'adresse à tous.
pub fn visible_to(c: &Conversation, groups: &[Group], operator: &str) -> bool {
    if c.channel == Channel::Officines {
        return true;
    }
    let members = members_of(c, groups);
    if members.is_empty() {
        return true;
    }
    let me = operator.trim().to_uppercase();
    !me.is_empty() && (members.contains(&me) || c.created_by.trim().to_uppercase() == me)
}

/// Combien de messages restent à lire dans une conversation, pour qui a
/// lu jusqu'à `read_at` (l'heure d'envoi du dernier message vu, ISO).
/// Ses propres messages ne comptent pas.
///
/// **L'heure, pas l'identifiant** : chaque poste tire ses identifiants
/// de son propre bloc, si bien qu'un message écrit après peut porter un
/// numéro plus petit.
pub fn unread(messages: &[Message], conversation_id: i64, read_at: &str, operator: &str) -> usize {
    let me = operator.trim().to_uppercase();
    messages
        .iter()
        .filter(|m| m.conversation_id == conversation_id && m.sent_at.as_str() > read_at)
        .filter(|m| {
            !(m.source.is_empty() && m.author.trim().to_uppercase() == me && !me.is_empty())
        })
        .count()
}

/// Le nom d'une conversation : son titre, ou à défaut ce qu'elle réunit
/// — « Toute l'équipe », le groupe, les initiales, les officines.
pub fn title_of(
    c: &Conversation,
    groups: &[Group],
    officine_name: &dyn Fn(&str) -> String,
    everyone: &str,
) -> String {
    if !c.title.trim().is_empty() {
        return c.title.trim().to_owned();
    }
    match c.channel {
        Channel::Officines => c
            .peers
            .iter()
            .map(|p| officine_name(p))
            .collect::<Vec<_>>()
            .join(", "),
        Channel::Equipe => {
            if let Some(g) = c.group_id.and_then(|id| groups.iter().find(|g| g.id == id)) {
                return g.name.clone();
            }
            let m = normalize(&c.members);
            if m.is_empty() {
                everyone.to_owned()
            } else {
                m.join(", ")
            }
        }
    }
}

/// La conversation d'équipe qui réunit exactement ces membres, si elle
/// existe — écrire à Claire rouvre la conversation avec Claire au lieu
/// d'en ouvrir une deuxième.
pub fn find_direct<'a>(list: &'a [Conversation], members: &[String]) -> Option<&'a Conversation> {
    let want = normalize(members);
    list.iter().find(|c| {
        c.channel == Channel::Equipe
            && c.group_id.is_none()
            && c.title.trim().is_empty()
            && normalize(&c.members) == want
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conv(id: i64, members: &[&str], group: Option<i64>, by: &str) -> Conversation {
        Conversation {
            id,
            uid: format!("u{id}"),
            channel: Channel::Equipe,
            title: String::new(),
            members: members.iter().map(|m| (*m).to_owned()).collect(),
            group_id: group,
            peers: Vec::new(),
            created_by: by.to_owned(),
            created_at: String::new(),
        }
    }

    fn msg(id: i64, conv: i64, author: &str, source: &str) -> Message {
        Message {
            id,
            uid: format!("m{id}"),
            conversation_id: conv,
            author: author.to_owned(),
            body: "x".to_owned(),
            patient_id: None,
            source: source.to_owned(),
            sent_at: format!("2026-09-24 10:0{id}:00"),
        }
    }

    /// **Qui voit quoi** : les membres et l'auteur ; toute l'équipe quand
    /// la conversation s'adresse à tous ; personne d'autre.
    #[test]
    fn a_conversation_is_seen_by_its_members_only() {
        let groups = vec![Group {
            id: 1,
            name: "Préparateurs".into(),
            members: vec!["am".into(), "JB".into()],
        }];
        let team = conv(1, &[], None, "CL");
        let pair = conv(2, &["CL", "YS"], None, "CL");
        let grp = conv(3, &[], Some(1), "CL");
        assert!(visible_to(&team, &groups, "MB"));
        assert!(
            visible_to(&team, &groups, ""),
            "à tous, même sans opérateur"
        );
        assert!(visible_to(&pair, &groups, "ys"));
        assert!(!visible_to(&pair, &groups, "MB"));
        assert!(!visible_to(&pair, &groups, ""));
        // Le groupe décide, pas la liste figée.
        assert!(visible_to(&grp, &groups, "AM"));
        assert!(visible_to(&grp, &groups, "CL"), "l'auteur la voit");
        assert!(!visible_to(&grp, &groups, "YS"));
        // Un groupe supprimé : la conversation revient à sa liste (vide :
        // toute l'équipe).
        assert!(visible_to(&grp, &[], "YS"));
        let mut net = conv(4, &["CL"], None, "CL");
        net.channel = Channel::Officines;
        assert!(visible_to(&net, &groups, "MB"), "l'officine parle");
    }

    #[test]
    fn unread_skips_one_s_own_messages() {
        let list = vec![
            msg(1, 7, "CL", ""),
            msg(2, 7, "YS", ""),
            msg(3, 7, "CL", ""),
            msg(4, 8, "YS", ""),
            // Une officine qui signe « CL » n'est pas moi.
            msg(5, 7, "CL", "Pharmacie du Port"),
        ];
        assert_eq!(unread(&list, 7, "", "CL"), 2);
        assert_eq!(unread(&list, 7, "2026-09-24 10:02:00", "CL"), 1);
        assert_eq!(unread(&list, 7, "", "YS"), 3);
        assert_eq!(unread(&list, 7, "", ""), 4);
        // Un numéro plus petit écrit plus tard, sur un autre poste, reste
        // à lire.
        let mut late = msg(0, 7, "YS", "");
        late.sent_at = "2026-09-24 11:00:00".into();
        assert_eq!(unread(&[late], 7, "2026-09-24 10:05:00", "CL"), 1);
    }

    #[test]
    fn a_conversation_is_named_by_what_it_gathers() {
        let groups = vec![Group {
            id: 1,
            name: "Préparateurs".into(),
            members: vec![],
        }];
        let name = |p: &str| format!("Officine {p}");
        assert_eq!(
            title_of(&conv(1, &[], None, "CL"), &groups, &name, "Toute l'équipe"),
            "Toute l'équipe"
        );
        assert_eq!(
            title_of(&conv(2, &["ys", "CL"], None, "CL"), &groups, &name, "-"),
            "CL, YS"
        );
        assert_eq!(
            title_of(&conv(3, &[], Some(1), "CL"), &groups, &name, "-"),
            "Préparateurs"
        );
        let mut net = conv(4, &[], None, "CL");
        net.channel = Channel::Officines;
        net.peers = vec!["ab".into(), "cd".into()];
        assert_eq!(
            title_of(&net, &groups, &name, "-"),
            "Officine ab, Officine cd"
        );
        let mut titled = conv(5, &["CL"], None, "CL");
        titled.title = " Garde du dimanche ".into();
        assert_eq!(title_of(&titled, &groups, &name, "-"), "Garde du dimanche");
    }

    /// **Un fichier se découpe, voyage et se recolle à l'identique** — et
    /// ne se recolle pas s'il manque un morceau ou si un octet a changé.
    #[test]
    fn a_file_is_chunked_and_reassembled_only_when_whole() {
        let bytes: Vec<u8> = (0..(CHUNK * 2 + 100)).map(|i| (i % 251) as u8).collect();
        let parts = chunks_of(&bytes);
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.len() <= CHUNK * 2));
        let meta = FileMeta {
            uid: "f".into(),
            message: "m".into(),
            name: "ordonnance.pdf".into(),
            size: bytes.len() as i64,
            hash: hash_of(&bytes),
            chunks: 3,
        };
        let numbered: Vec<(i64, String)> = parts
            .iter()
            .enumerate()
            .map(|(i, p)| (i as i64, p.clone()))
            .rev()
            .collect();
        assert_eq!(
            assemble(&meta, &numbered).as_deref(),
            Some(bytes.as_slice())
        );
        assert!(
            assemble(&meta, &numbered[..2]).is_none(),
            "un morceau manque"
        );
        let mut bad = numbered.clone();
        bad[0].1.replace_range(0..2, "ff");
        assert!(assemble(&meta, &bad).is_none(), "un octet a changé");
        // Vide : un morceau vide, recollé en rien.
        let empty = FileMeta {
            size: 0,
            hash: hash_of(&[]),
            chunks: 1,
            ..meta
        };
        assert_eq!(
            assemble(&empty, &[(0, chunks_of(&[])[0].clone())]),
            Some(vec![])
        );
    }

    #[test]
    fn a_received_name_cannot_choose_where_it_is_written() {
        assert_eq!(safe_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_name("C:\\Windows\\a.pdf"), "a.pdf");
        assert_eq!(safe_name(".bashrc"), "bashrc");
        assert_eq!(safe_name("a<b>.txt"), "a_b_.txt");
        assert_eq!(safe_name(""), "piece-jointe");
        assert_eq!(safe_name("CON.txt"), "_CON.txt");
        assert_eq!(safe_name("nul"), "_nul");
        assert_eq!(safe_name("com1.pdf"), "_com1.pdf");
        assert_eq!(safe_name("compte.pdf"), "compte.pdf");
        assert_eq!(safe_name("fin. "), "fin");
        assert_eq!(safe_name("Ordonnance Dupont.pdf"), "Ordonnance Dupont.pdf");
    }

    #[test]
    fn writing_to_the_same_people_reopens_their_conversation() {
        let list = vec![
            conv(1, &["CL", "YS"], None, "CL"),
            conv(2, &["CL", "YS"], Some(3), "CL"),
        ];
        assert_eq!(
            find_direct(&list, &["ys".into(), "CL".into()]).map(|c| c.id),
            Some(1)
        );
        assert!(find_direct(&list, &["MB".into()]).is_none());
        assert_eq!(
            Channel::from_key(Channel::Officines.key()),
            Channel::Officines
        );
        assert_eq!(Channel::from_key("?"), Channel::Equipe);
    }
}
