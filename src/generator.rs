use crate::config::Config;
use crate::templates;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::*;
use markdown::to_html;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::path::{Path};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub slug: String,
    pub original_slug: String,
    pub title: String,
    pub date: DateTime<Utc>,
    pub excerpt: Option<String>,
    pub content: String,
    pub html_content: String,
    pub first_letter: Option<char>,
    pub frontmatter: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub struct SiteGenerator {
    config: Config,
    posts: Vec<Post>,
}

impl SiteGenerator {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            posts: Vec::new(),
        }
    }

    pub async fn generate(&mut self) -> Result<()> {
        println!("{}", "Generating site...".cyan());
        
        // Create output directory
        fs::create_dir_all(&self.config.output_dir)
            .context("Failed to create output directory")?;
        
        // Load posts
        self.load_posts().await?;
        
        // Generate illuminated initials if needed
        if let Some(_api_key) = &self.config.openai_api_key {
            self.generate_initials().await?;
        }
        
        // Generate individual post pages
        self.generate_posts().await?;
        
        // Generate index page
        self.generate_index().await?;
        
        // Copy assets
        self.copy_assets().await?;
        
        println!("{}", format!("Generated {} posts", self.posts.len()).green());
        
        Ok(())
    }

    async fn load_posts(&mut self) -> Result<()> {
        let posts_dir = Path::new(&self.config.posts_dir);
        if !posts_dir.exists() {
            fs::create_dir_all(posts_dir)
                .context("Failed to create posts directory")?;
            return Ok(());
        }

        let mut posts = Vec::new();
        
        for entry in WalkDir::new(posts_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        {
            let content = fs::read_to_string(entry.path())
                .context(format!("Failed to read {}", entry.path().display()))?;
            
            let post = self.parse_post(&content, entry.path())?;
            posts.push(post);
        }
        
        // Sort by date (newest first)
        posts.sort_by(|a, b| b.date.cmp(&a.date));
        
        self.posts = posts;
        Ok(())
    }

    fn parse_post(&self, content: &str, path: &Path) -> Result<Post> {
        // Parse frontmatter using serde_yaml
        let (frontmatter, markdown) = self.parse_frontmatter(content);
        
        // Convert markdown to HTML
        let html_content = to_html(&markdown);
        
        // Extract first paragraph for illuminated initial
        let first_paragraph_match = Regex::new(r"<p>(.*?)</p>").unwrap();
        let first_paragraph = first_paragraph_match
            .captures(&html_content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        
        // Extract first letter from first paragraph
        let first_letter = first_paragraph
            .chars()
            .find(|c| c.is_alphabetic())
            .map(|c| c.to_uppercase().next().unwrap());
        
        // Extract title from frontmatter or filename
        let title = frontmatter
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
            })
            .to_string();
        
        // Extract date from frontmatter or file modification time
        let date = frontmatter
            .get("date")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| {
                fs::metadata(path)
                    .and_then(|m| m.modified())
                    .map(|t| {
                        let secs = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
                        DateTime::from_timestamp(secs, 0).unwrap_or_else(|| Utc::now())
                    })
                    .unwrap_or_else(|_| Utc::now())
            });
        
        // Extract excerpt from frontmatter or first paragraph
        let excerpt = frontmatter
            .get("excerpt")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                markdown
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .map(|line| line.trim().to_string())
            });
        
        let original_slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string();
        let slug = sanitize_slug(&original_slug);
        
        Ok(Post {
            slug,
            original_slug,
            title,
            date,
            excerpt,
            content: markdown,
            html_content,
            first_letter,
            frontmatter,
        })
    }

    fn parse_frontmatter(&self, content: &str) -> (HashMap<String, serde_json::Value>, String) {
        let mut frontmatter = HashMap::new();
        let mut lines = content.lines();
        
        // Check if content starts with frontmatter
        if let Some(first_line) = lines.next() {
            if first_line.trim() == "---" {
                let mut frontmatter_lines = Vec::new();
                
                // Collect frontmatter lines
                while let Some(line) = lines.next() {
                    if line.trim() == "---" {
                        break;
                    }
                    frontmatter_lines.push(line);
                }
                
                // Parse frontmatter using serde_yaml
                if !frontmatter_lines.is_empty() {
                    let yaml_content = frontmatter_lines.join("\n");
                    if let Ok(parsed) = serde_yaml::from_str::<serde_json::Value>(&yaml_content) {
                        if let serde_json::Value::Object(map) = parsed {
                            frontmatter = map.into_iter().collect();
                        }
                    }
                }
                
                return (frontmatter, lines.collect::<Vec<_>>().join("\n"));
            }
        }
        
        // No frontmatter found
        (frontmatter, content.to_string())
    }

    async fn generate_initials(&self) -> Result<()> {
        let posts_with_initials: Vec<_> = self.posts
            .iter()
            .filter(|post| post.first_letter.is_some())
            .collect();
        
        if !posts_with_initials.is_empty() {
            println!("{}", format!("Generating {} illuminated initials...", posts_with_initials.len()).cyan());
            
            let initials_dir = Path::new(&self.config.output_dir).join("initials");
            fs::create_dir_all(&initials_dir)?;
            
            // Generate initials using OpenAI if API key is available
            if let Some(api_key) = &self.config.openai_api_key {
                let mut tasks = Vec::new();
                
                for post in posts_with_initials {
                    if let Some(letter) = post.first_letter {
                        let initial_path = initials_dir.join(format!("{}.txt", letter));
                        if !initial_path.exists() {
                            println!("Generating illuminated initial '{}'", letter.to_uppercase());
                            let api_key = api_key.clone();
                            let title = post.title.clone();
                            let task = tokio::spawn(async move {
                                Self::generate_illuminated_initial_static(letter, &title, &api_key).await
                            });
                            tasks.push((task, initial_path, letter));
                        } else {
                            println!("Illuminated initial for '{}' already exists, skipping", letter);
                        }
                    }
                }
                
                // Wait for all tasks to complete
                for (task, initial_path, letter) in tasks {
                    match task.await {
                        Ok(Ok(image_url)) => {
                            println!("Successfully generated illuminated initial for '{}'", letter);
                            fs::write(initial_path, image_url)?;
                        }
                        Ok(Err(e)) => {
                            eprintln!("Failed to generate illuminated initial for '{}': {}", letter, e);
                        }
                        Err(e) => {
                            eprintln!("Task failed for illuminated initial '{}': {}", letter, e);
                        }
                    }
                }
            } else {
                println!("{}", "Warning: OPENAI_API_KEY not found in environment. Skipping illuminated initials.".yellow());
            }
        }
        
        Ok(())
    }

    pub async fn generate_illuminated_initial_static(letter: char, _title: &str, api_key: &str) -> Result<String> {
        let client = reqwest::Client::new();
        
        let prompt = format!(
            "A black background with white ink drawing featuring an illuminated initial '{}' in the Italian Futurist style, with geometric and abstract forms, swirling lines, and dynamic composition reminiscent of early 20th-century avant-garde art. The background should be pure black with white forms and lines.",
            letter
        );
        
        // Use the DALL-E API endpoint with gpt-image-1 model
        let request_body = serde_json::json!({
            "model": "gpt-image-1",
            "prompt": prompt,
            "n": 1,
            "size": "1024x1024"
        });
        
        let response = client
            .post("https://api.openai.com/v1/images/generations")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request_body)
            .send()
            .await?;
        
        let status = response.status();
        let response_text = response.text().await?;
        
        if !status.is_success() {
            return Err(anyhow::anyhow!("API call failed with status {}: {}", status, response_text));
        }
        
        // Parse the response to extract the base64 image data
        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
        
        if let Some(data_array) = response_json.get("data").and_then(|d| d.as_array()) {
            if let Some(first_image) = data_array.first() {
                if let Some(b64_json) = first_image.get("b64_json").and_then(|b| b.as_str()) {
                    return Ok(format!("data:image/png;base64,{}", b64_json));
                }
            }
        }
        
        Err(anyhow::anyhow!("Could not extract image data from API response"))
    }

    async fn generate_posts(&self) -> Result<()> {
        let mut tasks = Vec::new();
        
        for post in &self.posts {
            let config = self.config.clone();
            let post = post.clone();
            let all_posts = self.posts.clone();
            
            let task = tokio::spawn(async move {
                let post_dir = Path::new(&config.output_dir).join(&post.slug);
                fs::create_dir_all(&post_dir)?;
                
                let html = templates::render_post(&config, &post, &all_posts)?;
                let output_path = post_dir.join("index.html");
                fs::write(output_path, html)?;
                Ok::<(), anyhow::Error>(())
            });
            
            tasks.push(task);
        }
        
        // Wait for all tasks to complete
        for task in tasks {
            match task.await {
                Ok(Ok(())) => {
                    // Task completed successfully
                }
                Ok(Err(e)) => {
                    return Err(e);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Task failed: {}", e));
                }
            }
        }
        
        Ok(())
    }

    async fn generate_index(&self) -> Result<()> {
        let html = templates::render_index(&self.config, &self.posts)?;
        let output_path = Path::new(&self.config.output_dir).join("index.html");
        fs::write(output_path, html)?;
        
        Ok(())
    }

    async fn copy_assets(&self) -> Result<()> {
        // Copy CSS file
        let css_content = templates::generate_css(&self.config);
        let css_path = Path::new(&self.config.output_dir).join("style.css");
        fs::write(css_path, css_content)?;
        
        Ok(())
    }
} 

fn sanitize_slug(input: &str) -> String {
    // Lowercase and replace any non-alphanumeric with '-'
    let lowered = input.to_lowercase();
    let provisional: String = lowered
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse multiple '-' and trim from ends
    let re = Regex::new(r"-+").unwrap();
    let collapsed = re.replace_all(&provisional, "-").to_string();
    let trimmed = collapsed.trim_matches('-').to_string();
    if trimmed.is_empty() { "untitled".to_string() } else { trimmed }
}