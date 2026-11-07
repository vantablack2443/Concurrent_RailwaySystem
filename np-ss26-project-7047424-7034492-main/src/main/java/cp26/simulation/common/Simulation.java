package cp26.simulation.common;

import cp26.output.Output;

/**
 * A common interface to be implemented by simulation engines (e.g.,
 * {@link cp26.simulation.slug.Slug}).
 */
public interface Simulation extends Runnable {

    Output getOutput();

}
