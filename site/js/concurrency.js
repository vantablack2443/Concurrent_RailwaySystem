// What the threads did, rebuilt from the recorded events: one lane per zone
// worker (plus the main thread), the spans each worker spent blocked on its
// channel, train handoffs between workers, promises, and a mapping between
// simulation ticks and wall-clock time.
//
// Event rows are `[thread, wallNs, localTick, kind, a, b, c]`, see
// `crates/train-sim-rocket/src/viz.rs`.

/** Index of the last element of sorted `arr` that is <= `x`, or -1. */
export function lastAtOrBefore(arr, x) {
  let lo = 0;
  let hi = arr.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (arr[mid] <= x) {
      found = mid;
      lo = mid + 1;
    } else hi = mid - 1;
  }
  return found;
}

function interpolate(xs, ys, x, start, end) {
  const i = lastAtOrBefore(xs, x);
  const [x0, y0] = i < 0 ? start : [xs[i], ys[i]];
  const [x1, y1] = i + 1 < xs.length ? [xs[i + 1], ys[i + 1]] : end;
  if (x1 <= x0) return y1;
  return y0 + ((y1 - y0) * Math.min(Math.max(x - x0, 0), x1 - x0)) / (x1 - x0);
}

export function buildConcurrency(model) {
  const zoneCount = model.zones.length;
  const MAIN = zoneCount;
  const laneOf = (thread) => (thread === -1 ? MAIN : thread);

  const lanes = [
    ...model.zones.map((z) => ({ id: z.id, name: z.name, hue: z.hue })),
    { id: -1, name: "main", hue: null },
  ].map((lane) => ({
    ...lane,
    start: null,
    end: null,
    walls: [], // wall time of every event with a local tick ...
    ticks: [], // ... and that tick
    blocked: [], // [from, to] wall spans blocked on the channel
    blockedStarts: [],
    promiseSends: [],
    finished: null,
    blockStart: null,
  }));

  // Incoming neighbour zones, and the promises received from each.
  const incoming = model.zones.map(() => new Set());
  for (const t of model.tracks) {
    const a = model.stations[t.from].zone;
    const b = model.stations[t.to].zone;
    if (a !== b) incoming[b].add(a);
  }
  const promisesIn = model.zones.map((_, z) =>
    new Map([...incoming[z]].map((from) => [from, { walls: [], until: [] }])),
  );

  const handoffs = [];
  const pending = new Map();
  const progress = [];
  const broadcasts = [];
  const messages = []; // events worth listing in the message log

  for (const ev of model.events) {
    const [thread, wall, tick, kind, a, b, c] = ev;
    const lane = lanes[laneOf(thread)];
    if (lane.start === null) lane.start = wall;
    lane.end = wall;
    if (tick !== null) {
      lane.walls.push(wall);
      lane.ticks.push(tick);
    }
    switch (kind) {
      case "block":
        lane.blockStart = wall;
        break;
      case "unblock":
        if (lane.blockStart !== null) lane.blocked.push([lane.blockStart, wall]);
        lane.blockStart = null;
        break;
      case "psend":
        lane.promiseSends.push(wall);
        messages.push(ev);
        break;
      case "precv": {
        const p = promisesIn[thread].get(a);
        if (p) {
          p.walls.push(wall);
          p.until.push(b);
        }
        break;
      }
      case "hsend":
        pending.set(`${a}:${c}`, { train: a, from: thread, to: b, arrive: c, sendWall: wall, sendTick: tick });
        messages.push(ev);
        break;
      case "hrecv": {
        const h = pending.get(`${a}:${c}`);
        if (h) {
          h.recvWall = wall;
          handoffs.push(h);
          pending.delete(`${a}:${c}`);
        }
        break;
      }
      case "depart":
      case "despawn":
        progress.push([tick, wall]);
        break;
      case "finished":
        lane.finished = wall;
        messages.push(ev);
        break;
      case "extend":
        messages.push(ev);
        break;
      case "mtbcast":
        broadcasts.push({ wall, tick: a });
        messages.push(ev);
        break;
      case "sdbcast":
        broadcasts.push({ wall, tick: null, shutdown: true });
        messages.push(ev);
        break;
    }
  }
  for (const lane of lanes) {
    lane.blockedStarts = lane.blocked.map(([s]) => s);
    delete lane.blockStart;
  }

  const startWall = Math.min(...lanes.filter((l) => l.start !== null).map((l) => l.start));

  // Sim tick -> wall time: the moment every train event up to that tick had
  // been processed (prefix maximum, so it only moves forward).
  progress.sort((p, q) => p[0] - q[0] || p[1] - q[1]);
  const progTicks = [];
  const progWalls = [];
  let latest = 0;
  for (const [tick, wall] of progress) {
    latest = Math.max(latest, wall);
    if (progTicks.length && progTicks[progTicks.length - 1] === tick) {
      progWalls[progWalls.length - 1] = latest;
    } else {
      progTicks.push(tick);
      progWalls.push(latest);
    }
  }
  const lastLaneEnd = Math.max(...lanes.map((l) => l.end ?? 0));
  const tickEnd = [model.endTick + 2, lastLaneEnd];
  const wallEnd = [lastLaneEnd, model.endTick + 2];

  const messageWalls = messages.map((ev) => ev[1]);

  return {
    lanes,
    handoffs,
    broadcasts,
    incoming,
    messages,
    startWall,
    // Small margin after the last event, so final markers stay visible.
    endWall: lastLaneEnd + (lastLaneEnd - startWall) * 0.01,

    wallAtTick(t) {
      return interpolate(progTicks, progWalls, t, [0, startWall], tickEnd);
    },

    tickAtWall(w) {
      return interpolate(progWalls, progTicks, w, [startWall, 0], wallEnd);
    },

    /** Local tick and state of zone (lane) `z` at wall time `w`. */
    laneAt(z, w) {
      const lane = lanes[z];
      const i = lastAtOrBefore(lane.walls, w);
      const tick = i < 0 ? 0 : lane.ticks[i];
      let status = "running";
      if (lane.start === null || w < lane.start) status = "starting";
      else if (w >= lane.end) status = "stopped";
      else {
        const b = lastAtOrBefore(lane.blockedStarts, w);
        if (b >= 0 && w < lane.blocked[b][1]) status = "blocked";
      }
      return { tick, status, finished: lane.finished !== null && w >= lane.finished };
    },

    /** The incoming neighbour with the lowest promise to zone `z` at `w`. */
    lowestPromise(z, w) {
      let best = null;
      for (const [from, p] of promisesIn[z]) {
        const i = lastAtOrBefore(p.walls, w);
        const until = i < 0 ? -1 : p.until[i];
        // `null` = promised "never again", which never limits the zone.
        if (until === null) continue;
        if (!best || until < best.until) best = { from, until, none: until < 0 };
      }
      return best;
    },

    /** Up to `n` most recent log messages at or before wall time `w`. */
    recentMessages(w, n, withPromises) {
      const out = [];
      for (let i = lastAtOrBefore(messageWalls, w); i >= 0 && out.length < n; i--) {
        if (!withPromises && messages[i][3] === "psend") continue;
        out.push(messages[i]);
      }
      return out;
    },
  };
}
