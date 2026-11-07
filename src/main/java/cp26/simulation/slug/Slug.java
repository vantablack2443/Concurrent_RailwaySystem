package cp26.simulation.slug;

import cp26.output.*;
import cp26.scenario.*;
import cp26.simulation.common.*;

import java.util.*;

/**
 * A sequential reference implementation. As all simulation engines, this class
 * implements
 * the {@link Simulation} interface.
 */
public class Slug implements Simulation {

    /**
     * The parsed scenario file.
     */
    private final Scenario scenario;

    /**
     * Defines whether trace collection is enabled (defined by the
     * {@link Scenario}).
     */
    private final boolean traceEnabled;

    /**
     * Static information about the network.
     */
    private final RailwayNetwork network;

    /**
     * Maps station IDs to zone IDs. Given by the {@link RailwayNetwork}.
     */
    private final int[] stationToZone;

    /**
     * Given a station ID <code>s</code> and a track number <code>t</code> within
     * station <code>s</code>,
     * <code>trackStates[s][t]</code> contains <code>t</code>'s
     * {@link Router.TrackState}.
     */
    private final Router.TrackState[][] trackStates;

    /**
     * Trains created using {@link TrainService}, which is given by the
     * {@link Scenario},
     */
    private final List<Train> trains = new ArrayList<>();

    /**
     * The output trace, see {@link TraceEntry}.
     */
    private final List<TraceEntry> trace = new ArrayList<>();

    /**
     * Statistics per zone, see {@link Statistics}.
     */
    private final List<Statistics> zoneToStatistics = new ArrayList<>();

    /**
     * Constructs a {@link Slug} object. The simulation can be run using
     * {@link #run()}, the corresponding
     * {@link Output} can be retrieved using {@link #getOutput()} after the
     * simulation has finished.
     *
     * @param scenario The parsed scenario file.
     */
    public Slug(Scenario scenario) {
        this.scenario = scenario;
        this.traceEnabled = scenario.trace();

        this.network = new RailwayNetwork(scenario);
        this.stationToZone = network.getStationToZone();

        final int stationCount = network.stationCount();
        this.trackStates = new Router.TrackState[stationCount][];

        initTrackStates();
        createTrains();
        initStatistics();
    }

    /**
     * Initializes the {@link #trackStates}.
     */
    private void initTrackStates() {
        final double penalty = scenario.parameters().getPenalty();
        final int stationCount = network.stationCount();

        for (int i = 0; i < stationCount; i++)
            this.trackStates[i] = new Router.TrackState[0];

        for (Map.Entry<String, List<TrackSegment>> entry : scenario.connections().entrySet()) {

            final int origin = network.getStationId(entry.getKey());
            final List<TrackSegment> trackSegments = entry.getValue();

            final Router.TrackState[] trackStates = new Router.TrackState[trackSegments.size()];

            for (int i = 0; i < trackSegments.size(); i++) {
                final TrackSegment trackSegment = trackSegments.get(i);

                trackStates[i] = new Router.TrackState(
                        network.getStationId(trackSegment.destination()),
                        trackSegment.distance(),
                        penalty,
                        0);
            }

            this.trackStates[origin] = trackStates;
        }
    }

    /**
     * Creates all {@link Train} objects based on in the scenario defined
     * {@link TrainService}'s and adds them to the {@link #trains} list.
     */
    private void createTrains() {
        for (final TrainService service : scenario.services()) {
            final int origin = network.getStationId(service.origin());
            final int destination = network.getStationId(service.destination());

            for (final int departure : service.departures()) {
                this.trains.add(new Train(service.name(), origin, destination, departure));
            }
        }
    }

    /**
     * Initializes the {@link #zoneToStatistics} mapping.
     */
    private void initStatistics() {
        for (final Zone zone : scenario.zones()) {
            if (scenario.queries().contains(zone.name())) {

                zoneToStatistics.add(new Statistics(
                        0,
                        0,
                        new ArrayList<>()));
            } else {
                zoneToStatistics.add(null);
            }
        }
    }

    /**
     * Calling this method runs the simulation.
     */
    @Override
    public void run() {
        /*
         * Add all described trains into a priority queue.
         */
        final PriorityQueue<TrainArrival> arrivalQueue = new PriorityQueue<>(this.trains.size());
        int trainId = 0;
        for (final Train train : this.trains) {
            arrivalQueue.add(new TrainArrival(
                    train.startOn(),
                    trainId++,
                    train.origin(),
                    null));
        }

        /*
         * The simulation loop.
         */
        int lastTick = 0;
        while (!arrivalQueue.isEmpty()) {
            final TrainArrival arrival = arrivalQueue.poll();

            if (arrival.tick() < lastTick)
                throw new IllegalStateException("Trains out of order!");

            processArrival(arrival, arrivalQueue);
            lastTick = arrival.tick();
        }
    }

    /**
     * Processes a train arrival event.
     *
     * @param arrival      The train arrival event to process.
     * @param arrivalQueue The queue of train arrivals, passed such that it can be
     *                     updated by adding
     *                     new arrival events.
     */
    private void processArrival(final TrainArrival arrival, final PriorityQueue<TrainArrival> arrivalQueue) {
        final int now = arrival.tick();
        final int currentStationId = arrival.stationId();
        final Integer prevStationId = arrival.previousStationId();

        final int currentZoneId = network.getZoneIdFromStationId(currentStationId);

        final Train train = trains.get(arrival.trainId());
        final int destinationId = train.destination();
        final String trainName = train.name();

        final boolean wasMovingWithinZone = wasMovingWithinZone(prevStationId, currentStationId);

        int delta = 0; // Change in the zone's train count, used for statistics.
        String traceDestination = null;

        // Check if the train has arrived at its destination.
        if (destinationId == currentStationId) {

            // Check if we need to reduce the number of trains in the statistics.
            if (wasMovingWithinZone)
                delta = -1;

        } else {

            final int trackId = Router.nextTrack(
                    currentStationId,
                    destinationId,
                    now,
                    stationToZone,
                    trackStates);

            final Router.TrackState trackState = trackStates[currentStationId][trackId];

            /*
             * One distance unit + maximum between the time that the train arrived and when
             * the track is free.
             */
            final int arrivalTick = (int) (Math.max(now, trackState.occupiedUntil) + trackState.distance);
            final int nextStation = trackState.destination;

            /*
             * Add new event to the priority queue and update track occupation.
             */
            trackState.occupiedUntil = arrivalTick;
            arrivalQueue.add(new TrainArrival(
                    arrivalTick,
                    arrival.trainId(),
                    nextStation,
                    currentStationId));

            final boolean willStayInZone = network.getZoneIdFromStationId(nextStation) == currentZoneId;

            if (wasMovingWithinZone) {
                if (!willStayInZone)
                    delta = -1;
            } else {
                if (willStayInZone)
                    delta = 1;
            }

            if (traceEnabled)
                traceDestination = network.getStationName(nextStation);
        }

        extendTrace(now, trainName, currentStationId, traceDestination);
        updateStatistics(currentZoneId, now, delta);
    }

    /**
     * Determines whether a train was moving within the same zone.
     *
     * @param prevStationId    The ID of the previous station.
     * @param currentStationId The ID of the current station.
     *
     * @return <code>True</code> if the train moved within a zone,
     *         <code>false</code> otherwise.
     */
    private boolean wasMovingWithinZone(Integer prevStationId, int currentStationId) {
        return prevStationId != null && network.stationInTheSameZone(currentStationId, prevStationId);
    }

    /**
     * Extends the trace, if enabled ({@link #traceEnabled}).
     *
     * @param now              The tick the trace corresponds to.
     * @param trainName        The name of the train connection.
     * @param currentStationId The train's current station ID.
     * @param traceDestination The train's destination ID.
     */
    private void extendTrace(int now, String trainName, int currentStationId, String traceDestination) {
        if (traceEnabled) {

            trace.add(new TraceEntry(
                    now,
                    trainName,
                    network.getStationName(currentStationId),
                    traceDestination));
        }
    }

    /**
     * Updates the statistics, if there is a query specified for this zone.
     *
     * @param currentZoneId The current zone ID.
     * @param now           The tick at which this statistics snapshot is created.
     * @param delta         The change in the zone's train count during this tick.
     */
    private void updateStatistics(int currentZoneId, int now, int delta) {
        final Statistics stats = zoneToStatistics.get(currentZoneId);

        if (stats != null) {

            /*
             * Push statistics entry and update tick upon reaching a new tick.
             */
            if (stats.tick != now) {
                stats.pushEntry();
                stats.tick = now;
            }

            stats.updateTrainCount(delta);
        }
    }

    /**
     * Creates and returns the output generated by the simulation.
     *
     * @return An {@link Output} object representing the output.
     */
    @Override
    public Output getOutput() {
        /*
         * The trace should be sorted by tick in the output. For the slug
         * implementation, the trace is sorted
         * by construction, so the following is actually superfluous.
         */
        trace.sort(Comparator.comparingInt(TraceEntry::tick));

        final Map<String, List<StatisticsEntry>> stats = new HashMap<>();

        for (int i = 0; i < zoneToStatistics.size(); i++) {
            final Statistics s = zoneToStatistics.get(i);
            if (s == null)
                continue;

            s.pushEntry();

            final String name = scenario.zones().get(i).name();
            stats.put(name, s.log);
        }

        return new Output(scenario, trace, stats);
    }
}