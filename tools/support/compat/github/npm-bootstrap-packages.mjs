// Explicit approvals, consumed by both the runtime and its JS contract oracle.
import inventory from "../../../config/release/npm-bootstrap-packages.json" with { type: "json" };

export const bootstrapPackages = new Map(inventory.packages.map(({ path, name }) => [path, name]));
export const bootstrapArtifacts = new Map(
  inventory.packages.map(({ path, artifact }) => [path, artifact]),
);
export const requiredSuccessfulReleaseJobs = inventory.successfulJobs;
const legacy = inventory.packages.find(({ path }) => path === "npm/framework/nuxt-lint-config");
export const requiredFailedReleaseJobs = legacy.failedJobs;
export const requiredSkippedReleaseJobs = legacy.skippedJobs;

export function bootstrapJobContract(path = "npm/framework/nuxt-lint-config") {
  const found = inventory.packages.find((package_) => package_.path === path);
  if (!found)
    throw new Error(`Package path is not approved for npm bootstrap: ${path || "(empty)"}`);
  return found;
}

export function bootstrapArtifactContract(artifact) {
  const found = inventory.packages.find((package_) => package_.artifact === artifact);
  if (!found) throw new Error(`Artifact is not approved for npm bootstrap: ${artifact}`);
  return found;
}
