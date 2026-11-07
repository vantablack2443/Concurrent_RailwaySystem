# Concurrency Bahn 🚆🦀

**Zone-parallel railway simulation in Rust: one thread per zone, no locks, output identical to the sequential reference.**

### ▶ [Live visualization](https://vantablack2443.github.io/Concurrent_RailwaySystem/)

Built for the Concurrent Programming course (Saarland University, SS 2026). The course provides
the simulation model, a sequential reference implementation (Slug) and a test harness; the task
is to parallelise it. **Our work is the [`train-sim-rocket`](crates/train-sim-rocket) crate.**
Everything else is course material, kept so the project builds as a whole.

## The simulation

- **Stations** are nodes and **tracks** are directed edges with a travel time in ticks. A track
  holds **one train at a time**; later trains queue at the station.
- **Trains** route themselves: at every station they rerun Dijkstra over live costs (distance plus
  how long the track is still occupied), then despawn at their destination.
- **Zones** partition the stations, like countries. A train only sees traffic in its *current*
  zone; tracks elsewhere cost `distance × international_penalty`.

## How Rocket parallelises it

| Idea | How |
|---|---|
| **Ownership, not locks** | Each `ZoneWorker` is moved into its own thread and owns its event queue, track occupancy and routing graph. Shared scenario data is immutable behind `Arc`. Coordination is `std::sync::mpsc` only. |
| **Trains are messages** | Travel times are fixed, so the sending zone computes the arrival tick and sends `Message::TrainHandoff`; the receiver queues it directly. |
| **Promises** | Each zone tells its neighbours "nothing arrives from me before tick X". A zone only processes tick *t* once every incoming neighbour has promised past it. This conservative synchronisation makes the output **bit-identical to Slug**, whatever the scheduling. |
| **Global max tick** | A coordinator bounds how far zones may run and raises it only when a train is actually scheduled, so idle neighbours don't ratchet their clocks forever. |
| **Termination** | Each zone knows how many trains end in it, counts down, and reports `Finished` once. The coordinator then broadcasts `Shutdown` to every zone directly, so zones unreachable along one-way tracks still stop. |

Two details mattered: a drained zone keeps advancing its clock so its promises keep rising, and a
zone that can never receive anything again promises `Tick::MAX`.

## Results

| Scenario | Slug | Rocket | |
|---|---|---|---|
| Europe, 25 zones, 10 000 trains | 246 ms | 180 ms | **1.37× faster**, same stats |
| Europe, 25 zones, 500 trains | 7 ms | 9 ms | same trace and stats |

Median of 5 runs on 16 CPUs, simulation time only (`just viz-bench`). On small runs, starting
threads costs more than it saves.

Beyond the provided tests, we ran **differential tests against Slug** on modified penalties,
hand-built topologies (zones with no incoming tracks, isolated zones, pass-through zones, no
trains) and random networks. They found two deadlocks the provided scenarios don't cover.

## Visualization

The [live page](https://vantablack2443.github.io/Concurrent_RailwaySystem/) replays a recorded
run: trains moving and queuing on a zone map, each zone thread's own clock, a wall-clock timeline
of the threads with handoffs and promises, the message log, a six-step guided tour, and the Slug
vs Rocket comparison.

> **Made with Claude.** The visualization was built with Claude (Anthropic's AI assistant): the page
> in [`site/`](site), the off-by-default `viz` cargo feature that records worker events, the island
> scenario, the benchmark script and the Pages workflow. The simulation itself is our own work.

```bash
just viz-record   # record the island and europe_500 runs into site/data
just viz-bench    # time Slug vs Rocket and check outputs match
just viz-serve    # serve the page at http://localhost:8000
```

## Running

```bash
cargo build --release
cargo run --release -p train-sim -- --scenario scenarios/public/europe.json --out out.json
cargo run --release -p train-sim -- --slug --scenario scenarios/public/europe.json --out slug.json
cargo test --release -p train-sim-tests -- --test-threads=1
```

`--slug` runs the sequential reference, useful for diffing. `--test-threads=1` keeps the tests
from competing with the simulation for cores.

## Known limitations

- **Message volume.** Promises are O(arrivals × neighbours); most overhead is channel traffic,
  so the speedup is modest.
- **Time creep.** Across long gaps with no events, zones advance by the shortest border track per
  round of messages, so long timelines with short border tracks are slow.
- **Panics hang.** A panicking worker leaves the coordinator waiting, since the other workers
  keep the report channel open.

## Layout

```
crates/
  train-sim-rocket/   our concurrent implementation
    lib.rs              coordinator: zones, channels, max tick, termination
    worker.rs           per-zone loop: promises, routing, statistics
    network.rs          station/zone indices, neighbours, penalised graph
    event.rs            TrainArrival and its ordering
    viz.rs              optional event recording (made with Claude)
  train-sim/          CLI                                   (course)
  train-sim-core/     scenario/output types, Dijkstra       (course)
  train-sim-slug/     sequential reference                  (course)
  train-sim-tests/    public tests and harness              (course)
scenarios/            sample scenarios; viz/ has the island (course / Claude)
site/                 visualization page                    (Claude)
src/                  Java template                         (course, unused)
```
