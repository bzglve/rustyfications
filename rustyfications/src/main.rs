mod types;
mod utils;

use std::{error::Error, rc::Rc, sync::Arc, time::Duration};

use futures::{channel::mpsc, lock::Mutex, StreamExt};
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use gtk_layer_shell::{Edge, LayerShell};
#[allow(unused_imports)]
use log::*;
use notifications::{Action, Details, IFace, IFaceRef, Message, Reason, ServerInfo};
use types::{RuntimeData, Window};
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
            ServerInfo::new("rustyfications", "bzglve", "0.1.0", "1.2"),
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
                                    window.stop_timeout();
                                    window.start_timeout(clone!(
                                        #[strong]
                                        iface,
                                        #[strong]
                                        runtime_data,
                                        move |id| async move {
                                            close_hook(id, iface.clone(), runtime_data.clone())
                                                .await;
                                        }
                                    ));

                                    window.summary.set_label(&details.summary);

                                    window
                                        .body
                                        .set_label(&details.body.clone().unwrap_or_default());
                                    window.body.set_visible(details.body.is_some());
                                }
                                None => {
                                    debug!("NOT FOUND");
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
                            window_close(id, runtime_data.clone());
                        }
                    }
                }
            }
        ));

        load_css();
    });

    application.connect_activate(|application| {
        let w = gtk::Window::new();
        w.set_application(Some(application));
    });

    application.run();

    Ok(())
}

fn new_notification(
    details: Details,
    application: gtk::Application,
    iface: Rc<IFaceRef>,
    runtime_data: RuntimeData,
) {
    let window = window_build(
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

    let gesture_click = gtk::GestureClick::new();
    window.inner.add_controller(gesture_click.clone());

    // gesture_click.connect_pressed(move |_gesture, _n_press, _x, _y| {});

    gesture_click.connect_released(clone!(
        #[strong]
        iface,
        #[strong]
        runtime_data,
        move |_gesture, _n_press, _x, _y| {
            window_close(details.id, runtime_data.clone());

            glib::spawn_future_local(clone!(
                #[strong]
                iface,
                async move {
                    IFace::action_invoked(iface.signal_context(), details.id, Action::default())
                        .await
                        .unwrap();

                    IFace::notification_closed(
                        iface.signal_context(),
                        details.id,
                        Reason::Dismissed,
                    )
                    .await
                    .unwrap();
                }
            ));
        }
    ));

    let event_controller_motion = gtk::EventControllerMotion::new();
    window.inner.add_controller(event_controller_motion.clone());

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
                    close_hook(id, iface.clone(), runtime_data.clone()).await;
                }
            ));
        }
    ));

    window.start_timeout(move |id| async move {
        close_hook(id, iface.clone(), runtime_data.clone()).await;
    });
}

fn init_layer_shell(window: &impl LayerShell) {
    window.init_layer_shell();

    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, true);

    window.set_margin(Edge::Right, 5);
}

fn window_build(
    details: &Details,
    application: gtk::Application,
    iface: Rc<IFaceRef>,
    runtime_data: RuntimeData,
) -> Window {
    let window = Window::from_details(details.clone(), iface.clone());

    init_layer_shell(&window.inner);

    window.inner.set_application(Some(&application));

    runtime_data
        .borrow_mut()
        .windows
        .insert(details.id, window.clone());

    window
}

fn window_close(id: u32, runtime_data: RuntimeData) {
    if let Some(window) = runtime_data.borrow_mut().windows.remove(&id) {
        window.inner.close();
    }

    margins_update(runtime_data);
}

fn margins_update(runtime_data: RuntimeData) {
    let runtime_data = runtime_data.borrow();
    let windows = runtime_data.windows.iter();

    let iter: Box<dyn Iterator<Item = (&u32, &Window)>> = if NEW_ON_TOP {
        Box::new(windows.rev())
    } else {
        Box::new(windows)
    };

    let mut indent = 5;
    for (_, window) in iter {
        window.inner.set_margin(Edge::Top, indent);
        indent += window.inner.height() + 5;
    }
}
