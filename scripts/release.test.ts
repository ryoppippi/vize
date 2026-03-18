import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { test } from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "..");

function runBash(script: string) {
  return spawnSync("bash", ["-lc", script], {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["pipe", "pipe", "pipe"],
  });
}

test("release script fails clearly when stdin is not interactive", () => {
  const result = runBash("bash ./scripts/release.sh minor");

  assert.deepEqual(
    {
      status: result.status,
      stdout: result.stdout,
      stderr: result.stderr,
    },
    {
      status: 1,
      stdout: "Current version: 0.33.0\nNew version: 0.34.0 (tag: v0.34.0)\n\n",
      stderr: "Error: Confirmation requires an interactive terminal. Re-run with -y to skip the prompt.\n",
    },
  );
});

test("confirm_release skips prompting when -y is set", () => {
  const result = runBash("source scripts/release.sh; AUTO_CONFIRM=-y; confirm_release; printf 'confirmed\\n'");

  assert.deepEqual(
    {
      status: result.status,
      stdout: result.stdout,
      stderr: result.stderr,
    },
    {
      status: 0,
      stdout: "confirmed\n",
      stderr: "",
    },
  );
});
