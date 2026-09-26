// Cache identity includes the authoring runtime itself, including dirty edits.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const hash = createHash("sha256");
for (const file of ["index.js", "stages.js", "identity.js", "package.json"]) {
  const bytes = readFileSync(new URL(file, import.meta.url));
  hash.update(file).update(String(bytes.length)).update(bytes);
}
export const SDK_FINGERPRINT = hash.digest("hex");
