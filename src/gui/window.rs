use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

use gtk::{
    gdk_pixbuf::Pixbuf,
    gio,
    glib::{self, clone, JoinHandle},
    pango::{self, EllipsizeMode},
    prelude::*,
    Align, Justification, Orientation,
};
#[allow(unused_imports)]
use log::*;

use crate::{
    dbus::{Details, IFace, IFaceRef, Reason},
    types::RuntimeData,
    ICON_SIZE, WINDOW_CLOSE_ICON,
};

use super::utils::{init_layer_shell, pixbuf};

#[derive(Clone)]
pub struct Window {
    pub id: u32,
    // app_name: gtk::Label,
    icon: gtk::Image,
    summary: gtk::Label,
    app_icon: gtk::Image,
    body: gtk::Label,
    actions_box: gtk::Box,
    expire_timeout: Duration,
    thandle: Rc<RefCell<Option<JoinHandle<()>>>>,
    pub inner: gtk::Window,
}

impl Window {
    pub fn stop_timeout(&self) {
        debug!("Stopping timeout for window with id: {}", self.id);
        if let Some(h) = self.thandle.borrow().as_ref() {
            h.abort();
            info!("Timeout aborted for window id: {}", self.id);
        }
        *self.thandle.borrow_mut() = None;
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
}

impl Window {
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

    pub fn update_from_details(&mut self, value: &Details, iface: Rc<IFaceRef>) {
        debug!("Updating window from details: {:?}", value);
        if self.thandle.borrow().is_some() {
            self.stop_timeout();
        }

        let value = value.clone();

        // // TODO visibility of app name should depend on configuration
        // self.app_name
        //     .set_label(&value.app_name.clone().unwrap_or_default());
        // self.app_name.set_visible(value.app_name.is_some());

        self.summary.set_label(&value.summary);

        let mut desktop_entry: Option<gio::DesktopAppInfo> = None;
        if let Some(de) = value.hints.desktop_entry {
            desktop_entry = gio::DesktopAppInfo::new(&de);

            if desktop_entry.is_none() {
                desktop_entry = gio::DesktopAppInfo::new(&format!("{}.desktop", de));
            }
        }
        if desktop_entry.is_none() && value.app_name.is_some() {
            let an = value.app_name.unwrap();
            desktop_entry = gio::DesktopAppInfo::new(&an);

            if desktop_entry.is_none() {
                desktop_entry = gio::DesktopAppInfo::new(&format!("{}.desktop", an));
            }

            if desktop_entry.is_none() {
                desktop_entry = gio::DesktopAppInfo::new(&an.to_lowercase());
            }

            if desktop_entry.is_none() {
                desktop_entry = gio::DesktopAppInfo::new(&format!("{}.desktop", an.to_lowercase()));
            }
        }
        trace!("desktop_entry: {:?}", desktop_entry);

        if let Some(app_info) = desktop_entry {
            let icon = app_info.icon();
            if let Some(icon) = icon {
                let icon = icon.to_string().unwrap();

                if PathBuf::from(icon.clone()).is_absolute() {
                    self.app_icon.set_from_file(Some(icon));
                } else {
                    self.app_icon.set_icon_name(Some(&icon));
                }
            }
        } else {
            self.app_icon.set_icon_name(Some(WINDOW_CLOSE_ICON));
        }

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
                                if let Err(e) = IFace::action_invoked(iface.signal_context(), value.id, action.clone()).await {
                                    error!("Failed to invoke action: {} for window id: {}. Error: {:?}", action.key, value.id, e);
                                } else {
                                    info!("Action invoked: {} for window id: {}", action.key, value.id);
                                }
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

        self.expire_timeout = value.expire_timeout;
        debug!("Window update complete for id: {}", self.id);
    }

    fn build_widgets_tree(value: &Details) -> Self {
        trace!("Building widget tree for window id: {}", value.id);

        let inner = gtk::Window::builder()
            // optimal size to display 40 chars in 12px font and 5px margin
            .default_width(410)
            .default_height(30)
            .name("notification")
            .build();

        // let app_name = gtk::Label::builder()
        //     .name("app_name")
        //     .justify(Justification::Left)
        //     .halign(Align::Start)
        //     .ellipsize(EllipsizeMode::End)
        //     .sensitive(false)
        //     .build();

        let summary_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(5)
            .build();

        let summary = gtk::Label::builder()
            .name("summary")
            .justify(Justification::Left)
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .use_markup(true)
            .build();

        let app_icon = gtk::Image::builder()
            .name("app_icon")
            .hexpand(true)
            .halign(Align::End)
            .visible(true)
            .build();

        let event_conntroller_motion = gtk::EventControllerMotion::new();
        app_icon.add_controller(event_conntroller_motion.clone());

        event_conntroller_motion.connect_enter(clone!(
            #[strong]
            app_icon,
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

                app_icon.set_icon_name(Some(WINDOW_CLOSE_ICON));
            }
        ));

        event_conntroller_motion.connect_leave(clone!(
            #[strong]
            app_icon,
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

        summary_box.append(&summary);
        summary_box.append(&app_icon);

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
            .valign(Align::Start)
            .spacing(5)
            .build();
        // main_box.append(&app_name);
        main_box.append(&summary_box);
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

        inner.set_child(Some(&act_out_box));

        trace!("Widget tree built for window id: {}", value.id);
        Self {
            id: value.id,
            // app_name,
            icon,
            summary,
            app_icon,
            body,
            actions_box,
            expire_timeout: value.expire_timeout,
            thandle: Default::default(),
            inner,
        }
    }

    pub fn from_details(value: Details, iface: Rc<IFaceRef>) -> Self {
        info!("Creating window from details for id: {}", value.id);
        let mut _self = Self::build_widgets_tree(&value);

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

        _self.update_from_details(&value, iface);
        _self
    }

    pub fn close(&self, reason: Reason) {
        unsafe {
            self.inner.set_data("close-reason", reason);
        }
        self.inner.close();
    }

    pub fn has_default_action(&self) -> bool {
        if let Some(tooltip_text) = self.inner.tooltip_text() {
            return !tooltip_text.is_empty();
        }
        false
    }
}
