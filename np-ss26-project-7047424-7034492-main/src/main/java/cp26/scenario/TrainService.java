package cp26.scenario;

import com.fasterxml.jackson.annotation.JsonProperty;

import java.util.List;

/**
 * Schedule of a train connection.
 *
 * @param name        The name of the train connection.
 * @param origin      The departure location of the train.
 * @param destination The arrival location of the train.
 * @param departures  The times to depart from the origin.
 */
public record TrainService(
        @JsonProperty(value = "name") String name,
        @JsonProperty(value = "origin") String origin,
        @JsonProperty(value = "destination") String destination,
        @JsonProperty(value = "departures") List<Integer> departures) {
}
