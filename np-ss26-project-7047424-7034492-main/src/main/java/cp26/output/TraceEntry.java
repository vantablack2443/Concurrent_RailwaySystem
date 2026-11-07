package cp26.output;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Represents the trains that got mounted to a new Track in a Tick
 *
 * @param tick        The tick corresponding to this entry.
 * @param train       The train name.
 * @param origin      The station's name at which the train is currently
 *                    located.
 * @param destination The next station's name the train will travel to, or
 *                    <code>null</code> if the train
 *                    has reached its destination.
 */
public record TraceEntry(
                @JsonProperty(value = "tick", required = true) int tick,
                @JsonProperty(value = "train", required = true) String train,
                @JsonProperty(value = "origin", required = true) String origin,
                @JsonProperty(value = "destination", required = true) String destination) {
}
