import { Buffer } from "node:buffer";

/** Apply host-owned UTF-8 ranges atomically; overlapping suggestions refuse. */
export function applyFixes(source, fixes) {
  const bytes = Buffer.from(source);
  const sorted = [...fixes].sort((left, right) => left.start - right.start || left.end - right.end);
  const parts = [];
  let cursor = 0;
  let previous;
  for (const fix of sorted) {
    if (
      !Number.isInteger(fix.start) ||
      !Number.isInteger(fix.end) ||
      fix.start < 0 ||
      fix.end < fix.start ||
      fix.end > bytes.length ||
      typeof fix.text !== "string" ||
      !boundary(bytes, fix.start) ||
      !boundary(bytes, fix.end)
    )
      throw new RangeError("plugin autofix must use valid UTF-8 source boundaries");
    if (
      previous &&
      fix.start === previous.start &&
      fix.end === previous.end &&
      fix.text === previous.text
    )
      continue;
    if (fix.start < cursor || (previous && fix.start === previous.start)) {
      throw new Error("overlapping plugin autofixes cannot be applied together");
    }
    parts.push(bytes.subarray(cursor, fix.start), Buffer.from(fix.text));
    cursor = fix.end;
    previous = fix;
  }
  parts.push(bytes.subarray(cursor));
  return Buffer.concat(parts).toString();
}

function boundary(bytes, offset) {
  return offset === bytes.length || (bytes[offset] & 0xc0) !== 0x80;
}
