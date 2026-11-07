package cp26.simulation.rocket;

import cp26.scenario.Scenario;
import cp26.output.Output;
import cp26.simulation.common.Simulation;
import cp26.validator.Validator;

/**
 * Your implementation shall go into this class.
 *
 * <p>
 * This class has to implement the <em>Simulation</em> interface.
 * </p>
 */
public class Rocket implements Simulation {
    /**
     * Constructs a rocket with the given parameters.
     *
     * <p>
     * You must not change the signature of this constructor.
     * </p>
     *
     * @param scenario  The scenario to simulate.
     * @param validator The validator to be called.
     */
    public Rocket(Scenario scenario, Validator validator) {
        // your concurrent implementation goes here
    }

    @Override
    public Output getOutput() {
        throw new RuntimeException("not implemented");
    }

    @Override
    public void run() {
        throw new RuntimeException("not implemented");
    }
}
