use crate::config::Config;
use crate::generator::Post;
use anyhow::Result;
use regex;

pub fn render_post(config: &Config, post: &Post, all_posts: &[Post], annotation_meta_json: Option<String>) -> Result<String> {
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

    let annotation_meta = match annotation_meta_json {
        Some(json) if !json.is_empty() => format!("<script id=\"annotation-meta\" type=\"application/json\">{}</script>", json),
        _ => String::new(),
    };

    // Build optional meta description and publication time tags
    let meta_description = match &post.excerpt {
        Some(desc) if !desc.is_empty() => format!("<meta name=\"description\" content=\"{}\">", desc),
        _ => String::new(),
    };
    let meta_published = format!("<meta property=\"article:published_time\" content=\"{}\">", post.date.to_rfc3339());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    {}
    {}
    <title>{} - {}</title>
    <link rel="stylesheet" href="{}">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Crimson+Text:ital,wght@0,400;0,600;1,400&family=Inter:wght@400;600;700&display=swap" rel="stylesheet">
    {}
</head>
<body>
    <div class="container">
        <header>
            <div class="header-content">
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
    <script>
    document.addEventListener('DOMContentLoaded', function() {{
        var meta = {{}};
        var metaEl = document.getElementById('annotation-meta');
        if (metaEl) {{
            try {{ meta = JSON.parse(metaEl.textContent || '{{}}'); }} catch(e) {{ meta = {{}}; }}
        }}
        var paragraphs = document.querySelectorAll('.post-content p');
        paragraphs.forEach(function(p) {{
            var text = (p.textContent || '').trim();
            if (!text) return;
            var a = document.createElement('a');
            a.className = 'exa-link';
            a.target = '_blank';
            a.rel = 'noopener noreferrer';
            a.textContent = '↗';
            a.href = 'https://exa.ai/search?q=' + encodeURIComponent(text);
            p.appendChild(a);
        }});

        // Annotations: convert fenced blocks (```links / ```anno) into folded panels attached to the previous paragraph/list
        var codeBlocks = Array.prototype.slice.call(document.querySelectorAll('.post-content pre > code'));
        codeBlocks.forEach(function(code) {{
            var cls = (code.getAttribute('class') || '').toLowerCase();
            var text = (code.textContent || '').trim();
            var isAnnotated = false;
            var lines = [];

            // Detect by language class or explicit leading marker line
            if (cls.indexOf('language-links') !== -1 || cls.indexOf('language-anno') !== -1 || cls.indexOf('language-annotation') !== -1) {{
                isAnnotated = true;
                lines = text.split('\n');
            }} else if (/^(links|anno|annotation)\s*:?/i.test(text)) {{
                isAnnotated = true;
                lines = text.split('\n').slice(1);
            }}

            if (!isAnnotated) return;

            // Determine target block to attach to: previous paragraph or list
            var pre = code.parentElement && code.parentElement.tagName === 'PRE' ? code.parentElement : null;
            if (!pre) return;
            var target = pre.previousElementSibling;
            while (target && ['P','UL','OL'].indexOf(target.tagName) === -1) {{
                target = target.previousElementSibling;
            }}
            if (!target) return;

            // Build panel content: parse lines into links with optional descriptions
            var items = [];
            lines.forEach(function(raw) {{
                var line = raw.trim();
                if (!line) return;
                // trim leading bullets
                line = line.replace(/^[-*]\s+/, '');

                var title = null, url = null, desc = null, m;

                // [Title](url) - desc
                m = line.match(/^\[([^\]]+)\]\(([^)\s]+)\)(?:\s*[\-–—:]\s*(.+))?$/);
                if (m) {{
                    title = m[1];
                    url = m[2];
                    desc = m[3] ? m[3].trim() : null;
                }}

                // Title - url - desc
                if (!url) {{
                    m = line.match(/^(.+?)\s*[\-–—:]\s*(https?:\/\/\S+)(?:\s*[\-–—:]\s*(.+))?$/);
                    if (m) {{
                        title = m[1].trim();
                        url = m[2];
                        desc = m[3] ? m[3].trim() : null;
                    }}
                }}

                // url - desc
                if (!url) {{
                    m = line.match(/^(https?:\/\/\S+)(?:\s*[\-–—:]\s*(.+))?$/);
                    if (m) {{
                        url = m[1];
                        desc = m[2] ? m[2].trim() : null;
                    }}
                }}

                if (!url) return;
                if (!title) {{
                    try {{
                        var u = new URL(url);
                        title = u.hostname;
                    }} catch (e) {{
                        title = url;
                    }}
                }}

                var key = (function(u){{
                    try {{
                        var x = new URL(u);
                        x.hash = '';
                        x.search = '';
                        var base = x.toString();
                        return [u, base, base.endsWith('/') ? base.slice(0,-1) : base + '/'];
                    }} catch(e) {{ return [u]; }}
                }})(url);
                var metaEntry = null;
                for (var i=0;i<key.length;i++){{ if (meta[key[i]]) {{ metaEntry = meta[key[i]]; break; }} }}
                if (metaEntry) {{
                    if (metaEntry.title) title = metaEntry.title;
                    if (metaEntry.description) desc = metaEntry.description;
                }}

                items.push({{ title: title, url: url, desc: desc }});
            }});

            if (!items.length) return;

            // Create panel
            var panel = document.createElement('div');
            panel.className = 'annotation-panel';
            var ul = document.createElement('ul');
            ul.className = 'annotation-list';
            items.forEach(function(it) {{
                var li = document.createElement('li');
                var wrap = document.createElement('div');
                wrap.className = 'annotation-item';

                var titleLine = document.createElement('div');
                titleLine.className = 'annotation-item-titleline';
                var aTitle = document.createElement('a');
                aTitle.className = 'annotation-item-title';
                aTitle.href = it.url;
                aTitle.textContent = it.title;
                aTitle.target = '_blank';
                aTitle.rel = 'noopener noreferrer';

                var aUrl = document.createElement('a');
                aUrl.className = 'annotation-item-link';
                aUrl.href = it.url;
                aUrl.textContent = '(' + it.url + ')';
                aUrl.target = '_blank';
                aUrl.rel = 'noopener noreferrer';

                titleLine.appendChild(aTitle);
                titleLine.appendChild(document.createTextNode(' '));
                titleLine.appendChild(aUrl);
                wrap.appendChild(titleLine);

                if (it.desc) {{
                    var d = document.createElement('div');
                    d.className = 'annotation-item-desc';
                    d.textContent = it.desc;
                    wrap.appendChild(d);
                }}

                li.appendChild(wrap);
                ul.appendChild(li);
            }});
            panel.appendChild(ul);

            // Insert panel after target
            target.insertAdjacentElement('afterend', panel);

            // Add toggle inside target (does not affect layout)
            var btn = document.createElement('button');
            btn.type = 'button';
            btn.className = 'annotation-toggle';
            btn.setAttribute('aria-expanded', 'false');
            btn.setAttribute('title', 'Show related links');
            btn.textContent = '▾';
            target.style.position = target.style.position || 'relative';
            target.appendChild(btn);

            var toggle = function() {{
                var open = panel.classList.toggle('open');
                btn.classList.toggle('open', open);
                btn.setAttribute('aria-expanded', open ? 'true' : 'false');
                if (open) {{
                    panel.style.display = 'block';
                }} else {{
                    panel.style.display = 'none';
                }}
            }};
            btn.addEventListener('click', toggle);

            // Remove the original fenced block
            pre.parentElement && pre.parentElement.removeChild(pre);
        }});

        // Annotations: detect plain paragraph 'Links:' followed by a list and fold it under previous block
        var all = Array.prototype.slice.call(document.querySelectorAll('.post-content p'));
        all.forEach(function(marker) {{
            var txt = (marker.textContent || '').trim().toLowerCase();
            if (txt !== 'links:' && txt !== 'links' && txt !== 'annotations:' && txt !== 'annotations') return;
            var list = marker.nextElementSibling;
            if (!list || ['UL','OL'].indexOf(list.tagName) === -1) return;

            // Attach to previous meaningful block
            var target = marker.previousElementSibling;
            while (target && ['P','UL','OL','BLOCKQUOTE'].indexOf(target.tagName) === -1) {{
                target = target.previousElementSibling;
            }}
            if (!target) return;

            var panel = document.createElement('div');
            panel.className = 'annotation-panel';
            // Build list anew to include metadata
            var newList = document.createElement(list.tagName.toLowerCase());
            newList.className = 'annotation-list';
            var anchors = list.querySelectorAll('a[href]');
            anchors.forEach(function(a) {{
                var url = a.getAttribute('href');
                var title = (a.textContent || '').trim();
                if (!title) {{
                    try {{ title = new URL(url).hostname; }} catch(e) {{ title = url; }}
                }}
                var desc = null;
                var metaEntry = meta[url];
                if (metaEntry) {{
                    if (metaEntry.title) title = metaEntry.title;
                    if (metaEntry.description) desc = metaEntry.description;
                }}
                var li = document.createElement('li');
                var wrap = document.createElement('div');
                wrap.className = 'annotation-item';
                var titleLine = document.createElement('div');
                titleLine.className = 'annotation-item-titleline';
                var aTitle = document.createElement('a');
                aTitle.className = 'annotation-item-title';
                aTitle.href = url;
                aTitle.textContent = title;
                aTitle.target = '_blank';
                aTitle.rel = 'noopener noreferrer';
                var aUrl = document.createElement('a');
                aUrl.className = 'annotation-item-link';
                aUrl.href = url;
                aUrl.textContent = '(' + url + ')';
                aUrl.target = '_blank';
                aUrl.rel = 'noopener noreferrer';
                titleLine.appendChild(aTitle);
                titleLine.appendChild(document.createTextNode(' '));
                titleLine.appendChild(aUrl);
                wrap.appendChild(titleLine);
                if (desc) {{
                    var d = document.createElement('div');
                    d.className = 'annotation-item-desc';
                    d.textContent = desc;
                    wrap.appendChild(d);
                }}
                li.appendChild(wrap);
                newList.appendChild(li);
            }});
            panel.appendChild(newList);
            target.insertAdjacentElement('afterend', panel);

            var btn = document.createElement('button');
            btn.type = 'button';
            btn.className = 'annotation-toggle';
            btn.setAttribute('aria-expanded', 'false');
            btn.setAttribute('title', 'Show related links');
            btn.textContent = '▾';
            target.style.position = target.style.position || 'relative';
            target.appendChild(btn);

            var toggle = function() {{
                var open = panel.classList.toggle('open');
                btn.classList.toggle('open', open);
                btn.setAttribute('aria-expanded', open ? 'true' : 'false');
                panel.style.display = open ? 'block' : 'none';
            }};
            btn.addEventListener('click', toggle);

            // Remove original marker and list
            list.parentElement && list.parentElement.removeChild(list);
            marker.parentElement && marker.parentElement.removeChild(marker);
        }});
    }});
    </script>
</body>
    </html>"#,
        meta_description,
        meta_published,
        post.title,
        config.title,
        css_path,
        annotation_meta,
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
  justify-content: flex-end;
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
  padding-bottom: 12px; /* reserve space for underline */
  color: #f5f5f5;
  position: relative;
}

.post-title::after {
  content: '';
  position: absolute;
  bottom: 0;
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

.post-content hr {
  border: none;
  border-top: 1px solid #2a2a2a;
  height: 0;
  margin: 32px 0 24px 0;
}

.post-content p {
  margin-bottom: 1.5em;
  text-align: justify;
  hyphens: auto;
  position: relative;
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
  padding-bottom: 8px; /* reserve space for underline */
  color: #f5f5f5;
  position: relative;
  text-align: right;
}

.post-content h2::after {
  content: '';
  position: absolute;
  bottom: 0;
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

/* Exa search link per paragraph */
.exa-link {
  position: absolute;
  right: -1.2em;
  top: 0.1em;
  font-size: 0.9em;
  color: #8b8b8b;
  text-decoration: none;
  opacity: 0;
  transition: opacity 0.2s ease, color 0.2s ease;
  margin-left: 0.25em; /* used when inline on mobile */
}

.post-content p:hover .exa-link {
  opacity: 1;
}

.exa-link:hover {
  color: #f5f5f5;
}

/* Annotation toggle and panel */
.annotation-toggle {
  position: absolute;
  left: 50%;
  bottom: -0.6em;
  transform: translateX(-50%);
  background: transparent;
  color: #8b8b8b;
  border: none;
  cursor: pointer;
  font-family: 'Inter', sans-serif;
  font-size: 0.9em;
  line-height: 1;
  padding: 0;
  opacity: 0;
  transition: opacity 0.2s ease, color 0.2s ease, transform 0.2s ease;
}

.post-content p:hover .annotation-toggle,
.post-content ul:hover .annotation-toggle,
.post-content ol:hover .annotation-toggle,
.post-content blockquote:hover .annotation-toggle {
  opacity: 1;
}

.annotation-toggle:hover { color: #f5f5f5; }
.annotation-toggle.open { transform: translateX(-50%) rotate(180deg); }

.annotation-panel {
  display: none;
  margin: 0.6em 0 1.2em 0;
  padding: 10px 14px;
  border-left: 2px solid #2a2a2a;
  background-color: rgba(255,255,255,0.02);
}

.annotation-list {
  margin: 0;
  padding-left: 18px;
}

.annotation-list li { margin: 6px 0; }
.annotation-list a { color: #8b8b8b; }
.annotation-list a:hover { color: #f5f5f5; }

.annotation-item-titleline {
  font-family: 'Crimson Text', Georgia, serif;
}

.annotation-item-title {
  color: #f5f5f5;
  text-decoration: none;
}

.annotation-item-link {
  color: #8b8b8b;
  text-decoration: none;
}

.annotation-item-link:hover, .annotation-item-title:hover {
  color: #f5f5f5;
}

.annotation-item-desc {
  color: #d0d0d0;
  font-size: 0.95em;
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
  
  /* On mobile, render arrow as the last inline character */
  .post-content p { padding-right: 0; }
  .exa-link {
    position: static;
    right: auto;
    top: auto;
    display: inline;
    opacity: 1;
  }
  .annotation-toggle {
    position: static;
    left: auto;
    bottom: auto;
    transform: none;
    margin-left: 0.35em;
    opacity: 1;
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
    text-align: right;
    align-items: flex-end;
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
  
  .exa-link {
    display: none;
  }
  .annotation-toggle { display: none; }
  .annotation-panel { display: none; }
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