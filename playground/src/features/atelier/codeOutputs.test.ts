import { describe, expect, it, vi } from "vitest";
import type {
  CompileResult,
  CompilerOptions,
  SfcCompileResult,
  WasmModule,
} from "../../wasm/index";
import { compileCodeOutputs } from "./codeOutputs";

vi.mock("./formatters", () => ({
  formatCode: vi.fn(async (code: string, parser: string) => `[${parser}] ${code}`),
  transpileToJs: vi.fn((code: string) => `js:${code}`),
}));

function createCompileResult(code: string, templates?: string[]): CompileResult {
  return {
    code,
    preamble: "",
    ast: {},
    helpers: [],
    templates,
  };
}

function createSfcResult(
  code: string,
  {
    lang,
    templates,
  }: {
    lang?: string;
    templates?: string[];
  } = {},
): SfcCompileResult {
  return {
    descriptor: {
      filename: "example.vue",
      source: "",
      styles: [],
      customBlocks: [],
      scriptSetup: lang
        ? {
            content: "",
            loc: { start: 0, end: 0 },
            attrs: {},
            lang,
            setup: true,
          }
        : undefined,
    },
    script: {
      code,
    },
    template: createCompileResult(`${code}:template`, templates),
  };
}

describe("compileCodeOutputs", () => {
  it("compiles template outputs for SSR and Vapor variants", async () => {
    const compile = vi.fn((_: string, options: CompilerOptions) =>
      createCompileResult(options.ssr ? "ssr-code" : "dom-code"),
    );
    const compileVapor = vi.fn((_: string, options: CompilerOptions) =>
      createCompileResult(options.ssr ? "vapor-ssr-code" : "vapor-code", ["tpl-a"]),
    );
    const compiler = {
      compile,
      compileVapor,
    } as unknown as WasmModule;

    const outputs = await compileCodeOutputs({
      compiler,
      inputMode: "template",
      source: "<div />",
      options: { mode: "module" },
      baseOutput: createCompileResult("dom-code"),
      baseSfcResult: null,
    });

    expect(outputs.dom.code).toBe("dom-code");
    expect(outputs.ssr.code).toBe("ssr-code");
    expect(outputs.vapor.code).toBe("vapor-code");
    expect(outputs["vapor-ssr"].code).toBe("vapor-ssr-code");
    expect(outputs.vapor.templates).toEqual(["tpl-a"]);
    expect(compile).toHaveBeenNthCalledWith(1, "<div />", { mode: "module", ssr: true });
    expect(compileVapor).toHaveBeenNthCalledWith(1, "<div />", {
      mode: "module",
      ssr: false,
    });
    expect(compileVapor).toHaveBeenNthCalledWith(2, "<div />", {
      mode: "module",
      ssr: true,
    });
  });

  it("compiles SFC outputs with the expected mode combinations", async () => {
    const compileSfc = vi.fn((_: string, options: CompilerOptions) => {
      if (options.outputMode === "vapor" && options.ssr) {
        return createSfcResult("vapor-ssr", { lang: "ts", templates: ["tpl-vapor-ssr"] });
      }
      if (options.outputMode === "vapor") {
        return createSfcResult("vapor", { lang: "ts", templates: ["tpl-vapor"] });
      }
      if (options.ssr) {
        return createSfcResult("ssr", { lang: "ts" });
      }
      return createSfcResult("dom", { lang: "ts" });
    });
    const compiler = {
      compileSfc,
    } as unknown as WasmModule;

    const outputs = await compileCodeOutputs({
      compiler,
      inputMode: "sfc",
      source: "<template><div /></template>",
      options: { mode: "module", scriptExt: "preserve" },
      baseOutput: null,
      baseSfcResult: createSfcResult("dom", { lang: "ts" }),
    });

    expect(outputs.dom.code).toBe("dom");
    expect(outputs.dom.isTypeScript).toBe(true);
    expect(outputs.dom.formattedJsCode).toBe("[babel] js:dom");
    expect(outputs.ssr.code).toBe("ssr");
    expect(outputs.vapor.templates).toEqual(["tpl-vapor"]);
    expect(outputs["vapor-ssr"].templates).toEqual(["tpl-vapor-ssr"]);
    expect(compileSfc).toHaveBeenNthCalledWith(1, "<template><div /></template>", {
      mode: "module",
      scriptExt: "preserve",
      ssr: true,
      outputMode: "vdom",
    });
    expect(compileSfc).toHaveBeenNthCalledWith(2, "<template><div /></template>", {
      mode: "module",
      scriptExt: "preserve",
      ssr: false,
      outputMode: "vapor",
    });
    expect(compileSfc).toHaveBeenNthCalledWith(3, "<template><div /></template>", {
      mode: "module",
      scriptExt: "preserve",
      ssr: true,
      outputMode: "vapor",
    });
  });
});
