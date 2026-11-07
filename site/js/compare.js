// Slug vs Rocket panel, from `site/data/bench.json` (`just viz-bench`).

import { escapeHtml } from "./data.js";

const ms = (v) => (v < 1 ? "< 1 ms" : `${v} ms`);

function verdict(s) {
  if (s.slugMs < 1 && s.rocketMs < 1) return "both under 1 ms";
  const ratio = s.slugMs / s.rocketMs;
  if (ratio >= 1.05) return `<b class="good">${ratio.toFixed(2)}× faster</b>`;
  if (ratio <= 0.95) return `${(1 / ratio).toFixed(2)}× slower (thread overhead)`;
  return "about the same";
}

export async function renderCompare(el) {
  let bench;
  try {
    const res = await fetch("data/bench.json");
    if (!res.ok) throw new Error(res.status);
    bench = await res.json();
  } catch {
    el.hidden = true;
    return;
  }
  const rows = bench.scenarios
    .map((s) => {
      const equal = s.traceEqual && s.statsEqual;
      const what = s.traced ? "trace + stats" : "stats";
      return `<li>
        <div class="cmp-name">${escapeHtml(s.name)} <span class="${equal ? "good" : "bad"}">${equal ? "✓" : "✗"} same ${what}</span></div>
        <div class="cmp-times"><span>Slug ${ms(s.slugMs)}</span><span>Rocket ${ms(s.rocketMs)}</span></div>
        <div class="cmp-verdict">${verdict(s)}</div>
      </li>`;
    })
    .join("");
  el.innerHTML = `<h2>Slug vs Rocket</h2>
    <p class="hint">Sequential reference (Slug) against this concurrent version (Rocket): same output, and what the threads buy on bigger runs.</p>
    <ul class="compare">${rows}</ul>
    <p class="hint small">Median of ${bench.runs} runs on ${bench.cpus} CPUs, simulation time only, without the viz recording.</p>`;
}
