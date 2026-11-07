package cp26.common;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import cp26.output.Output;
import cp26.output.StatisticsEntry;
import cp26.output.TraceEntry;

public class Checker {

    private static class Count {
        int count;

        Count(int count) {
            this.count = count;
        }
    }

    private final List<String> problems = new ArrayList<>();

    public void check(Output output, Output expected) {
        compareTrace(output.trace(), expected.trace());
        compareStatistics(output.stats(), expected.stats());
    }

    private void compareTrace(List<TraceEntry> trace, List<TraceEntry> expected) {
        Map<TraceEntry, Count> multiset = new HashMap<>();
        int tick = 0;
        int iTrace = 0;
        int iExpected = 0;
        while (iTrace < trace.size() || iExpected < expected.size()) {
            int nextTick = tick + 1;

            // Add all trace entries with the current tick to the multiset
            // (assuming that the trace is sorted).
            for (; iTrace < trace.size(); iTrace++) {
                final TraceEntry entry = trace.get(iTrace);
                if (entry.tick() > tick) {
                    nextTick = entry.tick();
                    break;
                }
                if (entry.tick() < tick) {
                    addProblem("additional/out-of-order %s", entry);
                } else {
                    final Count c = multiset.putIfAbsent(entry, new Count(1));
                    if (c != null)
                        c.count++;
                }
            }

            // Remove each entry of the expected trace with the current tick
            // from the multiset. If trace entries are missing, report them.
            for (; iExpected < expected.size(); iExpected++) {
                final TraceEntry entry = expected.get(iExpected);
                if (entry.tick() > tick) {
                    nextTick = Math.max(nextTick, entry.tick());
                    break;
                }
                if (entry.tick() < tick) {
                    addProblem("missing %s", entry);
                } else {
                    Count c = multiset.get(entry);
                    if (c == null) {
                        addProblem("missing %s", entry);
                    } else if (c.count == 1) {
                        multiset.remove(entry);
                    } else {
                        c.count--;
                    }
                }
            }

            // Report left-over trace entries.
            for (final TraceEntry entry : multiset.keySet()) {
                addProblem("additional %s", entry);
            }
            multiset.clear();

            tick = nextTick;
        }
    }

    private void compareStatistics(
            Map<String, List<StatisticsEntry>> statistics,
            Map<String, List<StatisticsEntry>> expected) {
        boolean missingStats = false;
        for (final Map.Entry<String, List<StatisticsEntry>> map_entry : expected.entrySet()) {
            final String zone = map_entry.getKey();
            final List<StatisticsEntry> expectedEntries = map_entry.getValue();
            final List<StatisticsEntry> entries = statistics.get(zone);

            if (entries == null) {
                missingStats = true;
                addProblem("no statistics for zone: %s", zone);
                continue;
            }

            if (entries.size() != expectedEntries.size()) {
                addProblem(
                        "expected %d statistics entries in zone %s but got %d",
                        expectedEntries.size(),
                        zone,
                        entries.size());
            }

            for (int i = 0; i < Math.min(entries.size(), expectedEntries.size()); i++) {
                final StatisticsEntry gotStatistics = entries.get(i);
                final StatisticsEntry expectedStatistics = expectedEntries.get(i);

                if (!gotStatistics.equals(expectedStatistics)) {
                    addProblem(
                            "statistics for zone '%s' incorrect in tick %d (expected: %s, got: %s)",
                            zone,
                            expectedStatistics.tick(),
                            expectedStatistics,
                            gotStatistics);
                }
            }
        }

        if (missingStats || statistics.size() != expected.size()) {
            for (final String zone : statistics.keySet()) {
                if (!expected.containsKey(zone)) {
                    addProblem("non-existent query: %s", zone);
                    continue;
                }
            }
        }
    }

    public List<String> getProblems() {
        return this.problems;
    }

    private void addProblem(String problem, final Object... args) {
        this.problems.add(String.format(problem, args));
    }
}
