import assert from "node:assert/strict";
import { test } from "node:test";
import { buildMisskeyDevConfig, runMisskeyBeforeStart } from "./dev-app.ts";

test("buildMisskeyDevConfig uses a valid id generation method", () => {
  const config = buildMisskeyDevConfig(
    3001,
    "/Users/ubugeeei/projects/personal/oss/ubugeeei/vize/__agent_only/vize-dev.pid",
  );

  assert.match(config, /^id: aidx$/m);
  assert.match(config, /^port: 3001$/m);
  assert.match(
    config,
    /^pidFile: \/Users\/ubugeeei\/projects\/personal\/oss\/ubugeeei\/vize\/__agent_only\/vize-dev\.pid$/m,
  );
});

test("runMisskeyBeforeStart builds backend before readiness check", () => {
  const calls: string[] = [];

  runMisskeyBeforeStart("/misskey", "vize-dev.yml", {
    startLocalServices(misskeyRoot) {
      calls.push(`start:${misskeyRoot}`);
    },
    ensureBackendBuilt(misskeyRoot, configName) {
      calls.push(`build:${misskeyRoot}:${configName}`);
    },
    ensureNativeDependencies(misskeyRoot) {
      calls.push(`native:${misskeyRoot}`);
    },
    waitForLocalServices(misskeyRoot, configName) {
      calls.push(`ready:${misskeyRoot}:${configName}`);
    },
    ensureMigrated(misskeyRoot, configName) {
      calls.push(`migrate:${misskeyRoot}:${configName}`);
    },
  });

  assert.deepEqual(calls, [
    "start:/misskey",
    "build:/misskey:vize-dev.yml",
    "native:/misskey",
    "ready:/misskey:vize-dev.yml",
    "migrate:/misskey:vize-dev.yml",
  ]);
});
