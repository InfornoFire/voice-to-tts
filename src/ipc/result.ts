import type { UserFacingError } from "./bindings";

type CommandResult<T> = { status: "ok"; data: T } | { status: "error"; error: UserFacingError };

/** A failure reported by a backend command, carrying text meant for the user. */
export class CommandError extends Error {
  readonly detail: UserFacingError;

  constructor(detail: UserFacingError) {
    super(detail.message);
    this.name = "CommandError";
    this.detail = detail;
  }
}

/** Resolves with the data of a successful command, or throws a {@link CommandError}. */
export async function unwrap<T>(result: Promise<CommandResult<T>>): Promise<T> {
  const settled = await result;
  if (settled.status === "error") {
    throw new CommandError(settled.error);
  }
  return settled.data;
}
