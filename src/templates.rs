use crate::config::Config;
use crate::generator::Post;
use anyhow::Result;
use regex;

pub fn render_post(config: &Config, post: &Post, all_posts: &[Post]) -> Result<String> {
    let backlinks = find_backlinks(all_posts, &post.slug, &post.original_slug);
    
    let has_initial = post.first_letter.is_some();
    
    // Remove the first letter from the first paragraph if we have an illuminated initial
    let mut processed_content = post.html_content.clone();
    if has_initial {
        // Find the first paragraph and remove its first letter
        let re = regex::Regex::new(r"<p>([^<])(.*?)</p>").unwrap();
        processed_content = re.replace(&processed_content, |caps: &regex::Captures| {
            format!("<p>{}</p>", &caps[2])
        }).to_string();
    }
    // Rewrite internal links that may reference original, unsanitized slugs
    processed_content = rewrite_internal_links(&processed_content, all_posts);

    // Load the illuminated initial data URL if it exists
    let initial_html = if has_initial {
        let initial_path = std::path::Path::new(&config.output_dir).join("initials").join(format!("{}.txt", post.first_letter.unwrap()));
        if initial_path.exists() {
            if let Ok(image_data) = std::fs::read_to_string(initial_path) {
                format!(
                    r#"<div class="illuminated-initial">
                        <img src="{}" alt="Illuminated initial {}" class="initial-image">
                    </div>"#,
                    image_data,
                    post.first_letter.unwrap()
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    
    let backlinks_html = if backlinks.is_empty() {
        String::new()
    } else {
        let links: String = backlinks
            .iter()
            .map(|link| format!("<li><a href=\"{}\">{}</a></li>", link.url, link.title))
            .collect::<Vec<_>>()
            .join("\n                    ");
        
        format!(
            r#"
            <section class="backlinks">
                <h2>Backlinks</h2>
                <ul>
                    {}
                </ul>
            </section>"#,
            links
        )
    };
    
    // Use relative paths (works for both regular hosting and IPFS)
    let (css_path, home_path) = ("../style.css", "../");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - {}</title>
    <link rel="stylesheet" href="{}">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Crimson+Text:ital,wght@0,400;0,600;1,400&family=Inter:wght@400;600;700&display=swap" rel="stylesheet">
</head>
<body>
    <div class="container">
        <header>
            <div class="header-content">
                <a href="{}" class="pilcrow">¶</a>
                <a href="{}" class="main-title">{}</a>
            </div>
        </header>
        
        <main class="content">
            <article>
                <h1 class="post-title">{}</h1>
                <div class="post-content">
                    {}
                    {}
                </div>
            </article>
            {}
        </main>
        
        <footer>
            <a href="{}" class="home-link">← Back to all posts</a>
        </footer>
    </div>
</body>
</html>"#,
        post.title,
        config.title,
        css_path,
        home_path,
        home_path,
        config.title.to_uppercase(),
        post.title,
        initial_html,
        processed_content,
        backlinks_html,
        home_path
    );
    
    Ok(html)
}

fn rewrite_internal_links(content: &str, all_posts: &[Post]) -> String {
    let mut result = content.to_string();
    for p in all_posts {
        if p.original_slug != p.slug {
            let pairs = [
                // absolute
                (format!("href=\"/{}/\"", p.original_slug), format!("href=\"/{}/\"", p.slug)),
                (format!("href=\"/{}\"", p.original_slug), format!("href=\"/{}/\"", p.slug)),
                (format!("href=\"/{}.md\"", p.original_slug), format!("href=\"/{}/\"", p.slug)),
                // dot-relative
                (format!("href=\"./{}/\"", p.original_slug), format!("href=\"./{}/\"", p.slug)),
                (format!("href=\"./{}\"", p.original_slug), format!("href=\"./{}/\"", p.slug)),
                (format!("href=\"./{}.md\"", p.original_slug), format!("href=\"./{}/\"", p.slug)),
                // dotdot-relative
                (format!("href=\"../{}/\"", p.original_slug), format!("href=\"../{}/\"", p.slug)),
                (format!("href=\"../{}\"", p.original_slug), format!("href=\"../{}/\"", p.slug)),
                (format!("href=\"../{}.md\"", p.original_slug), format!("href=\"../{}/\"", p.slug)),
                // plain relative (no ./)
                (format!("href=\"{}/\"", p.original_slug), format!("href=\"{}/\"", p.slug)),
                (format!("href=\"{}\"", p.original_slug), format!("href=\"{}/\"", p.slug)),
                (format!("href=\"{}.md\"", p.original_slug), format!("href=\"{}/\"", p.slug)),
            ];
            for (from, to) in pairs {
                result = result.replace(&from, &to);
            }
        }
    }
    result
}

pub fn render_index(config: &Config, posts: &[Post]) -> Result<String> {
    let posts_list: String = posts
        .iter()
        .map(|post| {
            let excerpt_html = post.excerpt.as_ref().map_or(String::new(), |excerpt| {
                format!("<p class=\"excerpt\">{}</p>", excerpt)
            });
            
            let post_path = format!("./{}/", post.slug);
            
            format!(
                r#"<article class="post-preview">
    <div class="post-header">
        <h2><a href="{}">{}</a></h2>
        <time datetime="{}">{}</time>
    </div>
    {}
</article>"#,
                post_path,
                post.title,
                post.date.to_rfc3339(),
                post.date.format("%d/%m/%Y").to_string(),
                excerpt_html
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    // Use relative paths (works for both regular hosting and IPFS)
    let (css_path, home_path) = ("./style.css", "./");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <link rel="stylesheet" href="{}">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Crimson+Text:ital,wght@0,400;0,600;1,400&family=Inter:wght@400;600;700&display=swap" rel="stylesheet">
</head>
<body>
    <div class="container">
        <header>
            <div class="header-content">
                <a href="{}" class="pilcrow">¶</a>
                <a href="{}" class="main-title">{}</a>
            </div>
        </header>
        
        <main class="content">
            <section class="posts-list">
                {}
            </section>
        </main>
    </div>
</body>
</html>"#,
        config.title,
        css_path,
        home_path,
        home_path,
        config.title.to_uppercase(),
        posts_list
    );
    
    Ok(html)
}

pub fn generate_css(_config: &Config) -> String {
    // Use the exact CSS from the original implementation
    r#"/* Reset and base styles */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background-color: #0a0a0a;
  color: #f5f5f5;
  font-family: 'Crimson Text', Georgia, serif;
  line-height: 1.7;
  font-size: 18px;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* Container */
.container {
  max-width: 800px;
  margin: 0 auto;
  padding: 0 20px;
}

/* Header */
header {
  padding: 40px 0;
  margin-bottom: 60px;
}

.header-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.pilcrow {
  font-size: 24px;
  color: #f5f5f5;
  font-weight: 400;
  text-decoration: none;
  transition: color 0.2s ease;
}

.pilcrow:hover {
  color: #8b8b8b;
}

.main-title {
  font-family: 'Crimson Text', Georgia, serif;
  font-size: 32px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: #f5f5f5;
  position: relative;
  text-decoration: none;
  transition: color 0.2s ease;
}

.main-title:hover {
  color: #8b8b8b;
}

.main-title::after {
  content: '';
  position: absolute;
  bottom: -8px;
  left: 0;
  right: 0;
  height: 1px;
  background-color: #4a4a4a;
}

/* Content */
.content {
  margin-bottom: 80px;
}

/* Post titles */
.post-title {
  font-family: 'Crimson Text', Georgia, serif;
  font-size: 42px;
  font-weight: 700;
  line-height: 1.2;
  margin-bottom: 30px;
  color: #f5f5f5;
  position: relative;
}

.post-title::after {
  content: '';
  position: absolute;
  bottom: -12px;
  left: 0;
  right: 0;
  height: 1px;
  background-color: #4a4a4a;
}

/* Post content */
.post-content {
  font-size: 20px;
  line-height: 1.4;
  margin-bottom: 60px;
}

.post-content p {
  margin-bottom: 1.5em;
  text-align: justify;
  hyphens: auto;
}

.post-content h1 {
  font-family: 'Crimson Text', Georgia, serif;
  font-size: 32px;
  font-weight: 600;
  margin: 40px 0 20px 0;
  color: #f5f5f5;
  text-align: left;
}

.post-content h2 {
  font-family: 'Crimson Text', Georgia, serif;
  font-size: 28px;
  font-weight: 600;
  margin: 40px 0 20px 0;
  color: #f5f5f5;
  position: relative;
  text-align: right;
}

.post-content h2::after {
  content: '';
  position: absolute;
  bottom: -8px;
  left: 0;
  right: 0;
  height: 1px;
  background-color: #4a4a4a;
}

.post-content h3 {
  font-family: 'Crimson Text', Georgia, serif;
  font-size: 22px;
  font-weight: 600;
  margin: 30px 0 15px 0;
  color: #f5f5f5;
  text-align: right;
}

.post-content h3::before {
  content: '¶';
  position: absolute;
  left: 0;
  top: 0;
  font-size: 16px;
  color: #8b8b8b;
}

.post-content ul, .post-content ol {
  margin: 20px 0;
  padding-left: 30px;
}

.post-content li {
  margin-bottom: 8px;
}

.post-content blockquote {
  border-left: 3px solid #4a4a4a;
  padding-left: 20px;
  margin: 30px 0;
  font-style: italic;
  color: #d0d0d0;
}

.post-content code {
  background-color: #1a1a1a;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace;
  font-size: 0.9em;
}

.post-content pre {
  background-color: #1a1a1a;
  padding: 20px;
  border-radius: 6px;
  overflow-x: auto;
  margin: 20px 0;
}

.post-content pre code {
  background: none;
  padding: 0;
}

/* Illuminated initial */
.illuminated-initial {
  float: left;
  margin: 0 12px 20px 0;
  shape-outside: rectangle(0, 0, 80px, 80px);
}

.initial-image {
  width: 80px;
  height: 80px;
  object-fit: cover;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  border: 1px solid #4a4a4a;
}

/* Links */
a {
  color: #8b8b8b;
  text-decoration: underline;
  text-decoration-color: #4a4a4a;
  text-underline-offset: 2px;
  transition: color 0.2s ease;
}

a:hover {
  color: #f5f5f5;
  text-decoration-color: #8b8b8b;
}

/* Backlinks section */
.backlinks {
  margin-top: 60px;
  padding-top: 40px;
  border-top: 1px solid #2a2a2a;
}

.backlinks h2 {
  font-family: 'Crimson Text', Georgia, serif;
  font-size: 24px;
  font-weight: 600;
  margin-bottom: 20px;
  color: #f5f5f5;
}

.backlinks ul {
  list-style: none;
  padding: 0;
}

.backlinks li {
  margin-bottom: 12px;
}

.backlinks a {
  font-size: 16px;
  color: #8b8b8b;
}

/* Posts list (index page) */
.posts-list {
  display: flex;
  flex-direction: column;
  gap: 30px;
}

.post-preview {
  padding-bottom: 30px;
  border-bottom: 1px solid #2a2a2a;
}

.post-preview:last-child {
  border-bottom: none;
}

.post-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 8px;
}

.post-preview h2 {
  font-family: 'Crimson Text', Georgia, serif;
  font-size: 28px;
  font-weight: 600;
  margin: 0;
  flex: 1;
}

.post-preview h2 a {
  color: #f5f5f5;
  text-decoration: none;
}

.post-preview h2 a:hover {
  color: #8b8b8b;
}

.post-preview time {
  font-size: 14px;
  color: #8b8b8b;
  font-family: 'Inter', sans-serif;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  white-space: nowrap;
  margin-left: 20px;
}

.post-preview .excerpt {
  margin-top: 0;
  font-size: 16px;
  color: #d0d0d0;
  line-height: 1.5;
}

/* Footer */
footer {
  padding: 40px 0;
  border-top: 1px solid #2a2a2a;
  text-align: center;
}

.home-link {
  font-family: 'Crimson Text', Georgia, serif;
  font-size: 16px;
  color: #8b8b8b;
  text-decoration: none;
  transition: color 0.2s ease;
}

.home-link:hover {
  color: #f5f5f5;
}

/* Responsive design */
@media (max-width: 768px) {
  .container {
    padding: 0 15px;
  }
  
  .main-title {
    font-size: 24px;
  }
  
  .post-title {
    font-size: 32px;
  }
  
  .post-content {
    font-size: 18px;
  }
  
  .illuminated-initial {
    float: none;
    margin: 0 0 20px 0;
    text-align: center;
  }
  
  .initial-image {
    width: 60px;
    height: 60px;
  }
  
  .header-content {
    flex-direction: column;
    gap: 20px;
    text-align: center;
  }
  
  .post-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
  }
  
  .post-preview time {
    margin-left: 0;
    font-size: 12px;
  }
  
  .post-preview h2 {
    font-size: 24px;
  }
}

/* Print styles */
@media print {
  body {
    background: white;
    color: black;
  }
  
  .illuminated-initial {
    display: none;
  }
}"#.to_string()
}

#[derive(Debug)]
struct Backlink {
    title: String,
    url: String,
}

fn find_backlinks(posts: &[Post], current_slug: &str, current_original_slug: &str) -> Vec<Backlink> {
    let mut backlinks = Vec::new();
    
    for post in posts {
        if post.slug != current_slug {
            // Simple backlink detection - look for links to current post
            let patterns = [
                // sanitized slug
                format!("/{}/", current_slug),
                format!("/{}\"", current_slug),
                format!("/{}.md\"", current_slug),
                format!("./{}/", current_slug),
                format!("./{}\"", current_slug),
                format!("./{}.md\"", current_slug),
                format!("../{}/", current_slug),
                format!("../{}\"", current_slug),
                format!("../{}.md\"", current_slug),
                format!("{}/", current_slug),
                format!("{}\"", current_slug),
                format!("{}.md\"", current_slug),
                // original slug as might appear in authored markdown
                format!("/{}/", current_original_slug),
                format!("/{}\"", current_original_slug),
                format!("/{}.md\"", current_original_slug),
                format!("./{}/", current_original_slug),
                format!("./{}\"", current_original_slug),
                format!("./{}.md\"", current_original_slug),
                format!("../{}/", current_original_slug),
                format!("../{}\"", current_original_slug),
                format!("../{}.md\"", current_original_slug),
                format!("{}/", current_original_slug),
                format!("{}\"", current_original_slug),
                format!("{}.md\"", current_original_slug),
            ];
            if patterns.iter().any(|p| post.html_content.contains(p)) {
                backlinks.push(Backlink { title: post.title.clone(), url: format!("../{}/", post.slug) });
            }
        }
    }
    
    backlinks
} 