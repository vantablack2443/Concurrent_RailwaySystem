package cp26.scenario;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * A named location.
 *
 * @param name The name of the location.
 * @param pos  The coordinates of the location.
 */
public record Location(
        @JsonProperty(value = "name") String name,
        @JsonProperty(value = "pos") XY pos) {
}
