package cp26.output;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Represents how many trains are in a zone at a specific point in time.
 *
 * @param tick      Tick of the entry.
 * @param traveling Number of trains that are traveling inside a zone.
 */
public record StatisticsEntry(
                @JsonProperty(value = "tick", required = true) int tick,
                @JsonProperty(value = "traveling", required = true) int traveling) {
}