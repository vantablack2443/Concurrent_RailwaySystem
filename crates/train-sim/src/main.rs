use std::{io, path::PathBuf, sync::Arc, time::Instant};

use clap::Parser;
use train_sim_core::{output::Output, scenario::Scenario, validator::DummyValidator};

/// Command line arguments.
#[derive(Debug, Parser)]
pub struct Args {
    #[arg(long = "scenario")]
    scenario: PathBuf,
    #[arg(long = "out")]
    out: PathBuf,
    #[arg(long = "slug", default_value_t = false, conflicts_with_all = ["rocket", "starship"])]
    slug: bool,
    #[arg(long = "rocket", default_value_t = true, conflicts_with = "slug")]
    rocket: bool,
    #[arg(long = "starship", default_value_t = false, conflicts_with = "slug")]
    starship: bool,
    /// Write the events recorded for the visualization page to this path.
    #[cfg(feature = "viz")]
    #[arg(long = "viz-out", conflicts_with_all = ["slug", "starship"])]
    viz_out: Option<PathBuf>,
}

/// Entrypoint of the `train-sim` binary.
fn main() -> io::Result<()> {
    let args = Args::parse();
    let scenario = Scenario::load(&args.scenario)?;

    println!("Scenario: {}", scenario.name);

    let simulator: Box<dyn FnOnce() -> Output> = if args.slug {
        Box::new(move || train_sim_slug::creep(scenario))
    } else if let Some(viz_out) = viz_out(&args) {
        #[cfg(feature = "viz")]
        {
            Box::new(move || {
                let validator = Arc::new(DummyValidator);
                let (output, log) = train_sim_rocket::launch_with_viz(scenario, validator);
                let json = log.to_json(&output.scenario);
                std::fs::write(&viz_out, json.to_string()).expect("failed to write viz output");
                println!("Viz events: {} -> {}", log.events.len(), viz_out.display());
                output
            })
        }
        #[cfg(not(feature = "viz"))]
        match viz_out {}
    } else {
        Box::new(move || {
            let validator = Arc::new(DummyValidator);
            train_sim_rocket::launch(scenario, validator, args.starship)
        })
    };

    println!(
        "Running simulation... {}",
        if args.slug { "🐌" } else { "🚀" }
    );

    let start = Instant::now();
    let output = simulator();
    let duration = start.elapsed();

    println!("Time: {}ms", duration.as_millis());
    output.save(&args.out)?;

    Ok(())
}

#[cfg(feature = "viz")]
fn viz_out(args: &Args) -> Option<PathBuf> {
    args.viz_out.clone()
}

/// Without the `viz` feature there is no `--viz-out`; the uninhabited return
/// type lets the viz branch in `main` compile away.
#[cfg(not(feature = "viz"))]
fn viz_out(_args: &Args) -> Option<std::convert::Infallible> {
    None
}
