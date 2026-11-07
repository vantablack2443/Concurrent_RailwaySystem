set positional-arguments := true

# Print available recipes
help:
    @just --list

# Print available recipes
list:
    @just --list

lang := `python3 -c 'import tomllib;print(tomllib.loads(open("project.toml","r",encoding="utf-8-sig").read())["language"])' || echo java`
export CP_LANGUAGE := lang
build_command := if lang != "rust" { "./gradlew jar" } else { "cargo build" }

# Build the project
build:
    {{ build_command }}

build_jar := if lang != "rust" { "./gradlew jar" } else { "" }
run_command := if lang == "rust" { "cargo run --release -p train-sim --" } else { "java -ea -jar out/simulator.jar" }

# Build and run the project.
run *FLAGS:
    {{ build_jar }}
    {{ run_command }} "$@"

test_command := if lang == "rust" { "cargo test -p train-sim-tests --tests -- --show-output --test-threads=1" } else { "./gradlew test" }

# Run the tests
test:
    {{ test_command }}

no_lint_command := if lang != "rust" { "echo 'No lint command defined for Java'" } else { "" }

# Run clippy on Rust code
lint:
    @{{ no_lint_command }}
    cargo clippy

doc_command := if lang == "rust" { "cargo doc -p train-sim-rocket --document-private-items" } else { "./gradlew javadoc" }
doc_path := if lang == "rust" { "target" / "doc" / "train_sim_rocket" / "index.html" } else { "build" / "docs" / "javadoc" / "index.html" }

# Generate API documentation
doc:
    {{ doc_command }}

open_command := if os() == "macos" { "open" } else { if os() == "windows" { "start" } else { "xdg-open" } }

# Generate API documentation and open it in your web browser
doc-open: doc
    {{ open_command }} {{ doc_path }}

# Get the detected programming language
lang:
    @echo {{ lang }}

# Record visualization data for the GitHub Pages site into site/data
viz-record:
    cargo build --release -p train-sim --features viz
    mkdir -p site/data target/viz
    ./target/release/train-sim --scenario scenarios/viz/island.json --out target/viz/island.out.json --viz-out site/data/island.viz.json
    ./target/release/train-sim --scenario scenarios/public/europe_500.json --out target/viz/europe_500.out.json --viz-out site/data/europe_500.viz.json

# Benchmark Slug vs Rocket for the compare panel (writes site/data/bench.json)
viz-bench:
    cargo build --release -p train-sim
    python3 scripts/viz_bench.py
