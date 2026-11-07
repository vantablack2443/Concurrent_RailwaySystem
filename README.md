# CP Project: Concurrency Bahn Simulation – Template ☕️ & 🦀

Reference implementation and template for the concurrent programming project 2026.

## Structure

This project is structured as follows:

- `crates/`: Rust crates of the project.
  - `train-sim/`: Command line interface and main binary.
  - `train-sim-core/`: Core data structures and algorithms.
  - `train-sim-rocket/`: **Your Rust concurrent implementation goes here**.
  - `train-sim-slug/`: Sequential reference implementation.
  - `train-sim-tests/`: Public tests and testing infrastructure.
- `src/main/java/cp26/`: Java source code of the project.
  - `scenario/`: Data structures representing a scenario.
  - `output/`: Data structures for writing the simulation output.
  - `simulation/common/`: Useful simulation functionality.
  - `simulation/rocket/`: **Your Java implementation goes here**.
  - `simulation/slug/`: The sequential reference implementation.
  - `validator/`: The validator interface.
  - `Simulation.java`: Implements the `main` method.
- `src/test/java/cp26/`: Public tests for the project.
- `scenarios/`: Some sample scenarios.

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
