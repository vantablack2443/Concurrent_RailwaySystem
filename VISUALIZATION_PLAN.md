# Visualization Plan — Concurrent Railway Simulator

Goal: a static GitHub Pages site that explains the project visually — trains moving
on a map **and** the Rocket concurrency model (one thread per zone, channel
handoffs, promises, max-tick extension) underneath it.

## Decisions (from design interview)

| Topic | Decision |
|---|---|
| Story | Layered: railway map on top, concurrency (zone threads, messages) overlaid + swimlanes |
| Data | Precomputed JSON replayed by a static page — no WASM, no live sim |
| Maps | New hand-made "island" scenario (5 zones, ~16 stations) + existing `europe_500` |
| Stack | Vanilla JS, SVG (map) + Canvas (trains/swimlanes), no build step |
| Instrumentation | `viz` cargo feature in `train-sim-rocket`, off by default — graded path unchanged |
| Clock view | Map in sim-ticks; per-zone local-tick badges + wall-clock swimlanes per thread |
| Narrative | Guided tour (6 steps) + free play controls |
| Snapshot | Recorded run committed in `site/data/`; CI tests + deploys, does not re-record |
| Zones | Voronoi cells per station, merged by zone (d3-delaunay from CDN, pinned) |
| Extras | Slug vs Rocket panel: identical trace check + runtime/speedup on `europe` |
| Publishing | Public GitHub Pages (course over) |

## Why instrumentation is needed

`Output.trace` only has `(tick, train name, origin, destination)`:

- Train names are service names — `T0` departs 3× with same name, so trains can't be told apart.
- Trace tick = arrival at station. Real departure is `max(now, occupied_until)`, so waiting on a busy track is invisible.
- No `TrainHandoff` / `Promise` / `MaxTick` / blocking info at all — the concurrency is invisible.

## Phase 1 — Rust instrumentation (`viz` feature)

Files: `crates/train-sim-rocket/{Cargo.toml,src/lib.rs,src/worker.rs,src/viz.rs}`, `crates/train-sim/{Cargo.toml,src/main.rs}`.

1. `train-sim-rocket/Cargo.toml`: `[features] viz = ["dep:serde", "dep:serde_json"]` (serde already used by core).
2. New `src/viz.rs` (`#[cfg(feature = "viz")]`):
   ```rust
   pub struct VizEvent { zone: ZoneId, wall_ns: u64, local_tick: Tick, kind: VizKind }
   pub enum VizKind {
       Spawn { train: TrainId, station: StationId },
       Depart { train: TrainId, from: StationId, to: StationId, ready: Tick, depart: Tick, arrive: Tick },
       Despawn { train: TrainId, station: StationId },
       HandoffSend { train: TrainId, to_zone: ZoneId, arrive: Tick },
       HandoffRecv { train: TrainId, from_zone: ZoneId, arrive: Tick },
       PromiseSend { to_zone: ZoneId, until: Tick },
       PromiseRecv { from_zone: ZoneId, until: Tick },
       Blocked,            // entering blocking receiver.recv()
       Unblocked,
       ExtendMaxTick { wanted: Tick },
       MaxTickRecv { tick: Tick },
       Finished,
   }
   ```
   One shared `Instant` epoch in `SharedContext`; each worker owns a `Vec<VizEvent>` (no locks, no extra contention).
3. Hook points in `worker.rs` (current line numbers):
   - `run` L174: spawn of local trains; `Finished` at L218/L403.
   - blocking `self.receiver.recv()` at L272 → `Blocked` before, `Unblocked` after.
   - `handle_message` L298: `TrainHandoff` L305 → `HandoffRecv`; `Promise` L320 → `PromiseRecv`; `MaxTick` L328 → `MaxTickRecv`.
   - `send_promises` L337/L350 → `PromiseSend`.
   - `process_arrival` L363: departure decision → `Depart` with `ready=now`, `depart=max(now, occupied_until)`, `arrive`; `ExtendMaxTick` L441; handoff send L461.
   - Every hook wrapped in a small `#[cfg(feature = "viz")]` macro/inline fn so non-viz build is byte-identical in behavior.
4. `WorkerResult` gets `#[cfg(feature = "viz")] viz: Vec<VizEvent>`.
5. `launch()` signature stays untouched. Add `#[cfg(feature = "viz")] pub fn launch_with_viz(scenario, validator) -> (Output, VizLog)`; `launch` calls a shared inner fn.
6. CLI: `train-sim` gets optional `viz` feature forwarding to rocket, and `--viz-out <path>` flag.
7. Export format `*.viz.json` (written by CLI, compact arrays):
   ```json
   {
     "scenario": { "name", "gridSize", "zones": [{ "name", "stations": [id] }],
                   "stations": [{ "id", "name", "x", "y", "zone" }],
                   "tracks": [{ "from", "to", "distance" }] },
     "trains":  [{ "id", "service", "origin", "destination", "start" }],
     "moves":   [[train, from, to, ready, depart, arrive], ...],
     "zoneEvents": [[zone, wall_ns, local_tick, kind, a, b, c], ...],
     "meta": { "threads": 5, "cpu": "...", "wall_ms": 12 }
   }
   ```
   Promise events can be very frequent → exporter keeps only those where `until` changed and drops ones within <1 µs of the previous from same zone pair. Target file size: island <300 KB, europe_500 <3 MB.
8. Check: `cargo test --release -p train-sim-tests -- --test-threads=1` passes with and without `--features viz`; `cargo clippy --all-features` clean.

## Phase 2 — Island scenario

File: `scenarios/viz/island.json` (same format as `scenarios/public/*.json`).

- Grid 100×60. Zones: **Northvale**, **Harbor**, **Central**, **Highlands**, **Eastmarch**.
- ~16 stations, 3–4 per zone; Central is a hub.
- Deliberate single-track bottlenecks between zones (e.g. Harbor↔Central, Highlands↔Eastmarch) so queuing (`depart > ready`) and handoffs are visible.
- Mix of short intra-zone and long cross-island services; ~25 trains, departures spread over ticks 0–60.
- `penalty` 1.3 so a cheaper international detour exists — shows routing reacting to occupancy.
- `trace: true`, `statQueries` = all 5 zones.
- Validate: run Slug and Rocket on it, traces equal.

## Phase 3 — Static site

```
site/
  index.html
  style.css
  js/
    main.js        // load data, wire controls, scenario switcher
    clock.js       // sim-time playback: play/pause, speed 0.5–8×, scrub
    map.js         // SVG: Voronoi zones, tracks, stations, labels
    trains.js      // Canvas: train dots interpolated between depart→arrive; waiting = pulsing at station
    zones.js       // per-zone badge: local tick, state (running / blocked / finished), train count
    swimlanes.js   // Canvas: one lane per zone thread on wall-clock axis; blocked spans grey;
                   //   handoff arrows between lanes; promise ticks as small marks
    tour.js        // guided steps, highlights, jumps playback to set moments
    compare.js     // Slug vs Rocket panel
  data/
    island.viz.json
    europe_500.viz.json
    bench.json
```

Layout (desktop): map ~65% left, side panel right (zone badges, message log, legend); swimlanes full width under map; controls bar. Mobile: stacked, swimlanes collapsible.

Map details:
- Voronoi over stations clipped to grid bounds, cells unioned per zone, one color per zone (same color as its swimlane → "zone = thread").
- Tracks as lines; directed tracks shown with small arrowheads; track turns highlighted while occupied.
- Train dot color = service; on zone-border crossing, flash an arrow + entry in message log (`Harbor → Central: TrainHandoff T4#2 @ tick 31`).

Linking the two clocks:
- Map runs in sim ticks. Swimlanes run in wall-clock. Hovering/clicking a swimlane event jumps the map to that event's sim tick, and vice versa a map handoff highlights its swimlane arrow.
- Zone badges show each zone's *local tick at the current wall-clock instant* — the key visual: zones are at different ticks at the same time, and a zone stalls (grey, "waiting for promise from Central") until a neighbor's promise arrives.

Guided tour (6 steps):
1. The map: zones, stations, tracks, services.
2. Routing: Dijkstra with occupancy costs + international penalty (show detour taken).
3. Track occupancy: train waits because track busy (`depart > ready`).
4. Zones are threads: each zone owns its stations, runs its own event queue.
5. Handoffs and promises: crossing a border = `TrainHandoff`; `Promise` lets neighbors advance safely; blocking when no promise.
6. Max tick + shutdown: `ExtendMaxTick` → main broadcasts `MaxTick`; all `Finished` → `Shutdown`, join, merge traces.

Compare panel (`bench.json`):
- Trace equality Slug vs Rocket on island + europe_500 (✓ / ✗, computed when recording).
- Runtime of Slug vs Rocket on `europe` (10 000 trains), median of 5 runs, with core count of the machine.

Dependencies: `d3-delaunay` only, pinned exact version from cdnjs/jsdelivr. Everything else hand-written.

## Phase 4 — Recording + CI/Pages

`justfile` additions:
```
viz-record:   # build with viz feature, record island + europe_500 into site/data
viz-bench:    # run slug vs rocket on europe 5x, write site/data/bench.json
viz-serve:    # python3 -m http.server -d site 8000
```

`.github/workflows/pages.yml`:
- on push to `main` + manual dispatch.
- job `test`: install Rust 1.96.1, `cargo test --release -p train-sim-tests -- --test-threads=1`, `cargo build --release --features viz` (ensures instrumentation compiles).
- job `deploy` (needs test): `actions/configure-pages`, `actions/upload-pages-artifact` with `path: site`, `actions/deploy-pages`.
- Repo setting: Settings → Pages → Source = GitHub Actions.
- README: badge + link to live page + 1 GIF of the island run.

## Order of work + done criteria

1. Phase 1 instrumentation → viz build passes all tests, island-free smoke run on `simple.json` produces valid `viz.json`.
2. Phase 2 island scenario → Slug/Rocket traces match, file recorded.
3. Phase 3a map + trains + playback (no concurrency yet) → island animates correctly vs trace.
4. Phase 3b zone badges + swimlanes + message log → clocks linked both ways.
5. Phase 3c tour + compare panel + europe_500 switcher.
6. Phase 4 CI + Pages live, README updated.

## Risks

- **Logging perturbs timing**: events are pushed to thread-local Vecs, no locks; acceptable, and page labels the run "instrumented run".
- **Few-core recording machine** hides parallelism: record on ≥4 cores; `meta` shows core count.
- **europe_500 density** (25 zones, 112 stations, 4 302 moves): Canvas for trains/lanes, SVG only for static map; lanes default-filter to selected zones.
- **Promise flood** in log: compaction in exporter (Phase 1 step 7).
