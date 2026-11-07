package cp26.scenario;

import com.fasterxml.jackson.annotation.JsonProperty;

import java.util.*;

/**
 * Represents a simulation scenario.
 *
 * @param name        The name of the scenario.
 * @param parameters  The simulation parameters of the scenario.
 * @param gridSize    The size of the grid of the simulation.
 * @param trace       Indicates whether a trace should be captured.
 * @param queries     Statistic queries to compute.
 * @param services    The train services of the scenario.
 * @param zones       The zones into which the map is divided.
 * @param connections Contact Plan: Specifies the locations and the connections
 *                    to each one.
 */
public record Scenario(
        @JsonProperty(value = "name") String name,
        @JsonProperty(value = "parameters") Parameters parameters,
        @JsonProperty(value = "gridSize") XY gridSize,
        @JsonProperty(value = "trace") boolean trace,
        @JsonProperty(value = "statQueries") HashSet<String> queries,
        @JsonProperty(value = "services") List<TrainService> services,
        @JsonProperty(value = "zones") List<Zone> zones,
        @JsonProperty(value = "connections") HashMap<String, List<TrackSegment>> connections) {
}
