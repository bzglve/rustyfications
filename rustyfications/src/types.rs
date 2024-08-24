use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

pub use window::Window;

pub type RuntimeData = Rc<RefCell<_RuntimeData>>;

#[derive(Default)]
pub struct _RuntimeData {
    pub windows: BTreeMap<u32, Window>,
}

mod window {
    use gtk::{pango::EllipsizeMode, prelude::*, Align, Justification, Orientation};
    use notifications::Details;

    #[derive(Clone)]
    pub struct Window {
        // id: u32,
        // app_name: gtk::Label,
        // app_icon: gtk::Image, // TODO maybe search for icon type
        pub summary: gtk::Label,
        pub body: gtk::Label,
        // expire_timeout:
        pub inner: gtk::Window,
    }

    impl Window {}

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
                // .wrap(true)
                // .wrap_mode(WrapMode::Char)
                // .width_chars(40) // TODO check that we can hold 40 characters
                .build();

            let body = gtk::Label::builder()
                .label(value.body.clone().unwrap_or_default())
                .visible(value.body.is_some())
                .name("body")
                .sensitive(false)
                .justify(Justification::Left)
                .halign(Align::Start)
                .ellipsize(EllipsizeMode::End)
                .build();

            let main_box = gtk::Box::builder()
                .orientation(Orientation::Vertical)
                .spacing(6)
                .margin_top(6)
                .margin_start(6)
                .margin_bottom(6)
                .margin_end(6)
                .build();
            main_box.append(&summary);
            main_box.append(&body);

            let inner = gtk::Window::builder()
                .width_request(300) // TODO rm one of theese two
                .default_width(300)
                .build();
            inner.set_child(Some(&main_box));

            Self {
                // id: value.id,
                summary,
                body,
                inner,
            }
        }
    }
}
