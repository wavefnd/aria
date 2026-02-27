pub const TARGET_JAVA_VERSION: &str = include_str!("../../../VERSION_JAVA");

pub fn target_java_major() -> String {
    TARGET_JAVA_VERSION
        .trim()
        .split('.')
        .next()
        .unwrap_or("17")
        .to_string()
}

pub const BACKEND_ENV: &str = "ARIA_JAVAC_BACKEND";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Bootstrap,
    Aria,
}

impl BackendKind {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "bootstrap" => Ok(Self::Bootstrap),
            "aria" => Ok(Self::Aria),
            _ => Err(format!(
                "Unknown backend '{}'. Use one of: bootstrap, aria.",
                value
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::Aria => "aria",
        }
    }
}
