package cp26;

import cp26.common.TestCase;
import org.junit.jupiter.api.Test;

/**
 * This is an example on how a simulation engine, here the
 * {@link cp26.simulation.slug.Slug}, can be verified.
 */
public class RocketTests {

    @Test
    public void testSimple() {
        TestCase.getPublic("simple").runRocket();
    }

    @Test
    public void testEuropeWithTraces() {
        TestCase.getPublic("europe_500").runRocket();
    }

    @Test
    public void testEurope() {
        TestCase.getPublic("europe").runRocket();
    }
}
