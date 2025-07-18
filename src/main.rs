use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
use std::process;
use warp::Filter;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};

mod config;
mod generator;
mod templates;

use config::Config;
use generator::SiteGenerator;

#[derive(Parser)]
#[command(name = "scribe")]
#[command(about = "A minimal static site generator • ink • eternal")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate the static site
    Generate {
        /// Path to config file
        #[arg(short, long, default_value = "config.json")]
        config: PathBuf,
    },
    /// Serve the generated site locally
    Serve {
        /// Path to the dist directory to serve
        #[arg(short, long, default_value = "dist")]
        dist: PathBuf,
        
        /// Port to serve on
        #[arg(short, long, default_value = "3007")]
        port: u16,
        
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Generate illuminated initials for specific letters
    Initials {
        /// Letters to generate initials for (e.g., "ABC" or "a,b,c")
        #[arg(short, long)]
        letters: String,
        
        /// Path to config file
        #[arg(short, long, default_value = "config.json")]
        config: PathBuf,
        
        /// Output directory for initials
        #[arg(short, long, default_value = "initials")]
        output: PathBuf,
    },
    /// Create a new blog project
    Create {
        /// Directory to create the project in
        directory: PathBuf,
    },
    /// Pin generated site content to IPFS
    Pin {
        /// Path to the dist directory to pin
        #[arg(short, long, default_value = "dist")]
        dist: PathBuf,
        
        /// IPFS API endpoint
        #[arg(long, default_value = "http://127.0.0.1:5001")]
        ipfs_api: String,
        
        /// Pin name/description
        #[arg(short, long)]
        name: Option<String>,
        
        /// Recursive pin (pin all referenced content)
        #[arg(short, long, default_value = "true")]
        recursive: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Print ASCII art
    println!(
        r#"
   ◜ s c r i b e ◝
    ink • eternal
                                 
"#
    );
    
    match cli.command {
        Commands::Generate { config } => {
            // Load configuration
            let config = Config::load(&config)
                .context("Failed to load configuration")?;
            
            // Create generator
            let mut generator = SiteGenerator::new(config);
            
            // Generate site
            if let Err(e) = generator.generate().await {
                eprintln!("{}", format!("Error: {}", e).red());
                process::exit(1);
            }
        }
        Commands::Serve { dist, port, host } => {
            serve_site(dist, host, port).await?;
        }
        Commands::Initials { letters, config, output } => {
            generate_initials_command(letters, config, output).await?;
        }
        Commands::Create { directory } => {
            create_project(directory).await?;
        }
        Commands::Pin { dist, ipfs_api, name, recursive } => {
            pin_to_ipfs(dist, ipfs_api, name, recursive).await?;
        }
    }
    
    Ok(())
}

async fn serve_site(dist_path: PathBuf, host: String, port: u16) -> Result<()> {
    // Check if dist directory exists
    if !dist_path.exists() {
        eprintln!("{}", format!("Error: Directory '{}' does not exist. Run 'scribe generate' first.", dist_path.display()).red());
        process::exit(1);
    }

    if !dist_path.is_dir() {
        eprintln!("{}", format!("Error: '{}' is not a directory.", dist_path.display()).red());
        process::exit(1);
    }

    println!("{}", format!("Starting server...").green().bold());
    println!("{}", format!("Serving: {}", dist_path.display()).blue());
    println!("{}", format!("URL: http://{}:{}", host, port).blue());
    println!("{}", format!("Press Ctrl+C to stop").yellow());

    // Create the static file serving route
    let static_files = warp::fs::dir(dist_path.clone())
        .or(warp::path::end().and(warp::fs::file(dist_path.join("index.html"))));

    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "DELETE"]);

    let routes = static_files
        .with(cors)
        .with(warp::log("scribe"));

    // Parse the host address
    let addr: std::net::IpAddr = host.parse()
        .context("Invalid host address")?;

    // Start the server
    warp::serve(routes)
        .run((addr, port))
        .await;

    Ok(())
} 

async fn generate_initials_command(letters: String, config_path: PathBuf, output_dir: PathBuf) -> Result<()> {
    // Load configuration
    let config = Config::load(&config_path)
        .context("Failed to load configuration")?;
    
    // Check if OpenAI API key is available
    if config.openai_api_key.is_none() {
        eprintln!("{}", "Error: OPENAI_API_KEY not found in environment or config. Cannot generate illuminated initials.".red());
        process::exit(1);
    }
    
    let api_key = config.openai_api_key.as_ref().unwrap();
    
    // Parse letters (handle both "ABC" and "A,B,C" formats)
    let letters_to_generate: Vec<char> = if letters.contains(',') {
        letters
            .split(',')
            .filter_map(|s| s.trim().chars().next())
            .map(|c| c.to_uppercase().next().unwrap())
            .collect()
    } else {
        letters
            .chars()
            .filter(|c| c.is_alphabetic())
            .map(|c| c.to_uppercase().next().unwrap())
            .collect()
    };
    
    if letters_to_generate.is_empty() {
        eprintln!("{}", "Error: No valid letters provided.".red());
        process::exit(1);
    }
    
    // Create output directory
    std::fs::create_dir_all(&output_dir)
        .context("Failed to create output directory")?;
    
    println!("{}", format!("Generating illuminated initials for: {}", 
        letters_to_generate.iter().collect::<String>()).cyan());
    
    // Generate initials in parallel
    let mut tasks = Vec::new();
    
    for letter in letters_to_generate {
        let initial_path = output_dir.join(format!("{}.txt", letter));
        if !initial_path.exists() {
            println!("Generating illuminated initial '{}'", letter);
            let api_key = api_key.clone();
            let task = tokio::spawn(async move {
                SiteGenerator::generate_illuminated_initial_static(letter, "Custom", &api_key).await
            });
            tasks.push((task, initial_path, letter));
        } else {
            println!("Illuminated initial for '{}' already exists, skipping", letter);
        }
    }
    
    // Wait for all tasks to complete
    for (task, initial_path, letter) in tasks {
        match task.await {
            Ok(Ok(image_url)) => {
                println!("Successfully generated illuminated initial for '{}'", letter);
                std::fs::write(initial_path, image_url)?;
            }
            Ok(Err(e)) => {
                eprintln!("Failed to generate illuminated initial for '{}': {}", letter, e);
            }
            Err(e) => {
                eprintln!("Task failed for illuminated initial '{}': {}", letter, e);
            }
        }
    }
    
    println!("{}", "Illuminated initials generation complete!".green());
    
    Ok(())
}

async fn create_project(directory: PathBuf) -> Result<()> {
    use std::io::{self, Write};
    
    // Clear screen and show header
    print!("\x1B[2J\x1B[1;1H");
    println!("{}", r#"
   ◜ s c r i b e ◝
    ink • eternal
    
"#.cyan().bold());
    
    println!("{}", "Welcome to Scribe project creation!".green().bold());
    println!("{}", "Let's set up your new blog...".white());
    println!();
    
    // Check if directory exists and handle accordingly
    if directory.exists() && directory.read_dir()?.next().is_some() {
        println!("{}", format!("Directory '{}' already exists and is not empty.", directory.display()).yellow());
        print!("Continue anyway? (y/N): ");
        io::stdout().flush()?;
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        if !response.trim().to_lowercase().starts_with('y') {
            println!("{}", "Project creation cancelled.".red());
            return Ok(());
        }
        println!();
    }
    
    // Helper function for prompts
    let prompt = |question: &str, default: Option<&str>| -> Result<String> {
        loop {
            if let Some(def) = default {
                print!("{} [{}]: ", question.cyan().bold(), def.green());
            } else {
                print!("{}: ", question.cyan().bold());
            }
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();
            
            if input.is_empty() {
                if let Some(def) = default {
                    return Ok(def.to_string());
                } else {
                    println!("{}", "  This field is required. Please enter a value.".red());
                    continue;
                }
            }
            
            return Ok(input.to_string());
        }
    };
    
    // Collect configuration with nice prompts
    println!("{}", "Site Configuration".white().bold().underline());
    println!();
    
    let title = prompt("Site title", Some("My Blog"))?;
    let description = prompt(
        "Site description", 
        Some("A minimal blog powered by Scribe")
    )?;
    let author = prompt("Author name", None)?;
    
    print!("{} (optional): ", "Site URL".cyan().bold());
    io::stdout().flush()?;
    let mut url_input = String::new();
    io::stdin().read_line(&mut url_input)?;
    let url = if url_input.trim().is_empty() {
        None
    } else {
        Some(url_input.trim().to_string())
    };
    
    println!();
    
    // Show configuration summary
    println!("{}", "Configuration Summary".white().bold().underline());
    println!();
    println!("  {}: {}", "Title".white().bold(), title.green());
    println!("  {}: {}", "Description".white().bold(), description.green());
    println!("  {}: {}", "Author".white().bold(), author.green());
    if let Some(ref url_val) = url {
        println!("  {}: {}", "URL".white().bold(), url_val.green());
    } else {
        println!("  {}: {}", "URL".white().bold(), "Not set".yellow());
    }
    println!("  {}: {}", "Directory".white().bold(), directory.display().to_string().green());
    println!();
    
    // Confirm creation
    print!("{}", "Create project with these settings? (Y/n): ".cyan().bold());
    io::stdout().flush()?;
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;
    if confirm.trim().to_lowercase().starts_with('n') {
        println!("{}", "Project creation cancelled.".red());
        return Ok(());
    }
    
    println!();
    println!("{}", "Creating project...".yellow().bold());
    
    // Create directory if it doesn't exist
    if !directory.exists() {
        std::fs::create_dir_all(&directory)
            .context("Failed to create project directory")?;
        println!("  {} Directory created", "✓".green());
    }
    
    // Create config
    let config = Config {
        title,
        description: Some(description),
        author,
        url,
        posts_dir: "posts".to_string(),
        output_dir: "dist".to_string(),
        openai_api_key: None,
        theme: config::Theme::default(),
    };
    
    // Write config file
    let config_path = directory.join("config.json");
    let config_content = serde_json::to_string_pretty(&config)
        .context("Failed to serialize config")?;
    std::fs::write(&config_path, config_content)
        .context("Failed to write config file")?;
    println!("  {} Configuration file created", "✓".green());
    
    // Create posts directory
    let posts_dir = directory.join("posts");
    std::fs::create_dir_all(&posts_dir)
        .context("Failed to create posts directory")?;
    println!("  {} Posts directory created", "✓".green());
    
    // Create sample post with dynamic content
    let sample_post = format!(r#"---
title: "Welcome to {}"
date: "{}"
excerpt: "Your first post on your new Scribe-powered blog."
---

Welcome to your new Scribe-powered blog! This is your first post.

Scribe is a minimal static site generator that focuses on typography and clean design. Write your posts in Markdown and let Scribe handle the rest.

## What makes Scribe special?

**Beautiful Typography**: Scribe uses carefully selected fonts and spacing to make your content shine. Every detail is crafted for optimal reading experience.

**Illuminated Initials**: Add AI-generated decorative first letters to your posts for a touch of classical elegance.

**Fast and Minimal**: Built with Rust for blazing-fast generation. No bloat, just what you need.

**Developer Friendly**: Simple Markdown files, clean configuration, and powerful CLI tools.

## Getting Started

1. **Write**: Create new posts in the `posts/` directory using Markdown
2. **Generate**: Run `scribe generate` to build your static site  
3. **Serve**: Use `scribe serve` to preview your site locally
4. **Deploy**: Upload the `dist/` directory to any static hosting service

## Advanced Features

- **Illuminated Initials**: Set `OPENAI_API_KEY` to generate beautiful decorative letters
- **Backlinks**: Automatic detection of links between your posts
- **Custom Themes**: Modify colors and styles in your config file

Happy writing, and welcome to the world of beautiful, minimal blogging!
"#, 
        config.title,
        chrono::Utc::now().format("%Y-%m-%d")
    );
    
    let sample_post_path = posts_dir.join("welcome.md");
    std::fs::write(&sample_post_path, sample_post)
        .context("Failed to create sample post")?;
    println!("  {} Welcome post created", "✓".green());
    
    // Create .gitignore
    let gitignore_content = r#"# Generated site
dist/

# Environment variables
.env
*.env

# IDE and editor files
.vscode/
.idea/
*.swp
*.swo
*~

# Operating system files
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db

# Logs
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Runtime data
pids
*.pid
*.seed
*.pid.lock
"#;
    
    let gitignore_path = directory.join(".gitignore");
    std::fs::write(&gitignore_path, gitignore_content)
        .context("Failed to create .gitignore")?;
    println!("  {} Git ignore file created", "✓".green());
    
    // Create README
    let readme_content = format!(r#"# {}

{}

## Quick Start

```bash
# Generate your site
scribe generate

# Serve locally (development)
scribe serve

# Visit http://localhost:3007
```

## Writing Posts

Create new Markdown files in the `posts/` directory:

```markdown
---
title: "Your Post Title"
date: "2024-01-20"
excerpt: "A brief description of your post"
---

Your post content here...
```

## Illuminated Initials

To enable AI-generated decorative first letters:

1. Get an OpenAI API key
2. Set the environment variable: `export OPENAI_API_KEY="your-key-here"`
3. Generate specific letters: `scribe initials --letters "ABC"`

## Configuration

Edit `config.json` to customize your site's appearance and settings.

## Deployment

Upload the contents of the `dist/` directory to any static hosting service:
- GitHub Pages
- Netlify
- Vercel
- Your own server

Built with [Scribe](https://github.com/your-username/scribe) • ink • eternal
"#, config.title, config.description.as_deref().unwrap_or(""));
    
    let readme_path = directory.join("README.md");
    std::fs::write(&readme_path, readme_content)
        .context("Failed to create README")?;
    println!("  {} README file created", "✓".green());
    
    println!();
    println!("{}", "Project created successfully!".green().bold());
    
    // Show file tree
    println!();
    println!("{}", "Project structure:".white().bold());
    println!("{}                                                      ", directory.display().to_string().cyan().bold());
    println!("├── {}", "config.json".white());
    println!("├── {}", "README.md".white());
    println!("├── {}", ".gitignore".white());
    println!("└── {}/", "posts".white());
    println!("    └── {}", "welcome.md".white());
    
    println!();
    println!("{}", "Next steps:".yellow().bold());
    
    if directory != PathBuf::from(".") {
        println!("  1. {}", format!("cd {}", directory.display()).cyan());
    }
    
    println!("  {}. {}", if directory == PathBuf::from(".") { "1" } else { "2" }, "Set up OpenAI API key (optional):".white());
    println!("     {}", "export OPENAI_API_KEY=\"your-key-here\"".cyan());
    
    println!("  {}. {}", if directory == PathBuf::from(".") { "2" } else { "3" }, "Generate your site:".white());
    println!("     {}", "scribe generate".cyan());
    
    println!("  {}. {}", if directory == PathBuf::from(".") { "3" } else { "4" }, "Start development server:".white());
    println!("     {}", "scribe serve".cyan());
    
    println!();
    println!("{}", "Happy blogging!".cyan().bold());
    println!("{}", "Visit http://localhost:3007 after running the commands above.".white());
    
    Ok(())
}

async fn pin_to_ipfs(
    dist_path: PathBuf, 
    ipfs_api: String, 
    name: Option<String>, 
    recursive: bool
) -> Result<()> {
    // Check if dist directory exists
    if !dist_path.exists() {
        eprintln!("{}", format!("Error: Directory '{}' does not exist. Run 'scribe generate' first.", dist_path.display()).red());
        process::exit(1);
    }

    if !dist_path.is_dir() {
        eprintln!("{}", format!("Error: '{}' is not a directory.", dist_path.display()).red());
        process::exit(1);
    }

    println!("{}", format!("Connecting to IPFS node at {}...", ipfs_api).blue());
    
    // Create IPFS client
    let client = IpfsClient::from_str(&ipfs_api)
        .context("Failed to create IPFS client")?;
    
    // Test connection to IPFS node
    match client.version().await {
        Ok(version) => {
            println!("{} Connected to IPFS node (version: {})", "✓".green(), version.version);
        }
        Err(e) => {
            eprintln!("{}", format!("Error: Failed to connect to IPFS node at {}", ipfs_api).red());
            eprintln!("{}", format!("Make sure IPFS daemon is running. Error: {}", e).yellow());
            eprintln!("{}", "Start IPFS daemon with: ipfs daemon".cyan());
            process::exit(1);
        }
    }
    
    println!("{}", format!("Adding directory {} to IPFS...", dist_path.display()).yellow());
    
    // Add the directory to IPFS
    let add_result = client
        .add_path(&dist_path)
        .await
        .context("Failed to add directory to IPFS")?;
    
    // Find the root directory hash
    let mut root_hash = None;
    let mut total_files = 0;
    
    for item in add_result {
        total_files += 1;
        // The root directory will have the same name as the source directory
        if item.name == dist_path.file_name().unwrap().to_str().unwrap() {
            root_hash = Some(item.hash.clone());
        }
        println!("  {} Added: {} ({})", "✓".green(), item.name, item.hash);
    }
    
    let root_hash = root_hash.unwrap_or_else(|| {
        eprintln!("{}", "Error: Could not determine root directory hash".red());
        process::exit(1);
    });
    
    println!("{}", format!("Successfully added {} files to IPFS", total_files).green());
    println!("{}", format!("Root directory hash: {}", root_hash).cyan().bold());
    
    // Pin the content
    if recursive {
        println!("{}", "Pinning content recursively...".yellow());
        match client.pin_add(&root_hash, recursive).await {
            Ok(_) => {
                println!("{} Content pinned successfully!", "✓".green());
            }
            Err(e) => {
                eprintln!("{}", format!("Warning: Failed to pin content: {}", e).yellow());
                eprintln!("{}", "Content is still available on IPFS but may be garbage collected".yellow());
            }
        }
    }
    
    // Set pin name if provided
    if let Some(pin_name) = name {
        println!("{}", format!("Setting pin name to '{}'...", pin_name).yellow());
        // Note: pin naming is not available in all IPFS implementations
        // This is a placeholder for when the API supports it
        println!("{}", format!("Pin name '{}' noted (naming support varies by IPFS implementation)", pin_name).cyan());
    }
    
    println!();
    println!("{}", "IPFS Pinning Complete!".green().bold());
    println!();
    println!("{}", "Access your site via IPFS:".white().bold());
    println!("  {}: {}", "IPFS Hash".white(), root_hash.clone().cyan());
    println!("  {}: {}", "IPFS Gateway".white(), format!("https://ipfs.io/ipfs/{}", root_hash).blue());
    println!("  {}: {}", "Local Gateway".white(), format!("http://127.0.0.1:8080/ipfs/{}", root_hash).blue());
    
    // Show alternative gateways
    println!();
    println!("{}", "Alternative IPFS Gateways:".white().bold());
    println!("  • {}", format!("https://gateway.pinata.cloud/ipfs/{}", root_hash).blue());
    println!("  • {}", format!("https://cloudflare-ipfs.com/ipfs/{}", root_hash).blue());
    println!("  • {}", format!("https://dweb.link/ipfs/{}", root_hash).blue());
    
    println!();
    println!("{}", "💡 Pro Tips:".yellow().bold());
    println!("  • Pin your content on multiple IPFS nodes for better availability");
    println!("  • Consider using a pinning service like Pinata or Infura for production");
    println!("  • Share the IPFS hash for decentralized access to your site");
    
    Ok(())
} 