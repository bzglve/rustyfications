mod config;
mod dbus;
mod gui;
mod types;
mod utils;

use std::{error::Error, rc::Rc, sync::Arc, time::Duration};

use config::CONFIG;
use dbus::{Details, IFace, IFaceRef, Message, Reason, ServerInfo};
use futures::{
    channel::mpsc::{self, Receiver},
    lock::Mutex,
    StreamExt,
};
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use gui::{build_ui, utils::margins_update, window::Window};
#[allow(unused_imports)]
use log::*;
use types::RuntimeData;
use utils::{close_hook, logger_init, setup_styling};

pub static MAIN_APP_ID: &str = "com.bzglve.rustyfications";

fn main() -> Result<(), Box<dyn Error>> {
    logger_init()?;

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
        .connect()?,
    );

    application.connect_startup(move |application| {
        info!("Application startup initiated.");

        setup_styling();

        handle_notification(
            application.clone(),
            receiver.clone(),
            iface.clone(),
            runtime_data.clone(),
        );

        debug!("CONFIG: {:#?}", CONFIG.lock().unwrap());
    });

    application.connect_activate(build_ui);

    application.run();

    info!("Application terminated.");
    Ok(())
}

fn handle_notification(
    application: gtk::Application,
    receiver: Arc<Mutex<Receiver<Message>>>,
    iface: Rc<IFaceRef>,
    runtime_data: RuntimeData,
) {
    glib::spawn_future_local(async move {
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
    });
}

// FIXME too much windows breaks system
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

    // TODO move it into `Window::build()`
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
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            }
        ),
    );

    window.start_timeout();
}
