# Scribe

A minimal static site generator written in Rust • ink • eternal

## Features

- **Minimal Design**: Clean, typography-focused design
- **Markdown Support**: Write posts in Markdown with frontmatter
- **Illuminated Initials**: AI-generated decorative initials (requires OpenAI API)
- **Backlinks**: Automatic detection of links between posts
- **Responsive**: Mobile-friendly design
- **Fast**: Written in Rust for performance

## Installation

### From Source

```bash
git clone <repository-url>
cd scribe
cargo install --path .
```

### Using Cargo

```bash
cargo install scribe
```

## Usage

### Quick Start

1. Create a new site directory:
```bash
mkdir my-site && cd my-site
scribe
```

2. This will create the default directory structure:
```
my-site/
├── config.json
├── posts/
└── dist/
```

3. Add your posts to the `posts/` directory as Markdown files:
```markdown
---
title: My First Post
date: 2024-01-20T10:00:00Z
excerpt: A brief description of the post
---

Your post content here...
```

4. Generate the site:
```bash
scribe
```

5. The generated site will be in the `dist/` directory.

### Configuration

The `config.json` file allows you to customize your site:

```json
{
  "title": "My Site",
  "description": "A site about technology and design",
  "author": "Your Name",
  "url": "https://example.com",
  "posts_dir": "posts",
  "output_dir": "dist",
  "openai_api_key": "your-api-key-here",
  "theme": {
    "primary_color": "#f5f5f5",
    "background_color": "#0a0a0a",
    "text_color": "#f5f5f5",
    "accent_color": "#8b8b8b"
  }
}
```

### Command Line Options

- `-c, --config <FILE>`: Specify config file (default: config.json)
- `-f, --force`: Force regeneration of all content
- `-h, --help`: Show help
- `-V, --version`: Show version

## Directory Structure

```
site/
├── config.json          # Site configuration
├── posts/               # Markdown posts
│   ├── post-1.md
│   └── post-2.md
└── dist/                # Generated site
    ├── index.html
    ├── style.css
    ├── initials/        # Generated illuminated initials
    └── post-slug/
        └── index.html
```

## Post Format

Posts are written in Markdown with optional YAML frontmatter:

```markdown
---
title: Post Title
date: 2024-01-20T10:00:00Z
excerpt: Brief description of the post
---

Your post content here...

## Subheadings

More content...
```

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Locally

```bash
cargo run
```

## License

MIT License 