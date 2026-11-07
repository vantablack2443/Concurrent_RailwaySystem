package cp26.simulation.common;

/**
 * The main simulation event.
 *
 * @param tick              Tick in which the train will arrive.
 * @param trainId           ID of the train that is moving.
 * @param stationId         ID of the station on which the train arrives.
 * @param previousStationId ID of the train's previous station. May be
 *                          <code>null</code> if <code>station</code> is
 *                          the first on the train's journey.
 */
public record TrainArrival(
        int tick,
        int trainId,
        int stationId,
        Integer previousStationId) implements Comparable<TrainArrival> {

    /**
     * This method is overridden for the sake of the priority queue. We want events
     * with the lowest
     * {@link #tick} number, then by lowest {@link #trainId} if {@link #tick} is
     * tied, to be processed
     * first.
     *
     * @param other The other {@link TrainArrival} to compare against.
     *
     * @return {@code 0} if {@code x < y} and a value greater
     *         {@code 0} if {@code x > y}.
     */
    @Override
    public int compareTo(TrainArrival other) {
        int byTick = Long.compare(this.tick, other.tick);
        if (byTick != 0)
            return byTick;
        return Integer.compare(this.trainId, other.trainId);
    }
}
