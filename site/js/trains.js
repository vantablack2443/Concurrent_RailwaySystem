// Dynamic part of the map, drawn on a canvas every frame: occupied track
// lanes, moving trains, trains queued at a station for a free track, and
// trains fading out at their destination.

import { trainState } from "./data.js";

/** Longer queues at one station are drawn as a single counted badge. */
const MAX_RING = 8;

function cssVar(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

export class TrainLayer {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d");
    this.readTheme();
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => this.readTheme());
  }

  readTheme() {
    this.colors = { station: cssVar("--station"), wait: cssVar("--wait"), ink: cssVar("--ink") };
  }

  resize(proj) {
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.round(proj.width * dpr);
    this.canvas.height = Math.round(proj.height * dpr);
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  /** Point on the right-hand lane of the track `from → to`, `p` of the way. */
  lanePoint(proj, from, to, p) {
    const [x1, y1] = proj.points[from];
    const [x2, y2] = proj.points[to];
    const dx = x2 - x1;
    const dy = y2 - y1;
    const len = Math.hypot(dx, dy) || 1;
    const off = 2.8 * proj.unit;
    return [x1 + dx * p - (dy / len) * off, y1 + dy * p + (dx / len) * off];
  }

  /**
   * Draws all trains at tick `t` and returns their screen positions for hit
   * testing. `now` (ms) drives pulse animations. The route of `pinnedId` is
   * drawn as a dashed line.
   */
  draw(model, proj, t, now, hoverId, pinnedId = -1) {
    const { ctx, colors } = this;
    const u = proj.unit;
    ctx.clearRect(0, 0, proj.width, proj.height);
    if (pinnedId >= 0) this.drawRoute(model.trains[pinnedId], proj, t);

    const moving = [];
    const waitingAt = new Map();
    const arrived = [];
    for (const train of model.trains) {
      const state = trainState(train, t);
      if (!state) continue;
      if (state.kind === "moving") moving.push({ train, state });
      else if (state.kind === "waiting") {
        const list = waitingAt.get(state.station) ?? [];
        list.push({ train, state });
        waitingAt.set(state.station, list);
      } else arrived.push({ train, state });
    }

    // Occupied lanes underneath the trains.
    ctx.lineCap = "round";
    ctx.lineWidth = 3 * u;
    for (const { train, state } of moving) {
      const a = this.lanePoint(proj, state.move.from, state.move.to, 0);
      const b = this.lanePoint(proj, state.move.from, state.move.to, 1);
      ctx.strokeStyle = `hsl(${train.hue} 70% 50% / 0.45)`;
      ctx.beginPath();
      ctx.moveTo(a[0], a[1]);
      ctx.lineTo(b[0], b[1]);
      ctx.stroke();
    }

    const hits = [];
    const dot = (x, y, train, r) => {
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fillStyle = `hsl(${train.hue} 70% 46%)`;
      ctx.fill();
      const marked = train.id === hoverId || train.id === pinnedId;
      ctx.lineWidth = (marked ? 2.4 : 1.6) * u;
      ctx.strokeStyle = marked ? colors.ink : colors.station;
      ctx.stroke();
    };

    // Trains waiting for a track: a ring around the station, pulsing.
    const pulse = 0.5 + 0.5 * Math.sin(now / 260);
    for (const [station, list] of waitingAt) {
      const [sx, sy] = proj.points[station];
      if (list.length > MAX_RING) {
        this.queueBadge(sx, sy, list.length, u, pulse);
        hits.push({ x: sx, y: sy, queue: list, station });
        continue;
      }
      const slots = Math.max(6, list.length);
      const radius = 9 * u;
      list.forEach(({ train, state }, i) => {
        const angle = -Math.PI / 2 + (i * 2 * Math.PI) / slots;
        const x = sx + Math.cos(angle) * radius;
        const y = sy + Math.sin(angle) * radius;
        ctx.beginPath();
        ctx.arc(x, y, (5.5 + pulse * 2) * u, 0, Math.PI * 2);
        ctx.strokeStyle = colors.wait;
        ctx.globalAlpha = 0.35 + 0.5 * (1 - pulse);
        ctx.lineWidth = 1.5 * u;
        ctx.stroke();
        ctx.globalAlpha = 1;
        dot(x, y, train, 4 * u);
        hits.push({ x, y, train, state });
      });
    }

    for (const { train, state } of moving) {
      const [x, y] = this.lanePoint(proj, state.move.from, state.move.to, state.p);
      dot(x, y, train, 4.4 * u);
      hits.push({ x, y, train, state });
    }

    // Arrivals: a ring expanding and fading at the destination.
    for (const { train, state } of arrived) {
      const [x, y] = proj.points[state.station];
      ctx.beginPath();
      ctx.arc(x, y, (5 + state.fade * 14) * u, 0, Math.PI * 2);
      ctx.strokeStyle = `hsl(${train.hue} 70% 46% / ${1 - state.fade})`;
      ctx.lineWidth = 2 * u;
      ctx.stroke();
    }

    return hits;
  }

  /** Dashed route of a train: done part solid-ish, rest dashed. */
  drawRoute(train, proj, t) {
    const { ctx } = this;
    const u = proj.unit;
    ctx.save();
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.lineWidth = 3.2 * u;
    for (const move of train.moves) {
      const [x1, y1] = proj.points[move.from];
      const [x2, y2] = proj.points[move.to];
      const done = t >= move.arrive;
      ctx.setLineDash(done ? [] : [6 * u, 5 * u]);
      ctx.strokeStyle = `hsl(${train.hue} 75% 55% / ${done ? 0.35 : 0.9})`;
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
    }
    ctx.setLineDash([]);
    const last = train.moves[train.moves.length - 1];
    if (last) {
      const [dx, dy] = proj.points[last.to];
      ctx.beginPath();
      ctx.arc(dx, dy, 8 * u, 0, Math.PI * 2);
      ctx.strokeStyle = `hsl(${train.hue} 75% 55%)`;
      ctx.lineWidth = 2 * u;
      ctx.stroke();
    }
    ctx.restore();
  }

  queueBadge(x, y, count, u, pulse) {
    const { ctx, colors } = this;
    const r = (9 + Math.min(count, 60) * 0.12) * u;
    ctx.beginPath();
    ctx.arc(x, y, r + pulse * 2.5 * u, 0, Math.PI * 2);
    ctx.strokeStyle = colors.wait;
    ctx.globalAlpha = 0.3 + 0.4 * (1 - pulse);
    ctx.lineWidth = 2 * u;
    ctx.stroke();
    ctx.globalAlpha = 1;
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fillStyle = colors.wait;
    ctx.fill();
    ctx.fillStyle = "#1f2328";
    ctx.font = `600 ${Math.round(9.5 * u + 2)}px "IBM Plex Mono", monospace`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(String(count), x, y + 0.5);
  }
}
