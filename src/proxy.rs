//! **Le proxy web** d'un réseau qui ne laisse sortir que par lui — un
//! hôpital, une entreprise, un prestataire. Le navigateur s'en sert sans
//! qu'on y pense ; une connexion ouverte à la main ne le connaît pas, et
//! n'allait nulle part.
//!
//! Où le trouver, dans l'ordre : ce que l'officine a écrit (`[reseau]
//! proxy`, « aucun » pour s'en passer), les variables d'environnement
//! (`HTTPS_PROXY`…), puis le réglage du système (Windows : celui
//! d'Internet Explorer et d'Edge, dans le registre ; macOS : `scutil
//! --proxy`). Un script d'auto-configuration (PAC, WPAD) n'est pas lu :
//! l'adresse du proxy s'écrit alors à la main.
//!
//! Ce qui passe par lui : un tunnel `CONNECT` vers le port 443 — les
//! trackers du rendez-vous, la porte web des officines, la recherche
//! d'une nouvelle version. Le proxy voit à qui l'on parle, jamais ce qui
//! se dit : tout ce qui traverse le tunnel est chiffré de bout en bout. Un
//! proxy qui demande une connexion Windows (NTLM, Kerberos) n'est pas
//! traversé ; un nom et un mot de passe, si.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::RwLock;
use std::time::Duration;

/// D'où vient le proxy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Written,
    Environment,
    System,
}

/// **Un proxy HTTP** : où, et le nom et le mot de passe qu'il demande.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub auth: Option<(String, String)>,
    pub source: Source,
}

impl Proxy {
    /// Son adresse, sans le mot de passe : ce qui se dit à l'écran.
    pub fn shown(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Son adresse en URL, mot de passe compris : pour `ureq`.
    pub fn url(&self) -> String {
        match &self.auth {
            Some((u, p)) => format!("http://{u}:{p}@{}:{}", self.host, self.port),
            None => format!("http://{}:{}", self.host, self.port),
        }
    }
}

/// **Lire une adresse de proxy** : `http://[nom:mot@]hôte[:port][/]`, ou
/// `hôte:port`. Un proxy SOCKS n'est pas lu (`None`).
pub fn parse(text: &str, source: Source) -> Option<Proxy> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let rest = match t.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("http") => rest,
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("https") => rest,
        Some(_) => return None,
        None => t,
    };
    let rest = rest.split('/').next().unwrap_or("");
    let (auth, at) = match rest.rsplit_once('@') {
        Some((a, h)) => {
            let (u, p) = a.split_once(':').unwrap_or((a, ""));
            (Some((percent_decoded(u), percent_decoded(p))), h)
        }
        None => (None, rest),
    };
    let (host, port) = match at.rsplit_once(':') {
        Some((h, p)) if !h.ends_with(']') || at.starts_with('[') => {
            (h.to_owned(), p.parse::<u16>().ok()?)
        }
        _ => (at.to_owned(), 80),
    };
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned();
    if host.is_empty() || port == 0 || host.contains(char::is_whitespace) {
        return None;
    }
    Some(Proxy {
        host,
        port,
        auth,
        source,
    })
}

fn percent_decoded(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 3 <= b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// **Les variables d'environnement**, dans l'ordre où les outils les lisent.
pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Option<Proxy> {
    [
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ]
    .iter()
    .filter_map(|k| get(k))
    .find_map(|v| parse(&v, Source::Environment))
}

/// **Le réglage de Windows**, lu dans ce que `reg query` dit de la clé
/// `Internet Settings` : allumé (`ProxyEnable 0x1`), et `ProxyServer`
/// — `hôte:port`, ou un par protocole (`http=…;https=…`).
pub fn from_windows_registry(text: &str) -> Option<Proxy> {
    let value = |name: &str| {
        text.lines().find_map(|l| {
            let mut parts = l.split_whitespace();
            (parts.next()? == name).then(|| parts.nth(1).map(str::to_owned))?
        })
    };
    if value("ProxyEnable")?.trim() != "0x1" {
        return None;
    }
    let server = value("ProxyServer")?;
    let chosen = if server.contains('=') {
        let by = |p: &str| {
            server.split(';').find_map(|part| {
                let (k, v) = part.split_once('=')?;
                k.trim()
                    .eq_ignore_ascii_case(p)
                    .then(|| v.trim().to_owned())
            })
        };
        by("https").or_else(|| by("http"))?
    } else {
        server
    };
    parse(&chosen, Source::System)
}

/// **Le réglage de macOS**, lu dans ce que dit `scutil --proxy`.
pub fn from_scutil(text: &str) -> Option<Proxy> {
    let value = |name: &str| {
        text.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k.trim() == name).then(|| v.trim().to_owned())
        })
    };
    for (on, host, port) in [
        ("HTTPSEnable", "HTTPSProxy", "HTTPSPort"),
        ("HTTPEnable", "HTTPProxy", "HTTPPort"),
    ] {
        if value(on).as_deref() == Some("1") {
            if let (Some(h), Some(p)) = (value(host), value(port)) {
                return parse(&format!("{h}:{p}"), Source::System);
            }
        }
    }
    None
}

/// Le réglage du système, demandé une fois par processus.
fn system() -> Option<Proxy> {
    static SYSTEM: std::sync::OnceLock<Option<Proxy>> = std::sync::OnceLock::new();
    SYSTEM
        .get_or_init(|| {
            let run = |cmd: &str, args: &[&str]| {
                std::process::Command::new(cmd)
                    .args(args)
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            };
            if cfg!(windows) {
                run(
                    "reg",
                    &[
                        "query",
                        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                    ],
                )
                .and_then(|t| from_windows_registry(&t))
            } else if cfg!(target_os = "macos") {
                run("scutil", &["--proxy"]).and_then(|t| from_scutil(&t))
            } else {
                None
            }
        })
        .clone()
}

/// Ce que l'officine a écrit (`[reseau] proxy`) : vide, chercher ;
/// « aucun », s'en passer ; sinon, l'adresse.
static WRITTEN: RwLock<String> = RwLock::new(String::new());

/// Le réglage de l'officine, posé à l'ouverture et à chaque changement.
pub fn set_written(text: &str) {
    if let Ok(mut w) = WRITTEN.write() {
        *w = text.trim().to_owned();
    }
}

/// Le mot qui dit « pas de proxy ».
pub const NONE_WORD: &str = "aucun";

/// **Le proxy à prendre**, ou aucun.
pub fn current() -> Option<Proxy> {
    let written = WRITTEN.read().map(|w| w.clone()).unwrap_or_default();
    choose(&written, |k| std::env::var(k).ok(), system)
}

/// Le proxy que donnerait le réglage `written` : ce qu'Options montre
/// pendant qu'on l'écrit.
pub fn resolved(written: &str) -> Option<Proxy> {
    choose(written.trim(), |k| std::env::var(k).ok(), system)
}

/// [`current`], ses sources données.
fn choose(
    written: &str,
    env: impl Fn(&str) -> Option<String>,
    system: impl FnOnce() -> Option<Proxy>,
) -> Option<Proxy> {
    if written.eq_ignore_ascii_case(NONE_WORD) {
        return None;
    }
    if !written.is_empty() {
        return parse(written, Source::Written);
    }
    from_env(env).or_else(system)
}

/// Les noms d'utilisateur et mot de passe, en Basic.
fn basic(user: &str, password: &str) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let raw = format!("{user}:{password}").into_bytes();
    let mut out = String::new();
    for c in raw.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// La demande de tunnel.
fn request(proxy: &Proxy, host: &str, port: u16) -> String {
    let target = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let auth = proxy
        .auth
        .as_ref()
        .map(|(u, p)| format!("Proxy-Authorization: Basic {}\r\n", basic(u, p)))
        .unwrap_or_default();
    format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n{auth}\r\n")
}

/// **Un tunnel par le proxy** jusqu'à `host:port` : une connexion où ce
/// qui s'écrit va à la cible. `Err` quand le proxy refuse (le code qu'il
/// dit) ou ne répond pas.
pub fn connect(
    proxy: &Proxy,
    host: &str,
    port: u16,
    patience: Duration,
) -> std::io::Result<TcpStream> {
    let to = (proxy.host.as_str(), proxy.port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::other("proxy introuvable"))?;
    let mut s = TcpStream::connect_timeout(&to, patience)?;
    s.set_read_timeout(Some(patience))?;
    s.set_write_timeout(Some(patience))?;
    s.write_all(request(proxy, host, port).as_bytes())?;
    // The answer's head, a byte at a time: nothing after it may be eaten —
    // it is the target's.
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 8192 {
            return Err(std::io::Error::other("réponse du proxy trop longue"));
        }
        s.read_exact(&mut byte)?;
        head.push(byte[0]);
    }
    let status = String::from_utf8_lossy(&head);
    let line = status.lines().next().unwrap_or("");
    match line.split_whitespace().nth(1) {
        Some("200") => {
            s.set_read_timeout(None)?;
            s.set_write_timeout(None)?;
            Ok(s)
        }
        _ => Err(std::io::Error::other(format!("proxy : {line}"))),
    }
}

/// **Une connexion à `host:port`**, directe ou par le proxy quand il y en
/// a un : par lui d'abord — sur un réseau qui ne sort que par lui, l'essai
/// direct ne ferait qu'attendre —, puis en direct.
pub fn dial(host: &str, port: u16, patience: Duration) -> std::io::Result<TcpStream> {
    if let Some(p) = current() {
        if let Ok(s) = connect(&p, host, port, patience) {
            return Ok(s);
        }
    }
    let addr = (host, port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::other("adresse introuvable"))?;
    TcpStream::connect_timeout(&addr, patience)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proxy_address_is_read_in_every_usual_spelling() {
        let p = parse("http://proxy.chu.fr:3128/", Source::Environment).unwrap();
        assert_eq!(
            (p.host.as_str(), p.port, p.auth.clone()),
            ("proxy.chu.fr", 3128, None)
        );
        let p = parse("http://jean%40chu:mot%3Ade@10.0.0.1:8080", Source::Written).unwrap();
        assert_eq!(p.auth, Some(("jean@chu".into(), "mot:de".into())));
        assert_eq!(p.shown(), "10.0.0.1:8080");
        assert_eq!(parse("10.0.0.1:8080", Source::System).unwrap().port, 8080);
        assert_eq!(parse("proxy", Source::System).unwrap().port, 80);
        assert_eq!(
            parse("[fd00::1]:3128", Source::System).unwrap().host,
            "fd00::1"
        );
        assert_eq!(parse("socks5://h:1080", Source::System), None);
        assert_eq!(parse("", Source::System), None);
        assert_eq!(parse("h:0", Source::System), None);
    }

    #[test]
    fn the_written_setting_comes_first_and_aucun_turns_it_off() {
        let env = |k: &str| (k == "https_proxy").then(|| "http://e:1".to_owned());
        let sys = || parse("s:2", Source::System);
        assert_eq!(choose("", env, sys).unwrap().host, "e");
        assert_eq!(choose("w:3", env, sys).unwrap().source, Source::Written);
        assert_eq!(choose("Aucun", env, sys), None);
        assert_eq!(choose("", |_| None, sys).unwrap().source, Source::System);
    }

    #[test]
    fn windows_and_macos_say_their_proxy_their_way() {
        let reg = "\r\nHKEY_CURRENT_USER\\Software\\...\\Internet Settings\r\n    \
                   ProxyEnable    REG_DWORD    0x1\r\n    \
                   ProxyServer    REG_SZ    http=p.chu.fr:8080;https=s.chu.fr:8443\r\n";
        let p = from_windows_registry(reg).unwrap();
        assert_eq!((p.host.as_str(), p.port), ("s.chu.fr", 8443));
        assert_eq!(
            from_windows_registry(&reg.replace("0x1", "0x0")),
            None,
            "éteint"
        );
        let plain = "    ProxyEnable    REG_DWORD    0x1\n    ProxyServer    REG_SZ    p:3128\n";
        assert_eq!(from_windows_registry(plain).unwrap().port, 3128);
        let mac = "<dictionary> {\n  HTTPEnable : 1\n  HTTPPort : 3128\n  HTTPProxy : p.chu.fr\n  HTTPSEnable : 0\n}\n";
        assert_eq!(from_scutil(mac).unwrap().host, "p.chu.fr");
        assert_eq!(from_scutil("<dictionary> {\n}\n"), None);
    }

    #[test]
    fn basic_credentials_are_base64() {
        assert_eq!(
            basic("Aladdin", "open sesame"),
            "QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );
        assert_eq!(basic("a", "b"), "YTpi");
        assert_eq!(basic("ab", "c"), "YWI6Yw==");
        let r = request(
            &parse("http://u:p@h:1", Source::Written).unwrap(),
            "82.64.1.2",
            443,
        );
        assert!(r.starts_with("CONNECT 82.64.1.2:443 HTTP/1.1\r\n"));
        assert!(r.contains("Proxy-Authorization: Basic dTpw\r\n"));
        assert!(request(&parse("h:1", Source::Written).unwrap(), "::1", 443)
            .starts_with("CONNECT [::1]:443"));
    }

    /// **Un tunnel par un proxy de test** : il lit la demande, répond 200,
    /// puis passe les octets ; un proxy qui refuse est dit.
    #[test]
    fn a_tunnel_goes_through_and_a_refusal_is_said() {
        use std::net::TcpListener;
        let target = TcpListener::bind("127.0.0.1:0").unwrap();
        let tport = target.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut c, _) = target.accept().unwrap();
            c.write_all(b"pong").unwrap();
        });
        let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
        let pport = proxy.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for (n, conn) in proxy.incoming().enumerate() {
                let mut c = conn.unwrap();
                let mut head = Vec::new();
                let mut b = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    c.read_exact(&mut b).unwrap();
                    head.push(b[0]);
                }
                let line = String::from_utf8_lossy(&head)
                    .lines()
                    .next()
                    .unwrap()
                    .to_owned();
                if n > 0 {
                    c.write_all(b"HTTP/1.1 403 Forbidden\r\n\r\n").unwrap();
                    return;
                }
                let at = line.split_whitespace().nth(1).unwrap().to_owned();
                let mut up = TcpStream::connect(at).unwrap();
                c.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .unwrap();
                let mut got = [0u8; 4];
                up.read_exact(&mut got).unwrap();
                c.write_all(&got).unwrap();
            }
        });
        let p = parse(&format!("127.0.0.1:{pport}"), Source::Written).unwrap();
        let mut s = connect(&p, "127.0.0.1", tport, Duration::from_secs(5)).unwrap();
        let mut got = [0u8; 4];
        s.read_exact(&mut got).unwrap();
        assert_eq!(&got, b"pong");
        let e = connect(&p, "127.0.0.1", tport, Duration::from_secs(5)).unwrap_err();
        assert!(e.to_string().contains("403"), "{e}");
    }
}
