mod action;
mod hints;
mod id;
mod server_info;

use std::{cmp::Ordering, collections::HashMap, time::Duration, vec};

pub use action::Action;
use futures::channel::mpsc::Sender;
pub use hints::Hints;
pub use id::Id;
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

static BUS_NAME: &str = "org.freedesktop.Notifications";
static BUS_OBJECT_PATH: &str = "/org/freedesktop/Notifications";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Details {
    pub id: u32,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
    pub summary: String,
    pub body: Option<String>,
    pub actions: Vec<Action>,
    pub hints: Hints,
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
    #[allow(dead_code)]
    Undefined = 4,
}

#[derive(Debug)]
pub struct IFace {
    server_info: ServerInfo,
    sender: Sender<Message>,
}

#[interface(name = "org.freedesktop.Notifications")]
impl IFace {
    fn get_capabilities(&self) -> Vec<&str> {
        // TODO this list probably can be in config
        // to allow user modify what he want to use
        // e.g. disable sound or icons
        // all other stuff that implemented one of capabilities should also check and skip processing if off
        vec![
            "action-icons",
            "actions",
            "body",
            "body-hyperlinks",
            // "body-images",
            "body-markup",
            // "icon-multi",
            "icon-static",
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
        actions: Vec<&str>,
        hints: HashMap<&str, Value>,
        expire_timeout: i32,
    ) -> u32 {
        let notification_id = if replaces_id != 0 && replaces_id <= Id::current_glob() {
            replaces_id
        } else {
            Id::bump_glob()
        };

        trace!("raw hints keys: {:?}", hints.keys());
        let hints: Hints = Hints::from(hints);

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
            summary: format!("<b>{}</b>", summary), // TODO if markup feature is enabled
            body: if body.is_empty() {
                None
            } else {
                Some(body.to_owned())
            },
            actions: actions
                .chunks_exact(2)
                .map(|t| Action::new(t[0], t[1]))
                .collect(),
            hints,
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
        action_key: Action,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn activation_token(
        ctxt: &SignalContext<'_>,
        id: u32,
        activation_token: &str,
    ) -> zbus::Result<()>;
}

impl IFace {
    pub fn new(server_info: ServerInfo, sender: Sender<Message>) -> Self {
        Self {
            server_info,
            sender,
        }
    }

    pub fn connect(self) -> Result<InterfaceRef<Self>, zbus::Error> {
        let connection = ConnectionBuilder::session()?
            .name(BUS_NAME)?
            .serve_at(BUS_OBJECT_PATH, self)?
            .build()?;

        let i = connection.object_server().interface(BUS_OBJECT_PATH)?;
        Ok(i)
    }
}

pub type IFaceRef = InterfaceRef<IFace>;
