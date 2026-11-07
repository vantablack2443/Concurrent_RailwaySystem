// Simulation-time playback. Runs a requestAnimationFrame loop for the whole
// page lifetime (so idle animations such as pulses keep going while paused)
// and advances the tick only while playing.

/** Seconds a full run takes at 1× speed. */
const RUN_SECONDS = 24;

export class Clock {
  constructor(onFrame) {
    this.onFrame = onFrame;
    this.tick = 0;
    this.end = 1;
    this.speed = 1;
    this.playing = false;
    this.ticksPerSecond = 1;
    this.listeners = new Set();
    this.last = performance.now();
    requestAnimationFrame((now) => this.loop(now));
  }

  setEnd(end) {
    this.end = end;
    this.ticksPerSecond = Math.max(end / RUN_SECONDS, 1);
    this.seek(0);
  }

  play() {
    if (this.tick >= this.end) this.tick = 0;
    this.playing = true;
    this.emit();
  }

  pause() {
    this.playing = false;
    this.emit();
  }

  toggle() {
    if (this.playing) this.pause();
    else this.play();
  }

  seek(tick) {
    this.tick = Math.min(Math.max(tick, 0), this.end);
    this.emit();
  }

  onChange(fn) {
    this.listeners.add(fn);
  }

  emit() {
    for (const fn of this.listeners) fn(this);
  }

  loop(now) {
    const dt = Math.min((now - this.last) / 1000, 0.1);
    this.last = now;
    if (this.playing) {
      this.tick += dt * this.ticksPerSecond * this.speed;
      if (this.tick >= this.end) {
        this.tick = this.end;
        this.playing = false;
      }
      this.emit();
    }
    this.onFrame(this.tick, now);
    requestAnimationFrame((n) => this.loop(n));
  }
}
