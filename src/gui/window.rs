use std::{cell::RefCell, rc::Rc, time::Duration};

use dbus::{Details, IFace, IFaceRef};
use gtk::{
    glib::{self, clone, JoinHandle},
    pango::{self, EllipsizeMode},
    prelude::*,
    Align, Justification, Orientation,
};

use crate::{dbus, types::RuntimeData, DEFAULT_EXPIRE_TIMEOUT};

use super::utils::init_layer_shell;

#[derive(Clone)]
pub struct Window {
    id: u32,
    // app_name: gtk::Label,
    // app_icon: gtk::Image, // TODO maybe search for icon type
    summary: gtk::Label,
    body: gtk::Label,
    actions_box: gtk::Box,
    expire_timeout: Duration,
    thandle: Rc<RefCell<Option<JoinHandle<()>>>>,
    pub inner: gtk::Window,
}

impl Window {
    pub fn stop_timeout(&self) {
        if let Some(h) = self.thandle.borrow().as_ref() {
            h.abort();
        }
        *self.thandle.borrow_mut() = None;
    }

    pub fn start_timeout<F, Fut>(&self, f: F)
    where
        F: FnOnce(u32) -> Fut + 'static,
        Fut: std::future::Future<Output = ()>,
    {
        if self.thandle.borrow().is_none() {
            self.thandle
                .borrow_mut()
                .replace(glib::spawn_future_local(clone!(
                    #[strong(rename_to=expire_timeout)]
                    self.expire_timeout,
                    #[strong(rename_to=inner)]
                    self.inner,
                    #[strong(rename_to=id)]
                    self.id,
                    async move {
                        glib::timeout_future(expire_timeout).await;

                        inner.close();

                        f(id).await
                    }
                )));
        }
    }
}

// From<Details> for
impl Window {
    pub fn build(
        details: &Details,
        application: gtk::Application,
        iface: Rc<IFaceRef>,
        runtime_data: RuntimeData,
    ) -> Self {
        let window = Window::from_details(details.clone(), iface.clone());

        init_layer_shell(&window.inner);

        window.inner.set_application(Some(&application));

        runtime_data
            .borrow_mut()
            .windows
            .insert(details.id, window.clone());

        window
    }

    pub fn update_from_details(&mut self, value: &Details, iface: Rc<IFaceRef>) {
        if self.thandle.borrow().is_some() {
            self.stop_timeout();
        }

        let value = value.clone();

        self.summary.set_label(&value.summary);

        self.body.set_label(&value.body.clone().unwrap_or_default());
        self.body.set_visible(value.body.is_some());

        let actions_box = self.actions_box.clone();
        actions_box
            .observe_children()
            .into_iter()
            .filter_map(|child| child.ok().and_downcast::<gtk::Widget>())
            .for_each(|child| actions_box.remove(&child));
        actions_box.set_visible(false);
        for action in &value.actions {
            actions_box.set_visible(true);
            actions_box.append(&{
                let btn = gtk::Button::builder().hexpand(true).build();
                if !action.icon {
                    btn.set_label(&action.to_string());
                } else {
                    // btn.set_icon_name(&lookup_icon(&action.to_string()));
                    btn.set_icon_name(&action.to_string());
                    btn.set_tooltip_text(Some(&action.to_string()));
                }

                btn.connect_clicked(clone!(
                    #[strong]
                    iface,
                    #[strong]
                    action,
                    move |_| {
                        glib::spawn_future_local(clone!(
                            #[strong]
                            iface,
                            #[strong]
                            action,
                            async move {
                                IFace::action_invoked(iface.signal_context(), value.id, action)
                                    .await
                                    .unwrap();
                            }
                        ));
                    }
                ));

                btn
            });
        }

        self.expire_timeout = value.expire_timeout.unwrap_or(DEFAULT_EXPIRE_TIMEOUT);
    }

    pub fn from_details(value: Details, iface: Rc<IFaceRef>) -> Self {
        // let app_name = gtk::Label::builder()
        //     .label(value.app_name.unwrap_or_default())
        //     .visible(value.app_name.is_some())
        //     .name("app_name")
        //     .sensitive(false)
        //     .build();
        // let app_icon = gtk::Image::builder().visible(false).build();
        let summary = gtk::Label::builder()
            .label(format!("<b>{}</b>", value.summary))
            .name("summary")
            .justify(Justification::Left)
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .use_markup(true)
            .build();

        let body = gtk::Label::builder()
            .label(value.body.clone().unwrap_or_default())
            .visible(value.body.is_some())
            .name("body")
            .justify(Justification::Left)
            .valign(Align::Start)
            .halign(Align::Start)
            .wrap(true)
            .wrap_mode(pango::WrapMode::WordChar)
            .use_markup(true)
            .build();

        let actions_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(5)
            .visible(false)
            .build();
        for action in value.actions {
            actions_box.set_visible(true);
            actions_box.append(&{
                let btn = gtk::Button::builder().hexpand(true).build();
                if !action.icon {
                    btn.set_label(&action.to_string());
                } else {
                    // btn.set_icon_name(&lookup_icon(&action.to_string()));
                    btn.set_icon_name(&action.to_string());
                    btn.set_tooltip_text(Some(&action.to_string()));
                }

                btn.connect_clicked(clone!(
                    #[strong]
                    iface,
                    #[strong]
                    action,
                    move |_| {
                        glib::spawn_future_local(clone!(
                            #[strong]
                            iface,
                            #[strong]
                            action,
                            async move {
                                IFace::action_invoked(iface.signal_context(), value.id, action)
                                    .await
                                    .unwrap();
                            }
                        ));
                    }
                ));

                btn
            });
        }

        let main_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(5)
            .margin_top(5)
            .margin_start(5)
            .margin_bottom(5)
            .margin_end(5)
            .build();
        main_box.append(&summary);
        main_box.append(&body);
        main_box.append(&actions_box);

        let inner = gtk::Window::builder()
            // optimal size to display 40 chars in 12px font and 5px margin
            .default_width(410)
            .default_height(30)
            .name("notification")
            .build();
        inner.set_child(Some(&main_box));

        Self {
            id: value.id,
            summary,
            body,
            actions_box,
            expire_timeout: value.expire_timeout.unwrap_or(DEFAULT_EXPIRE_TIMEOUT),
            thandle: Default::default(),
            inner,
        }
    }
}
