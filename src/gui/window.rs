// TODO probably we need some kind of window factory

use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

use gtk::{
    gdk_pixbuf::Pixbuf,
    gio,
    glib::{self, clone, JoinHandle},
    pango::{self, EllipsizeMode},
    prelude::*,
    Align, Justification, Orientation,
};
use gtk_layer_shell::{KeyboardMode, LayerShell};
#[allow(unused_imports)]
use log::*;

use crate::{
    config::CONFIG,
    dbus::{Action, Details, IFace, IFaceRef, Reason},
    types::RuntimeData,
};

use super::utils::{init_layer_shell, pixbuf};

#[derive(Clone)]
pub struct Window {
    pub id: u32,
    app_name: gtk::Label,
    icon: gtk::Image,
    summary: gtk::Label,
    app_icon: gtk::Image,
    body: gtk::Label,
    reply_entry: gtk::Entry,
    reply_revealer: gtk::Revealer,
    actions_box: gtk::Box,
    expire_timeout: Duration,
    thandle: Rc<RefCell<Option<JoinHandle<()>>>>,
    pub inner: gtk::Window,
}

impl Window {
    pub fn stop_timeout(&self) {
        if let Some(h) = self.thandle.borrow_mut().take() {
            h.abort();
            info!("Timeout aborted for window id: {}", self.id);
        }
    }

    pub fn start_timeout(&self) {
        if self.thandle.borrow().is_none() {
            info!("Starting timeout for window id: {}", self.id);
            self.thandle
                .borrow_mut()
                .replace(glib::spawn_future_local(clone!(
                    #[strong(rename_to=s)]
                    self,
                    async move {
                        glib::timeout_future(s.expire_timeout).await;

                        s.close(Reason::Expired);
                        info!("Window closed due to timeout for id: {}", s.id);
                    }
                )));
        } else {
            warn!("Timeout already running for window id: {}", self.id);
        }
    }

    pub fn build(
        details: &Details,
        application: gtk::Application,
        iface: Rc<IFaceRef>,
        runtime_data: RuntimeData,
    ) -> Self {
        info!("Building window from details: {:?}", details);
        let window = Window::from_details(details.clone(), iface.clone());
        init_layer_shell(&window.inner);
        window.inner.set_application(Some(&application));

        window.setup_reply_handler(details, iface.clone());
        runtime_data
            .borrow_mut()
            .windows
            .insert(details.id, window.clone());

        info!(
            "Window built and added to runtime data with id: {}",
            details.id
        );
        window
    }

    fn setup_reply_handler(&self, details: &Details, iface: Rc<IFaceRef>) {
        self.reply_entry.connect_activate(clone!(
            #[strong]
            details,
            #[strong]
            iface,
            move |entry| {
                if !entry.text().is_empty() {
                    glib::spawn_future_local(clone!(
                        #[strong]
                        entry,
                        #[strong]
                        iface,
                        async move {
                            IFace::notification_replied(
                                iface.signal_context(),
                                details.id,
                                &entry.text(),
                            )
                            .await
                            .unwrap();
                        }
                    ));
                } else {
                    // TODO need to somehow notify user in the ui
                    warn!("The entry cannot be empty!");
                }
            }
        ));
    }

    pub fn update_from_details(&mut self, details: &Details, iface: Rc<IFaceRef>) {
        self.stop_timeout();
        self.update_labels(details);
        self.update_icon(details);

        self.reply_entry
            .set_visible(details.actions.iter().any(|a| a.key == "inline-reply"));

        self.update_actions(details, iface);

        if let Some(default_action) = details.actions.iter().find(|a| a.key == "default") {
            self.inner.set_tooltip_text(Some(&default_action.text));
        }

        self.expire_timeout = details.expire_timeout;
        debug!("Window update complete for id: {}", self.id);
    }

    fn update_labels(&mut self, details: &Details) {
        self.app_name
            .set_label(details.app_name.as_deref().unwrap_or_default());
        self.app_name
            .set_visible(CONFIG.lock().unwrap().show_app_name);

        self.summary.set_label(&details.summary);

        self.body
            .set_label(details.body.as_deref().unwrap_or_default());
        self.body.set_visible(details.body.is_some());
    }

    fn update_icon(&mut self, details: &Details) {
        let app_info = self.find_app_info(details);
        self.set_app_icon(app_info);
        self.set_image_icon(details);
    }

    fn find_app_info(&self, details: &Details) -> Option<gio::DesktopAppInfo> {
        details
            .hints
            .desktop_entry
            .as_deref()
            .and_then(|de| {
                gio::DesktopAppInfo::new(de)
                    .or_else(|| gio::DesktopAppInfo::new(&format!("{}.desktop", de)))
            })
            .or_else(|| {
                details.app_name.as_deref().and_then(|an| {
                    gio::DesktopAppInfo::new(an)
                        .or_else(|| gio::DesktopAppInfo::new(&format!("{}.desktop", an)))
                        .or_else(|| gio::DesktopAppInfo::new(&an.to_lowercase()))
                        .or_else(|| {
                            gio::DesktopAppInfo::new(&format!("{}.desktop", an.to_lowercase()))
                        })
                })
            })
    }

    fn set_app_icon(&self, app_info: Option<gio::DesktopAppInfo>) {
        if let Some(icon_name) =
            app_info.and_then(|app| app.icon().and_then(|icon| icon.to_string()))
        {
            if PathBuf::from(icon_name.clone()).is_absolute() {
                self.app_icon.set_from_file(Some(icon_name));
            } else {
                self.app_icon.set_icon_name(Some(&icon_name));
            }
        } else {
            self.app_icon
                .set_icon_name(Some(&CONFIG.lock().unwrap().window_close_icon));
        }
    }

    fn set_image_icon(&self, details: &Details) {
        let pixbuf: Option<Pixbuf> = details
            .hints
            .image_data
            .clone()
            .map(Pixbuf::from)
            .or_else(|| {
                details
                    .hints
                    .image_path
                    .as_deref()
                    .and_then(pixbuf::new_from_str)
            })
            .or_else(|| details.app_icon.as_deref().and_then(pixbuf::new_from_str))
            .or_else(|| details.hints.icon_data.clone().map(Pixbuf::from))
            .map(|pb| pixbuf::crop_square(&pb));

        self.icon.set_visible(pixbuf.is_some());
        if let Some(pb) = pixbuf {
            self.icon.set_from_pixbuf(Some(&pb));
        }
    }

    fn update_actions(&self, details: &Details, iface: Rc<IFaceRef>) {
        self.actions_box.set_visible(false);
        self.actions_box
            .observe_children()
            .into_iter()
            .filter_map(|child| child.ok().and_downcast::<gtk::Widget>())
            .for_each(|child| self.actions_box.remove(&child));
        for action in details.actions.iter().filter(|a| a.key != "default") {
            // let action_button =
            self.actions_box
                .append(&self.create_action_button(action, details, iface.clone()));
            self.actions_box.set_visible(true);
        }
    }

    fn create_action_button(
        &self,
        action: &Action,
        details: &Details,
        iface: Rc<IFaceRef>,
    ) -> gtk::Button {
        let details = details.clone();

        let button = gtk::Button::builder().hexpand(true).build();
        if !details.hints.action_icons {
            button.set_label(&action.text);
        } else {
            let config = CONFIG.lock().unwrap().clone();
            let redef = config.icons_alias.get(&action.key).unwrap_or(&action.key);
            button.set_icon_name(redef);
        }
        button.set_tooltip_text(Some(&action.text));

        if action.key == "inline-reply" {
            button.connect_clicked(clone!(
                #[strong(rename_to=s)]
                self,
                move |_| {
                    if s.reply_entry.text().is_empty() {
                        s.reply_revealer
                            .set_reveal_child(!s.reply_revealer.reveals_child());
                    } else {
                        s.reply_entry.emit_activate();
                    }
                }
            ));
        } else {
            button.connect_clicked(clone!(
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
                            if let Err(e) = IFace::action_invoked(
                                iface.signal_context(),
                                details.id,
                                action.clone(),
                            )
                            .await
                            {
                                error!(
                                    "Failed to invoke action: {} for window id: {}. Error: {:?}",
                                    action.key, details.id, e
                                );
                            }
                        }
                    ));
                }
            ));
        }

        button
    }

    pub fn close(&self, reason: Reason) {
        unsafe {
            self.inner.set_data("close-reason", reason);
        }
        self.inner.close();
    }

    pub fn has_default_action(&self) -> bool {
        self.inner
            .tooltip_text()
            .map_or(false, |text| !text.is_empty())
    }

    // we are changing keyboard_mode here to proper passive work
    // this is workaround but
    // if set this permanent then window will steal focus
    // but notification must be passive. Without interrupting user

    // FIXME probably this is not good idea to check it by css_classes
    pub fn toggle_hover(&self) {
        if self.inner.has_css_class("hover") {
            self.inner.set_keyboard_mode(KeyboardMode::None);
            self.inner.remove_css_class("hover");
            self.start_timeout();
        } else {
            self.inner.set_keyboard_mode(KeyboardMode::OnDemand);
            self.inner.add_css_class("hover");
            self.stop_timeout();
        }
    }

    fn build_widgets_tree(details: &Details) -> Self {
        let config = CONFIG.lock().unwrap().clone();

        let inner = gtk::Window::builder()
            .default_width(config.window_size.0)
            .default_height(config.window_size.1)
            .name("notification")
            .build();

        let app_name = gtk::Label::builder()
            .name("app_name")
            .justify(Justification::Left)
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .sensitive(false)
            .build();
        let app_icon = gtk::Image::builder()
            .name("app_icon")
            .hexpand(true)
            .halign(Align::End)
            .visible(true)
            .build();
        let app_name_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(5)
            .visible(CONFIG.lock().unwrap().show_app_name)
            .build();
        app_name_box.append(&app_name);
        if CONFIG.lock().unwrap().show_app_name {
            app_name_box.append(&app_icon);
        }

        let summary = gtk::Label::builder()
            .name("summary")
            .justify(Justification::Left)
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .use_markup(true)
            .build();

        let icon = gtk::Image::builder()
            .name("image")
            .visible(false)
            .pixel_size(CONFIG.lock().unwrap().icon_size)
            .valign(Align::Center)
            .halign(Align::End)
            .build();

        let summary_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(5)
            .build();
        summary_box.append(&summary);
        if !CONFIG.lock().unwrap().show_app_name {
            summary_box.append(&app_icon);
        }

        let body = gtk::Label::builder()
            .name("body")
            .justify(Justification::Left)
            .valign(Align::Fill)
            .halign(Align::Start)
            .wrap(true)
            .wrap_mode(pango::WrapMode::WordChar)
            .use_markup(true)
            .build();

        let reply_entry = gtk::Entry::builder()
            .name("reply-entry")
            .placeholder_text("Reply")
            .build();
        let reply_revealer = gtk::Revealer::builder()
            .name("reply-revealer")
            .reveal_child(false)
            .child(&reply_entry)
            .build();

        let actions_box = gtk::Box::builder()
            .name("actions")
            .orientation(Orientation::Horizontal)
            .spacing(5)
            .build();

        let content = gtk::Box::builder()
            .name("content")
            .orientation(Orientation::Vertical)
            .valign(Align::Start)
            .spacing(5)
            .build();
        content.append(&app_name_box);
        content.append(&summary_box);
        content.append(&body);
        content.append(&reply_revealer);

        let body_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(5)
            .build();
        body_box.append(&icon);
        body_box.append(&content);

        let main_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(5)
            .margin_top(5)
            .margin_start(5)
            .margin_bottom(5)
            .margin_end(5)
            .build();
        main_box.append(&body_box);
        main_box.append(&actions_box);

        inner.set_child(Some(&main_box));

        Self {
            id: details.id,
            app_name,
            app_icon,
            icon,
            summary,
            body,
            reply_entry,
            reply_revealer,
            actions_box,
            expire_timeout: details.expire_timeout,
            thandle: Default::default(),
            inner,
        }
    }

    pub fn from_details(value: Details, iface: Rc<IFaceRef>) -> Self {
        let mut _self = Self::build_widgets_tree(&value);
        _self.update_from_details(&value, iface.clone());

        // close_button_events
        let gesture_click = gtk::GestureClick::builder().build();
        _self.app_icon.add_controller(gesture_click.clone());
        gesture_click.connect_released(clone!(
            #[strong(rename_to=s)]
            _self,
            move |gesture, _, _, _| {
                s.close(Reason::Dismissed);

                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        ));

        let event_conntroller_motion = gtk::EventControllerMotion::new();
        _self
            .app_icon
            .add_controller(event_conntroller_motion.clone());

        event_conntroller_motion.connect_enter(clone!(
            #[strong(rename_to=app_icon)]
            _self.app_icon,
            move |_, _, _| {
                if app_icon.icon_name().is_some() || app_icon.file().is_some() {
                    if let Some(icon_name) = app_icon.icon_name() {
                        unsafe {
                            app_icon.set_data("icon-name", icon_name);
                        }
                    }
                    if let Some(file) = app_icon.file() {
                        unsafe {
                            app_icon.set_data("file", file);
                        }
                    }
                }

                app_icon.set_icon_name(Some(&CONFIG.lock().unwrap().window_close_icon.clone()));
            }
        ));

        event_conntroller_motion.connect_leave(clone!(
            #[strong(rename_to=app_icon)]
            _self.app_icon,
            move |_| {
                let icon_name = unsafe {
                    app_icon
                        .data::<glib::GString>("icon-name")
                        .map(|v| v.as_ref().clone())
                };
                let file = unsafe {
                    app_icon
                        .data::<glib::GString>("file")
                        .map(|v| v.as_ref().clone())
                };

                if let Some(icon_name) = icon_name {
                    app_icon.set_icon_name(Some(&icon_name));
                } else {
                    app_icon.set_from_file(file);
                }
            }
        ));
        // close_button_events_end

        // hover events

        // FIXME new window breaks focus
        // it invokes leave and notification can be lost while we are "holding" it
        let event_controller_motion = gtk::EventControllerMotion::new();
        _self.inner.add_controller(event_controller_motion.clone());
        event_controller_motion.connect_enter(clone!(
            #[strong(rename_to=s)]
            _self,
            move |_, _, _| {
                s.toggle_hover();
            }
        ));
        event_controller_motion.connect_leave(clone!(
            #[strong(rename_to=s)]
            _self,
            move |_| {
                s.toggle_hover();
            }
        ));

        // click_gestures

        // lmb
        let gesture_click_l = gtk::GestureClick::builder().button(1).build();
        _self.inner.add_controller(gesture_click_l.clone());
        gesture_click_l.connect_released(clone!(
            #[strong]
            iface,
            #[strong(rename_to=s)]
            _self,
            move |gesture, _, _, _| {
                debug!("Left mouse button released.");
                glib::spawn_future_local(clone!(
                    #[strong]
                    iface,
                    #[strong]
                    s,
                    async move {
                        if s.has_default_action() {
                            IFace::action_invoked(
                                iface.signal_context(),
                                value.id,
                                Action::default(),
                            )
                            .await
                            .unwrap();

                            s.close(Reason::Dismissed);
                        }
                    }
                ));

                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        ));

        // rmb
        let gesture_click_r = gtk::GestureClick::builder().button(3).build();
        _self.inner.add_controller(gesture_click_r.clone());
        gesture_click_r.connect_released(clone!(
            #[strong(rename_to=s)]
            _self,
            move |gesture, _, _, _| {
                debug!("Right mouse button released.");
                s.close(Reason::Dismissed);

                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        ));

        // click_gestures_end

        _self
    }
}
