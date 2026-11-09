/** Logs `message` with the error it is given, for a promise that nothing waits for. */
export function logFailure(message: string) {
  return (error: unknown) => console.error(message, error);
}
