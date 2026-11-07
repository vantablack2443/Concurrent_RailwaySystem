// Static part of the map, drawn as SVG: land, zone regions (Voronoi cells of
// the stations, merged by zone), zone borders, tracks, stations and labels.
// Everything is laid out in screen pixels through one shared projection, so
// the trains canvas can draw on top with the same coordinates.

import { Delaunay } from "https://cdn.jsdelivr.net/npm/d3-delaunay@6.0.4/+esm";

const SVG_NS = "http://www.w3.org/2000/svg";
const PAD = 28;
/** Station labels are only drawn for small networks. */
const MAX_LABELLED_STATIONS = 40;

export function zoneColor(zone, alpha = 1) {
  return `hsl(${zone.hue.toFixed(1)} 52% 46% / ${alpha})`;
}

/**
 * Fits the stations into a `width`×`height` pixel box. Uses the stations'
 * bounding box rather than `gridSize`, since some scenarios place stations
 * outside the declared grid.
 */
export function createProjection(model, width, height) {
  const xs = model.stations.map((s) => s.x);
  const ys = model.stations.map((s) => s.y);
  const [minX, maxX] = [Math.min(...xs), Math.max(...xs)];
  const [minY, maxY] = [Math.min(...ys), Math.max(...ys)];
  const gw = Math.max(maxX - minX, 1);
  const gh = Math.max(maxY - minY, 1);
  // Room around the stations for the land outline and labels.
  const margin = Math.min(PAD + 40, 16 + width * 0.05);
  const scale = Math.min((width - 2 * margin) / gw, (height - 2 * margin) / gh);
  const ox = (width - gw * scale) / 2 - minX * scale;
  const oy = (height - gh * scale) / 2 - minY * scale;
  const points = model.stations.map((s) => [ox + s.x * scale, oy + s.y * scale]);
  return {
    width,
    height,
    scale,
    bounds: [PAD, PAD, width - PAD, height - PAD],
    points,
    // Small networks get bigger stations and trains.
    unit: model.stations.length > MAX_LABELLED_STATIONS ? 0.75 : 1.15,
  };
}

function el(name, attrs = {}, parent) {
  const node = document.createElementNS(SVG_NS, name);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  if (parent) parent.appendChild(node);
  return node;
}

const fmt = (p) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`;

/** Closed Catmull-Rom spline through `pts`, as an SVG path. */
function smoothClosedPath(pts) {
  const n = pts.length;
  let d = `M${fmt(pts[0])}`;
  for (let i = 0; i < n; i++) {
    const p0 = pts[(i - 1 + n) % n];
    const p1 = pts[i];
    const p2 = pts[(i + 1) % n];
    const p3 = pts[(i + 2) % n];
    const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    d += `C${fmt(c1)} ${fmt(c2)} ${fmt(p2)}`;
  }
  return `${d}Z`;
}

/** A smooth "landmass" around all stations: the padded convex hull. */
function landPath(delaunay, points, pad) {
  const hull = Array.from(delaunay.hull, (i) => points[i]);
  const cx = hull.reduce((s, p) => s + p[0], 0) / hull.length;
  const cy = hull.reduce((s, p) => s + p[1], 0) / hull.length;
  const padded = hull.map(([x, y]) => {
    const dx = x - cx;
    const dy = y - cy;
    const len = Math.hypot(dx, dy) || 1;
    return [x + (dx / len) * pad, y + (dy / len) * pad];
  });
  return smoothClosedPath(padded);
}

/** Voronoi edges shared by cells of two different zones. */
function zoneBorderPath(voronoi, model) {
  const owners = new Map();
  const key = (p) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`;
  model.stations.forEach((station, i) => {
    const poly = voronoi.cellPolygon(i);
    if (!poly) return;
    for (let k = 0; k < poly.length - 1; k++) {
      const a = key(poly[k]);
      const b = key(poly[k + 1]);
      const edge = a < b ? `${a}|${b}` : `${b}|${a}`;
      const entry = owners.get(edge) ?? { zones: new Set(), a: poly[k], b: poly[k + 1] };
      entry.zones.add(station.zone);
      owners.set(edge, entry);
    }
  });
  let d = "";
  for (const { zones, a, b } of owners.values()) {
    if (zones.size > 1) d += `M${fmt(a)}L${fmt(b)}`;
  }
  return d;
}

export function renderMap(svg, model, proj) {
  svg.replaceChildren();
  svg.setAttribute("viewBox", `0 0 ${proj.width} ${proj.height}`);
  const { points } = proj;
  const delaunay = Delaunay.from(points);
  const voronoi = delaunay.voronoi(proj.bounds.map((v, i) => v + (i < 2 ? -PAD : PAD)));
  const land = landPath(delaunay, points, Math.min(56, Math.max(26, 6 * proj.scale)));

  const defs = el("defs", {}, svg);
  const clip = el("clipPath", { id: "land-clip" }, defs);
  el("path", { d: land }, clip);

  el("path", { d: land, class: "land" }, svg);

  const regions = el("g", { "clip-path": "url(#land-clip)" }, svg);
  model.stations.forEach((station, i) => {
    const d = voronoi.renderCell(i);
    if (!d) return;
    const zone = model.zones[station.zone];
    el("path", { d, class: "cell", style: `fill: hsl(${zone.hue.toFixed(1)} 52% 50% / var(--zone-alpha))` }, regions);
  });
  el("path", { d: zoneBorderPath(voronoi, model), class: "zone-border" }, regions);

  // Two-way tracks are drawn once; direction shows up as lanes on the canvas.
  const tracks = el("g", {}, svg);
  const drawn = new Set();
  for (const t of model.tracks) {
    const key = t.from < t.to ? `${t.from}-${t.to}` : `${t.to}-${t.from}`;
    if (drawn.has(key)) continue;
    drawn.add(key);
    const [x1, y1] = points[t.from];
    const [x2, y2] = points[t.to];
    el("line", { x1, y1, x2, y2, class: "track" }, tracks);
  }

  const showLabels = model.stations.length <= MAX_LABELLED_STATIONS;
  const stations = el("g", {}, svg);
  const labels = el("g", {}, svg);
  const r = 4.2 * proj.unit;
  model.stations.forEach((s, i) => {
    const [x, y] = points[i];
    el("circle", { cx: x, cy: y, r: s.degree >= 4 ? r * 1.25 : r, class: s.degree >= 4 ? "station hub" : "station" }, stations);
    if (showLabels) {
      const right = x < proj.width - 110;
      const text = el("text", {
        x: right ? x + r + 6 : x - r - 6,
        y: y - r - 3,
        "text-anchor": right ? "start" : "end",
        class: "station-label",
      }, labels);
      text.textContent = s.name;
    }
  });

  // Zone names at the centroid of their stations, nudged below the stations.
  for (const zone of model.zones) {
    if (zone.stations.length === 0) continue;
    const cx = zone.stations.reduce((s, i) => s + points[i][0], 0) / zone.stations.length;
    const cy = zone.stations.reduce((s, i) => s + points[i][1], 0) / zone.stations.length;
    const text = el("text", {
      x: cx,
      y: cy + (showLabels ? 26 : 4),
      "text-anchor": "middle",
      class: "zone-label",
      fill: zoneColor(zone),
    }, labels);
    text.textContent = zone.name;
  }
}
