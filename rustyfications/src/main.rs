mod types;

use std::{rc::Rc, sync::Arc, time::Duration};

use futures::{channel::mpsc, lock::Mutex, StreamExt};
use gtk::{
    glib::{self, clone, JoinHandle},
    pango::EllipsizeMode,
    prelude::*,
    Align, Justification, Orientation,
};
use gtk_layer_shell::{Edge, LayerShell};
#[allow(unused_imports)]
use log::*;
use notifications::{Details, IFace, IFaceRef, Message, Reason, ServerInfo};
use types::{RuntimeData, Window};

pub static MAIN_APP_ID: &str = "com.bzglve.rustyfications";

pub static RESPECT_EXPIRE_TIMEOUT: bool = false;
pub static DEFAULT_EXPIRE_TIMEOUT: Duration = Duration::from_secs(5);

fn main() -> Result<(), glib::Error> {
    env_logger::init();

    let runtime_data = RuntimeData::default();

    let application = gtk::Application::new(Some(MAIN_APP_ID), Default::default());

    let (sender, receiver) = mpsc::channel(100);
    let receiver = Arc::new(Mutex::new(receiver));

    let iface = Rc::new(
        IFace::connect(
            ServerInfo::new("rustyfications", "bzglve", "0.1.0", "1.2"),
            sender,
        )
        .unwrap(),
    );

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
                debug!("input: {:?}", input);

                match input {
                    Message::New(details) => new_notification(
                        details,
                        application.clone(),
                        iface.clone(),
                        runtime_data.clone(),
                    ),
                    Message::Replace(details) => {
                        let windows = runtime_data.borrow().windows.clone();
                        match windows.get(&details.id) {
                            Some(window) => {
                                let timeout;
                                unsafe {
                                    timeout = window
                                        .inner
                                        .data::<JoinHandle<()>>("timeout")
                                        .map(|v| v.as_ref());
                                }
                                if let Some(timeout) = timeout {
                                    timeout.abort();
                                }

                                if let Some(expire_timeout) = details.expire_timeout {
                                    window_expire(
                                        expire_timeout,
                                        details.id,
                                        iface.clone(),
                                        runtime_data.clone(),
                                    );
                                } else if !RESPECT_EXPIRE_TIMEOUT {
                                    window_expire(
                                        DEFAULT_EXPIRE_TIMEOUT,
                                        details.id,
                                        iface.clone(),
                                        runtime_data.clone(),
                                    );
                                }

                                // TODO update window widgets
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

    application.connect_activate(|application| {
        let window = gtk::Window::builder().build();
        window.set_child(Some(&gtk::Label::new(Some("Hello"))));

        window.init_layer_shell();

        window.set_application(Some(application));

        // window.present();
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
    let window = window_build(&details, application.clone(), runtime_data.clone());

    margins_update(runtime_data.clone());

    window.inner.present();

    // window.set_margin(
    //     Edge::Top,
    //     (runtime_data.borrow().windows.len() as i32) * 100,
    // );

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

    event_controller_motion.connect_enter(move |_ecm, x, y| {
        debug!("ECM ENTER | {} | {} |", x, y);
    });

    event_controller_motion.connect_leave(move |_ecm| {
        debug!("ECM LEAVE");
    });

    if let Some(expire_timeout) = details.expire_timeout {
        window_expire(
            expire_timeout,
            details.id,
            iface.clone(),
            runtime_data.clone(),
        );
    } else if !RESPECT_EXPIRE_TIMEOUT {
        window_expire(
            DEFAULT_EXPIRE_TIMEOUT,
            details.id,
            iface.clone(),
            runtime_data.clone(),
        );
    }
}

fn init_layer_shell(window: &impl LayerShell) {
    window.init_layer_shell();

    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, true);

    window.set_margin(Edge::Right, 4);

    // window.set_keyboard_mode(KeyboardMode::OnDemand);
}

fn window_build(
    details: &Details,
    application: gtk::Application,
    runtime_data: RuntimeData,
) -> Window {
    let window = Window::from(details.clone());

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
    // runtime_data
    //     .borrow()
    //     .windows
    //     .iter()
    //     .enumerate()
    //     .for_each(|(i, (_, window))| window.set_margin(Edge::Top, (i as i32) * 100));

    margins_update(runtime_data);
}

fn margins_update(runtime_data: RuntimeData) {
    use itertools::Itertools;

    let mut h = 4;

    if let Some((_, w)) = runtime_data.borrow().windows.iter().next() {
        w.inner.set_margin(Edge::Top, h)
    }

    for ((_, lw), (_, rw)) in runtime_data.borrow().windows.iter().tuple_windows() {
        h += lw.inner.height() + 4;

        rw.inner.set_margin(Edge::Top, h);
    }
}

fn window_expire(value: Duration, id: u32, iface: Rc<IFaceRef>, runtime_data: RuntimeData) {
    let timeout = glib::spawn_future_local(clone!(
        #[strong]
        iface,
        #[strong]
        runtime_data,
        async move {
            glib::timeout_future(value).await;

            window_close(id, runtime_data.clone());

            IFace::notification_closed(iface.signal_context(), id, Reason::Expired)
                .await
                .unwrap();
        }
    ));

    if let Ok(borrowed) = runtime_data.try_borrow() {
        if let Some(window) = borrowed.windows.get(&id) {
            unsafe {
                window.inner.set_data("timeout", timeout);
            }
        }
    } else {
        warn!("Cannot borrow runtime_data on: {:?}", id)
    }
}
