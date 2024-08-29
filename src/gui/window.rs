use std::{cell::RefCell, rc::Rc, time::Duration};

use gtk::{
    gdk_pixbuf::Pixbuf,
    glib::{self, clone, JoinHandle},
    pango::{self, EllipsizeMode},
    prelude::*,
    Align, Justification, Orientation,
};

use crate::{
    dbus::{Details, IFace, IFaceRef},
    types::RuntimeData,
    DEFAULT_EXPIRE_TIMEOUT, ICON_SIZE,
};

use super::utils::{init_layer_shell, pixbuf};

#[derive(Clone)]
pub struct Window {
    id: u32,
    app_name: gtk::Label,
    icon: gtk::Image,
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

        self.app_name
            .set_label(&value.app_name.clone().unwrap_or_default());
        self.app_name.set_visible(value.app_name.is_some());

        self.summary.set_label(&value.summary);

        self.body.set_label(&value.body.clone().unwrap_or_default());
        self.body.set_visible(value.body.is_some());

        let actions_box = self.actions_box.clone();

        let default_action = value.actions.iter().find(|a| a.key == "default").cloned();
        actions_box
            .observe_children()
            .into_iter()
            .filter_map(|child| child.ok().and_downcast::<gtk::Widget>())
            .for_each(|child| actions_box.remove(&child));
        actions_box.set_visible(false);
        for action in value.actions.iter().filter(|a| a.key != "default") {
            actions_box.set_visible(true);
            actions_box.append(&{
                let btn = gtk::Button::builder().hexpand(true).build();
                if !value.hints.action_icons {
                    btn.set_label(&action.text);
                } else {
                    btn.set_icon_name(&action.key);
                }
                btn.set_tooltip_text(Some(&action.text));

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

        self.icon.set_visible(false);
        let mut pixbuf: Option<Pixbuf> = None;
        if let Some(image_data) = value.hints.image_data {
            pixbuf = Some(Pixbuf::from(image_data));
        } else if let Some(image_path) = value.hints.image_path {
            pixbuf = pixbuf::new_from_str(&image_path);
        } else if let Some(icon_src) = value.app_icon {
            pixbuf = pixbuf::new_from_str(&icon_src);
        } else if let Some(icon_data) = value.hints.icon_data {
            pixbuf = Some(Pixbuf::from(icon_data));
        }
        if let Some(pixbuf) = pixbuf {
            let pixbuf = pixbuf::crop_square(&pixbuf);
            self.icon.set_from_pixbuf(Some(&pixbuf));
            self.icon.set_visible(true);
        }

        if let Some(default_action) = default_action {
            self.inner.set_tooltip_text(Some(&default_action.text));
        }

        self.expire_timeout = value.expire_timeout.unwrap_or(DEFAULT_EXPIRE_TIMEOUT);
    }

    fn build_widgets_tree(value: &Details) -> Self {
        let app_name = gtk::Label::builder()
            .name("app_name")
            .justify(Justification::Left)
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .sensitive(false)
            .build();

        let summary = gtk::Label::builder()
            .name("summary")
            .justify(Justification::Left)
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .use_markup(true)
            .build();

        let body = gtk::Label::builder()
            .name("body")
            .justify(Justification::Left)
            .valign(Align::Fill)
            .halign(Align::Start)
            .wrap(true)
            .wrap_mode(pango::WrapMode::WordChar)
            .use_markup(true)
            .build();

        let actions_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(5)
            .build();

        let main_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(5)
            .build();
        main_box.append(&app_name);
        main_box.append(&summary);
        main_box.append(&body);

        let outer_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(5)
            .build();
        let icon = gtk::Image::builder()
            .visible(false)
            .pixel_size(ICON_SIZE)
            .valign(Align::Center)
            .halign(Align::End)
            .build();

        outer_box.append(&icon);
        outer_box.append(&main_box);

        let act_out_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(5)
            .margin_top(5)
            .margin_start(5)
            .margin_bottom(5)
            .margin_end(5)
            .build();
        act_out_box.append(&outer_box);
        act_out_box.append(&actions_box);

        let inner = gtk::Window::builder()
            // optimal size to display 40 chars in 12px font and 5px margin
            .default_width(410)
            .default_height(30)
            .name("notification")
            .build();
        inner.set_child(Some(&act_out_box));

        Self {
            id: value.id,
            app_name,
            icon,
            summary,
            body,
            actions_box,
            expire_timeout: value.expire_timeout.unwrap_or(DEFAULT_EXPIRE_TIMEOUT),
            thandle: Default::default(),
            inner,
        }
    }

    pub fn from_details(value: Details, iface: Rc<IFaceRef>) -> Self {
        let mut _self = Self::build_widgets_tree(&value);
        _self.update_from_details(&value, iface);
        _self
    }
}
