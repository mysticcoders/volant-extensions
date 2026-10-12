/**
 * The Volant ABI 2 entry point: reads the request from stdin, runs the handler and writes the reply to
 * stdout through Javy's stream I/O. Input and output are each capped at 64 KiB by Volant.
 */
import { handle } from "./kit.ts";
import { handler } from "./npm.ts";

declare const Javy: { IO: { readSync(fd: number, data: Uint8Array): number; writeSync(fd: number, data: Uint8Array): number } };

const STDIN = 0;
const STDOUT = 1;
const LIMIT = 65_536;

/** Reads all of stdin, up to Volant's 64 KiB input limit. */
function readInput(): string {
  const buffer = new Uint8Array(LIMIT);
  let used = 0;
  while (used < buffer.length) {
    const read = Javy.IO.readSync(STDIN, buffer.subarray(used));
    if (!read) break;
    used += read;
  }
  return new TextDecoder().decode(buffer.subarray(0, used));
}

/** Writes all of a string to stdout. */
function writeOutput(text: string): void {
  const bytes = new TextEncoder().encode(text);
  for (let offset = 0; offset < bytes.length;) {
    const written = Javy.IO.writeSync(STDOUT, bytes.subarray(offset));
    if (!written) return;
    offset += written;
  }
}

writeOutput(handle(readInput(), handler));
