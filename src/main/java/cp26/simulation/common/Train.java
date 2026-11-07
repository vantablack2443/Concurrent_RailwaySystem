package cp26.simulation.common;

/**
 * This record represents a simulated train.
 *
 * @param name        The name of the train connection.
 * @param origin      The ID of the departure station.
 * @param destination The ID of the arrival station.
 * @param startOn     The trains starting tick.
 */
public record Train(
        String name,
        int origin,
        int destination,
        int startOn) {
}
