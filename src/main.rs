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
use gui::{build_ui, utils::margins_update, window::Window};
#[allow(unused_imports)]
use log::*;
use types::RuntimeData;
use utils::{close_hook, load_css};

pub static MAIN_APP_ID: &str = "com.bzglve.rustyfications";

pub static DEFAULT_EXPIRE_TIMEOUT: Duration = Duration::from_secs(5);
pub static NEW_ON_TOP: bool = true;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let runtime_data = RuntimeData::default();

    let application = gtk::Application::new(Some(MAIN_APP_ID), Default::default());

    let (sender, receiver) = mpsc::channel(100);
    let receiver = Arc::new(Mutex::new(receiver));

    let iface = Rc::new(
        IFace::new(
            ServerInfo::new("rustyfications", "bzglve", env!("CARGO_PKG_VERSION"), "1.2"),
            sender,
        )
        .connect()
        .unwrap(),
    );

    application.connect_startup(move |application| {
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
                    debug!("{:?}", input);

                    match input {
                        Message::New(details) => new_notification(
                            details,
                            application.clone(),
                            iface.clone(),
                            runtime_data.clone(),
                        ),
                        Message::Replace(details) => {
                            let mut windows = runtime_data.borrow().windows.clone();
                            match windows.get_mut(&details.id) {
                                Some(window) => {
                                    window.update_from_details(&details, iface.clone());

                                    window.start_timeout(clone!(
                                        #[strong]
                                        iface,
                                        #[strong]
                                        runtime_data,
                                        move |id| async move {
                                            close_hook(
                                                id,
                                                Reason::Expired,
                                                iface.clone(),
                                                runtime_data.clone(),
                                            )
                                            .await;
                                        }
                                    ));
                                }
                                None => {
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
                            if let Some(w) = runtime_data.borrow().windows.get(&id) {
                                w.inner.close()
                            }
                            close_hook(id, Reason::Closed, iface.clone(), runtime_data.clone())
                                .await;
                        }
                    }
                }
            }
        ));

        load_css();
    });

    application.connect_activate(build_ui);

    application.run();

    Ok(())
}

fn new_notification(
    details: Details,
    application: gtk::Application,
    iface: Rc<IFaceRef>,
    runtime_data: RuntimeData,
) {
    let window = Window::build(
        &details,
        application.clone(),
        iface.clone(),
        runtime_data.clone(),
    );

    window.inner.present();

    if !NEW_ON_TOP {
        margins_update(runtime_data.clone());
    } else {
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
    }

    window.start_timeout(clone!(
        #[strong]
        iface,
        #[strong]
        runtime_data,
        move |id| async move {
            close_hook(id, Reason::Expired, iface.clone(), runtime_data.clone()).await;
        }
    ));

    let gesture_click = gtk::GestureClick::new();
    window.inner.add_controller(gesture_click.clone());

    // gesture_click.connect_pressed(move |_gesture, _n_press, _x, _y| {});

    // TODO we need something more predictable
    // currently only tooltip indicates what will be on click
    gesture_click.connect_released(clone!(
        #[strong]
        iface,
        #[strong]
        runtime_data,
        #[strong]
        window,
        move |_, _, _, _| {
            glib::spawn_future_local(clone!(
                #[strong]
                iface,
                #[strong]
                runtime_data,
                #[strong]
                window,
                async move {
                    IFace::action_invoked(iface.signal_context(), details.id, Action::default())
                        .await
                        .unwrap();

                    window.inner.close();
                    close_hook(
                        details.id,
                        Reason::Dismissed,
                        iface.clone(),
                        runtime_data.clone(),
                    )
                    .await;
                }
            ));
        }
    ));

    let event_controller_motion = gtk::EventControllerMotion::new();
    window.inner.add_controller(event_controller_motion.clone());

    // FIXME new window breaks focus
    // it invokes leave and notification can be lost while we are holding it
    event_controller_motion.connect_enter(clone!(
        #[strong]
        window,
        move |_ecm, _x, _y| {
            window.inner.add_css_class("hover");

            window.stop_timeout();
        }
    ));

    event_controller_motion.connect_leave(clone!(
        #[strong]
        window,
        #[strong]
        iface,
        #[strong]
        runtime_data,
        move |_ecm| {
            window.inner.remove_css_class("hover");

            window.start_timeout(clone!(
                #[strong]
                iface,
                #[strong]
                runtime_data,
                move |id| async move {
                    close_hook(id, Reason::Expired, iface.clone(), runtime_data.clone()).await;
                }
            ));
        }
    ));
}
