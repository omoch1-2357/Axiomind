//! Standalone web server binary
//!
//! Usage: cargo run -p axm_web --bin axm-web-server

use axm_web::{ServerConfig, WebServer};
use clap::Parser;
use std::path::PathBuf;

/// Axiomind Web Server - Heads-up no-limit hold'em training table
#[derive(Parser, Debug)]
#[command(name = "axm-web-server")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Host address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port number to bind to
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Static files directory path
    #[arg(short = 'd', long)]
    static_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    axm_web::init_logging();

    // Parse command line arguments using clap
    let args = Args::parse();

    // Determine static directory
    let static_path = if let Some(dir) = args.static_dir {
        dir
    } else {
        // Try to find static directory relative to workspace root
        let current_dir = std::env::current_dir()?;
        let candidates = vec![
            current_dir.join("rust").join("web").join("static"),
            current_dir.join("static"),
            PathBuf::from("static"),
        ];

        candidates
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| {
                eprintln!("Error: Could not find static directory.");
                eprintln!("Tried:");
                eprintln!("  - rust/web/static");
                eprintln!("  - static");
                eprintln!("Please specify with --static-dir");
                std::process::exit(1);
            })
    };

    tracing::info!("Static directory: {}", static_path.display());
    tracing::info!("Static directory exists: {}", static_path.exists());

    // Create server configuration
    let config = ServerConfig::new(args.host.clone(), args.port, static_path);

    tracing::info!("Starting Axiomind Web Server");
    tracing::info!("  Host: {}", config.host());
    tracing::info!("  Port: {}", config.port());
    tracing::info!("  Static: {}", config.static_dir().display());

    // Create and start server
    let server = WebServer::new(config)?;
    let handle = server.start().await?;

    tracing::info!("Server running at http://{}", handle.address());
    println!("\n✅ Server running at http://{}", handle.address());
    println!("   Press Ctrl+C to stop\n");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutting down server");
    println!("\n🛑 Shutting down...");
    handle.shutdown().await?;
    tracing::info!("Server stopped cleanly");
    println!("✅ Server stopped cleanly\n");

    Ok(())
}
