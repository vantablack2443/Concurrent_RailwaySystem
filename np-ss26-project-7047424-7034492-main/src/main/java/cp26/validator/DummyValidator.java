package cp26.validator;

/**
 * A dummy validator that does nothing.
 */
public class DummyValidator implements Validator {

    @Override
    public void process(int tick, int zoneId, int trainId) {
        // do nothing
    }

}
