package cp26.simulation.common;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

import cp26.scenario.Location;
import cp26.scenario.Scenario;
import cp26.scenario.Zone;

/**
 * Encapsulates static information about the simulation context.
 */
public class RailwayNetwork {

    /**
     * Maps a zone's ID to the station IDs belonging to that zone.
     */
    private final int[][] zoneToStations;

    /**
     * Maps a station's ID to its zone ID.
     */
    private final int[] stationToZone;

    /**
     * Maps the station's name to its ID.
     */
    private final Map<String, Integer> nameToStationId;

    /**
     * Maps a station's ID to its name.
     */
    private final String[] stationIdToName;

    /**
     * Constructs a {@link RailwayNetwork} object, by running the initialization
     * process.
     *
     * @param scenario The parsed scenario.
     */
    public RailwayNetwork(Scenario scenario) {
        final List<Zone> zones = scenario.zones();
        final int zoneCount = zones.size();

        int stationCount = 0;
        for (final Zone z : zones)
            stationCount += z.locations().size();

        this.stationToZone = new int[stationCount];
        this.stationIdToName = new String[stationCount];
        this.zoneToStations = new int[zoneCount][];
        this.nameToStationId = HashMap.newHashMap(stationCount);

        int stationId = 0;

        for (int zoneId = 0; zoneId < zoneCount; zoneId++) {
            final Zone zone = zones.get(zoneId);
            final List<Location> locations = zone.locations();

            int size = locations.size();
            int[] stations = new int[size];

            int i = 0;
            for (final Location loc : locations) {

                stations[i++] = stationId;

                nameToStationId.put(loc.name(), stationId);
                stationIdToName[stationId] = loc.name();
                stationToZone[stationId] = zoneId;

                stationId++;
            }

            zoneToStations[zoneId] = stations;
        }
    }

    public int[] getStationToZone() {
        return stationToZone;
    }

    /**
     * Returns whether two stations are in the same zone.
     *
     * @param stationA The station ID of station A.
     * @param stationB The station ID of station B.
     * @return {@code true} if both stations are in the same zone, {@code false}
     *         otherwise.
     */
    public boolean stationInTheSameZone(int stationA, int stationB) {
        return this.stationToZone[stationA] == this.stationToZone[stationB];
    }

    /**
     * Returns the number of stations in the network.
     *
     * @return The number of stations.
     */
    public int stationCount() {
        return this.stationToZone.length;
    }

    /**
     * Returns the mapping from zone ID to station IDs.
     *
     * @return The zone to station mapping.
     */
    public int[][] getZoneToStations() {
        return this.zoneToStations;
    }

    /**
     * Returns the zone ID given a station ID.
     *
     * @param stationId The station ID.
     * @return The zone ID.
     */
    public int getZoneIdFromStationId(int stationId) {
        return this.stationToZone[stationId];
    }

    /**
     * Returns a station's ID given the station name.
     *
     * @param stationName The station name.
     * @return The station's ID.
     */
    public int getStationId(String stationName) {
        return this.nameToStationId.get(stationName);
    }

    /**
     * Returns a station's name given the station ID.
     *
     * @param stationId The station ID.
     * @return The station's name.
     */
    public String getStationName(int stationId) {
        return this.stationIdToName[stationId];
    }
}