use crate::config::Config;
use crate::templates;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::*;
use markdown::to_html;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::{HashMap, HashSet};
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
        
        // Convert markdown to HTML (autolink raw URLs first)
        let autolinked_markdown = Self::autolink_markdown(&markdown);
        let html_content = to_html(&autolinked_markdown);
        
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

    /// Convert bare URLs in Markdown text to autolink format `<url>` while avoiding
    /// fenced code blocks and inline code spans.
    fn autolink_markdown(markdown: &str) -> String {
        let mut result_lines: Vec<String> = Vec::new();
        let mut in_code_block = false;
        for line in markdown.lines() {
            let trimmed_start = line.trim_start();
            if trimmed_start.starts_with("```") {
                // Toggle fenced code block state and pass line through unchanged
                in_code_block = !in_code_block;
                result_lines.push(line.to_string());
                continue;
            }

            if in_code_block {
                result_lines.push(line.to_string());
                continue;
            }

            // Process inline code spans delimited by backticks, only autolink in non-code parts
            let mut processed_line = String::with_capacity(line.len());
            let mut in_inline_code = false;
            let mut buffer = String::new();
            for ch in line.chars() {
                if ch == '`' {
                    if in_inline_code {
                        // Flush buffer as-is (inside code)
                        processed_line.push_str(&buffer);
                        buffer.clear();
                        processed_line.push('`');
                        in_inline_code = false;
                    } else {
                        // Flush buffer with autolinking (outside code)
                        processed_line.push_str(&Self::autolink_text(&buffer));
                        buffer.clear();
                        processed_line.push('`');
                        in_inline_code = true;
                    }
                } else {
                    buffer.push(ch);
                }
            }
            if in_inline_code {
                // Remaining buffer is inside code
                processed_line.push_str(&buffer);
            } else {
                processed_line.push_str(&Self::autolink_text(&buffer));
            }

            result_lines.push(processed_line);
        }

        result_lines.join("\n")
    }

    /// Autolink bare http/https URLs in a plain text segment (no inline/fenced code).
    fn autolink_text(text: &str) -> String {
        // Match a conservative URL, we'll handle trailing punctuation separately
        let re = Regex::new(r"https?://[^\s<>()]+").unwrap();
        let mut result = String::with_capacity(text.len());
        let mut last_end = 0;
        for m in re.find_iter(text) {
            // Avoid transforming URLs that are part of a Markdown link: "](" immediately before
            let start = m.start();
            let end = m.end();
            let before = &text[..start];
            let after = &text[end..];

            let is_md_link_target = before.ends_with("](");
            // Avoid transforming URLs already inside angle brackets: "<url>"
            let prev_byte_is_lt = before.as_bytes().last().map(|b| *b == b'<').unwrap_or(false);
            let next_byte_is_gt = after.as_bytes().first().map(|b| *b == b'>').unwrap_or(false);
            if is_md_link_target || (prev_byte_is_lt && next_byte_is_gt) {
                continue;
            }

            // Push preceding text
            result.push_str(&text[last_end..start]);

            let matched = m.as_str();
            let (core, trailing) = Self::split_trailing_punctuation(matched);
            // Use explicit Markdown link syntax for broad renderer compatibility
            result.push('[');
            result.push_str(&core);
            result.push(']');
            result.push('(');
            result.push_str(&core);
            result.push(')');
            result.push_str(&trailing);

            last_end = end;
        }
        // Remainder
        result.push_str(&text[last_end..]);
        result
    }

    /// Split off common trailing punctuation that should not be part of the URL.
    fn split_trailing_punctuation(s: &str) -> (String, String) {
        let mut end = s.len();
        let bytes = s.as_bytes();
        while end > 0 {
            let b = bytes[end - 1];
            if matches!(b as char, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']') {
                end -= 1;
            } else {
                break;
            }
        }
        (s[..end].to_string(), s[end..].to_string())
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
                
                // Build annotation metadata JSON (URL -> { title, description })
                let annotation_meta_json = build_annotation_meta_json(&post).await;

                let html = templates::render_post(&config, &post, &all_posts, annotation_meta_json)?;
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

/// Extract external URLs from annotation sections in raw markdown and fetch metadata.
async fn build_annotation_meta_json(post: &Post) -> Option<String> {
    let markdown = &post.content;
    // Collect URLs from fenced blocks ```links/```anno and from a 'Links:' marker followed by list
    let mut urls: HashSet<String> = HashSet::new();

    // Simple stateful parse for fenced blocks
    let mut in_links_block = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            let lang = trimmed.trim_start_matches("```").trim().to_lowercase();
            if !in_links_block && (lang == "links" || lang == "anno" || lang == "annotation") {
                in_links_block = true;
                continue;
            }
            if in_links_block && (lang.is_empty() || lang == "links" || lang == "anno" || lang == "annotation") {
                in_links_block = false;
                continue;
            }
        }
        if in_links_block {
            if let Some(u) = extract_url_from_line(trimmed) {
                urls.insert(u);
            }
        }
    }

    // Also collect any anchors from rendered HTML content as a fallback
    let html = &post.html_content;
    if let Some(re) = Regex::new(r#"(?is)<a[^>]+href\s*=\s*([\"'])(https?://[^\"'>\s]+)\1"#).ok() {
        for cap in re.captures_iter(html) {
            if let Some(m) = cap.get(2) { urls.insert(m.as_str().to_string()); }
        }
    }

    // Parse list after 'Links:' marker
    let mut lines_iter = markdown.lines().peekable();
    while let Some(line) = lines_iter.next() {
        let t = line.trim();
        if t.eq_ignore_ascii_case("links:") || t.eq_ignore_ascii_case("links") || t.eq_ignore_ascii_case("annotations:") || t.eq_ignore_ascii_case("annotations") {
            // Consume subsequent list items
            while let Some(next) = lines_iter.peek() {
                let nt = next.trim();
                if nt.starts_with("-") || nt.starts_with("*") || Regex::new(r"^\d+[\.)]\s+").unwrap().is_match(nt) { // bullet or ordered list
                    let content = if nt.starts_with("-") || nt.starts_with("*") {
                        nt.trim_start_matches(|c: char| c == '-' || c == '*' || c.is_whitespace()).trim().to_string()
                    } else {
                        Regex::new(r"^\d+[\.)]\s+").unwrap().replace(nt, "").to_string()
                    };
                    if let Some(u) = extract_url_from_line(content.trim()) {
                        urls.insert(u);
                    }
                    lines_iter.next();
                } else {
                    break;
                }
            }
        }
    }

    if urls.is_empty() { return None; }

    // Fetch metadata concurrently with a simple cap
    let client = reqwest::Client::new();
    let mut tasks = Vec::new();
    for url in urls.into_iter().take(32) { // limit to 32 per post
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            let meta = fetch_url_metadata(&client, &url).await.unwrap_or_default();
            (url, meta)
        }));
    }

    let mut map: HashMap<String, serde_json::Value> = HashMap::new();
    for t in tasks {
        if let Ok((url, meta)) = t.await {
            let key_main = canonicalize_url(&url);
            map.insert(key_main.clone(), meta.clone());
            // also insert with/without trailing slash variants to maximize client hits
            if key_main.ends_with('/') {
                let no_slash = key_main.trim_end_matches('/').to_string();
                map.insert(no_slash, meta.clone());
            } else {
                let with_slash = format!("{}/", key_main);
                map.insert(with_slash, meta.clone());
            }
            // also insert the raw URL that was authored
            map.insert(url, meta);
        }
    }

    if map.is_empty() { return None; }
    Some(serde_json::to_string(&map).unwrap_or_else(|_| String::new()))
}

async fn fetch_url_metadata(client: &reqwest::Client, url: &str) -> Result<serde_json::Value> {
    use std::time::Duration;
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .timeout(Duration::from_secs(8))
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() { return Ok(serde_json::json!({})); }
    let bytes = resp.bytes().await?;
    let text = String::from_utf8_lossy(&bytes);

    // Extract: <title>, og:title, meta description (order-insensitive attributes)
    let title_tag = Regex::new(r"(?is)<title[^>]*>(.*?)</title>")
        .ok()
        .and_then(|re| re.captures(&text).and_then(|c| c.get(1)).map(|m| html_unescape(m.as_str())));
    let og_title = Regex::new(r#"(?is)<meta[^>]*\bproperty\s*=\s*([\"'])og:title\1[^>]*\bcontent\s*=\s*([\"'])(.*?)\2|<meta[^>]*\bcontent\s*=\s*([\"'])(.*?)\4[^>]*\bproperty\s*=\s*([\"'])og:title\6"#)
        .ok()
        .and_then(|re| re.captures(&text)).and_then(|c| c.get(3).or_else(|| c.get(5))).map(|m| html_unescape(m.as_str()));
    let tw_title = Regex::new(r#"(?is)<meta[^>]*\bname\s*=\s*([\"'])twitter:title\1[^>]*\bcontent\s*=\s*([\"'])(.*?)\2|<meta[^>]*\bproperty\s*=\s*([\"'])twitter:title\4[^>]*\bcontent\s*=\s*([\"'])(.*?)\5|<meta[^>]*\bcontent\s*=\s*([\"'])(.*?)\7[^>]*\bname\s*=\s*([\"'])twitter:title\8|<meta[^>]*\bcontent\s*=\s*([\"'])(.*?)\10[^>]*\bproperty\s*=\s*([\"'])twitter:title\11"#)
        .ok()
        .and_then(|re| re.captures(&text))
        .and_then(|c| c.get(3).or_else(|| c.get(6)).or_else(|| c.get(8)).or_else(|| c.get(11)))
        .map(|m| html_unescape(m.as_str()));
    let name_desc_any = Regex::new(r#"(?is)<meta[^>]*\bname\s*=\s*([\"'])description\1[^>]*\bcontent\s*=\s*([\"'])(.*?)\2|<meta[^>]*\bcontent\s*=\s*([\"'])(.*?)\4[^>]*\bname\s*=\s*([\"'])description\6"#)
        .ok()
        .and_then(|re| re.captures(&text)).and_then(|c| c.get(3).or_else(|| c.get(5))).map(|m| html_unescape(m.as_str()));
    let og_desc = Regex::new(r#"(?is)<meta[^>]*\bproperty\s*=\s*([\"'])og:description\1[^>]*\bcontent\s*=\s*([\"'])(.*?)\2|<meta[^>]*\bcontent\s*=\s*([\"'])(.*?)\4[^>]*\bproperty\s*=\s*([\"'])og:description\6"#)
        .ok()
        .and_then(|re| re.captures(&text)).and_then(|c| c.get(3).or_else(|| c.get(5))).map(|m| html_unescape(m.as_str()));
    let tw_desc = Regex::new(r#"(?is)<meta[^>]*\bname\s*=\s*([\"'])twitter:description\1[^>]*\bcontent\s*=\s*([\"'])(.*?)\2|<meta[^>]*\bproperty\s*=\s*([\"'])twitter:description\4[^>]*\bcontent\s*=\s*([\"'])(.*?)\5|<meta[^>]*\bcontent\s*=\s*([\"'])(.*?)\7[^>]*\bname\s*=\s*([\"'])twitter:description\8|<meta[^>]*\bcontent\s*=\s*([\"'])(.*?)\10[^>]*\bproperty\s*=\s*([\"'])twitter:description\11"#)
        .ok()
        .and_then(|re| re.captures(&text))
        .and_then(|c| c.get(3).or_else(|| c.get(6)).or_else(|| c.get(8)).or_else(|| c.get(11)))
        .map(|m| html_unescape(m.as_str()));

    let title = tw_title.or(og_title).or(title_tag);
    let description = tw_desc.or(name_desc_any).or(og_desc);
    let mut obj = serde_json::Map::new();
    if let Some(t) = title { obj.insert("title".to_string(), serde_json::Value::String(t)); }
    if let Some(d) = description { obj.insert("description".to_string(), serde_json::Value::String(d)); }
    Ok(serde_json::Value::Object(obj))
}

fn html_unescape(s: &str) -> String {
    let s = s.replace("&amp;", "&")
             .replace("&lt;", "<")
             .replace("&gt;", ">")
             .replace("&quot;", "\"")
             .replace("&#39;", "'");
    Regex::new(r"\s+").map(|re| re.replace_all(&s, " ").to_string()).unwrap_or(s)
}

fn extract_url_from_line(line: &str) -> Option<String> {
    // [Title](url) - desc
    if let Some(caps) = Regex::new(r"\((https?://[^)\s]+)\)").ok().and_then(|re| re.captures(line)) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    // Title - url or bare url
    if let Some(caps) = Regex::new(r"(https?://\S+)").ok().and_then(|re| re.captures(line)) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    None
}

fn canonicalize_url(url: &str) -> String {
    // Lowercase scheme/host, remove fragment and query, collapse multiple slashes, keep trailing slash as-is
    let mut s = url.trim().to_string();
    if let Some(hash) = s.find('#') { s.truncate(hash); }
    if let Some(q) = s.find('?') { s.truncate(q); }
    // split scheme://host/path
    if let Some(pos) = s.find("://") {
        let (scheme, rest) = s.split_at(pos);
        let rest = &rest[3..];
        let mut parts = rest.splitn(2, '/');
        let host = parts.next().unwrap_or("").to_lowercase();
        let path = parts.next().unwrap_or("");
        let mut rebuilt = String::new();
        rebuilt.push_str(&scheme.to_lowercase());
        rebuilt.push_str("://");
        rebuilt.push_str(&host);
        if !path.is_empty() { rebuilt.push('/'); rebuilt.push_str(path); }
        // remove duplicate slashes in path
        let mut result = String::new();
        let mut prev_slash = false;
        for ch in rebuilt.chars() {
            if ch == '/' {
                if !prev_slash { result.push(ch); }
                prev_slash = true;
            } else { result.push(ch); prev_slash = false; }
        }
        result
    } else {
        s
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