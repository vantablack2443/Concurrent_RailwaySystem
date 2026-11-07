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
}

/// Entrypoint of the `train-sim` binary.
fn main() -> io::Result<()> {
    let args = Args::parse();
    let scenario = Scenario::load(&args.scenario)?;

    println!("Scenario: {}", scenario.name);

    let simulator: Box<dyn FnOnce() -> Output> = if args.slug {
        Box::new(move || train_sim_slug::creep(scenario))
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
