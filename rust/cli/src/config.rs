use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub starting_stack: u32,
    pub level: u8,
    pub seed: Option<u64>,
    pub adaptive: bool,
    pub ai_version: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueSource {
    Default,
    File,
    Env,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigSources {
    pub starting_stack: ValueSource,
    pub level: ValueSource,
    pub seed: ValueSource,
    pub adaptive: ValueSource,
    pub ai_version: ValueSource,
}

impl Default for ConfigSources {
    fn default() -> Self {
        Self {
            starting_stack: ValueSource::Default,
            level: ValueSource::Default,
            seed: ValueSource::Default,
            adaptive: ValueSource::Default,
            ai_version: ValueSource::Default,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigResolved {
    pub config: Config,
    pub sources: ConfigSources,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            starting_stack: 20_000,
            level: 1,
            seed: None,
            adaptive: true,
            ai_version: "latest".into(),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Invalid(String),
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}
impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Parse(e)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[allow(dead_code)]
pub fn load() -> Result<Config, ConfigError> {
    load_with_sources().map(|resolved| resolved.config)
}

pub fn load_with_sources() -> Result<ConfigResolved, ConfigError> {
    let mut cfg = Config::default();
    let mut sources = ConfigSources::default();

    if let Ok(path) = std::env::var("axiomind_CONFIG") {
        let s = fs::read_to_string(path)?;
        let f: FileConfig = toml::from_str(&s)?;
        if let Some(v) = f.starting_stack {
            cfg.starting_stack = v;
            sources.starting_stack = ValueSource::File;
        }
        if let Some(v) = f.level {
            cfg.level = v;
            sources.level = ValueSource::File;
        }
        if let Some(v) = f.seed {
            cfg.seed = Some(v);
            sources.seed = ValueSource::File;
        }
        if let Some(v) = f.adaptive {
            cfg.adaptive = v;
            sources.adaptive = ValueSource::File;
        }
        if let Some(v) = f.ai_version {
            cfg.ai_version = v;
            sources.ai_version = ValueSource::File;
        }
    }

    if let Ok(seed) = std::env::var("axiomind_SEED")
        && !seed.is_empty()
    {
        cfg.seed = Some(
            seed.parse()
                .map_err(|_| ConfigError::Invalid("Invalid seed".into()))?,
        );
        sources.seed = ValueSource::Env;
    }
    if let Ok(level) = std::env::var("axiomind_LEVEL")
        && !level.is_empty()
    {
        cfg.level = level
            .parse()
            .map_err(|_| ConfigError::Invalid("Invalid level".into()))?;
        sources.level = ValueSource::Env;
    }
    if let Ok(adap) = std::env::var("axiomind_ADAPTIVE")
        && !adap.is_empty()
    {
        cfg.adaptive =
            parse_bool(&adap).ok_or_else(|| ConfigError::Invalid("Invalid adaptive".into()))?;
        sources.adaptive = ValueSource::Env;
    }
    if let Ok(ver) = std::env::var("axiomind_AI_VERSION")
        && !ver.is_empty()
    {
        cfg.ai_version = ver;
        sources.ai_version = ValueSource::Env;
    }

    validate(&cfg)?;
    Ok(ConfigResolved {
        config: cfg,
        sources,
    })
}

#[derive(Debug, Deserialize)]
struct FileConfig {
    #[serde(default)]
    starting_stack: Option<u32>,
    #[serde(default)]
    level: Option<u8>,
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    adaptive: Option<bool>,
    #[serde(default)]
    ai_version: Option<String>,
}

fn validate(cfg: &Config) -> Result<(), ConfigError> {
    if cfg.level == 0 {
        return Err(ConfigError::Invalid(
            "Invalid configuration: level must be >=1".into(),
        ));
    }
    if cfg.starting_stack == 0 {
        return Err(ConfigError::Invalid(
            "Invalid configuration: starting_stack must be >0".into(),
        ));
    }
    Ok(())
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}
