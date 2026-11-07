package cp26.common;

import cp26.output.Output;
import cp26.scenario.Scenario;
import cp26.simulation.rocket.Rocket;
import cp26.simulation.slug.Slug;
import cp26.validator.DummyValidator;
import cp26.validator.Validator;
import tools.jackson.databind.ObjectMapper;

import java.io.BufferedReader;
import java.nio.file.Files;
import java.nio.file.Path;

import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Utility class, which allows to run tests against the {@link Slug} and
 * {@link Rocket} simulation engine.
 */
public class TestCase {

    /**
     * An object mapper using the input stream to map <code>json</code> files to
     * either {@link Scenario} or
     * {@link Output} objects.
     */
    private static final ObjectMapper objectMapper = new ObjectMapper();

    /**
     * The parsed scenario file.
     */
    private final Scenario scenario;

    /**
     * The parsed expected output file. (Note: marked by
     * <code>[name].result.json</code>)
     */
    private final Output expectedOutput;

    /**
     * Constructs a {@link TestCase}.
     *
     * @param name     The test's name, e.g., <code>simple</code> for
     *                 <code>simple.json</code>.
     * @param category The test's directory under <code>scenarios/</code>,
     *                 e.g., <code>public</code>
     */
    private TestCase(String name, String category) {
        try (final BufferedReader scenarioReader = Files
                .newBufferedReader(Path.of("scenarios", category, name + ".json"));
                final BufferedReader outputReader = Files
                        .newBufferedReader(Path.of("scenarios", category, name + ".result.json"))) {

            this.scenario = objectMapper.readValue(scenarioReader, Scenario.class);
            this.expectedOutput = objectMapper.readValue(outputReader, Output.class);
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    }

    /**
     * Constructs and returns a <b>public</b> test case, based on the provided name
     * (see {@link #TestCase(String, String)}).
     * <br>
     * Make sure that for the specified test a respective
     * <code>[name].result.json</code> exists. As this file is parsed
     * into the {@link #expectedOutput}.
     *
     * @param name The test's name.
     *
     * @return A <b>public</b> test case.
     */
    public static TestCase getPublic(String name) {
        return new TestCase(name, "public");
    }

    /**
     * Returns the {@link Scenario} object corresponding to the test case.
     * <br>
     * <i>Useful for inspection.</i>
     *
     * @return A {@link Scenario} object.
     */
    public Scenario getScenario() {
        return this.scenario;
    }

    /**
     * Returns the expected {@link Output} object.
     *
     * @return The expected {@link Output} object.
     */
    public Output getExpectedOutput() {
        return this.expectedOutput;
    }

    /**
     * This method runs the {@link Slug} simulation engine on the {@link #scenario}
     * and compares it against
     * the {@link #expectedOutput} using the {@link Checker}.
     * <br>
     * It is asserted that the checker's problem list is empty, otherwise the test
     * fails.
     *
     * @return The corresponding {@link Checker} object, may be useful for further
     *         inspection.
     */
    public Checker runSlug() {
        final Slug slug = new Slug(this.scenario);

        final long start = System.nanoTime();
        slug.run();
        final long end = System.nanoTime();

        System.out.println("Slug execution time: " + (end - start) / 1_000_000 + " ms");

        final Output output = slug.getOutput();
        final Checker checker = new Checker();
        checker.check(output, this.expectedOutput);

        for (String problem : checker.getProblems())
            System.err.println(problem);

        assertTrue(checker.getProblems().isEmpty(), "invalid output");
        return checker;
    }

    /**
     * This method runs the {@link Rocket} simulation engine on the
     * {@link #scenario} and compares it against
     * the {@link #expectedOutput} using the {@link Checker}.
     * <br>
     * It is asserted that the checker's problem list is empty, otherwise the test
     * fails.
     *
     * @param validator An object implementing the {@link Validator} interface,
     *                  which allows for further analysis.
     *
     * @return The corresponding {@link Checker} object, may be useful for further
     *         inspection.
     */
    public Checker runRocket(Validator validator) {
        final Rocket rocket = new Rocket(this.scenario, validator);

        final long start = System.nanoTime();
        rocket.run();
        final long end = System.nanoTime();

        System.out.println("Rocket execution time: " + (end - start) / 1_000_000 + " ms");

        final Output output = rocket.getOutput();
        final Checker checker = new Checker();
        checker.check(output, this.expectedOutput);

        for (String problem : checker.getProblems())
            System.err.println(problem);

        assertTrue(checker.getProblems().isEmpty(), "invalid output");
        return checker;
    }

    /**
     * This method runs the {@link Rocket} simulation engine on the
     * {@link #scenario} and compares it against
     * the {@link #expectedOutput} using the {@link Checker}, by calling
     * {@link #runRocket(Validator)} with a
     * {@link DummyValidator} object.
     * <br>
     * It is asserted that the checker's problem list is empty, otherwise the test
     * fails.
     *
     *
     * @return The corresponding {@link Checker} object, may be useful for further
     *         inspection.
     */
    public Checker runRocket() {
        return this.runRocket(new DummyValidator());
    }
}
