//! **Lancer BPM-Caddy à l'ouverture de la session**, sans rien installer
//! en tant qu'administrateur : un fichier `.desktop` dans
//! `~/.config/autostart` (Linux, la règle XDG que suivent KDE, GNOME,
//! Xfce…), une valeur sous `HKCU\…\Run` (Windows), un agent dans
//! `~/Library/LaunchAgents` (macOS). Chaque fois pour la seule personne
//! connectée, et défait par le même geste.

use std::path::{Path, PathBuf};

/// Le nom sous lequel l'entrée est rangée, partout.
const NAME: &str = "BPM-Caddy";

/// La commande lancée à l'ouverture de session : un programme et ses
/// arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// L'entrée est-elle posée ?
pub fn is_enabled() -> bool {
    imp::read().is_some()
}

/// La commande posée, si elle l'est — pour voir qu'elle pointe encore
/// vers le programme (un dossier déplacé la laisse vers rien).
pub fn current() -> Option<String> {
    imp::read()
}

/// La commande telle que [`current`] la relit sur cette plateforme —
/// pour comparer ce qui est posé à ce qu'on poserait.
pub fn render(command: &Command) -> String {
    if cfg!(windows) {
        windows_command_line(command)
    } else if cfg!(target_os = "macos") {
        std::iter::once(command.program.to_string_lossy().into_owned())
            .chain(command.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        desktop_exec(command)
    }
}

/// Poser l'entrée, ou la remplacer.
pub fn enable(command: &Command) -> Result<(), String> {
    imp::write(command)
}

/// Retirer l'entrée ; ce qui n'est pas là est déjà retiré.
pub fn disable() -> Result<(), String> {
    imp::remove()
}

/// Une ligne `Exec=` (spécification Desktop Entry) : chaque argument
/// entre guillemets s'il le faut, `"`, `` ` ``, `$` et `\` échappés, et
/// `%` doublé — sans quoi `%f` serait lu comme un code de champ.
pub fn desktop_exec(command: &Command) -> String {
    let quote = |s: &str| -> String {
        let plain = !s.is_empty()
            && s.chars().all(|c| {
                c.is_ascii_alphanumeric()
                    || matches!(c, '/' | '.' | '-' | '_' | '+' | ',' | ':' | '=')
            });
        let s = s.replace('%', "%%");
        if plain {
            return s;
        }
        let mut out = String::from("\"");
        for c in s.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
        out
    };
    std::iter::once(command.program.to_string_lossy().into_owned())
        .chain(command.args.iter().cloned())
        .map(|a| quote(&a))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Le fichier `.desktop` entier.
pub fn desktop_entry(command: &Command, comment: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={NAME}\nComment={}\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        comment.replace('\n', " "),
        desktop_exec(command)
    )
}

/// Une ligne de commande Windows : le programme entre guillemets, chaque
/// argument aussi s'il porte une espace (les règles de `CommandLineToArgvW`
/// pour ce qu'on y met : ni guillemet ni barre finale dans nos arguments).
pub fn windows_command_line(command: &Command) -> String {
    let quote = |s: &str| {
        if s.is_empty() || s.contains([' ', '\t', '"']) {
            format!("\"{}\"", s.replace('"', "\\\""))
        } else {
            s.to_owned()
        }
    };
    let mut line = format!("\"{}\"", command.program.to_string_lossy());
    for a in &command.args {
        line.push(' ');
        line.push_str(&quote(a));
    }
    line
}

/// L'agent `launchd` (macOS), en XML.
pub fn launch_agent(command: &Command) -> String {
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let mut args = format!(
        "        <string>{}</string>\n",
        esc(&command.program.to_string_lossy())
    );
    for a in &command.args {
        args.push_str(&format!("        <string>{}</string>\n", esc(a)));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n    <key>Label</key>\n    <string>fr.bpm-caddy</string>\n\
         \x20   <key>ProgramArguments</key>\n    <array>\n{args}    </array>\n\
         \x20   <key>RunAtLoad</key>\n    <true/>\n</dict>\n</plist>\n"
    )
}

#[allow(dead_code)]
fn write_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

#[allow(dead_code)]
fn remove_file(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::{desktop_entry, remove_file, write_file, Command};
    use std::path::PathBuf;

    fn path() -> Option<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|c| c.join("autostart").join("bpm-caddy.desktop"))
    }

    pub fn read() -> Option<String> {
        let text = std::fs::read_to_string(path()?).ok()?;
        // Désactivée à la main par le bureau : comptée comme absente.
        if text.lines().any(|l| l.trim() == "Hidden=true") {
            return None;
        }
        text.lines()
            .find_map(|l| l.strip_prefix("Exec="))
            .map(str::to_owned)
    }

    pub fn write(command: &Command) -> Result<(), String> {
        let path = path().ok_or("pas de dossier personnel")?;
        write_file(
            &path,
            &desktop_entry(command, "Officine : messagerie et postes"),
        )
    }

    pub fn remove() -> Result<(), String> {
        match path() {
            Some(p) => remove_file(&p),
            None => Ok(()),
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{launch_agent, remove_file, write_file, Command};
    use std::path::PathBuf;

    fn path() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("LaunchAgents")
                .join("fr.bpm-caddy.plist")
        })
    }

    pub fn read() -> Option<String> {
        let text = std::fs::read_to_string(path()?).ok()?;
        let at = text.find("<array>")?;
        let end = text[at..].find("</array>")? + at;
        Some(
            text[at..end]
                .split("<string>")
                .skip(1)
                .filter_map(|s| s.split("</string>").next())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    pub fn write(command: &Command) -> Result<(), String> {
        let path = path().ok_or("pas de dossier personnel")?;
        write_file(&path, &launch_agent(command))
    }

    pub fn remove() -> Result<(), String> {
        match path() {
            Some(p) => remove_file(&p),
            None => Ok(()),
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::{windows_command_line, Command, NAME};
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
    };

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn read() -> Option<String> {
        let (key, name) = (wide(RUN), wide(NAME));
        let mut len: u32 = 0;
        // SAFETY: both strings are NUL-terminated and outlive the calls;
        // the first call only asks for the size, the second writes at
        // most `len` bytes into a buffer of that size.
        unsafe {
            if RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut len,
            ) != ERROR_SUCCESS
            {
                return None;
            }
            let mut buf = vec![0u16; (len as usize).div_ceil(2)];
            if RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buf.as_mut_ptr().cast(),
                &mut len,
            ) != ERROR_SUCCESS
            {
                return None;
            }
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            Some(String::from_utf16_lossy(&buf[..end]))
        }
    }

    pub fn write(command: &Command) -> Result<(), String> {
        let (key, name) = (wide(RUN), wide(NAME));
        let value = wide(&windows_command_line(command));
        // SAFETY: NUL-terminated UTF-16 strings that outlive the call;
        // the size given is the value's, terminator included.
        let r = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        };
        if r == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("registre : erreur {r}"))
        }
    }

    pub fn remove() -> Result<(), String> {
        let (key, name) = (wide(RUN), wide(NAME));
        // SAFETY: NUL-terminated strings that outlive the call.
        let r = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) };
        if r == ERROR_SUCCESS || r == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(format!("registre : erreur {r}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(program: &str, args: &[&str]) -> Command {
        Command {
            program: PathBuf::from(program),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn a_desktop_exec_line_quotes_what_the_spec_says_to() {
        assert_eq!(
            desktop_exec(&cmd("/opt/bpm/bpm-caddy", &["--arriere-plan"])),
            "/opt/bpm/bpm-caddy --arriere-plan"
        );
        assert_eq!(
            desktop_exec(&cmd("/home/a b/bpm $x", &["50%"])),
            r#""/home/a b/bpm \$x" "50%%""#
        );
        assert_eq!(desktop_exec(&cmd("/x", &[""])), r#"/x """#);
    }

    #[test]
    fn the_desktop_entry_is_one_application_started_at_login() {
        let e = desktop_entry(&cmd("/opt/bpm-caddy", &["--arriere-plan"]), "a\nb");
        assert!(e.starts_with("[Desktop Entry]\n"));
        assert!(e.contains("\nType=Application\n"));
        assert!(e.contains("\nExec=/opt/bpm-caddy --arriere-plan\n"));
        assert!(e.contains("\nComment=a b\n"));
    }

    #[test]
    fn a_windows_command_line_quotes_the_program_always() {
        assert_eq!(
            windows_command_line(&cmd(r"C:\Program Files\bpm.exe", &["--arriere-plan"])),
            r#""C:\Program Files\bpm.exe" --arriere-plan"#
        );
        assert_eq!(
            windows_command_line(&cmd(r"C:\bpm.exe", &["a b"])),
            r#""C:\bpm.exe" "a b""#
        );
    }

    #[test]
    fn the_launch_agent_escapes_its_strings() {
        let p = launch_agent(&cmd("/Apps/A&B", &["--arriere-plan"]));
        assert!(p.contains("<string>/Apps/A&amp;B</string>"));
        assert!(p.contains("<string>--arriere-plan</string>"));
        assert!(p.contains("<key>RunAtLoad</key>\n    <true/>"));
    }
}
