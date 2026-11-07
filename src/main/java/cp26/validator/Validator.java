package cp26.validator;

/**
 * An interface for automated testing.
 *
 * <p>
 * We use this interface to gain insights into your program and automatically
 * validate some requirements of the project. You must not wrap the validator in
 * a monitor or use some other kind of synchronization. We guarantee that it is
 * safe to concurrently call every method of classes implementing this
 * interface. However, you need to ensure that the thread calling a validator is
 * not interrupted while the validator runs.
 * </p>
 */
public interface Validator {

    /**
     * Call this method before processing the next event in the priority queue.
     *
     * @param tick    The tick on which the even is occurring.
     * @param zoneId  The zone ID related to the event.
     * @param trainId The train ID related to the event
     */
    public void process(int tick, int zoneId, int trainId);
}
