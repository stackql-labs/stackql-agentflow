use std::env;

use agentflow::{
    observability::{ConsoleSink, FileSink},
    tools::FilesystemTool,
    Pipeline,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env if present (ANTHROPIC_API_KEY)
    let _ = dotenvy::dotenv();

    // Structured logging (set RUST_LOG=info or debug to see more)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    // Topic can be passed as a CLI arg; fall back to a default
    let topic = env::args()
        .nth(1)
        .unwrap_or_else(|| "how Rust's ownership model prevents memory bugs".to_string());

    let api_key = env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set (or present in .env)");

    // Locate the pipeline YAML relative to the workspace root.
    // When running with `cargo run -p hello-world` the CWD is the workspace root.
    let yaml_path = locate_pipeline_yaml();

    println!("=========================================================");
    println!("  stackql-agentflow  |  hello-world demo");
    println!("=========================================================");
    println!("  topic : {}", topic);
    println!("---------------------------------------------------------");
    println!("  observability UI  ->  http://localhost:4000");
    println!("  audit log         ->  hello-world-run.jsonl");
    println!("  ctrl+c to exit after the run completes");
    println!("=========================================================");
    println!();

    let mut pipeline = Pipeline::from_file(&yaml_path, &api_key)?;

    // Built-in filesystem tool
    pipeline.register_tool(FilesystemTool);

    // Console sink: prints every event as JSON to stdout
    pipeline.register_sink(Box::new(ConsoleSink));

    // File sink: appends JSONL for audit retention
    let file_sink = FileSink::new("hello-world-run.jsonl").await?;
    pipeline.register_sink(Box::new(file_sink));

    pipeline.run(&format!("Topic: {}", topic)).await?;

    println!();
    println!("Pipeline complete. Keeping the observability server alive.");
    println!("Press ctrl+c to exit.");

    // Keep the process alive so the user can inspect the UI
    tokio::signal::ctrl_c().await?;

    Ok(())
}

/// Find pipeline.yaml — works whether cargo run is invoked from the workspace
/// root or from inside examples/hello-world.
fn locate_pipeline_yaml() -> String {
    let candidates = [
        "examples/hello-world/pipeline.yaml",
        "pipeline.yaml",
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    // Fallback: let the error propagate from Pipeline::from_file
    "examples/hello-world/pipeline.yaml".to_string()
}

