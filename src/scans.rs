//! Les pièces numérisées : l'ordonnance, la déclaration d'accident du
//! travail, le courrier du spécialiste, le compte rendu de biologie.
//!
//! Une officine reçoit du papier toute la journée et le range dans un
//! classeur qui n'est pas à côté du dossier. Ce module est la moitié
//! *pure* de ce qui permet de l'y mettre : ce qu'un fichier est
//! réellement, ce qu'une pièce peut être, et comment on demande au
//! scanner de l'officine de produire le fichier.
//!
//! Trois décisions, et elles tiennent tout :
//!
//! * **Le type d'un fichier se lit dans ses octets, jamais dans son
//!   nom.** L'application rouvre ensuite la pièce en la confiant au
//!   système ; accepter un fichier parce qu'il s'appelle `.pdf` serait
//!   accepter de rendre au système, plus tard, ce qu'on lui a donné sans
//!   le regarder. Quatre formats entrent — PDF, PNG, JPEG, TIFF — et le
//!   reste est refusé à l'entrée, où le refus se comprend.
//! * **La commande du scanner est en configuration, pas dans le code.**
//!   Comme la séquence APDU de la carte Vitale : `scanimage` sur un
//!   poste Linux, autre chose ailleurs, et l'officine sait quel est son
//!   matériel mieux que ce binaire. Vide par défaut — sans commande, il
//!   n'y a qu'« Importer… », qui suffit puisque tous les logiciels de
//!   scanner savent écrire un PDF quelque part.
//! * **Les pièces vivent dans la base**, chiffrée, et non dans un
//!   dossier à côté. Une ordonnance numérisée posée en clair à côté
//!   d'une base chiffrée annule le chiffrement de la base. C'est la
//!   raison pour laquelle il y a une taille maximale.
//!
//! Pur et testé, comme `revue` et `ordonnancier`. Rien ici ne lit un
//! disque : ce module reçoit des octets et rend un verdict.

/// Ce qu'une pièce est, du point de vue du dossier.
///
/// Pas une étiquette libre : la liste se groupe dessus et se filtre
/// dessus, et un genre inventé une fois resterait seul pour toujours.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DocKind {
    Ordonnance,
    /// Accident du travail ou maladie professionnelle : la feuille
    /// AT/MP, qui n'est ni une ordonnance ni un courrier et se retrouve
    /// toujours au mauvais endroit.
    AtMp,
    Biologie,
    Courrier,
    Facture,
    Autre,
}

impl DocKind {
    /// Sa clé dans la base, stable : une pièce se relit dans dix ans.
    pub fn as_key(self) -> &'static str {
        match self {
            DocKind::Ordonnance => "ORDONNANCE",
            DocKind::AtMp => "AT_MP",
            DocKind::Biologie => "BIOLOGIE",
            DocKind::Courrier => "COURRIER",
            DocKind::Facture => "FACTURE",
            DocKind::Autre => "AUTRE",
        }
    }

    /// Une clé inconnue est « Autre » : une pièce écrite par une version
    /// plus récente reste visible et rangeable, plutôt que de disparaître
    /// d'une liste où quelqu'un l'a mise exprès.
    pub fn from_key(key: &str) -> DocKind {
        match key {
            "ORDONNANCE" => DocKind::Ordonnance,
            "AT_MP" => DocKind::AtMp,
            "BIOLOGIE" => DocKind::Biologie,
            "COURRIER" => DocKind::Courrier,
            "FACTURE" => DocKind::Facture,
            _ => DocKind::Autre,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            DocKind::Ordonnance => "scan_kind_ordonnance",
            DocKind::AtMp => "scan_kind_atmp",
            DocKind::Biologie => "scan_kind_biologie",
            DocKind::Courrier => "scan_kind_courrier",
            DocKind::Facture => "scan_kind_facture",
            DocKind::Autre => "scan_kind_autre",
        }
    }

    /// Sa couleur dans la palette de données, fixe par genre.
    pub fn series(self) -> usize {
        match self {
            DocKind::Ordonnance => 0,
            DocKind::AtMp => 3,
            DocKind::Biologie => 2,
            DocKind::Courrier => 5,
            DocKind::Facture => 4,
            DocKind::Autre => 1,
        }
    }

    pub const ALL: [DocKind; 6] = [
        DocKind::Ordonnance,
        DocKind::AtMp,
        DocKind::Biologie,
        DocKind::Courrier,
        DocKind::Facture,
        DocKind::Autre,
    ];
}

/// À quoi une pièce est attachée.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Subject {
    /// Un dossier patient.
    Patient,
    /// Une fiche médicament : une notice, un courrier de retrait de lot.
    Drug,
    /// L'officine elle-même : le registre AT, une facture de grossiste,
    /// un document qui n'appartient à personne en particulier.
    Officine,
}

impl Subject {
    pub fn as_key(self) -> &'static str {
        match self {
            Subject::Patient => "PATIENT",
            Subject::Drug => "DRUG",
            Subject::Officine => "OFFICINE",
        }
    }
}

/// Un format de pièce que l'application accepte de garder et de rendre.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Media {
    Pdf,
    Png,
    Jpeg,
    Tiff,
}

impl Media {
    pub fn as_key(self) -> &'static str {
        match self {
            Media::Pdf => "application/pdf",
            Media::Png => "image/png",
            Media::Jpeg => "image/jpeg",
            Media::Tiff => "image/tiff",
        }
    }

    /// L'extension sous laquelle la pièce est ressortie pour être
    /// ouverte : c'est elle qui décide de l'application que le système
    /// lance, et elle vient du **contenu** et jamais du nom d'origine.
    pub fn extension(self) -> &'static str {
        match self {
            Media::Pdf => "pdf",
            Media::Png => "png",
            Media::Jpeg => "jpg",
            Media::Tiff => "tiff",
        }
    }

    pub fn from_key(key: &str) -> Option<Media> {
        [Media::Pdf, Media::Png, Media::Jpeg, Media::Tiff]
            .into_iter()
            .find(|m| m.as_key() == key)
    }
}

/// Ce qu'un fichier **est**, lu dans ses premiers octets.
///
/// Jamais dans son extension. L'application garde la pièce puis la
/// ressort sur un disque et la confie au système ; croire un `.pdf` sur
/// parole, c'est accepter de rendre plus tard au système un fichier
/// qu'on n'a jamais regardé. Les quatre nombres magiques ci-dessous
/// tiennent sur une ligne chacun et coupent court à la question.
///
/// `None` n'est pas une erreur de lecture : c'est un fichier que cette
/// application ne saura pas rouvrir, et il est refusé à l'entrée, là où
/// le refus se comprend et se corrige.
pub fn sniff(bytes: &[u8]) -> Option<Media> {
    if bytes.starts_with(b"%PDF-") {
        return Some(Media::Pdf);
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(Media::Png);
    }
    // JPEG : SOI, puis un marqueur. Les deux premiers octets suffisent
    // en pratique, le troisième écarte les deux octets isolés.
    if bytes.len() >= 3 && bytes.starts_with(&[0xFF, 0xD8]) && bytes[2] == 0xFF {
        return Some(Media::Jpeg);
    }
    // TIFF, dans les deux boutismes — un scanner à plat en produit
    // encore, et « II* » comme « MM\0* » sont l'en-tête.
    if bytes.starts_with(b"II\x2A\x00") || bytes.starts_with(b"MM\x00\x2A") {
        return Some(Media::Tiff);
    }
    None
}

/// Pourquoi une pièce est refusée.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// Vide : un scanner qui a échoué écrit un fichier de zéro octet, et
    /// le garder ferait croire que la pièce est au dossier.
    Empty,
    /// Un format que l'application ne saura pas rouvrir.
    Unknown,
    /// Plus grand que ce que l'officine s'autorise.
    TooBig,
}

impl Refusal {
    pub fn label_key(self) -> &'static str {
        match self {
            Refusal::Empty => "scan_err_empty",
            Refusal::Unknown => "scan_err_unknown",
            Refusal::TooBig => "scan_err_too_big",
        }
    }
}

/// Ces octets peuvent-ils entrer dans la base, et comme quoi ?
///
/// `max_mb` est le plafond que l'officine se donne (`[scans] max_mb`).
/// Il existe parce que les pièces vivent **dans** la base : une base
/// chiffrée qu'on sauvegarde tous les jours et qu'on recopie sur un
/// partage n'est pas l'endroit d'un TIFF de deux cents mégaoctets, et
/// une numérisation à 600 ppp en produit un sans prévenir.
pub fn accept(bytes: &[u8], max_mb: u32) -> Result<Media, Refusal> {
    if bytes.is_empty() {
        return Err(Refusal::Empty);
    }
    let media = sniff(bytes).ok_or(Refusal::Unknown)?;
    // Le plafond est comparé avant tout le reste sauf le format, pour
    // que le message dise « trop gros » et non « format inconnu » sur un
    // PDF de trois cents mégaoctets.
    if bytes.len() as u64 > u64::from(max_mb) * 1024 * 1024 {
        return Err(Refusal::TooBig);
    }
    Ok(media)
}

/// Une taille en octets, écrite comme un être humain la lit.
pub fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} o");
    }
    let kb = bytes as f64 / 1024.0;
    if kb < 1024.0 {
        return format!("{kb:.0} Ko");
    }
    format!("{:.1} Mo", kb / 1024.0)
}

/// La commande du scanner, avec le chemin de sortie mis à sa place.
///
/// `{out}` est remplacé par le fichier que l'application ira lire. Une
/// commande qui ne le nomme pas écrirait quelque part que l'application
/// ne connaît pas, et rien n'entrerait au dossier : c'est refusé, avec
/// une phrase qui le dit, plutôt que d'échouer en silence.
///
/// La commande est **découpée d'abord et remplie ensuite**, et l'ordre
/// est ce qui compte. Rempli d'abord, un chemin de sortie contenant une
/// espace — « C:\Users\Jean Martin\… », qui est la moitié des postes —
/// se retrouverait coupé en deux arguments, et le scanner écrirait dans
/// un fichier nommé « C:\Users\Jean ». `{out}` est un emplacement que
/// l'application remplit, pas du texte que le shell relit : il vaut un
/// argument, avec ou sans guillemets autour.
pub fn scanner_command(template: &str, out: &std::path::Path) -> Result<Vec<String>, String> {
    let template = template.trim();
    if template.is_empty() {
        return Err(crate::strings::tr("scan_err_no_command").to_owned());
    }
    if !template.contains("{out}") {
        return Err(crate::strings::tr("scan_err_no_out").to_owned());
    }
    let path = out.display().to_string();
    let parts: Vec<String> = split_args(template)
        .into_iter()
        .map(|a| a.replace("{out}", &path))
        .collect();
    if parts.is_empty() {
        return Err(crate::strings::tr("scan_err_no_command").to_owned());
    }
    Ok(parts)
}

/// Découper une ligne de commande en arguments, guillemets compris.
fn split_args(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut any = false;
    for c in line.chars() {
        match quote {
            Some(q) if c == q => {
                quote = None;
            }
            Some(_) => cur.push(c),
            None if c == '\'' || c == '"' => {
                quote = Some(c);
                // Des guillemets vides sont un argument vide et non
                // l'absence d'argument : `--nom ""` a deux arguments.
                any = true;
            }
            None if c.is_whitespace() => {
                if !cur.is_empty() || any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            None => cur.push(c),
        }
    }
    if !cur.is_empty() || any {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le format se lit dans les octets, et un fichier bien nommé ne
    /// passe pas pour autant.
    ///
    /// C'est la propriété qui compte : l'application ressort la pièce
    /// sur un disque et la confie au système, donc ce qu'elle a accepté
    /// est ce qu'elle rendra.
    #[test]
    fn the_format_is_read_in_the_bytes_and_never_in_the_name() {
        assert_eq!(sniff(b"%PDF-1.7\n..."), Some(Media::Pdf));
        assert_eq!(
            sniff(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0]),
            Some(Media::Png)
        );
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(Media::Jpeg));
        assert_eq!(sniff(b"II\x2A\x00rest"), Some(Media::Tiff));
        assert_eq!(sniff(b"MM\x00\x2Arest"), Some(Media::Tiff));

        // Ce qui n'est aucun des quatre est refusé, quel que soit le nom
        // qu'on lui a donné : un exécutable, un script, un document
        // bureautique, deux octets qui commencent comme un JPEG.
        for wrong in [
            &b"MZ\x90\x00"[..],
            b"#!/bin/sh\nrm -rf /",
            b"PK\x03\x04",
            b"{\"json\": true}",
            b"",
            &[0xFF, 0xD8],
        ] {
            assert_eq!(sniff(wrong), None, "{wrong:?}");
        }
        // Et l'extension rendue vient du contenu.
        assert_eq!(Media::Jpeg.extension(), "jpg");
        for m in [Media::Pdf, Media::Png, Media::Jpeg, Media::Tiff] {
            assert_eq!(Media::from_key(m.as_key()), Some(m));
        }
        assert_eq!(Media::from_key("application/x-msdownload"), None);
    }

    /// Trois refus, et chacun dit ce qui ne va pas.
    #[test]
    fn a_piece_is_refused_at_the_door_and_told_why() {
        assert_eq!(accept(b"", 20), Err(Refusal::Empty));
        // Un scanner qui a échoué écrit zéro octet, et garder ce
        // fichier ferait croire que la pièce est au dossier.
        assert_eq!(accept(b"pas un document", 20), Err(Refusal::Unknown));
        assert_eq!(accept(b"%PDF-1.7", 20), Ok(Media::Pdf));

        // Le plafond, en mégaoctets pleins.
        let big = {
            let mut v = b"%PDF-1.7".to_vec();
            v.resize(3 * 1024 * 1024, 0);
            v
        };
        assert_eq!(accept(&big, 2), Err(Refusal::TooBig));
        assert_eq!(accept(&big, 3), Ok(Media::Pdf));
        // Le format est jugé avant la taille, pour qu'un gros fichier
        // illisible soit refusé pour la bonne raison.
        let big_junk = vec![b'x'; 3 * 1024 * 1024];
        assert_eq!(accept(&big_junk, 2), Err(Refusal::Unknown));
        // Un plafond à zéro refuse tout ce qui n'est pas vide, ce qui
        // est une façon d'éteindre la fonction sans la retirer.
        assert_eq!(accept(b"%PDF-1.7", 0), Err(Refusal::TooBig));
    }

    /// La commande du scanner nomme son fichier de sortie, ou elle est
    /// refusée avec une phrase qui le dit.
    #[test]
    fn the_scanner_command_must_say_where_it_writes() {
        let out = std::path::Path::new("/tmp/bpm caddy/scan.pdf");
        let argv = scanner_command("scanimage --format=pdf -o {out}", out).unwrap();
        assert_eq!(
            argv,
            vec!["scanimage", "--format=pdf", "-o", "/tmp/bpm caddy/scan.pdf"]
        );
        // Avec ou sans guillemets, le chemin reste **un** argument : il
        // est découpé avant d'être rempli, et non l'inverse. Rempli
        // d'abord, « C:\\Users\\Jean Martin\\scan.pdf » deviendrait deux
        // arguments et le scanner écrirait dans « C:\\Users\\Jean ».
        let argv = scanner_command("scanner.exe \"{out}\"", out).unwrap();
        assert_eq!(argv, vec!["scanner.exe", "/tmp/bpm caddy/scan.pdf"]);
        // Et `{out}` collé à autre chose reste collé.
        let argv = scanner_command("scanimage --out={out}", out).unwrap();
        assert_eq!(argv, vec!["scanimage", "--out=/tmp/bpm caddy/scan.pdf"]);
        // Sans `{out}`, l'application ne saurait pas quoi lire : refusé
        // plutôt qu'exécuté pour rien.
        assert!(scanner_command("scanimage --format=pdf", out).is_err());
        assert!(scanner_command("   ", out).is_err());
        assert!(scanner_command("", out).is_err());
    }

    /// Le découpage en arguments : guillemets, espaces, et un argument
    /// délibérément vide.
    #[test]
    fn a_command_line_is_split_the_way_a_shell_would() {
        assert_eq!(split_args("a b c"), vec!["a", "b", "c"]);
        assert_eq!(split_args("  a   b  "), vec!["a", "b"]);
        assert_eq!(split_args("a \"b c\" d"), vec!["a", "b c", "d"]);
        assert_eq!(split_args("a 'b c' d"), vec!["a", "b c", "d"]);
        // Un guillemet dans l'autre sorte de guillemet est un caractère.
        assert_eq!(
            split_args("echo \"l'ordonnance\""),
            vec!["echo", "l'ordonnance"]
        );
        // Un argument vide est un argument.
        assert_eq!(split_args("cmd \"\" x"), vec!["cmd", "", "x"]);
        assert!(split_args("").is_empty());
        assert!(split_args("   ").is_empty());
    }

    /// Les genres et les sujets se relisent, et une clé inconnue ne fait
    /// pas disparaître une pièce que quelqu'un a rangée exprès.
    #[test]
    fn an_unknown_key_never_loses_a_piece() {
        for k in DocKind::ALL {
            assert_eq!(DocKind::from_key(k.as_key()), k);
            assert!(!crate::strings::tr(k.label_key()).is_empty());
        }
        assert_eq!(DocKind::from_key("QUELQUE CHOSE"), DocKind::Autre);
        assert_eq!(DocKind::from_key(""), DocKind::Autre);
        // Les trois sujets ont des clés distinctes : deux qui se
        // confondraient rangeraient les pièces d'un dossier sur un
        // autre. Elles ne se relisent nulle part — la base est
        // interrogée par égalité sur la clé — donc il n'y a pas de
        // `from_key` ici : du code défensif que rien n'appelle est du
        // code que rien ne vérifie.
        let keys: std::collections::HashSet<&str> =
            [Subject::Patient, Subject::Drug, Subject::Officine]
                .into_iter()
                .map(Subject::as_key)
                .collect();
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn a_size_is_written_the_way_it_is_read() {
        assert_eq!(human_size(0), "0 o");
        assert_eq!(human_size(512), "512 o");
        assert_eq!(human_size(2048), "2 Ko");
        assert_eq!(human_size(1024 * 1024), "1.0 Mo");
        assert_eq!(human_size(3 * 1024 * 1024 + 512 * 1024), "3.5 Mo");
    }
}
