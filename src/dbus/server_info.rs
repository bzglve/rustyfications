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
