use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub title: String,
    pub description: Option<String>,
    pub author: String,
    pub url: Option<String>,
    pub posts_dir: String,
    pub output_dir: String,
    pub openai_api_key: Option<String>,
    pub theme: Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub primary_color: String,
    pub background_color: String,
    pub text_color: String,
    pub accent_color: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "Scribe".to_string(),
            description: Some("A minimal static site generator • ink • eternal".to_string()),
            author: "Author".to_string(),
            url: None,
            posts_dir: "posts".to_string(),
            output_dir: "dist".to_string(),
            openai_api_key: None,
            theme: Theme::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary_color: "#f5f5f5".to_string(),
            background_color: "#0a0a0a".to_string(),
            text_color: "#f5f5f5".to_string(),
            accent_color: "#8b8b8b".to_string(),
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = if path.as_ref().exists() {
            let content = fs::read_to_string(path)
                .context("Failed to read config file")?;
            let config: Config = serde_json::from_str(&content)
                .context("Failed to parse config file")?;
            config
        } else {
            // Create default config
            let config = Config::default();
            let content = serde_json::to_string_pretty(&config)
                .context("Failed to serialize default config")?;
            fs::write(path, content)
                .context("Failed to write default config")?;
            config
        };
        
        // Load OpenAI API key from environment variable (like the JS version)
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            config.openai_api_key = Some(api_key);
        }
        
        Ok(config)
    }
} 