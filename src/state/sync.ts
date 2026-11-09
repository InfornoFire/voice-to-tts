/**
 * Keeps state in step with the backend, which reports it through events and a snapshot.
 *
 * Listening comes first so no change is missed, and the snapshot never overrides an event.
 */
export async function syncWithBackend<T>(
  listen: (handler: (event: { payload: T }) => void) => Promise<unknown>,
  snapshot: () => Promise<T>,
  apply: (value: T) => void,
) {
  let receivedEvent = false;
  await listen((event) => {
    receivedEvent = true;
    apply(event.payload);
  });

  const current = await snapshot();
  if (!receivedEvent) {
    apply(current);
  }
}
