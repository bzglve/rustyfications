use std::collections::HashMap;

pub use idata::IData;
use zbus::zvariant::OwnedValue as Value;

mod idata {
    use gtk::{
        gdk_pixbuf::{Colorspace, Pixbuf},
        glib,
    };
    use zbus::zvariant::OwnedValue as Value;

    #[derive(Clone, PartialEq, Eq, Value)]
    pub struct IData {
        /// Width of image in pixels
        width: i32,
        /// Height of image in pixels
        height: i32,
        /// Distance in bytes between row starts
        rowstride: i32,
        /// Whether the image has an alpha channel
        has_alpha: bool,
        /// Must always be 8
        bits_per_sample: i32,
        /// If has_alpha is TRUE, must be 4, otherwise 3
        channels: i32,
        /// The image data, in RGB byte order
        data: Vec<u8>,
    }

    impl IData {
        pub fn data_debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("IData").field("data", &self.data).finish()
        }
    }

    impl std::fmt::Debug for IData {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("IData")
                .field("width", &self.width)
                .field("height", &self.height)
                .field("rowstride", &self.rowstride)
                .field("has_alpha", &self.has_alpha)
                .field("bits_per_sample", &self.bits_per_sample)
                .field("channels", &self.channels)
                .field("data", &"[...]")
                .finish()
        }
    }

    impl From<IData> for Pixbuf {
        fn from(value: IData) -> Self {
            Self::from_bytes(
                &glib::Bytes::from(&value.data),
                Colorspace::Rgb,
                value.has_alpha,
                value.bits_per_sample,
                value.width,
                value.height,
                value.rowstride,
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hints {
    pub action_icons: bool,
    pub desktop_entry: Option<String>,
    pub image_data: Option<IData>,
    pub image_path: Option<String>,
    pub icon_data: Option<IData>,
}

impl From<HashMap<&str, Value>> for Hints {
    fn from(mut value: HashMap<&str, Value>) -> Self {
        let action_icons: bool = value
            .remove("action-icons")
            .and_then(|v| v.try_into().ok())
            .unwrap_or(false);

        let desktop_entry = {
            let v: Option<String> = value
                .remove("desktop-entry")
                .and_then(|v| v.try_into().ok());
            if let Some(s) = v {
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            } else {
                v
            }
        };

        let image_data = value.remove("image-data").and_then(|v| v.try_into().ok());
        let image_path = {
            let v: Option<String> = value.remove("image-path").and_then(|v| v.try_into().ok());
            if let Some(s) = v {
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            } else {
                v
            }
        };
        let icon_data = value.remove("icon_data").and_then(|v| v.try_into().ok());

        Self {
            action_icons,
            desktop_entry,
            image_data,
            image_path,
            icon_data,
        }
    }
}
