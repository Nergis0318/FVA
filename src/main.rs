use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{fmt, EnvFilter};

use fva::config::Config;
use fva::engine::FvaEngine;
use fva::mcp::FvaServer;
use fva::query::context::ContextBuilder;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// FVA — Fast Vector Agent: hybrid codebase intelligence for AI coding agents.
#[derive(Parser)]
#[command(name = "fva", version, about, long_about = None)]
struct Cli {
    /// Project root to index.
    #[arg(short, long, global = true, value_name = "PATH")]
    path: Option<String>,

    /// Config file path.
    #[arg(short, long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Log level override.
    #[arg(long, global = true, env = "RUST_LOG")]
    log_level: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start MCP server on stdio (default).
    Serve,
    /// Run full index (AST + vectors + call graph) and exit.
    Index,
    /// Print indexing status.
    Status,
    /// Hybrid search from CLI.
    Search {
        /// Search query.
        query: String,
        /// Max results.
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Print version info.
    Version,
}

fn init_logging(config: &Config, cli_level: Option<&str>) {
    let level = cli_level
        .or(Some(config.mcp.log_level.as_str()))
        .unwrap_or("info");

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    if config.mcp.log_file.is_empty() {
        fmt().with_env_filter(filter).with_target(false).init();
    } else {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.mcp.log_file);
        match file {
            Ok(f) => {
                fmt()
                    .with_env_filter(filter)
                    .with_target(false)
                    .with_writer(std::sync::Mutex::new(f))
                    .init();
            }
            Err(e) => {
                eprintln!("warning: cannot open log file: {e}");
                fmt().with_env_filter(filter).with_target(false).init();
            }
        }
    }
}

fn ensure_data_dirs(config: &Config, root: &std::path::Path) {
    let data_dir = config.resolve_data_dir(root);
    let _ = std::fs::create_dir_all(&data_dir);
    let _ = std::fs::create_dir_all(data_dir.join("frecency"));
    let _ = std::fs::create_dir_all(data_dir.join("history"));
    let _ = std::fs::create_dir_all(data_dir.join("vectors"));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if matches!(cli.command, Some(Commands::Version)) {
        println!("fva {} — Fast Vector Agent", env!("CARGO_PKG_VERSION"));
        println!("Phases 1-4: FFF + Tree-sitter + Vectors + Call Graph + MCP");
        return Ok(());
    }

    let config = Config::load(cli.config.as_deref())?;
    init_logging(&config, cli.log_level.as_deref());

    let root = config.resolve_root(cli.path.as_deref())?;
    ensure_data_dirs(&config, &root);

    tracing::info!("FVA starting — root: {}", root.display());

    let engine = Arc::new(FvaEngine::new(config, root)?);

    // Wait for FFF scan in background
    let fff_clone = engine.fff.clone();
    tokio::task::spawn_blocking(move || {
        if fff_clone.wait_for_scan(Duration::from_secs(120)) {
            tracing::info!("FFF scan complete — {} files", fff_clone.total_files());
        }
    });

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Index => {
            let count = engine.indexer.index_all()?;
            println!(
                "Indexed {count} chunks — {:?}",
                engine.indexer.stats()
            );
            println!("Vectors: {:?}", engine.vectors.stats());
            println!("Graph: {:?}", engine.graph.stats());
            engine.shutdown();
        }

        Commands::Status => {
            // Load persisted index if in-memory store is empty
            if engine.indexer.stats().indexed_files == 0 {
                let _ = engine.indexer.index_all();
            }
            let status = serde_json::json!({
                "fff_files": engine.fff.total_files(),
                "ast": engine.indexer.stats(),
                "vectors": engine.vectors.stats(),
                "graph": engine.graph.stats(),
                "embedder": engine.embedder.name(),
            });
            println!("{}", serde_json::to_string_pretty(&status)?);
            engine.shutdown();
        }

        Commands::Search { query, limit } => {
            if engine.indexer.stats().indexed_files == 0 {
                let _ = engine.indexer.index_all();
            }
            let result = engine.query.hybrid_search(&query, limit);
            let ctx = engine.context.build(&query, None, &result);
            println!("{}", ContextBuilder::format_context(&ctx));
            engine.shutdown();
        }

        Commands::Version => unreachable!("version handled above"),

        Commands::Serve => {
            engine.indexer.spawn_background_index();

            let server = FvaServer::new(engine.clone());

            let engine_shutdown = engine.clone();
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.ok();
                tracing::info!("shutting down FVA...");
                engine_shutdown.shutdown();
                let _ = engine_shutdown
                    .indexer
                    .wait_for_index(Duration::from_secs(5));
                std::process::exit(0);
            });

            tracing::info!("MCP server starting on stdio");
            let service = server
                .serve(stdio())
                .await
                .map_err(|e| format!("MCP server error: {e}"))?;

            service.waiting().await?;
            engine.shutdown();
        }
    }

    Ok(())
}