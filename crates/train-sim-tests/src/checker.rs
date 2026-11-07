use std::collections::{HashMap, hash_map::Entry};

use train_sim_core::output::{Output, StatisticsEntry, TraceEntry};

#[derive(Debug, Clone, Default)]
pub struct Checker {
    problems: Vec<String>,
}

impl Checker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    pub fn has_problems(&self) -> bool {
        !self.problems.is_empty()
    }

    pub fn check(&mut self, output: &Output, expected: &Output) {
        self.compare_trace(&output.trace, &expected.trace);
        self.compare_statistics(&output.statistics, &expected.statistics);
    }

    fn compare_trace(&mut self, trace: &[TraceEntry], expected: &[TraceEntry]) {
        let mut multiset: HashMap<&TraceEntry, usize> = HashMap::new();
        let mut tick = 0;
        let mut i_trace = 0;
        let mut i_expected = 0;
        while i_trace < trace.len() || i_expected < expected.len() {
            let mut next_tick = tick + 1;

            // Add all trace entries with the current tick to the multiset
            // (assuming that the trace is sorted).
            while let Some(entry) = trace.get(i_trace) {
                if entry.tick > tick {
                    next_tick = entry.tick;
                    break;
                }
                if entry.tick < tick {
                    self.problems
                        .push(format!("additional/out-of-order {entry:?}"));
                } else {
                    *multiset.entry(entry).or_default() += 1;
                }
                i_trace += 1;
            }

            // Remove each entry of the expected trace with the current tick
            // from the multiset. If trace entries are missing, report them.
            while let Some(entry) = expected.get(i_expected) {
                if entry.tick > tick {
                    next_tick = std::cmp::max(next_tick, entry.tick);
                    break;
                }
                if entry.tick < tick {
                    self.problems.push(format!("missing {entry:?}"));
                } else {
                    match multiset.entry(entry) {
                        Entry::Occupied(e) if *e.get() == 1 => {
                            e.remove();
                        }
                        Entry::Occupied(mut e) => *e.get_mut() -= 1,
                        Entry::Vacant(_) => self.problems.push(format!("missing {entry:?}")),
                    }
                }
                i_expected += 1;
            }

            // Report left-over trace entries.
            for (entry, _) in multiset.drain() {
                self.problems.push(format!("additional {entry:?}"))
            }

            tick = next_tick;
        }
    }

    fn compare_statistics(
        &mut self,
        statistics: &HashMap<String, Vec<StatisticsEntry>>,
        expected: &HashMap<String, Vec<StatisticsEntry>>,
    ) {
        let mut missing_stats = false;
        for (zone, expected_entries) in expected.iter() {
            let Some(entries) = statistics.get(zone) else {
                self.problems
                    .push(format!("no statistics for zone '{zone}'"));
                missing_stats = true;
                continue;
            };

            if entries.len() != expected_entries.len() {
                self.problems.push(format!(
                    "expected {} statistics entries in zone '{zone}' but got {}",
                    expected_entries.len(),
                    entries.len()
                ));
            }

            for (got_statistics, expected_statistics) in entries.iter().zip(expected_entries.iter())
            {
                if got_statistics != expected_statistics {
                    let tick = expected_statistics.tick;
                    self.problems.push(format!(
                        "statistics for zone '{zone}' incorrect in tick {tick} (expected: {expected_statistics}, got: {got_statistics})"
                    ));
                }
            }
        }

        if missing_stats || statistics.len() != expected.len() {
            for zone in statistics.keys() {
                if !expected.contains_key(zone) {
                    self.problems.push(format!("non-existent zone '{zone}'"));
                }
            }
        }
    }
}

pub fn check(output: &Output, expected: &Output) -> Checker {
    let mut checker = Checker::new();
    checker.check(output, expected);
    checker
}
