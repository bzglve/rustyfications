use zbus::zvariant::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    key: String,
    text: String,
    pub icon: bool,
}

impl Action {
    pub fn new(key: &str, text: &str, icon: bool) -> Self {
        Self {
            key: key.to_owned(),
            text: text.to_owned(),
            icon,
        }
    }
}

// TODO need to be sure that this is like that
impl Default for Action {
    fn default() -> Self {
        Self {
            key: "default".to_owned(),
            text: "Default".to_owned(),
            icon: false,
        }
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for Action {
    fn to_string(&self) -> String {
        self.text.clone()
    }
}

impl serde::Serialize for Action {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.key)
    }
}

impl Type for Action {
    fn signature() -> zbus::zvariant::Signature<'static> {
        zbus::zvariant::Signature::from_str_unchecked("s")
    }
}
