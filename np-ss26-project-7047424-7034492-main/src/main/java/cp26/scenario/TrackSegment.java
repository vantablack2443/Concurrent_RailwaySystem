package cp26.scenario;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Represents a track segment.
 *
 * @param destination The name of the destination Location.
 * @param distance    The distance until reaching the destination.
 */
public record TrackSegment(
        @JsonProperty(value = "destination") String destination,
        @JsonProperty(value = "distance") int distance) {
}
