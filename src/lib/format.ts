const BYTES_PER_MEGABYTE = 1024 * 1024;

/** A size in bytes as megabytes, with a decimal only where it matters. */
export function formatSize(bytes: number): string {
  const megabytes = bytes / BYTES_PER_MEGABYTE;
  return `${megabytes.toFixed(megabytes < 10 ? 1 : 0)} MB`;
}
