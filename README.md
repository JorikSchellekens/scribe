# Scribe

A minimal static site generator written in Rust • ink • eternal

## Features

- **Minimal Design**: Clean, typography-focused design
- **Markdown Support**: Write posts in Markdown with frontmatter
- **Illuminated Initials**: AI-generated decorative initials (requires OpenAI API)
- **IPFS Support**: Pin your site to IPFS for decentralized hosting
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

### IPFS Pinning

Scribe can pin your generated site to IPFS for decentralized hosting:

1. First, make sure you have IPFS installed and running:
```bash
# Install IPFS (if not already installed)
# Visit https://ipfs.io/docs/install/ for installation instructions

# Start IPFS daemon
ipfs daemon
```

2. Generate your site and pin it to IPFS:
```bash
# Generate the site
scribe generate

# Pin to IPFS
scribe pin

# Pin with custom options
scribe pin --dist dist --name "My Blog v1.0" --ipfs-api http://127.0.0.1:5001
```

3. Your site will be available via IPFS gateways:
   - IPFS Hash: `QmX...` (returned by the pin command)
   - Public Gateway: `https://ipfs.io/ipfs/QmX...`
   - Local Gateway: `http://127.0.0.1:8080/ipfs/QmX...`

**IPFS Pin Options:**
- `--dist <DIR>`: Directory to pin (default: dist)
- `--ipfs-api <URL>`: IPFS API endpoint (default: http://127.0.0.1:5001)
- `--name <NAME>`: Optional pin name/description
- `--recursive`: Pin recursively (default: true)

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

### Commands

**Generate**: Build the static site
```bash
scribe generate [OPTIONS]
```
- `-c, --config <FILE>`: Specify config file (default: config.json)

**Serve**: Start local development server  
```bash
scribe serve [OPTIONS]
```
- `-d, --dist <DIR>`: Directory to serve (default: dist)
- `-p, --port <PORT>`: Port to serve on (default: 3007)
- `--host <HOST>`: Host to bind to (default: 127.0.0.1)

**Create**: Create a new blog project
```bash
scribe create <DIRECTORY>
```

**Initials**: Generate illuminated initials
```bash
scribe initials [OPTIONS]
```
- `-l, --letters <LETTERS>`: Letters to generate (e.g., "ABC" or "A,B,C")
- `-c, --config <FILE>`: Config file (default: config.json)
- `-o, --output <DIR>`: Output directory (default: initials)

**Pin**: Pin site to IPFS
```bash
scribe pin [OPTIONS]
```
- `-d, --dist <DIR>`: Directory to pin (default: dist)
- `--ipfs-api <URL>`: IPFS API endpoint (default: http://127.0.0.1:5001)
- `-n, --name <NAME>`: Pin name/description
- `-r, --recursive`: Pin recursively (default: true)

**Global Options:**
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