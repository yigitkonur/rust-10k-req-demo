//! Blaze API CLI - High-performance batch API client.
//!
//! Run `blaze --help` for usage information.

use anyhow::Result;
use blaze_api::{Args, Config, Processor};
use console::style;
use tracing::{error, info, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse_args();

    // Setup logging
    setup_logging(&args);

    // Print banner
    if !args.json_logs {
        print_banner();
    }

    // Load configuration
    let config = match Config::from_args(&args) {
        Ok(c) => c,
        Err(e) => {
            error!("Configuration error: {}", e);
            eprintln!("{} {}", style("Error:").red().bold(), e);
            std::process::exit(1);
        }
    };

    // Validate input file exists
    if !args.input.exists() {
        error!("Input file not found: {:?}", args.input);
        eprintln!(
            "{} Input file not found: {}",
            style("Error:").red().bold(),
            args.input.display()
        );
        std::process::exit(1);
    }

    // Dry run mode
    if args.dry_run {
        println!("\n{}", style("DRY RUN MODE").yellow().bold());
        println!("Configuration validated successfully.\n");
        print_config_summary(&args, &config);
        return Ok(());
    }

    // Print configuration summary
    if args.verbose && !args.json_logs {
        print_config_summary(&args, &config);
    }

    // Create processor and run
    let processor = Processor::new(config)?;

    info!(
        input = %args.input.display(),
        output = ?args.output,
        "Starting processing"
    );

    let result = processor
        .process_file(
            args.input.clone(),
            args.output.clone(),
            args.errors.clone(),
            !args.no_progress && !args.json_logs,
        )
        .await?;

    // Print results
    if !args.json_logs {
        result.print_summary();

        if let Some(output) = &args.output {
            println!(
                "\n{} Results saved to: {}",
                style("✓").green().bold(),
                output.display()
            );
        }

        if result.failure_count > 0 {
            println!(
                "{} Errors saved to: {}",
                style("⚠").yellow().bold(),
                args.errors.display()
            );
        }
    } else {
        // JSON output for programmatic consumption
        let json_result = serde_json::json!({
            "status": "complete",
            "total_processed": result.total_processed,
            "success_count": result.success_count,
            "failure_count": result.failure_count,
            "success_rate": result.success_rate(),
            "elapsed_seconds": result.elapsed.as_secs_f64(),
            "avg_latency_ms": result.avg_latency_ms,
            "throughput_rps": result.overall_rps,
        });
        println!("{}", serde_json::to_string(&json_result)?);
    }

    // Exit with error code if there were failures
    if result.failure_count > 0 && result.success_count == 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn setup_logging(args: &Args) {
    let level = if args.verbose { Level::DEBUG } else { Level::INFO };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("blaze_api={},blaze={}", level, level)));

    if args.json_logs {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_target(false)
                    .with_thread_ids(false)
                    .compact(),
            )
            .init();
    }
}

fn print_banner() {
    let banner = r#"
    ____  __                       ___    ____  ____
   / __ )/ /___ _____  ___        /   |  / __ \/  _/
  / __  / / __ `/_  / / _ \      / /| | / /_/ // /  
 / /_/ / / /_/ / / /_/  __/     / ___ |/ ____// /   
/_____/_/\__,_/ /___/\___/     /_/  |_/_/   /___/   
                                                    
    "#;

    println!("{}", style(banner).cyan().bold());
    println!(
        "    {}",
        style("High-Performance Batch API Client").white().dim()
    );
    println!(
        "    {}",
        style(format!("v{}", blaze_api::VERSION)).white().dim()
    );
    println!();
}

fn print_config_summary(args: &Args, config: &Config) {
    println!("{}", style("Configuration:").bold());
    println!("  Input:      {}", args.input.display());
    if let Some(output) = &args.output {
        println!("  Output:     {}", output.display());
    }
    println!("  Errors:     {}", args.errors.display());
    println!("  Rate Limit: {} req/sec", config.request.rate_limit);
    println!("  Workers:    {}", config.request.workers);
    println!("  Timeout:    {:?}", config.request.timeout);
    println!("  Retries:    {}", config.retry.max_attempts);
    println!("  Endpoints:  {}", config.endpoints.len());
    for (i, ep) in config.endpoints.iter().enumerate() {
        println!(
            "    {}. {} (weight: {}, max: {})",
            i + 1,
            ep.url,
            ep.weight,
            ep.max_concurrent
        );
    }
    println!();
}
