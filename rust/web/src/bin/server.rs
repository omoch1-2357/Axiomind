//! Standalone web server binary
//!
//! Usage: cargo run -p axm_web --bin axm-web-server

use axm_web::{ServerConfig, WebServer};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    axm_web::init_logging();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut host = "127.0.0.1".to_string();
    let mut port = 8080u16;
    let mut static_dir: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--host" | "-h" => {
                if i + 1 < args.len() {
                    host = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --host requires a value");
                    std::process::exit(1);
                }
            }
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or_else(|_| {
                        eprintln!("Error: invalid port number");
                        std::process::exit(1);
                    });
                    i += 2;
                } else {
                    eprintln!("Error: --port requires a value");
                    std::process::exit(1);
                }
            }
            "--static-dir" | "-d" => {
                if i + 1 < args.len() {
                    static_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("Error: --static-dir requires a value");
                    std::process::exit(1);
                }
            }
            "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // Determine static directory
    let static_path = if let Some(dir) = static_dir {
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
    let config = ServerConfig::new(host.clone(), port, static_path);

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

fn print_help() {
    println!("Axiomind Web Server");
    println!();
    println!("Usage: axm-web-server [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --host, -h <HOST>           Host to bind to (default: 127.0.0.1)");
    println!("  --port, -p <PORT>           Port to bind to (default: 8080)");
    println!("  --static-dir, -d <DIR>      Static files directory");
    println!("  --help                      Show this help message");
}
