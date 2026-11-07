// Per-zone clock badges drawn on the map canvas: each zone thread's own
// local tick at the current wall-clock moment, and whether it is working,
// blocked on its channel, or stopped.

/** Badges are only drawn on maps with few zones; larger maps use the list. */
const MAX_BADGES = 8;

export function drawZoneBadges(ctx, model, proj, states, colors) {
  if (model.zones.length > MAX_BADGES) return;
  ctx.font = `500 11px "IBM Plex Mono", monospace`;
  ctx.textBaseline = "middle";
  ctx.textAlign = "left";
  model.zones.forEach((zone, z) => {
    const anchor = proj.zoneAnchors[z];
    if (!anchor) return;
    const { tick, status } = states[z];
    const text = `t ${tick}`;
    const w = ctx.measureText(text).width + 24;
    const h = 18;
    const x = anchor[0] - w / 2;
    const y = anchor[1] + 34;

    ctx.beginPath();
    ctx.roundRect(x, y, w, h, 9);
    ctx.fillStyle = colors.surface;
    ctx.fill();
    ctx.lineWidth = 1.2;
    ctx.strokeStyle = `hsl(${zone.hue.toFixed(1)} 52% 46%)`;
    ctx.stroke();

    // Status dot: filled = working, hollow amber = blocked, grey = stopped.
    ctx.beginPath();
    ctx.arc(x + 10, y + h / 2, 4, 0, Math.PI * 2);
    if (status === "blocked") {
      ctx.lineWidth = 2;
      ctx.strokeStyle = colors.wait;
      ctx.stroke();
    } else {
      ctx.fillStyle = status === "running" ? `hsl(${zone.hue.toFixed(1)} 60% 48%)` : colors.soft;
      ctx.fill();
    }

    ctx.fillStyle = colors.ink;
    ctx.fillText(text, x + 18, y + h / 2 + 0.5);
  });
}
