mod dbus;
mod gui;
mod types;
mod utils;

use std::{error::Error, rc::Rc, sync::Arc, time::Duration};

use dbus::{Action, Details, IFace, IFaceRef, Message, Reason, ServerInfo};
use futures::{channel::mpsc, lock::Mutex, StreamExt};
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use gtk_layer_shell::{KeyboardMode, LayerShell};
use gui::{build_ui, utils::margins_update, window::Window};
#[allow(unused_imports)]
use log::*;
use sys_logger::{connected_to_journal, JournalLog};
use types::RuntimeData;
use utils::{close_hook, load_css};

pub static MAIN_APP_ID: &str = "com.bzglve.rustyfications";

// TODO move to config
pub static DEFAULT_EXPIRE_TIMEOUT: Duration = Duration::from_secs(5);
pub static NEW_ON_TOP: bool = true;
pub static ICON_SIZE: i32 = 72;
pub static LOG_LEVEL: LevelFilter = LevelFilter::Trace;

pub static WINDOW_CLOSE_ICON: &str = "window-close";

fn main() -> Result<(), Box<dyn Error>> {
    if connected_to_journal() {
        JournalLog::new()
            .unwrap()
            .with_extra_fields(vec![("VERSION", env!("CARGO_PKG_VERSION"))])
            .install()
            .unwrap();
    } else {
        env_logger::init();
    }
    log::set_max_level(LOG_LEVEL);

    info!("Starting application...");

    let runtime_data = RuntimeData::default();

    let application = gtk::Application::new(Some(MAIN_APP_ID), Default::default());

    let (sender, receiver) = mpsc::channel(100);
    let receiver = Arc::new(Mutex::new(receiver));

    let iface = Rc::new(
        IFace::new(
            ServerInfo::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_AUTHORS"),
                env!("CARGO_PKG_VERSION"),
                "1.2",
            ),
            sender,
        )
        .connect()
        .unwrap(),
    );

    application.connect_startup(move |application| {
        info!("Application startup initiated.");
        let settings = gtk::Settings::default().unwrap();
        settings.connect_gtk_theme_name_notify(|_| load_css());
        settings.connect_gtk_application_prefer_dark_theme_notify(|_| load_css());

        glib::spawn_future_local(clone!(
            #[strong]
            application,
            #[strong]
            receiver,
            #[strong]
            iface,
            #[strong]
            runtime_data,
            async move {
                loop {
                    let input = receiver.lock().await.select_next_some().await;
                    debug!("Received input: {:?}", input);

                    match input {
                        Message::New(details) => {
                            info!("New notification");
                            new_notification(
                                details,
                                application.clone(),
                                iface.clone(),
                                runtime_data.clone(),
                            );
                        }
                        Message::Replace(details) => {
                            debug!("Replacing notification");
                            let mut windows = runtime_data.borrow().windows.clone();
                            match windows.get_mut(&details.id) {
                                Some(window) => {
                                    info!(
                                        "Updating existing notification window with id: {}",
                                        details.id
                                    );
                                    window.update_from_details(&details, iface.clone());

                                    window.start_timeout();
                                }
                                None => {
                                    warn!(
                                        "Notification to replace not found, creating new: {:?}",
                                        details
                                    );
                                    new_notification(
                                        details,
                                        application.clone(),
                                        iface.clone(),
                                        runtime_data.clone(),
                                    );
                                }
                            }
                        }
                        Message::Close(id) => {
                            info!("Closing notification with id: {}", id);
                            if let Some(w) = runtime_data.borrow().windows.get(&id) {
                                w.close(Reason::Closed)
                            }
                        }
                    }
                }
            }
        ));

        load_css();
    });

    application.connect_activate(build_ui);

    application.run();

    info!("Application terminated.");
    Ok(())
}

fn new_notification(
    details: Details,
    application: gtk::Application,
    iface: Rc<IFaceRef>,
    runtime_data: RuntimeData,
) {
    info!("Creating new notification window for id: {}", details.id);

    let window = Window::build(
        &details,
        application.clone(),
        iface.clone(),
        runtime_data.clone(),
    );

    window.inner.connect_unrealize(clone!(
        #[strong]
        window,
        #[strong]
        iface,
        #[strong]
        runtime_data,
        move |_window| {
            let reason = unsafe {
                window
                    .inner
                    .data::<Reason>("close-reason")
                    .map(|v| *v.as_ref())
            };
            if reason.is_none() {
                panic!("Can't get close Reason from window data. Probably you called `gtk::Window::close()` intead of `crate::gui::Window::close()`");
            }
            let reason = reason.unwrap();

            glib::spawn_future_local(close_hook(
                window.id,
                reason,
                iface.clone(),
                runtime_data.clone(),
            ));
        }
    ));

    window.inner.present();

    glib::timeout_add_local(
        Duration::from_millis(50),
        clone!(
            #[strong]
            window,
            #[strong]
            runtime_data,
            move || {
                if window.inner.is_mapped() {
                    margins_update(runtime_data.clone());
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            }
        ),
    );

    window.start_timeout();

    debug!("Setting up gesture controls for window.");

    // lmb
    let gesture_click_1 = gtk::GestureClick::builder().button(1).build();
    window.inner.add_controller(gesture_click_1.clone());

    gesture_click_1.connect_released(clone!(
        #[strong]
        iface,
        #[strong]
        window,
        move |gesture, _, _, _| {
            debug!("Left mouse button released.");
            glib::spawn_future_local(clone!(
                #[strong]
                iface,
                #[strong]
                window,
                async move {
                    if window.has_default_action() {
                        IFace::action_invoked(
                            iface.signal_context(),
                            details.id,
                            Action::default(),
                        )
                        .await
                        .unwrap();

                        window.close(Reason::Dismissed);
                    }
                }
            ));

            gesture.set_state(gtk::EventSequenceState::Claimed);
        }
    ));

    // rmb
    let gesture_click_3 = gtk::GestureClick::builder().button(3).build();
    window.inner.add_controller(gesture_click_3.clone());

    gesture_click_3.connect_released(clone!(
        #[strong]
        window,
        move |gesture, _, _, _| {
            debug!("Right mouse button released.");
            window.close(Reason::Dismissed);

            gesture.set_state(gtk::EventSequenceState::Claimed);
        }
    ));

    let event_controller_motion = gtk::EventControllerMotion::new();
    window.inner.add_controller(event_controller_motion.clone());

    // FIXME new window breaks focus
    // it invokes leave and notification can be lost while we are "holding" it
    event_controller_motion.connect_enter(clone!(
        #[strong]
        window,
        move |_, _, _| {
            window.inner.add_css_class("hover");

            // this is workaround to proper passive work
            // if set this permanent then window will steal focus
            // but notification must be passive. Without interrupting user
            window.inner.set_keyboard_mode(KeyboardMode::OnDemand);

            window.stop_timeout();
        }
    ));

    event_controller_motion.connect_leave(clone!(
        #[strong]
        window,
        move |_| {
            window.inner.remove_css_class("hover");

            window.inner.set_keyboard_mode(KeyboardMode::None);

            window.start_timeout();
        }
    ));
}
