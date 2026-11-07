package cp26.scenario;

import com.fasterxml.jackson.annotation.JsonProperty;

import java.util.List;

/**
 * A zone as described in the scenario.
 */
public record Zone(
        @JsonProperty(value = "name") String name,
        @JsonProperty(value = "locations") List<Location> locations) {
}
