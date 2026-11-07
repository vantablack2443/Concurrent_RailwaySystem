// Thread swimlanes on a wall-clock axis: one lane per zone worker plus the
// main thread. Solid = working, faint = blocked on its channel; arrows are
// train handoffs between workers, ticks along the top of a lane are promises
// sent, diamonds on the main lane are max-tick broadcasts.
//
// The static picture is drawn once into an offscreen canvas; every frame only
// blits it and adds the cursor and highlights.

import { escapeHtml } from "./data.js";

const LABEL_W = 96;
const AXIS_H = 20;
const RIGHT_PAD = 12;

export function formatWall(ns) {
  if (ns < 1e6) return `${Math.round(ns / 1e3)} µs`;
  return `${(ns / 1e6).toFixed(ns < 1e7 ? 2 : 1)} ms`;
}

function cssVar(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

export class Swimlanes {
  constructor(box, canvas, tooltip) {
    this.box = box;
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d");
    this.tooltip = tooltip;
    this.offscreen = document.createElement("canvas");
    this.hover = null;
    this.hoverTrain = -1;
    this.onSeek = () => {};
    canvas.addEventListener("pointermove", (e) => this.pointer(e));
    canvas.addEventListener("pointerleave", () => {
      this.hover = null;
      this.hoverTrain = -1;
      tooltip.hidden = true;
    });
    canvas.addEventListener("click", (e) => {
      const w = this.wallAtX(e.offsetX);
      if (w !== null) this.onSeek(this.conc.tickAtWall(w));
    });
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => this.layout());
    new ResizeObserver(() => this.layout()).observe(box);
  }

  setData(model, conc) {
    this.model = model;
    this.conc = conc;
    this.laneH = conc.lanes.length <= 10 ? 22 : 11;
    this.layout();
  }

  laneY(i) {
    return 4 + i * this.laneH;
  }

  x(w) {
    const { startWall, endWall } = this.conc;
    return LABEL_W + ((w - startWall) / (endWall - startWall)) * (this.width - LABEL_W - RIGHT_PAD);
  }

  wallAtX(x) {
    if (x < LABEL_W) return null;
    const { startWall, endWall } = this.conc;
    const f = (x - LABEL_W) / (this.width - LABEL_W - RIGHT_PAD);
    return startWall + Math.min(Math.max(f, 0), 1) * (endWall - startWall);
  }

  layout() {
    if (!this.conc) return;
    const height = this.laneY(this.conc.lanes.length) + AXIS_H + 4;
    this.box.style.height = `${height}px`;
    this.width = this.box.clientWidth;
    this.height = height;
    const dpr = window.devicePixelRatio || 1;
    for (const c of [this.canvas, this.offscreen]) {
      c.width = Math.round(this.width * dpr);
      c.height = Math.round(height * dpr);
    }
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    this.colors = {
      ink: cssVar("--ink"),
      soft: cssVar("--ink-soft"),
      line: cssVar("--line"),
      surface: cssVar("--surface"),
      accent: cssVar("--accent"),
      wait: cssVar("--wait"),
    };
    this.drawStatic(dpr);
  }

  laneColor(lane, alpha) {
    return lane.hue === null ? `hsl(220 8% 55% / ${alpha})` : `hsl(${lane.hue.toFixed(1)} 52% 50% / ${alpha})`;
  }

  drawStatic(dpr) {
    const ctx = this.offscreen.getContext("2d");
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, this.width, this.height);
    const { lanes, handoffs, broadcasts } = this.conc;
    const h = this.laneH - 3;
    const small = this.laneH < 16;

    ctx.font = `${small ? 10 : 12}px "IBM Plex Sans", sans-serif`;
    ctx.textBaseline = "middle";

    lanes.forEach((lane, i) => {
      const y = this.laneY(i);
      ctx.fillStyle = this.colors.soft;
      ctx.textAlign = "right";
      ctx.fillText(lane.name, LABEL_W - 8, y + h / 2);
      if (lane.start === null) return;

      // Faint for the whole lifetime, solid where the thread was not blocked.
      ctx.fillStyle = this.laneColor(lane, 0.16);
      ctx.fillRect(this.x(lane.start), y, Math.max(this.x(lane.end) - this.x(lane.start), 1), h);
      ctx.fillStyle = this.laneColor(lane, 0.85);
      let from = lane.start;
      for (const [s, e] of [...lane.blocked, [lane.end, lane.end]]) {
        if (s > from) ctx.fillRect(this.x(from), y, Math.max(this.x(s) - this.x(from), 0.8), h);
        from = Math.max(from, e);
      }

      // Promises sent: short ticks along the top edge.
      if (!small) {
        ctx.fillStyle = this.colors.ink;
        ctx.globalAlpha = 0.35;
        for (const w of lane.promiseSends) ctx.fillRect(this.x(w), y - 2, 1, 4);
        ctx.globalAlpha = 1;
      }

      // Finished: every train destined for this zone has arrived.
      if (lane.finished !== null) {
        const fx = this.x(lane.finished);
        ctx.fillStyle = this.colors.ink;
        ctx.beginPath();
        ctx.moveTo(fx, y + h);
        ctx.lineTo(fx, y - 1);
        ctx.lineTo(fx + 6, y + 2);
        ctx.lineTo(fx, y + 5);
        ctx.fill();
      }
    });

    // Max-tick broadcasts reach every lane.
    const mainY = this.laneY(lanes.length - 1);
    for (const b of broadcasts) {
      const bx = this.x(b.wall);
      ctx.strokeStyle = this.colors.soft;
      ctx.globalAlpha = 0.35;
      ctx.setLineDash([2, 3]);
      ctx.beginPath();
      ctx.moveTo(bx, 2);
      ctx.lineTo(bx, mainY);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.globalAlpha = 1;
      ctx.fillStyle = b.shutdown ? this.colors.accent : this.colors.ink;
      ctx.beginPath();
      ctx.moveTo(bx, mainY);
      ctx.lineTo(bx + 4, mainY + h / 2);
      ctx.lineTo(bx, mainY + h);
      ctx.lineTo(bx - 4, mainY + h / 2);
      ctx.fill();
    }

    // Handoffs: sender lane -> receiver lane.
    ctx.lineWidth = 1;
    const alpha = handoffs.length > 300 ? 0.07 : 0.3;
    for (const ho of handoffs) this.arrow(ctx, ho, alpha);

    // Wall-clock axis.
    const axisY = this.laneY(lanes.length) + 2;
    const { startWall, endWall } = this.conc;
    const span = endWall - startWall;
    const raw = span / 6;
    const pow = 10 ** Math.floor(Math.log10(raw));
    const step = [1, 2, 5, 10].map((m) => m * pow).find((s) => s >= raw);
    ctx.fillStyle = this.colors.soft;
    ctx.textAlign = "center";
    ctx.font = `11px "IBM Plex Mono", monospace`;
    for (let w = 0; w <= span; w += step) {
      const ax = this.x(startWall + w);
      ctx.fillRect(ax, axisY, 1, 4);
      ctx.fillText(formatWall(w), ax, axisY + 12);
    }
  }

  arrow(ctx, ho, alpha) {
    const train = this.model.trains[ho.train];
    const h = this.laneH - 3;
    const x1 = this.x(ho.sendWall);
    const x2 = this.x(ho.recvWall);
    const y1 = this.laneY(ho.from) + h / 2;
    const y2 = this.laneY(ho.to) + h / 2;
    ctx.strokeStyle = `hsl(${train.hue} 70% 50% / ${alpha})`;
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.bezierCurveTo(x1 + 12, y1, x2 - 12, y2, x2, y2);
    ctx.stroke();
  }

  /** Redraws with the cursor at wall time `w` and `train` highlighted. */
  draw(w, train) {
    if (!this.conc || !this.width) return;
    const { ctx } = this;
    ctx.clearRect(0, 0, this.width, this.height);
    ctx.drawImage(this.offscreen, 0, 0, this.width, this.height);

    // Shade what has not happened yet.
    const cx = this.x(w);
    const bottom = this.laneY(this.conc.lanes.length);
    ctx.fillStyle = this.colors.surface;
    ctx.globalAlpha = 0.6;
    ctx.fillRect(cx, 0, this.width - RIGHT_PAD - cx + RIGHT_PAD, bottom);
    ctx.globalAlpha = 1;

    const highlight = this.hoverTrain >= 0 ? this.hoverTrain : train;
    if (highlight >= 0) {
      ctx.lineWidth = 2.2;
      for (const ho of this.conc.handoffs) if (ho.train === highlight) this.arrow(ctx, ho, 1);
      ctx.lineWidth = 1;
    }

    ctx.fillStyle = this.colors.accent;
    ctx.fillRect(cx - 1, 0, 2, bottom);
  }

  pointer(e) {
    const { conc, model } = this;
    if (!conc) return;
    const w = this.wallAtX(e.offsetX);
    const lane = Math.floor((e.offsetY - 4) / this.laneH);
    if (w === null || lane < 0 || lane >= conc.lanes.length) {
      this.tooltip.hidden = true;
      this.hoverTrain = -1;
      return;
    }

    // Nearest handoff end within a few pixels.
    let best = null;
    let bestD = 36;
    for (const ho of conc.handoffs) {
      for (const [wall, z] of [[ho.sendWall, ho.from], [ho.recvWall, ho.to]]) {
        const d = (this.x(wall) - e.offsetX) ** 2 + (this.laneY(z) + this.laneH / 2 - e.offsetY) ** 2;
        if (d < bestD) {
          bestD = d;
          best = ho;
        }
      }
    }

    const zone = (z) => escapeHtml(model.zones[z].name);
    let html;
    if (best) {
      this.hoverTrain = best.train;
      html = `<strong>Handoff ${zone(best.from)} → ${zone(best.to)}</strong>
        ${escapeHtml(model.trains[best.train].label)} · arrives t ${best.arrive}
        <div class="mono">in the channel for ${formatWall(best.recvWall - best.sendWall)}</div>`;
    } else {
      this.hoverTrain = -1;
      const info = conc.lanes[lane];
      if (info.id === -1) {
        html = `<strong>main thread</strong>Waits for reports, broadcasts max tick and shutdown.`;
      } else {
        const s = conc.laneAt(lane, w);
        const lowest = conc.lowestPromise(lane, w);
        html = `<strong>${escapeHtml(info.name)} thread</strong>
          Local tick ${s.tick} · ${s.status}
          ${lowest ? `<div class="mono">${lowest.none ? `no promise from ${zone(lowest.from)} yet` : `lowest promise: ${zone(lowest.from)}, until t ${lowest.until}`}</div>` : ""}
          <div class="mono">at ${formatWall(w - conc.startWall)} · click to jump here</div>`;
      }
    }
    this.tooltip.innerHTML = html;
    this.tooltip.hidden = false;
    const tw = this.tooltip.offsetWidth;
    const left = e.offsetX + 14 + tw > this.width ? e.offsetX - 14 - tw : e.offsetX + 14;
    this.tooltip.style.left = `${Math.max(4, left)}px`;
    this.tooltip.style.top = `${Math.max(0, e.offsetY - 40)}px`;
  }
}
