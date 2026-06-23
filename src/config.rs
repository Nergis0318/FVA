use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{FvaError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub indexer: IndexerConfig,
    #[serde(default)]
    pub fff: FffConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default)]
    pub vector: VectorConfig,
    #[serde(default)]
    pub query: QueryConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default = "default_root")]
    pub root: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            root: default_root(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    #[serde(default = "default_true")]
    pub watch: bool,
    #[serde(default = "default_debounce")]
    pub debounce_ms: u64,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default = "default_true")]
    pub respect_gitignore: bool,
    #[serde(default = "default_true")]
    pub git_boost: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            watch: true,
            debounce_ms: default_debounce(),
            max_file_size: default_max_file_size(),
            languages: vec![],
            respect_gitignore: true,
            git_boost: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FffConfig {
    #[serde(default = "default_frecency_db")]
    pub frecency_db: String,
    #[serde(default = "default_history_db")]
    pub history_db: String,
    #[serde(default = "default_max_cached_files")]
    pub max_cached_files: usize,
    #[serde(default = "default_true")]
    pub enable_warmup: bool,
    #[serde(default = "default_true")]
    pub enable_content_indexing: bool,
}

impl Default for FffConfig {
    fn default() -> Self {
        Self {
            frecency_db: default_frecency_db(),
            history_db: default_history_db(),
            max_cached_files: default_max_cached_files(),
            enable_warmup: true,
            enable_content_indexing: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_embedding_provider")]
    pub provider: String,
    #[serde(default)]
    pub voyage_api_key: String,
    #[serde(default = "default_embedding_model")]
    pub model: String,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_dimensions")]
    pub dimensions: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: default_embedding_provider(),
            voyage_api_key: String::new(),
            model: default_embedding_model(),
            batch_size: default_batch_size(),
            dimensions: default_dimensions(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    #[serde(default = "default_vector_backend")]
    pub backend: String,
    #[serde(default = "default_vector_db")]
    pub db_path: String,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            backend: default_vector_backend(),
            db_path: default_vector_db(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    #[serde(default = "default_max_results")]
    pub default_max_results: usize,
    #[serde(default = "default_fff_weight")]
    pub fff_weight: f32,
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f32,
    #[serde(default = "default_graph_weight")]
    pub graph_weight: f32,
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: usize,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            default_max_results: default_max_results(),
            fff_weight: default_fff_weight(),
            vector_weight: default_vector_weight(),
            graph_weight: default_graph_weight(),
            max_context_tokens: default_max_context_tokens(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default = "default_server_name")]
    pub server_name: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub log_file: String,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            server_name: default_server_name(),
            log_level: default_log_level(),
            log_file: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub sandbox_indexing: bool,
    #[serde(default = "default_true")]
    pub no_telemetry: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            sandbox_indexing: true,
            no_telemetry: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            indexer: IndexerConfig::default(),
            fff: FffConfig::default(),
            embedding: EmbeddingConfig::default(),
            vector: VectorConfig::default(),
            query: QueryConfig::default(),
            mcp: McpConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

fn default_root() -> String {
    ".".to_string()
}
fn default_true() -> bool {
    true
}
fn default_debounce() -> u64 {
    300
}
fn default_max_file_size() -> u64 {
    10 * 1024 * 1024
}
fn default_frecency_db() -> String {
    ".fva/frecency".to_string()
}
fn default_history_db() -> String {
    ".fva/history".to_string()
}
fn default_max_cached_files() -> usize {
    30_000
}
fn default_embedding_provider() -> String {
    "local".to_string()
}
fn default_embedding_model() -> String {
    "voyage-code-3".to_string()
}
fn default_batch_size() -> usize {
    32
}
fn default_dimensions() -> usize {
    1024
}
fn default_vector_backend() -> String {
    "flat".to_string()
}
fn default_vector_db() -> String {
    "vectors".to_string()
}
fn default_max_results() -> usize {
    20
}
fn default_fff_weight() -> f32 {
    0.3
}
fn default_vector_weight() -> f32 {
    0.5
}
fn default_graph_weight() -> f32 {
    0.2
}
fn default_max_context_tokens() -> usize {
    8000
}
fn default_server_name() -> String {
    "fva".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let mut config = Self::default();

        if let Some(path) = path {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                let file_config: Config = toml::from_str(&content)?;
                config = file_config;
            }
        } else {
            for candidate in Self::config_search_paths() {
                if candidate.exists() {
                    let content = std::fs::read_to_string(&candidate)?;
                    let file_config: Config = toml::from_str(&content)?;
                    config = file_config;
                    break;
                }
            }
        }

        Ok(config)
    }

    pub fn config_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            paths.push(cwd.join("config.toml"));
            paths.push(cwd.join("fva.toml"));
        }
        if let Some(config_dir) = dirs::config_dir() {
            paths.push(config_dir.join("fva").join("config.toml"));
        }
        paths
    }

    pub fn resolve_root(&self, cli_override: Option<&str>) -> Result<PathBuf> {
        let root = cli_override.unwrap_or(&self.project.root);
        let path = PathBuf::from(root);
        let canonical = dunce::canonicalize(&path)
            .map_err(|e| FvaError::Config(format!("invalid project root '{}': {e}", root)))?;
        Ok(canonical)
    }

    pub fn resolve_data_dir(&self, root: &Path) -> PathBuf {
        root.join(".fva")
    }
}