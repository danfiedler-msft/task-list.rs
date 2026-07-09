use std::env;

/// Deployment environment, selected by `TASKLIST_ENV`. In later tasks this steers the
/// adapters between local emulators (`Local`) and real Azure via managed identity
/// (`Cloud`). For the skeleton it is just the seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Environment {
    #[default]
    Local,
    Cloud,
}

impl Environment {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "cloud" => Self::Cloud,
            _ => Self::Local,
        }
    }
}

/// Runtime configuration loaded from the environment. Non-secret only; secrets arrive via
/// Key Vault in the cloud (a later task).
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Selected environment (`TASKLIST_ENV`).
    pub environment: Environment,
    /// Socket address the API binds to (`TASKLIST_BIND_ADDRESS`).
    pub bind_address: String,
    /// Directory containing the built SPA to serve (`TASKLIST_WEB_DIST`).
    pub web_dist_dir: String,
}

impl AppConfig {
    /// Load configuration from environment variables, applying skeleton defaults.
    pub fn from_env() -> Self {
        let environment = env::var("TASKLIST_ENV")
            .map(|v| Environment::parse(&v))
            .unwrap_or_default();
        let bind_address =
            env::var("TASKLIST_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
        let web_dist_dir = env::var("TASKLIST_WEB_DIST").unwrap_or_else(|_| "web/dist".to_owned());

        Self {
            environment,
            bind_address,
            web_dist_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cloud_case_insensitively() {
        assert_eq!(Environment::parse("Cloud"), Environment::Cloud);
        assert_eq!(Environment::parse("CLOUD"), Environment::Cloud);
    }

    #[test]
    fn defaults_to_local_for_anything_else() {
        assert_eq!(Environment::parse(""), Environment::Local);
        assert_eq!(Environment::parse("local"), Environment::Local);
        assert_eq!(Environment::parse("prod"), Environment::Local);
    }
}
