// Guided tour: six steps through the island run, each jumping playback to a
// moment that shows one idea and highlighting the part of the page to look at.
//
// `api` is provided by main.js: show(scenario), seek(tick), pause(), play(),
// pin(trainLabel | null), setPromises(bool), focus(elementIds).

const STEPS = [
  {
    title: "A railway split into zones",
    body: `The island has <b>5 zones</b>, 16 stations and 26 trains from 9 services. Each train
      leaves its origin at a scheduled tick and travels station by station to its destination.
      Press play, or hover a train or station for details.`,
    tick: 0,
    focus: ["map"],
  },
  {
    title: "Routing reacts to traffic",
    body: `The three <b>Coastal Express</b> trains all run Harbor Quay → Eastport, but take different
      routes. At every station the zone runs Dijkstra: tracks in its own zone cost their length
      plus the wait until they're free; tracks elsewhere cost length × 1.3, since the zone can't
      see their traffic. Here is the first one's route.`,
    tick: 2,
    pin: "Coastal Express #1",
    focus: ["map"],
  },
  {
    title: "One train per track",
    body: `A track holds one train at a time. Four <b>Harbor Shuttles</b> leave Saltmere close
      together, but the single track to Millbrook is busy, so they queue (amber rings). This
      one was ready at t 6 and can only leave at t 12.`,
    tick: 8,
    pin: "Harbor Shuttle #3",
    focus: ["map"],
  },
  {
    title: "Every zone is its own thread",
    body: `Rocket runs one worker thread per zone, each with its own event queue and its own clock.
      The badges show each thread's local tick <i>at the same moment</i>: they differ, because
      threads advance independently. Below, each lane is a thread over real time: solid while
      working, faint while blocked waiting for a message.`,
    tick: 14,
    focus: ["map", "lanes-card", "zones-card"],
  },
  {
    title: "Handoffs and promises",
    body: `When a train crosses a border, the sending thread computes its arrival tick and sends a
      <b>TrainHandoff</b> over the receiving zone's channel. A zone only processes an event at tick T
      once every incoming neighbour has <b>promised</b> that nothing will arrive at or before T —
      the promises are the marks on top of the lanes. This shuttle is handed from Harbor to Central.`,
    tick: 3,
    pin: "Harbor Shuttle #1",
    promises: true,
    focus: ["lanes-card", "log-card"],
  },
  {
    title: "Max tick and shutdown",
    body: `Threads also may not run past a global <b>max tick</b>; a zone that schedules close to it
      asks the main thread, which broadcasts a higher one (diamonds on the main lane — the island
      never needs one, Europe does). When every zone has delivered its trains (flags), main
      broadcasts <b>Shutdown</b>, joins the threads and merges their traces by tick: the output
      matches the sequential Slug exactly.`,
    tick: Infinity,
    focus: ["lanes-card", "compare"],
    final: true,
  },
];

export function createTour(el, api) {
  let step = -1;

  function render() {
    if (step < 0) {
      el.className = "tour idle";
      el.innerHTML = `<button class="tour-start" id="tour-start">
          <span class="tour-badge">Tour</span> How the concurrent simulation works, in 6 steps</button>`;
      el.querySelector("#tour-start").addEventListener("click", () => go(0));
      return;
    }
    const s = STEPS[step];
    el.className = "tour active";
    el.innerHTML = `
      <div class="tour-head">
        <span class="tour-badge">${step + 1} / ${STEPS.length}</span>
        <h2>${s.title}</h2>
        <button class="tour-close" aria-label="Close tour">×</button>
      </div>
      <p>${s.body}</p>
      <div class="tour-nav">
        <button class="btn-text" data-go="${step - 1}" ${step === 0 ? "disabled" : ""}>Back</button>
        ${s.final ? `<button class="btn-text" data-europe>Try Europe</button>` : ""}
        <button class="btn-text primary" data-go="${s.final ? -1 : step + 1}">${s.final ? "Explore freely" : "Next"}</button>
      </div>`;
    el.querySelector(".tour-close").addEventListener("click", () => go(-1));
    for (const b of el.querySelectorAll("[data-go]")) {
      b.addEventListener("click", () => go(Number(b.dataset.go)));
    }
    el.querySelector("[data-europe]")?.addEventListener("click", async () => {
      go(-1);
      await api.show("europe_500");
      api.play();
    });
  }

  async function go(next) {
    step = next;
    render();
    if (step < 0) {
      api.focus([]);
      api.pin(null);
      return;
    }
    const s = STEPS[step];
    await api.show("island");
    api.pause();
    api.seek(s.tick);
    api.pin(s.pin ?? null);
    api.setPromises(Boolean(s.promises));
    api.focus(s.focus);
  }

  render();
  return { start: () => go(0) };
}
