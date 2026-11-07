import { loadScenario } from "./data.js";
import { Clock } from "./clock.js";
import { createProjection, renderMap, zoneColor } from "./map.js";
import { TrainLayer } from "./trains.js";

const $ = (id) => document.getElementById(id);
const mapBox = $("map");
const svg = $("map-svg");
const tooltip = $("tooltip");
const playBtn = $("play");
const scrub = $("scrub");
const tickOut = $("tick");

const trains = new TrainLayer($("map-canvas"));
const state = { model: null, proj: null, hits: [], mouse: null, hoverId: -1, zoneRows: [], tooltipHtml: "" };

const clock = new Clock((tick, now) => frame(tick, now));
clock.onChange(syncControls);

function layout() {
  if (!state.model) return;
  const { width, height } = mapBox.getBoundingClientRect();
  state.proj = createProjection(state.model, width, height);
  renderMap(svg, state.model, state.proj);
  trains.resize(state.proj);
}

new ResizeObserver(layout).observe(mapBox);

async function show(name) {
  $("loading").hidden = false;
  clock.pause();
  try {
    state.model = await loadScenario(name);
  } catch (err) {
    $("loading").textContent = err.message;
    return;
  }
  $("loading").hidden = true;
  layout();
  buildZoneList();
  buildMeta();
  clock.setEnd(state.model.endTick + 2);
  scrub.max = clock.end;
  const url = new URL(location.href);
  url.searchParams.set("s", name);
  history.replaceState(null, "", url);
}

function frame(tick, now) {
  const { model, proj } = state;
  if (!model || !proj) return;
  state.hits = trains.draw(model, proj, tick, now, state.hoverId);
  updateZoneCounts();
  updateTooltip();
}

// Playback controls

function syncControls() {
  playBtn.classList.toggle("playing", clock.playing);
  playBtn.setAttribute("aria-label", clock.playing ? "Pause" : "Play");
  if (document.activeElement !== scrub) scrub.value = clock.tick;
  tickOut.textContent = `t = ${clock.tick.toFixed(1)} / ${state.model?.endTick ?? 0}`;
}

playBtn.addEventListener("click", () => clock.toggle());
$("restart").addEventListener("click", () => clock.seek(0));
scrub.addEventListener("input", () => clock.seek(Number(scrub.value)));
$("speed").addEventListener("change", (e) => (clock.speed = Number(e.target.value)));
$("scenario").addEventListener("change", (e) => show(e.target.value));

document.addEventListener("keydown", (e) => {
  if (e.target instanceof HTMLSelectElement || e.target instanceof HTMLInputElement) return;
  if (e.code === "Space") {
    e.preventDefault();
    clock.toggle();
  } else if (e.code === "ArrowRight") clock.seek(clock.tick + 1);
  else if (e.code === "ArrowLeft") clock.seek(clock.tick - 1);
});

// Zone list: trains currently travelling from, or waiting in, each zone.

function buildZoneList() {
  const list = $("zones");
  list.replaceChildren();
  state.zoneRows = state.model.zones.map((zone) => {
    const li = document.createElement("li");
    li.innerHTML = `<span class="swatch"></span><span class="name"></span><span class="count"></span>`;
    li.querySelector(".swatch").style.background = zoneColor(zone);
    li.querySelector(".name").textContent = zone.name;
    list.appendChild(li);
    return { count: li.querySelector(".count"), last: "" };
  });
}

function updateZoneCounts() {
  const { model, hits, zoneRows } = state;
  const moving = new Array(model.zones.length).fill(0);
  const waiting = new Array(model.zones.length).fill(0);
  for (const hit of hits) {
    if (hit.queue) {
      waiting[model.stations[hit.station].zone] += hit.queue.length;
      continue;
    }
    const s = hit.state;
    if (s.kind === "moving") moving[model.stations[s.move.from].zone]++;
    else waiting[model.stations[s.station].zone]++;
  }
  zoneRows.forEach((row, i) => {
    const text = `<b>${moving[i]}</b> moving · <span class="w">${waiting[i]}</span> waiting`;
    if (text !== row.last) {
      row.count.innerHTML = text;
      row.last = text;
    }
  });
}

function buildMeta() {
  const { model } = state;
  const rows = [
    ["Zones (threads)", model.zones.length],
    ["Stations", model.stations.length],
    ["Trains", model.trains.length],
    ["Services", model.services],
    ["Border penalty", `×${model.penalty}`],
    ["Recorded on", `${model.meta.cpus} CPUs`],
    ["Wall time", `${(model.meta.wallNs / 1e6).toFixed(2)} ms`],
  ];
  $("meta").innerHTML = `<h2>${escapeHtml(model.name)}</h2><dl>${rows
    .map(([k, v]) => `<dt>${k}</dt><dd>${escapeHtml(String(v))}</dd>`)
    .join("")}</dl>`;
}

// Tooltip: nearest train under the pointer, otherwise nearest station.

mapBox.addEventListener("pointermove", (e) => {
  const rect = mapBox.getBoundingClientRect();
  state.mouse = [e.clientX - rect.left, e.clientY - rect.top];
});
mapBox.addEventListener("pointerleave", () => {
  state.mouse = null;
});

function nearest(items, pos, [mx, my], radius) {
  let best = null;
  let bestD = radius * radius;
  for (const item of items) {
    const [x, y] = pos(item);
    const d = (x - mx) ** 2 + (y - my) ** 2;
    if (d < bestD) {
      bestD = d;
      best = item;
    }
  }
  return best;
}

function updateTooltip() {
  const { model, proj, mouse } = state;
  state.hoverId = -1;
  if (!mouse) {
    tooltip.hidden = true;
    return;
  }
  const station = (id) => model.stations[id];
  const zoneOf = (id) => model.zones[station(id).zone];
  let html = null;
  let at = null;

  const hit = nearest(state.hits, (h) => [h.x, h.y], mouse, 10);
  if (hit?.queue) {
    at = [hit.x, hit.y];
    const shown = hit.queue.slice(0, 5).map(({ train }) => escapeHtml(train.label)).join("<br>");
    const more = hit.queue.length > 5 ? `<br>… and ${hit.queue.length - 5} more` : "";
    html = `<strong>${hit.queue.length} trains waiting at ${escapeHtml(station(hit.station).name)}</strong>
      <div class="mono">${shown}${more}</div>`;
  } else if (hit) {
    const { train, state: s } = hit;
    state.hoverId = train.id;
    at = [hit.x, hit.y];
    if (s.kind === "moving") {
      const { from, to, depart, arrive } = s.move;
      const crossing = station(from).zone !== station(to).zone;
      html = `<strong>${escapeHtml(train.label)}</strong>
        ${escapeHtml(station(from).name)} → ${escapeHtml(station(to).name)}
        <div class="mono">t ${depart} → ${arrive}${crossing ? ` · handoff ${escapeHtml(zoneOf(from).name)} → ${escapeHtml(zoneOf(to).name)}` : ""}</div>`;
    } else {
      const { from, to, ready, depart } = s.move;
      html = `<strong>${escapeHtml(train.label)}</strong>
        Waiting at ${escapeHtml(station(from).name)} for the track to ${escapeHtml(station(to).name)}
        <div class="mono">ready t ${ready} · leaves t ${depart}</div>`;
    }
  } else {
    const id = nearest(model.stations.map((s) => s.id), (i) => proj.points[i], mouse, 10);
    if (id !== null) {
      at = proj.points[id];
      html = `<strong>${escapeHtml(station(id).name)}</strong>
        Zone ${escapeHtml(zoneOf(id).name)}
        <div class="mono">${station(id).degree} outgoing tracks</div>
        ${station(id).scenarioPos ? `<div class="mono">Scenario puts it at (${station(id).scenarioPos.x}, ${station(id).scenarioPos.y}), outside the grid; drawn between its neighbours.</div>` : ""}`;
    }
  }

  if (!html) {
    tooltip.hidden = true;
    return;
  }
  if (html !== state.tooltipHtml) {
    tooltip.innerHTML = html;
    state.tooltipHtml = html;
  }
  tooltip.hidden = false;
  const w = tooltip.offsetWidth;
  const left = at[0] + 14 + w > proj.width ? at[0] - 14 - w : at[0] + 14;
  tooltip.style.left = `${Math.max(4, left)}px`;
  tooltip.style.top = `${Math.max(4, at[1] - 10)}px`;
}

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

const initial = new URLSearchParams(location.search).get("s");
const select = $("scenario");
if (initial && [...select.options].some((o) => o.value === initial)) select.value = initial;
show(select.value);
