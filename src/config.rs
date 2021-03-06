use log::debug;
use log::error;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Holds the complete config in an aggregated manner.
#[derive(Debug, Deserialize)]
pub struct ShellConfig {
    version: usize,
    clusters: Vec<ClusterConfig>,
}

impl ShellConfig {
    /// Tries to load the configuration from different paths.
    ///
    /// It first tries the `.cbsh/config` in the current directory, and if not found there
    /// it then tries the home directory (so `~/.cbsh/config`).
    pub fn new() -> Self {
        let mut config = try_config_from_path(std::env::current_dir().unwrap())
            .or_else(|| try_config_from_path(dirs::home_dir().unwrap()))
            .unwrap_or(ShellConfig::default());

        let standalone_credentials = try_credentials_from_path(std::env::current_dir().unwrap())
            .or_else(|| try_credentials_from_path(dirs::home_dir().unwrap()));

        if let Some(standalone) = standalone_credentials {
            for value in config.clusters_mut() {
                let identifier = value.identifier().to_owned();
                let mut config_credentials = value.credentials_mut();

                for cred in &standalone.clusters {
                    if cred.identifier() == identifier {
                        if config_credentials.username.is_none() && cred.username.is_some() {
                            config_credentials.username = cred.username.clone()
                        }
                        if config_credentials.password.is_none() && cred.password.is_some() {
                            config_credentials.password = cred.password.clone()
                        }
                    }
                }
            }
        }

        config
    }

    /// Builds the config from a raw input string.
    pub fn from_str(input: &str) -> Self {
        // Note: ideally this propagates up into a central error handling facility,
        // but for now just logging it nicely and bailing out is probably goint to be fine.
        match toml::from_str(input) {
            Ok(i) => i,
            Err(e) => {
                error!("Failed to parse config file: {}", e);
                std::process::exit(-1);
            }
        }
    }

    /// Returns the individual configurations for all the clusters configured.
    pub fn clusters(&self) -> &Vec<ClusterConfig> {
        &self.clusters
    }

    pub fn clusters_mut(&mut self) -> &mut Vec<ClusterConfig> {
        &mut self.clusters
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            clusters: vec![],
            version: 1,
        }
    }
}

fn try_config_from_path(mut path: PathBuf) -> Option<ShellConfig> {
    path.push(".cbsh");
    path.push("config");

    let read = fs::read_to_string(&path);
    match read {
        Ok(r) => Some(ShellConfig::from_str(&r)),
        Err(e) => {
            debug!("Could not locate {:?} becaue of {:?}", path, e);
            None
        }
    }
}

fn try_credentials_from_path(mut path: PathBuf) -> Option<StandaloneCredentialsConfig> {
    path.push(".cbsh");
    path.push("credentials");

    let read = fs::read_to_string(&path);
    match read {
        Ok(r) => Some(StandaloneCredentialsConfig::from_str(&r)),
        Err(e) => {
            debug!("Could not locate {:?} becaue of {:?}", path, e);
            None
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ClusterConfig {
    identifier: String,
    hostnames: Vec<String>,
    #[serde(rename(deserialize = "default-bucket"))]
    default_bucket: Option<String>,
    #[serde(rename(deserialize = "default-scope"))]
    default_scope: Option<String>,
    #[serde(rename(deserialize = "default-collection"))]
    default_collection: Option<String>,

    #[serde(flatten)]
    credentials: ClusterCredentials,
    #[serde(flatten)]
    timeouts: ClusterConfigTimeouts,
    #[serde(flatten)]
    tls: ClusterTlsConfig,
}

impl ClusterConfig {
    pub fn identifier(&self) -> &str {
        self.identifier.as_ref()
    }

    pub fn hostnames(&self) -> &Vec<String> {
        &self.hostnames
    }
    pub fn username(&self) -> String {
        self.credentials.username.as_ref().unwrap().clone()
    }
    pub fn password(&self) -> String {
        self.credentials.password.as_ref().unwrap().clone()
    }
    pub fn default_bucket(&self) -> Option<String> {
        self.default_bucket.as_ref().map(|s| s.clone())
    }
    pub fn default_scope(&self) -> Option<String> {
        self.default_scope.as_ref().map(|s| s.clone())
    }
    pub fn default_collection(&self) -> Option<String> {
        self.default_collection.as_ref().map(|s| s.clone())
    }
    pub fn credentials_mut(&mut self) -> &mut ClusterCredentials {
        &mut self.credentials
    }
    pub fn timeouts(&self) -> &ClusterConfigTimeouts {
        &self.timeouts
    }
    pub fn tls(&self) -> &ClusterTlsConfig {
        &self.tls
    }
}

#[derive(Debug, Deserialize)]
pub struct ClusterCredentials {
    username: Option<String>,
    password: Option<String>,
}

impl ClusterCredentials {}

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterConfigTimeouts {
    #[serde(default)]
    #[serde(rename(deserialize = "data-timeout"), with = "humantime_serde")]
    data_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(rename(deserialize = "connect-timeout"), with = "humantime_serde")]
    connect_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(rename(deserialize = "query-timeout"), with = "humantime_serde")]
    query_timeout: Option<Duration>,
}

impl Default for ClusterConfigTimeouts {
    fn default() -> Self {
        ClusterConfigTimeouts {
            data_timeout: Some(Duration::from_millis(2500)),
            connect_timeout: Some(Duration::from_millis(7000)),
            query_timeout: Some(Duration::from_millis(75000)),
        }
    }
}

impl ClusterConfigTimeouts {
    pub fn data_timeout(&self) -> Option<&Duration> {
        self.data_timeout.as_ref()
    }

    pub fn query_timeout(&self) -> Option<&Duration> {
        self.query_timeout.as_ref()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClusterTlsConfig {
    #[serde(rename(deserialize = "tls-cert-path"))]
    cert_path: Option<String>,
    #[serde(rename(deserialize = "tls-validate-hostnames"))]
    #[serde(default = "default_as_true")]
    validate_hostnames: bool,
    #[serde(rename(deserialize = "tls-accept-all-certs"))]
    #[serde(default = "default_as_false")]
    accept_all_certs: bool,
}

impl ClusterTlsConfig {
    pub fn enabled(&self) -> bool {
        self.accept_all_certs || self.cert_path.is_some()
    }

    pub fn cert_path(&self) -> &Option<String> {
        &self.cert_path
    }

    pub fn validate_hostnames(&self) -> bool {
        self.validate_hostnames
    }

    pub fn accept_all_certs(&self) -> bool {
        self.accept_all_certs
    }
}

impl Default for ClusterTlsConfig {
    fn default() -> Self {
        Self {
            cert_path: None,
            validate_hostnames: true,
            accept_all_certs: false,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StandaloneCredentialsConfig {
    version: usize,
    clusters: Vec<StandaloneClusterCredentials>,
}

impl StandaloneCredentialsConfig {
    /// Builds the config from a raw input string.
    pub fn from_str(input: &str) -> Self {
        toml::from_str(input).unwrap()
    }
}

impl Default for StandaloneCredentialsConfig {
    fn default() -> Self {
        Self {
            clusters: vec![],
            version: 1,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StandaloneClusterCredentials {
    identifier: String,
    username: Option<String>,
    password: Option<String>,
}

impl StandaloneClusterCredentials {
    fn identifier(&self) -> &str {
        self.identifier.as_ref()
    }
}

fn default_as_false() -> bool {
    false
}
fn default_as_true() -> bool {
    true
}
