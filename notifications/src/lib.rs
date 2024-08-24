use std::{cmp::Ordering, collections::HashMap, time::Duration, vec};

use futures::channel::mpsc::Sender;
#[allow(unused_imports)]
use log::*;
pub use server_info::ServerInfo;
pub use zbus::blocking::object_server::InterfaceRef;
use zbus::{
    blocking::connection::Builder as ConnectionBuilder,
    interface,
    object_server::SignalContext,
    zvariant::{OwnedValue as Value, Type},
};

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum Action {}

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum Hint {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Details {
    pub id: u32,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
    pub summary: String,
    pub body: Option<String>,
    // pub actions: Vec<Action>,  // TODO
    // pub hints: Vec<Hint>, // TODO
    pub expire_timeout: Option<Duration>,
}

#[derive(Debug)]
pub enum Message {
    New(Details),
    Replace(Details),
    Close(u32),
}

/// The reason the notification was closed
#[derive(serde::Serialize, Type)]
#[repr(u32)]
pub enum Reason {
    /// The notification expired
    Expired = 1,
    /// The notification was dismissed by the user
    Dismissed = 2,
    /// The notification was closed by a call to [CloseNotification](NotificationsIFace::close_notification())
    Closed = 3,
    /// Undefined/reserved reasons
    Undefined = 4,
}

mod server_info {
    use super::Type;

    #[derive(serde::Serialize, Type, Debug, Clone)]
    pub struct ServerInfo {
        /// The product name of the server.
        name: String,
        /// The vendor name. For example, "KDE," "GNOME," "freedesktop.org," or "Microsoft."
        vendor: String,
        /// The server's version number.
        version: String,
        /// The specification version the server is compliant with.
        spec_version: String,
    }

    impl ServerInfo {
        pub fn new(name: &str, vendor: &str, version: &str, spec_version: &str) -> Self {
            Self {
                name: name.to_owned(),
                vendor: vendor.to_owned(),
                version: version.to_owned(),
                spec_version: spec_version.to_owned(),
            }
        }
    }

    impl From<ServerInfo> for (String, String, String, String) {
        fn from(value: ServerInfo) -> Self {
            (value.name, value.vendor, value.version, value.spec_version)
        }
    }
}

#[derive(Debug)]
pub struct IFace {
    notify_counter: u32,
    server_info: ServerInfo,
    sender: Sender<Message>,
}

#[interface(name = "org.freedesktop.Notifications")]
impl IFace {
    fn get_capabilities(&self) -> Vec<&str> {
        vec![
            // "action-icons",
            // "actions",
            "body",
            // "body-hyperlinks",
            // "body-images",
            // "body-markup",
            // "icon-multi",
            // "icon-static",
            // "persistence",
            // "sound",
        ]
    }

    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &mut self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        _actions: Vec<&str>,
        _hints: HashMap<&str, Value>,
        expire_timeout: i32,
    ) -> u32 {
        let notification_id = if replaces_id != 0 && replaces_id <= self.notify_counter {
            replaces_id
        } else {
            self.notify_counter += 1;
            self.notify_counter
        };

        let details = Details {
            id: notification_id,
            app_name: if app_name.is_empty() {
                None
            } else {
                Some(app_name.to_owned())
            },
            app_icon: if app_icon.is_empty() {
                None
            } else {
                Some(app_icon.to_owned())
            },
            summary: summary.to_owned(),
            body: if body.is_empty() {
                None
            } else {
                Some(body.to_owned())
            },
            expire_timeout: match expire_timeout.cmp(&0) {
                Ordering::Less => None,
                Ordering::Equal => Some(Duration::MAX),
                Ordering::Greater => Some(Duration::from_millis(expire_timeout as u64)),
            },
        };

        if notification_id != replaces_id {
            if let Err(e) = self.sender.try_send(Message::New(details)) {
                error!("Failed to send notification message: {}", e);
            }
        } else if let Err(e) = self.sender.try_send(Message::Replace(details)) {
            error!("Failed to send notification message: {}", e);
        }

        notification_id
    }

    async fn close_notification(
        &mut self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
        id: u32,
    ) {
        if let Err(e) = self.sender.try_send(Message::Close(id)) {
            error!("Failed to send close notification message: {}", e);
        }
        if let Err(e) = Self::notification_closed(&ctxt, id, Reason::Closed).await {
            error!("Failed to emit notification closed signal: {}", e);
        }
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        self.server_info.clone().into()
    }

    #[zbus(signal)]
    pub async fn notification_closed(
        ctxt: &SignalContext<'_>,
        id: u32,
        reason: Reason,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn action_invoked(
        ctxt: &SignalContext<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn activation_token(
        ctxt: &SignalContext<'_>,
        id: u32,
        activation_token: &str,
    ) -> zbus::Result<()>;
}

static BUS_NAME: &str = "org.freedesktop.Notifications";
static BUS_OBJECT_PATH: &str = "/org/freedesktop/Notifications";

impl IFace {
    pub fn connect(
        server_info: ServerInfo,
        sender: Sender<Message>,
    ) -> Result<InterfaceRef<Self>, zbus::Error> {
        let iface = Self {
            notify_counter: 0,
            server_info,
            sender,
        };
        let connection = ConnectionBuilder::session()?
            .name(BUS_NAME)?
            .serve_at(BUS_OBJECT_PATH, iface)?
            .build()?;

        let i = connection.object_server().interface(BUS_OBJECT_PATH)?;
        Ok(i)
    }
}

pub type IFaceRef = InterfaceRef<IFace>;
