# CP Project: Concurrency Bahn Simulation – Template ☕️ & 🦀

Reference implementation and template for the concurrent programming project 2026.

## Visualization 🗺️

**Live page: https://vantablack2443.github.io/Concurrent_RailwaySystem/**

An interactive replay that shows how the concurrent (Rocket) simulation works:

- a map with zones, stations and tracks, and trains moving, queuing for busy tracks and arriving,
- one clock badge per zone thread showing its own local tick at the same moment,
- a wall-clock timeline per thread: working vs. blocked, train handoffs, promises and max-tick broadcasts,
- a log of the messages exchanged between threads,
- a six-step guided tour, and a Slug vs Rocket comparison (same output, runtime).

It replays a run recorded with the `viz` cargo feature, which makes every zone worker log its events
into a thread-local buffer. The feature is off by default, so the normal build and `launch()` are unchanged.

| Command | What it does |
|---|---|
| `just viz-record` | Records `scenarios/viz/island.json` and `europe_500` into `site/data/*.viz.json` |
| `just viz-bench` | Times Slug vs Rocket, checks their outputs match, writes `site/data/bench.json` |
| `just viz-serve` | Serves the page at http://localhost:8000 |

The recording can also be done by hand:

```bash
cargo run --release -p train-sim --features viz -- \
  --scenario scenarios/viz/island.json --out out.json --viz-out site/data/island.viz.json
```

The page lives in `site/` (plain HTML, CSS and JavaScript modules, no build step). On every push to
`main`, `.github/workflows/pages.yml` runs the tests, checks the `viz` build compiles and deploys
`site/` to GitHub Pages (Settings → Pages → Source: *GitHub Actions*).

## Structure

This project is structured as follows:

- `crates/`: Rust crates of the project.
  - `train-sim/`: Command line interface and main binary.
  - `train-sim-core/`: Core data structures and algorithms.
  - `train-sim-rocket/`: **Your Rust concurrent implementation goes here**.
  - `train-sim-slug/`: Sequential reference implementation.
  - `train-sim-tests/`: Public tests and testing infrastructure.
- `site/`: The visualization page (see above), with recorded data in `site/data/`.
- `scripts/viz_bench.py`: Slug vs Rocket benchmark for the page.
- `src/main/java/cp26/`: Java source code of the project.
  - `scenario/`: Data structures representing a scenario.
  - `output/`: Data structures for writing the simulation output.
  - `simulation/common/`: Useful simulation functionality.
  - `simulation/rocket/`: **Your Java implementation goes here**.
  - `simulation/slug/`: The sequential reference implementation.
  - `validator/`: The validator interface.
  - `Simulation.java`: Implements the `main` method.
- `src/test/java/cp26/`: Public tests for the project.
- `scenarios/`: Some sample scenarios; `scenarios/viz/` has the island scenario for the page.

We initialized the `project.toml` according to your language preferences, but in case you want to switch the programming language, it is possible there. If you implement the bonus, also enable the `bonus-implemented` flag there.

## Getting Started

A detailed description on how to install Java and Rust can be found in the
[forum](https://cp26.pseuco.com/t/how-to-project-in-vs-code/130).

We recommend you use a proper *Integrated Development Environment* (IDE) for
the project. A good option is [VS Code](https://code.visualstudio.com/). How to
set up the project in VS Code is documented in
[the same forum post](https://cp26.pseuco.com/t/how-to-project-in-vs-code/130).

Which IDE or editor you use is up to you. However, we only provide help for
VS Code. In case you use some other code editor, do not expect help.

## Running the Project

### Java ☕

We use [Gradle](https://gradle.org/) to build the project.
To build the Javadoc run:

```bash
./gradlew javaDoc
```

Afterwards you find the documentation in `build/docs`.

To build a `simulation.jar`-File for your project run:

```bash
./gradlew jar
```

You find the compiled `.jar`-File in `out`.

To run the *public* tests on your project run:

```bash
./gradlew test
```

### Rust 🦀

Remember, Cargo is your friend:

- `cargo build` builds the project.
- `cargo doc` generates API documentation. `cargo doc --open` additionally opens the generated documentation in a web browser.
- `cargo test --release -- --test-threads=1` runs the tests. Note that the `--test-threads=1` is there to not interfere with the performance of the simulation, given that `cargo` run test concurrently by default.

We will use Rust 1.96.1 to test your code.

## For Convenience: A `justfile` 🎉

We provide a [`justfile`](https://just.systems/man/en/). To install `just` via Cargo, run `cargo install just`. You can then run tests simply with `just test`. Note that our `.gitlab-ci.yml` also uses `just`. A complete list of available commands can be obtained via `just help` or `just list`:

``` md
    build      # Build the project
    doc        # Generate API documentation
    doc-open   # Generate API documentation and open it in your web browser
    help       # Print available recipes
    lang       # Get the detected programming language
    lint       # Run clippy on Rust code
    list       # Print available recipes
    run *FLAGS # Build and run the project
    test       # Run the tests 
```

To build and open the documentation, run `just doc-open`.

## Integrated Development Environment

We recommend you use a proper *Integrated Development Environment* (IDE) for this project. A good open source IDE is [VS Code](https://code.visualstudio.com/). Which IDE or editor you use is up to you. However, we only provide help for VS Code. In case you use something else, do not expect help.

In case you decide to use VS Code, open the `train-simulator.code-workspace` workspace. After opening the workspace, VS Code should ask you whether you want to install the *recommended extensions*. For maximal convenience, please do so.
In particular, the *Rust Analyzer* extension is highly recommended.
