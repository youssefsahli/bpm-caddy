//! **L'icône de la zone de notification.**
//!
//! Un poste qui attend les patients à la passerelle, ou les autres postes
//! de l'officine, doit rester ouvert ; une fenêtre qu'on doit laisser
//! ouverte est une fenêtre qu'on ferme. L'icône garde l'application en
//! vie, fenêtre fermée : un clic la rouvre, son menu la quitte, son
//! infobulle dit l'état.
//!
//! Ce crate ne sait rien de l'application : il prend une image, trois
//! libellés, et rend des [`Event`] sur un canal. Il ne dépend d'aucune
//! bibliothèque graphique, pour se vérifier pour Windows depuis un poste
//! Linux (`cargo check -p bpm-tray --target x86_64-pc-windows-gnu`).
//!
//! [`Tray::spawn`] rend `None` là où il n'y a pas de zone de notification
//! (macOS, un bureau Linux sans StatusNotifierItem) : l'application ne
//! doit alors jamais se cacher — une fenêtre fermée sans icône pour la
//! rouvrir serait un programme qui tourne sans qu'on puisse l'atteindre.

pub mod autostart;

use std::sync::mpsc::Sender;
use std::sync::Arc;

/// Ce qu'on a demandé à l'icône.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Montrer la fenêtre (un clic sur l'icône, ou l'entrée du menu).
    Open,
    /// Quitter l'application.
    Quit,
}

/// Les mots de l'icône, donnés par l'application (ses chaînes sont dans
/// son fichier de traductions, pas ici).
#[derive(Clone, Debug)]
pub struct Labels {
    pub title: String,
    pub open: String,
    pub quit: String,
}

/// Une image RGBA, rangée ligne après ligne.
#[derive(Clone, Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Ce qui est envoyé à l'application : l'événement, puis `wake` — pour
/// qu'une fenêtre ouverte redessine et le lise.
// Sans icône (macOS), rien ne l'envoie.
#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
#[derive(Clone)]
struct Out {
    tx: Sender<Event>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
impl Out {
    fn send(&self, event: Event) {
        let _ = self.tx.send(event);
        (self.wake)();
    }
}

/// L'icône tant qu'elle vit ; la lâcher la retire.
pub struct Tray {
    inner: imp::Inner,
}

impl Tray {
    /// Poser l'icône. `wake` est appelé après chaque événement envoyé.
    pub fn spawn(
        image: Image,
        labels: Labels,
        tx: Sender<Event>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Option<Tray> {
        imp::spawn(image, labels, Out { tx, wake }).map(|inner| Tray { inner })
    }

    /// L'état, en infobulle (« En attente des patients », « 2 messages »…).
    pub fn set_status(&self, status: &str) {
        imp::set_status(&self.inner, status);
    }
}

/// **Répondre aux messages du système adressés à ce fil**, sous Windows :
/// à appeler régulièrement par un fil qui a eu des fenêtres et n'en fait
/// plus tourner la boucle (la question « puis-je fermer la session ? »
/// attendrait sinon). Ailleurs, rien.
pub fn pump_messages() {
    imp::pump_messages();
}

/// Le texte de l'infobulle : le titre, puis l'état s'il y en a un.
/// (Sous Linux, le protocole porte les deux à part.)
#[cfg_attr(not(windows), allow(dead_code))]
fn tooltip(title: &str, status: &str) -> String {
    if status.trim().is_empty() {
        title.to_owned()
    } else {
        format!("{title}\n{status}")
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{Event, Image, Labels, Out};
    use ksni::blocking::{Handle, TrayMethods};

    pub struct Inner(Handle<Sni>);

    pub struct Sni {
        icon: ksni::Icon,
        labels: Labels,
        status: String,
        out: Out,
    }

    impl ksni::Tray for Sni {
        fn id(&self) -> String {
            "bpm-caddy".into()
        }
        fn title(&self) -> String {
            self.labels.title.clone()
        }
        fn category(&self) -> ksni::Category {
            ksni::Category::ApplicationStatus
        }
        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            vec![self.icon.clone()]
        }
        fn tool_tip(&self) -> ksni::ToolTip {
            ksni::ToolTip {
                title: self.labels.title.clone(),
                description: self.status.clone(),
                ..Default::default()
            }
        }
        fn activate(&mut self, _x: i32, _y: i32) {
            self.out.send(Event::Open);
        }
        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            use ksni::menu::StandardItem;
            vec![
                StandardItem {
                    label: self.labels.open.replace('_', "__"),
                    activate: Box::new(|t: &mut Sni| t.out.send(Event::Open)),
                    ..Default::default()
                }
                .into(),
                ksni::MenuItem::Separator,
                StandardItem {
                    label: self.labels.quit.replace('_', "__"),
                    activate: Box::new(|t: &mut Sni| t.out.send(Event::Quit)),
                    ..Default::default()
                }
                .into(),
            ]
        }
        // Un hôte qui s'en va (le panneau du bureau redémarre) n'arrête
        // pas le service : l'icône revient avec lui.
        fn watcher_offline(&self, _reason: ksni::OfflineReason) -> bool {
            true
        }
    }

    pub fn spawn(image: Image, labels: Labels, out: Out) -> Option<Inner> {
        // ARGB32, octets dans l'ordre du réseau.
        let mut data = image.rgba;
        for px in data.as_chunks_mut::<4>().0 {
            px.rotate_right(1);
        }
        let icon = ksni::Icon {
            width: image.width as i32,
            height: image.height as i32,
            data,
        };
        let sni = Sni {
            icon,
            labels,
            status: String::new(),
            out,
        };
        // Sans hôte StatusNotifierItem (GNOME nu), `spawn` échoue : pas
        // d'icône, et l'application ne se cache jamais.
        sni.spawn().ok().map(Inner)
    }

    pub fn set_status(inner: &Inner, status: &str) {
        let status = status.to_owned();
        inner.0.update(move |t| t.status = status);
    }

    pub fn pump_messages() {}

    impl Drop for Inner {
        fn drop(&mut self) {
            self.0.shutdown();
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::{tooltip, Event, Image, Labels, Out};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// L'icône vit sur son propre fil, qui fait tourner sa boucle de
    /// messages Win32 : la fenêtre de l'application peut alors être
    /// détruite et recréée sans l'emporter.
    pub struct Inner {
        status: Arc<Mutex<Option<String>>>,
        stop: Arc<AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    pub fn pump_messages() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
        };
        // SAFETY: a zeroed MSG is a valid out-parameter, and the messages
        // are this thread's own, dispatched on it.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn spawn(image: Image, labels: Labels, out: Out) -> Option<Inner> {
        let status = Arc::new(Mutex::new(None::<String>));
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (st, sp) = (Arc::clone(&status), Arc::clone(&stop));
        let thread = std::thread::Builder::new()
            .name("bpm-tray".into())
            .spawn(move || run(image, labels, out, st, sp, ready_tx))
            .ok()?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(true) => Some(Inner {
                status,
                stop,
                thread: Some(thread),
            }),
            _ => None,
        }
    }

    fn run(
        image: Image,
        labels: Labels,
        out: Out,
        status: Arc<Mutex<Option<String>>>,
        stop: Arc<AtomicBool>,
        ready: std::sync::mpsc::Sender<bool>,
    ) {
        use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
        use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
        };

        let menu = Menu::new();
        let open = MenuItem::new(&labels.open, true, None);
        let quit = MenuItem::new(&labels.quit, true, None);
        let built = menu
            .append(&open)
            .and_then(|_| menu.append(&PredefinedMenuItem::separator()))
            .and_then(|_| menu.append(&quit));
        let icon = tray_icon::Icon::from_rgba(image.rgba, image.width, image.height).ok();
        let tray = match (built, icon) {
            (Ok(()), Some(icon)) => TrayIconBuilder::new()
                .with_icon(icon)
                .with_tooltip(&labels.title)
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .ok(),
            _ => None,
        };
        let Some(tray) = tray else {
            let _ = ready.send(false);
            return;
        };
        let (open_id, quit_id) = (open.id().clone(), quit.id().clone());
        let o = out.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if e.id == open_id {
                o.send(Event::Open);
            } else if e.id == quit_id {
                o.send(Event::Quit);
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                out.send(Event::Open);
            }
        }));
        let _ = ready.send(true);
        // SAFETY: a zeroed MSG is a valid out-parameter; PeekMessageW,
        // TranslateMessage and DispatchMessageW are called on the thread
        // that owns the icon's hidden window, with a pointer to a live MSG.
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        while !stop.load(Ordering::Relaxed) {
            unsafe {
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            let said = status.lock().ok().and_then(|mut s| s.take());
            if let Some(s) = said {
                let _ = tray.set_tooltip(Some(tooltip(&labels.title, &s)));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        drop(tray);
    }

    pub fn set_status(inner: &Inner, status: &str) {
        if let Ok(mut s) = inner.status.lock() {
            *s = Some(status.to_owned());
        }
    }

    impl Drop for Inner {
        /// Attendre que le fil ait retiré l'icône : un programme qui se
        /// termine avant laisse une icône fantôme jusqu'au passage de la
        /// souris.
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod imp {
    use super::{Image, Labels, Out};

    pub struct Inner;

    pub fn spawn(_image: Image, _labels: Labels, _out: Out) -> Option<Inner> {
        None
    }

    pub fn set_status(_inner: &Inner, _status: &str) {}

    pub fn pump_messages() {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_tooltip_says_the_status_under_the_title() {
        assert_eq!(super::tooltip("BPM-Caddy", ""), "BPM-Caddy");
        assert_eq!(super::tooltip("BPM-Caddy", "  "), "BPM-Caddy");
        assert_eq!(
            super::tooltip("BPM-Caddy", "En attente des patients"),
            "BPM-Caddy\nEn attente des patients"
        );
    }
}
