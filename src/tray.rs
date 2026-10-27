//! Native tray integration. Linux uses StatusNotifierItem over D-Bus;
//! Windows and macOS create their menus on the window event-loop thread.

use crate::message::Message;
use crate::state::{Language, RoutingMode};
use crate::ui::i18n::tr;
use futures::SinkExt;
use std::sync::{Mutex, OnceLock};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayState {
    pub language: Language,
    pub core_running: bool,
    pub system_proxy_enabled: bool,
    pub routing_mode: RoutingMode,
}

static EVENT_RX: OnceLock<Mutex<Option<mpsc::Receiver<Message>>>> = OnceLock::new();
static EVENT_TX: OnceLock<mpsc::Sender<Message>> = OnceLock::new();

fn send(message: Message) {
    let tx = EVENT_TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel(32);
        let _ = EVENT_RX.set(Mutex::new(Some(rx)));
        tx
    });
    let _ = tx.try_send(message);
}

pub fn events() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(
        32,
        |mut output: futures::channel::mpsc::Sender<Message>| async move {
            // Initialize the channel before a platform has produced an event.
            send(Message::NoOp);
            struct Lease(Option<mpsc::Receiver<Message>>);
            impl Drop for Lease {
                fn drop(&mut self) {
                    if let Some(slot) = EVENT_RX.get() {
                        *slot.lock().unwrap_or_else(|e| e.into_inner()) = self.0.take();
                    }
                }
            }
            let mut lease = Lease(
                EVENT_RX
                    .get()
                    .and_then(|slot| slot.lock().unwrap_or_else(|e| e.into_inner()).take()),
            );
            if let Some(rx) = lease.0.as_mut() {
                while let Some(message) = rx.recv().await {
                    if output.send(message).await.is_err() {
                        break;
                    }
                }
            }
        },
    )
}

pub use platform::Tray;

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use ksni::TrayMethods;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use tokio::sync::watch;

    pub struct Tray {
        state: watch::Sender<TrayState>,
        available: Arc<AtomicBool>,
    }

    impl Tray {
        pub fn new(state: TrayState) -> Self {
            let (tx, mut rx) = watch::channel(state);
            let available = Arc::new(AtomicBool::new(false));
            let online = available.clone();
            // Neither D-Bus connection setup nor menu updates block Iced.
            std::thread::spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                runtime.block_on(async move {
                    loop {
                        let state = *rx.borrow_and_update();
                        let service = LinuxTray {
                            state,
                            available: online.clone(),
                        };
                        match service.spawn().await {
                            Ok(handle) => {
                                online.store(true, Ordering::Release);
                                while rx.changed().await.is_ok() {
                                    let state = *rx.borrow_and_update();
                                    if handle
                                        .update(move |tray| tray.state = state)
                                        .await
                                        .is_none()
                                    {
                                        break;
                                    }
                                }
                                online.store(false, Ordering::Release);
                                handle.shutdown().await;
                                return;
                            }
                            Err(_) => {
                                // GNOME may have no AppIndicator extension, or the
                                // session bus may not be ready during autostart.
                                // Keep the window usable and retry without log spam.
                                online.store(false, Ordering::Release);
                                tokio::select! {
                                    changed = rx.changed() => {
                                        if changed.is_err() { return; }
                                    }
                                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                                }
                            }
                        }
                    }
                });
            });
            Self {
                state: tx,
                available,
            }
        }

        pub fn available(&self) -> bool {
            self.available.load(Ordering::Acquire)
        }

        pub fn update(&self, state: TrayState) {
            self.state.send_if_modified(|current| {
                if *current == state {
                    return false;
                }
                *current = state;
                true
            });
        }
    }

    struct LinuxTray {
        state: TrayState,
        available: Arc<AtomicBool>,
    }

    impl ksni::Tray for LinuxTray {
        fn id(&self) -> String {
            "sing-box-gui".into()
        }
        fn title(&self) -> String {
            "sing-box GUI".into()
        }
        fn activate(&mut self, _: i32, _: i32) {
            send(Message::TrayIconClicked);
        }

        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            static ICONS: OnceLock<Vec<ksni::Icon>> = OnceLock::new();
            ICONS
                .get_or_init(|| {
                    let Ok(image) =
                        image::load_from_memory(include_bytes!("../assets/tray-icon.png"))
                    else {
                        return Vec::new();
                    };
                    [22, 32, 48]
                        .into_iter()
                        .map(|size| {
                            let mut rgba = image
                                .resize(size, size, image::imageops::FilterType::Lanczos3)
                                .into_rgba8()
                                .into_raw();
                            // The SNI wire format is big-endian ARGB, not RGBA.
                            for pixel in rgba.as_chunks_mut::<4>().0 {
                                pixel.rotate_right(1);
                            }
                            ksni::Icon {
                                width: size as i32,
                                height: size as i32,
                                data: rgba,
                            }
                        })
                        .collect()
                })
                .clone()
        }

        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            use ksni::menu::{CheckmarkItem, RadioGroup, RadioItem, StandardItem, SubMenu};
            let label = |key| tr(self.state.language, key).to_owned();
            let action = |key, event: &'static str| {
                StandardItem {
                    label: label(key),
                    activate: Box::new(move |_| send(Message::TrayMenuClicked(event.into()))),
                    ..Default::default()
                }
                .into()
            };
            vec![
                action("tray_show_window", "show_window"),
                ksni::MenuItem::Separator,
                action(
                    if self.state.core_running {
                        "tray_stop_core"
                    } else {
                        "tray_start_core"
                    },
                    "toggle_core",
                ),
                CheckmarkItem {
                    label: label("tray_system_proxy"),
                    checked: self.state.system_proxy_enabled,
                    activate: Box::new(|_| send(Message::ToggleSystemProxy)),
                    ..Default::default()
                }
                .into(),
                SubMenu {
                    label: label("tray_proxy_mode"),
                    submenu: vec![
                        RadioGroup {
                            selected: match self.state.routing_mode {
                                RoutingMode::Rule => 0,
                                RoutingMode::Global => 1,
                                RoutingMode::Direct => 2,
                            },
                            select: Box::new(|_, index| {
                                if let Some(mode) =
                                    [RoutingMode::Rule, RoutingMode::Global, RoutingMode::Direct]
                                        .get(index)
                                {
                                    send(Message::RoutingModeChanged(*mode));
                                }
                            }),
                            options: ["tray_rules", "tray_global", "tray_direct"]
                                .into_iter()
                                .map(|key| RadioItem {
                                    label: label(key),
                                    ..Default::default()
                                })
                                .collect(),
                        }
                        .into(),
                    ],
                    ..Default::default()
                }
                .into(),
                ksni::MenuItem::Separator,
                action("tray_exit", "exit_app"),
            ]
        }

        fn watcher_online(&self) {
            self.available.store(true, Ordering::Release);
        }

        fn watcher_offline(&self, _: ksni::OfflineReason) -> bool {
            if self.available.swap(false, Ordering::AcqRel) {
                send(Message::TrayUnavailable);
            }
            true
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod platform {
    use super::*;
    use tray_icon::{
        TrayIcon, TrayIconBuilder,
        menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    };

    pub struct Tray {
        icon: Option<TrayIcon>,
        show: MenuItem,
        exit: MenuItem,
        core: MenuItem,
        proxy: CheckMenuItem,
        modes: Submenu,
        rule: CheckMenuItem,
        global: CheckMenuItem,
        direct: CheckMenuItem,
    }

    impl Tray {
        pub fn new(state: TrayState) -> Self {
            let show = MenuItem::with_id("show_window", "", true, None);
            let exit = MenuItem::with_id("exit_app", "", true, None);
            let core = MenuItem::with_id("toggle_core", "", true, None);
            let proxy = CheckMenuItem::with_id("toggle_system_proxy", "", true, false, None);
            let modes = Submenu::new("", true);
            let rule = CheckMenuItem::with_id("mode_rule", "", true, false, None);
            let global = CheckMenuItem::with_id("mode_global", "", true, false, None);
            let direct = CheckMenuItem::with_id("mode_direct", "", true, false, None);
            let _ = modes.append_items(&[&rule, &global, &direct]);
            let menu = Menu::new();
            let _ = menu.append_items(&[
                &show,
                &PredefinedMenuItem::separator(),
                &core,
                &proxy,
                &modes,
                &PredefinedMenuItem::separator(),
                &exit,
            ]);

            tray_icon::menu::MenuEvent::set_event_handler(Some(
                |event: tray_icon::menu::MenuEvent| send(Message::TrayMenuClicked(event.id.0)),
            ));
            tray_icon::TrayIconEvent::set_event_handler(Some(|event| {
                if matches!(
                    event,
                    tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } | tray_icon::TrayIconEvent::DoubleClick {
                        button: tray_icon::MouseButton::Left,
                        ..
                    }
                ) {
                    send(Message::TrayIconClicked);
                }
            }));
            let icon = image::load_from_memory(include_bytes!("../assets/tray-icon.png"))
                .ok()
                .and_then(|image| {
                    tray_icon::Icon::from_rgba(
                        image
                            .resize(32, 32, image::imageops::FilterType::Lanczos3)
                            .into_rgba8()
                            .into_raw(),
                        32,
                        32,
                    )
                    .ok()
                })
                .and_then(|icon| {
                    TrayIconBuilder::new()
                        .with_menu(Box::new(menu))
                        .with_tooltip("sing-box GUI")
                        .with_icon(icon)
                        .build()
                        .ok()
                });
            let tray = Self {
                icon,
                show,
                exit,
                core,
                proxy,
                modes,
                rule,
                global,
                direct,
            };
            tray.update(state);
            tray
        }

        pub fn available(&self) -> bool {
            self.icon.is_some()
        }

        pub fn update(&self, state: TrayState) {
            let label = |key| tr(state.language, key);
            self.show.set_text(label("tray_show_window"));
            self.exit.set_text(label("tray_exit"));
            self.core.set_text(label(if state.core_running {
                "tray_stop_core"
            } else {
                "tray_start_core"
            }));
            self.proxy.set_text(label("tray_system_proxy"));
            self.proxy.set_checked(state.system_proxy_enabled);
            self.modes.set_text(label("tray_proxy_mode"));
            self.rule.set_text(label("tray_rules"));
            self.global.set_text(label("tray_global"));
            self.direct.set_text(label("tray_direct"));
            self.rule
                .set_checked(state.routing_mode == RoutingMode::Rule);
            self.global
                .set_checked(state.routing_mode == RoutingMode::Global);
            self.direct
                .set_checked(state.routing_mode == RoutingMode::Direct);
        }
    }
}
