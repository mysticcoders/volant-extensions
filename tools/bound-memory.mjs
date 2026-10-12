// Add an explicit maximum to the compiler's single wasm32 memory, before hashing.
export function boundMemory(bytes, maximum = 256) {
  if (!WebAssembly.validate(bytes)) throw new Error("Invalid compiled module");
  const input = new Uint8Array(bytes); let position = 8, found = false;
  function number(end) {
    let value = 0, shift = 0;
    while (position < end && shift <= 28) {
      const byte = input[position++];
      if (shift === 28 && byte > 15) throw new Error("Invalid integer");
      value += (byte & 127) * 2 ** shift;
      if (byte < 128) return value;
      shift += 7;
    }
    throw new Error("Truncated integer");
  }
  function leb(value) {
    const result = [];
    do { const byte = value % 128; value = Math.floor(value / 128); result.push(byte | (value ? 128 : 0)); } while (value);
    return result;
  }
  const chunks = [input.slice(0, 8)];
  while (position < input.length) {
    const start = position, type = input[position++], length = number(input.length), end = position + length;
    if (end > input.length) throw new Error("Truncated section");
    if (type === 5) {
      if (found || number(end) !== 1) throw new Error("Expected one memory");
      found = true;
      const flags = number(end), initial = number(end);
      if (flags !== 0 && flags !== 1) throw new Error("Expected unshared wasm32 memory");
      const limit = flags === 1 ? Math.min(number(end), maximum) : maximum;
      if (initial > limit || position !== end) throw new Error("Module exceeds memory budget");
      const body = [1, 1, ...leb(initial), ...leb(limit)];
      chunks.push(Uint8Array.from([5, ...leb(body.length), ...body]));
    } else chunks.push(input.slice(start, end));
    position = end;
  }
  if (!found) throw new Error("Missing memory");
  const result = Buffer.concat(chunks);
  if (!WebAssembly.validate(result)) throw new Error("Invalid bounded module");
  return result;
}
