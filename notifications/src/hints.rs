use std::collections::HashMap;

use zbus::zvariant::OwnedValue as Value;

#[derive(Debug)]
pub struct Hints {
    pub action_icons: bool,
}

impl From<HashMap<&str, Value>> for Hints {
    fn from(mut value: HashMap<&str, Value>) -> Self {
        let action_icons: bool = value
            .remove("action-icons")
            .and_then(|v| v.try_into().ok())
            .unwrap_or(false);

        Self { action_icons }
    }
}
