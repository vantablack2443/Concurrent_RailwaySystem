package cp26.simulation.common;

import java.util.*;

/**
 * A utility class for train routing.
 */
public class Router {

    /**
     * Stores the current cost and station used by Dijkstra's algorithm.
     *
     * @param cost    The current cost.
     * @param station The current station.
     */
    private record CostInfo(
            long cost,
            int station) implements Comparable<CostInfo> {

        @Override
        public int compareTo(CostInfo other) {
            int c = Long.compare(this.cost, other.cost);
            if (c != 0)
                return c;
            return Integer.compare(this.station, other.station);
        }
    }

    /**
     * Mutable state associated with a track. See attribute docs for more
     * information.
     */
    public static class TrackState {

        /**
         * The destination station ID.
         */
        public final int destination;

        /**
         * The track distance given by the scenario.
         */
        public final long distance;

        /**
         * The penalized distance (<code>distance * penalty</code>).
         */
        public final long penalizedDistance;

        /**
         * The tick until which the track is occupied.
         */
        public long occupiedUntil;

        public TrackState(
                int destination,
                long distance,
                double penalty,
                long occupiedUntil) {
            this.destination = destination;
            this.distance = distance;
            this.penalizedDistance = (long) Math.floor(distance * penalty);
            this.occupiedUntil = occupiedUntil;
        }
    }

    /**
     * Computes the next track to take when traveling from station <code>from</code>
     * to station <code>to</code>.
     * <br>
     * Tracks within the train's current zone are routed dynamically by taking
     * track occupation into account. Outside the current zone, penalized distances
     * are used instead.
     * <br>
     * This method implements Dijkstra's shortest path algorithm and returns the
     * first track of the computed route.
     *
     * @param from        The current station ID.
     * @param to          The destination station ID.
     * @param now         The tick at which this method is invoked.
     * @param stationZone Maps station IDs to zone IDs.
     * @param trackStates The current track states.
     *
     * @return The ID of the next outgoing trace from <code>from</code>.
     */
    public static int nextTrack(
            final int from,
            final int to,
            final int now,
            final int[] stationZone,
            final TrackState[][] trackStates) {
        if (from == to)
            throw new IllegalArgumentException();
        final int n = trackStates.length;

        final int[] prevStation = new int[n];
        final int[] trackUsed = new int[n];
        final long[] costs = new long[n];

        Arrays.fill(prevStation, -1);
        Arrays.fill(trackUsed, -1);
        Arrays.fill(costs, Long.MAX_VALUE);

        /*
         * A priority queue of candidate locations to explore next, ordered by their
         * cheapest known path cost.
         */
        PriorityQueue<CostInfo> heap = new PriorityQueue<>();

        costs[from] = 0;
        heap.add(new CostInfo(0, from));

        while (!heap.isEmpty()) {
            CostInfo top = heap.poll();
            final long currentCost = top.cost();
            final int station = top.station();

            if (currentCost != costs[station])
                continue;

            /*
             * Once the destination is reached, reconstruct the path and return
             * the first track used from the origin station.
             */
            if (station == to) {
                int cursor = to;
                while (true) {
                    final int prev = prevStation[cursor];
                    if (prev == from)
                        return trackUsed[cursor];
                    cursor = prev;
                }
            }

            final TrackState[] edges = trackStates[station];
            final boolean staysInTheZone = stationZone[station] == stationZone[from];

            for (int i = 0; i < edges.length; i++) {
                TrackState trackState = edges[i];

                final int destination = trackState.destination;

                /*
                 * Within the current zone, routing takes track occupation into account. Outside
                 * the zone, the precomputed penalized distance is used.
                 */
                long edgeCost;
                if (staysInTheZone) {
                    final long wait = trackState.occupiedUntil - now;
                    edgeCost = trackState.distance + (wait > 0 ? wait : 0);
                } else {
                    edgeCost = trackState.penalizedDistance;
                }

                final long newCost = currentCost + edgeCost;

                if (newCost < costs[destination]) {
                    costs[destination] = newCost;
                    prevStation[destination] = station;
                    trackUsed[destination] = i;
                    heap.add(new CostInfo(newCost, destination));
                }
            }
        }

        throw new IllegalStateException("no route");
    }
}