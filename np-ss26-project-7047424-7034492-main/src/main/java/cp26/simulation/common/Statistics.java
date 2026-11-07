package cp26.simulation.common;

import java.util.List;

import cp26.output.StatisticsEntry;

/**
 * Maintains aggregated train statistics for a single zone.
 * <br>
 * The log does not store every tick, but only ticks where the number of
 * traveling trains changes.
 */
public class Statistics {

    /**
     * The current simulation tick associated with this statistics snapshot.
     */
    public int tick;

    /**
     * The current number of trains traveling within the zone.
     */
    public int trainCount;

    /**
     * Compressed history of train counts over time.
     * <br>
     * Each entry is only stored when the value changes.
     */
    public final List<StatisticsEntry> log;

    public Statistics(int tick, int trainCount, List<StatisticsEntry> log) {
        this.tick = tick;
        this.trainCount = trainCount;
        this.log = log;
    }

    /**
     * Appends a new log entry if the train count changed since the last recorded
     * entry.
     * <br>
     * This keeps the log compact by avoiding duplicate consecutive states.
     */
    public void pushEntry() {
        if (log.isEmpty() || log.getLast().traveling() != trainCount)
            log.add(new StatisticsEntry(tick, trainCount));
    }

    /**
     * Applies a delta to the current train count.
     *
     * @param delta Change in number of traveling trains (can be negative or
     *              positive).
     */
    public void updateTrainCount(int delta) {
        this.trainCount += delta;
    }
}