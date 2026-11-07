package cp26.scenario;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Container for the simulation parameters.
 */
public class Parameters {

    private static final double DEFAULT_PENALTY = 1.0;

    @JsonProperty("penalty")
    private final Double penalty;

    /**
     * Constructs an {@link Parameters} object containing the simulation parameters.
     *
     * @param penalty Track penalty to use on an international track
     */
    @JsonCreator
    public Parameters(
            @JsonProperty(value = "penalty") Double penalty) {
        this.penalty = penalty != null ? penalty : DEFAULT_PENALTY;
    }

    /**
     * Returns the track penalty to use on an international track.
     *
     * @return The track penalty to use on an international track.
     */
    public double getPenalty() {
        return this.penalty;
    }
}
