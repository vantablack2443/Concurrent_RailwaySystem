package cp26.output;

import com.fasterxml.jackson.annotation.JsonProperty;

import cp26.scenario.Scenario;

import java.util.List;
import java.util.Map;

/**
 * The output to be computed by the simulator.
 * <br>
 * Contains the original scenario, a trace of train movements (if enabled) and
 * per-zone statistics over time.
 * 
 * @param scenario The simulation scenario.
 * @param trace    The trace.
 * @param stats    The collected statistics. <br>
 *                 Make sure that the inner vectors are strictly sorted by tick.
 * 
 */
public record Output(
        @JsonProperty(value = "scenario", required = true) Scenario scenario,
        @JsonProperty(value = "trace", required = true) List<TraceEntry> trace,
        @JsonProperty(value = "stats", required = true) Map<String, List<StatisticsEntry>> stats) {
}
