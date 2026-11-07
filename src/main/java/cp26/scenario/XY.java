package cp26.scenario;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Represents a two-dimensional vector with an <em>x</em>- and a
 * <em>y</em>-component.
 *
 * @param x The <em>x</em>-component.
 * @param y The <em>y</em>-component.
 */
public record XY(
        @JsonProperty(value = "x") int x,
        @JsonProperty(value = "y") int y) {
}
