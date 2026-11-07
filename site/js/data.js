// Loads a `*.viz.json` file (written by `train-sim --viz-out`) and turns it
// into the model the map and trains render from.

/** Ticks a train stays visible at its destination after arriving. */
export const ARRIVAL_FADE = 2;

export async function loadScenario(name) {
  const res = await fetch(`data/${name}.viz.json`);
  if (!res.ok) throw new Error(`Could not load data/${name}.viz.json (${res.status})`);
  return buildModel(await res.json());
}

function hue(i, offset) {
  return (i * 137.508 + offset) % 360;
}

export function buildModel(raw) {
  const { scenario, meta } = raw;

  const zones = scenario.zones.map((name, id) => ({
    id,
    name,
    hue: hue(id, 18),
    stations: [],
  }));

  const stations = scenario.stations.map((s, id) => ({ ...s, id, degree: 0 }));
  for (const s of stations) zones[s.zone].stations.push(s.id);

  const tracks = scenario.tracks.map(([from, to, distance]) => ({ from, to, distance }));
  for (const t of tracks) {
    stations[t.from].degree++;
  }

  // Some scenarios place a station outside the declared grid (europe has
  // "Dan Haag" at y = 709 of 270). Draw such stations between their
  // neighbours instead, so they don't squash the rest of the map.
  const { x: gw, y: gh } = scenario.gridSize;
  const outside = (s) => s.x < 0 || s.y < 0 || s.x > gw || s.y > gh;
  for (const s of stations) {
    if (!outside(s)) continue;
    const neighbours = tracks
      .filter((t) => t.from === s.id && !outside(stations[t.to]))
      .map((t) => stations[t.to]);
    if (neighbours.length === 0) continue;
    s.scenarioPos = { x: s.x, y: s.y };
    s.x = neighbours.reduce((sum, n) => sum + n.x, 0) / neighbours.length;
    s.y = neighbours.reduce((sum, n) => sum + n.y, 0) / neighbours.length;
  }

  // Services get one color each; trains of a service are numbered by departure.
  const serviceIndex = new Map();
  const departureCount = new Map();
  const trains = raw.trains.map((t, id) => {
    if (!serviceIndex.has(t.service)) serviceIndex.set(t.service, serviceIndex.size);
    const n = (departureCount.get(t.service) ?? 0) + 1;
    departureCount.set(t.service, n);
    return {
      ...t,
      id,
      label: `${t.service} #${n}`,
      hue: hue(serviceIndex.get(t.service), 200),
      moves: [],
    };
  });

  let endTick = 0;
  for (const [train, from, to, ready, depart, arrive] of raw.moves) {
    trains[train].moves.push({ from, to, ready, depart, arrive });
    endTick = Math.max(endTick, arrive);
  }
  for (const t of trains) t.moves.sort((a, b) => a.ready - b.ready);

  return {
    name: scenario.name,
    grid: scenario.gridSize,
    penalty: scenario.penalty,
    zones,
    stations,
    tracks,
    trains,
    services: serviceIndex.size,
    events: raw.events,
    meta,
    endTick,
  };
}

/**
 * Where `train` is at (fractional) tick `t`:
 * - `null` before it starts or after it has gone,
 * - `{ kind: "waiting", station, move }` while its next track is occupied,
 * - `{ kind: "moving", move, p }` with `p` in [0, 1) along the track,
 * - `{ kind: "arrived", station, fade }` just after reaching its destination.
 */
export function trainState(train, t) {
  const moves = train.moves;
  if (moves.length === 0 || t < moves[0].ready) return null;

  // Last move that was ready at or before `t`.
  let lo = 0;
  let hi = moves.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (moves[mid].ready <= t) lo = mid;
    else hi = mid - 1;
  }
  const move = moves[lo];

  if (t < move.depart) return { kind: "waiting", station: move.from, move };
  if (t < move.arrive) {
    return { kind: "moving", move, p: (t - move.depart) / (move.arrive - move.depart) };
  }
  if (lo === moves.length - 1 && t < move.arrive + ARRIVAL_FADE) {
    return { kind: "arrived", station: move.to, fade: (t - move.arrive) / ARRIVAL_FADE };
  }
  return null;
}
