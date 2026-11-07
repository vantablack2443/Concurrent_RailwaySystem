package cp26;

import com.beust.jcommander.IStringConverter;
import com.beust.jcommander.JCommander;
import com.beust.jcommander.Parameter;
import com.beust.jcommander.ParameterException;

import cp26.scenario.Scenario;
import cp26.simulation.common.Simulation;
import cp26.simulation.rocket.Rocket;
import cp26.simulation.slug.Slug;
import cp26.simulation.starship.Starship;
import cp26.validator.DummyValidator;
import tools.jackson.databind.ObjectMapper;

import java.io.File;
import java.io.IOException;

/**
 * Entrypoint of the binary.
 */
public class Simulator {

    public static class FileConverter implements IStringConverter<File> {
        @Override
        public File convert(String value) {
            return new File(value);
        }
    }

    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Parameter(names = "--help", help = true)
    private boolean help = false;

    @Parameter(names = "--scenario", required = true, converter = FileConverter.class)
    private File scenarioFile;

    @Parameter(names = "--out", required = true, converter = FileConverter.class)
    private File outputFile;

    @Parameter(names = "--slug")
    private boolean slug = false;

    @Parameter(names = "--rocket")
    private boolean rocket = false;

    @Parameter(names = "--starship")
    private boolean starship = false;

    public static void main(String[] args) throws IOException {
        final Simulator simulator = new Simulator();
        try {
            final JCommander jc = JCommander.newBuilder().addObject(simulator).args(args).build();
            if (simulator.help) {
                jc.usage();
                return;
            }
        } catch (ParameterException e) {
            System.err.println(e.getMessage());
            System.exit(1);
        }

        simulator.run();
    }

    private void run() throws IOException {
        final Scenario scenario = parseScenario();
        final Simulation simulation = createSimulation(scenario);

        runSimulation(simulation);

        MAPPER.writeValue(this.outputFile, simulation.getOutput());
    }

    private void runSimulation(Simulation simulation) {
        final long start = System.nanoTime();
        simulation.run();
        final long end = System.nanoTime();

        System.out.println("Simulation time: " + (end - start) / 1_000_000 + " ms");
    }

    private Scenario parseScenario() throws IOException {
        final Scenario scenario = MAPPER.readValue(scenarioFile, Scenario.class);

        System.out.println("Parsed Scenario: " + scenario.name());
        return scenario;
    }

    private Simulation createSimulation(Scenario scenario) {
        if ((this.slug ? 1 : 0) + (this.rocket ? 1 : 0) + (this.starship ? 1 : 0) > 1) {
            System.err.println("Please decide for one of --slug, --rocket, or --starship");
            System.exit(1);
        }
        if (slug) {
            return new Slug(scenario);
        } else if (starship) {
            return new Starship(scenario);
        } else {
            return new Rocket(scenario, new DummyValidator());
        }
    }
}
