use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

pub use window::Window;

pub type RuntimeData = Rc<RefCell<_RuntimeData>>;

#[derive(Default)]
pub struct _RuntimeData {
    pub windows: BTreeMap<u32, Window>,
}

mod window {
    use std::{cell::RefCell, rc::Rc, time::Duration};

    use gtk::{
        glib::{self, clone, JoinHandle},
        pango::{self, EllipsizeMode},
        prelude::*,
        Align, Justification, Orientation,
    };
    use notifications::Details;

    use crate::DEFAULT_EXPIRE_TIMEOUT;

    #[derive(Clone)]
    pub struct Window {
        id: u32,
        // app_name: gtk::Label,
        // app_icon: gtk::Image, // TODO maybe search for icon type
        pub summary: gtk::Label,
        pub body: gtk::Label,
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

    impl From<Details> for Window {
        fn from(value: Details) -> Self {
            // let app_name = gtk::Label::builder()
            //     .label(value.app_name.unwrap_or_default())
            //     .visible(value.app_name.is_some())
            //     .name("app_name")
            //     .sensitive(false)
            //     .build();
            // let app_icon = gtk::Image::builder().visible(false).build();
            let summary = gtk::Label::builder()
                .label(value.summary)
                .name("summary")
                .justify(Justification::Left)
                .halign(Align::Start)
                .ellipsize(EllipsizeMode::End)
                .build();

            let body = gtk::Label::builder()
                .label(value.body.clone().unwrap_or_default())
                .visible(value.body.is_some())
                .name("body")
                .sensitive(false)
                .justify(Justification::Left)
                .valign(Align::Start)
                .halign(Align::Start)
                .wrap(true)
                .wrap_mode(pango::WrapMode::WordChar)
                .use_markup(true)
                .build();

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
                inner,
                expire_timeout: value.expire_timeout.unwrap_or(DEFAULT_EXPIRE_TIMEOUT),
                thandle: Default::default(),
            }
        }
    }
}
