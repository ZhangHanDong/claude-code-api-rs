use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    pub server: ServerConfig,
    pub claude: ClaudeConfig,
    pub auth: AuthConfig,
    #[serde(default)]
    pub file_access: FileAccessConfig,
    #[serde(default)]
    pub mcp: MCPConfig,
    #[serde(default)]
    pub process_pool: ProcessPoolConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClaudeConfig {
    pub command: String,
    pub timeout_seconds: u64,
    pub max_concurrent_sessions: usize,
    #[serde(default)]
    pub use_interactive_sessions: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuthConfig {
    pub enabled: bool,
    pub secret_key: String,
    pub token_expiry_hours: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct FileAccessConfig {
    pub skip_permissions: bool,
    pub additional_dirs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MCPConfig {
    pub enabled: bool,
    pub config_file: Option<String>,
    pub config_json: Option<String>,
    pub strict: bool,
    pub debug: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProcessPoolConfig {
    pub size: usize,
    pub min_idle: usize,
    pub max_idle: usize,
}

impl Default for ProcessPoolConfig {
    fn default() -> Self {
        Self {
            size: 5,
            min_idle: 2,
            max_idle: 5,
        }
    }
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("claude.command", "claude")?
            .set_default("claude.timeout_seconds", 300)?
            .set_default("claude.max_concurrent_sessions", 10)?
            .set_default("claude.use_interactive_sessions", false)?
            .set_default("auth.enabled", false)?
            .set_default("auth.secret_key", "change-me-in-production")?
            .set_default("auth.token_expiry_hours", 24)?
            .set_default("file_access.skip_permissions", false)?
            .set_default("file_access.additional_dirs", Vec::<String>::new())?
            .set_default("mcp.enabled", false)?
            .set_default("mcp.strict", false)?
            .set_default("mcp.debug", false)?
            .add_source(File::with_name(&format!("config/{run_mode}")).required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(Environment::with_prefix("CLAUDE_CODE").separator("__"))
            .build()?;

        s.try_deserialize()
    }
}
