import type { InputMode } from "../../presets";
import type {
  CompilerOptions,
  CompileResult,
  SfcCompileResult,
  WasmModule,
} from "../../wasm/index";
import { formatCode, transpileToJs } from "./formatters";

export const CODE_OUTPUT_TARGETS = ["dom", "ssr", "vapor", "vapor-ssr"] as const;

export type CodeOutputTarget = (typeof CODE_OUTPUT_TARGETS)[number];

export interface CodeOutputVariant {
  code: string;
  formattedCode: string;
  formattedJsCode: string;
  isTypeScript: boolean;
  templates: string[];
  error: string | null;
}

export type CodeOutputs = Record<CodeOutputTarget, CodeOutputVariant>;

export const CODE_OUTPUT_LABELS: Record<CodeOutputTarget, string> = {
  dom: "DOM",
  ssr: "SSR",
  vapor: "Vapor",
  "vapor-ssr": "Vapor SSR",
};

function createEmptyCodeOutputVariant(): CodeOutputVariant {
  return {
    code: "",
    formattedCode: "",
    formattedJsCode: "",
    isTypeScript: false,
    templates: [],
    error: null,
  };
}

export function createEmptyCodeOutputs(): CodeOutputs {
  return {
    dom: createEmptyCodeOutputVariant(),
    ssr: createEmptyCodeOutputVariant(),
    vapor: createEmptyCodeOutputVariant(),
    "vapor-ssr": createEmptyCodeOutputVariant(),
  };
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

async function formatVariantCode(code: string, isTypeScript: boolean) {
  if (!code) {
    return {
      formattedCode: "",
      formattedJsCode: "",
    };
  }

  const formattedCode = await formatCode(code, isTypeScript ? "typescript" : "babel");
  const formattedJsCode = isTypeScript ? await formatCode(transpileToJs(code), "babel") : "";

  return {
    formattedCode,
    formattedJsCode,
  };
}

function isSfcTypeScript(result: SfcCompileResult): boolean {
  const scriptSetupLang = result.descriptor.scriptSetup?.lang;
  const scriptLang = result.descriptor.script?.lang;
  return (
    scriptSetupLang === "ts" ||
    scriptSetupLang === "tsx" ||
    scriptLang === "ts" ||
    scriptLang === "tsx"
  );
}

function getSfcOutputCode(result: SfcCompileResult): string {
  return result.script?.code || result.template?.code || "";
}

async function buildTemplateVariant(result: CompileResult): Promise<CodeOutputVariant> {
  const code = result.code || "";
  const { formattedCode, formattedJsCode } = await formatVariantCode(code, false);
  return {
    code,
    formattedCode,
    formattedJsCode,
    isTypeScript: false,
    templates: result.templates || [],
    error: null,
  };
}

async function buildSfcVariant(result: SfcCompileResult): Promise<CodeOutputVariant> {
  const code = getSfcOutputCode(result);
  const isTypeScript = isSfcTypeScript(result);
  const { formattedCode, formattedJsCode } = await formatVariantCode(code, isTypeScript);
  return {
    code,
    formattedCode,
    formattedJsCode,
    isTypeScript,
    templates: result.template?.templates || [],
    error: null,
  };
}

async function compileTemplateVariant(
  compiler: WasmModule,
  source: string,
  options: CompilerOptions,
  target: Exclude<CodeOutputTarget, "dom">,
): Promise<CodeOutputVariant> {
  try {
    const ssr = target === "ssr" || target === "vapor-ssr";
    const vapor = target === "vapor" || target === "vapor-ssr";
    const result = vapor
      ? compiler.compileVapor(source, { ...options, ssr })
      : compiler.compile(source, { ...options, ssr });
    return await buildTemplateVariant(result);
  } catch (error) {
    return {
      ...createEmptyCodeOutputVariant(),
      error: getErrorMessage(error),
    };
  }
}

async function compileSfcVariant(
  compiler: WasmModule,
  source: string,
  options: CompilerOptions,
  target: Exclude<CodeOutputTarget, "dom">,
): Promise<CodeOutputVariant> {
  try {
    const ssr = target === "ssr" || target === "vapor-ssr";
    const outputMode = target === "ssr" ? "vdom" : "vapor";
    const result = compiler.compileSfc(source, {
      ...options,
      ssr,
      outputMode,
    });
    return await buildSfcVariant(result);
  } catch (error) {
    return {
      ...createEmptyCodeOutputVariant(),
      error: getErrorMessage(error),
    };
  }
}

interface CompileCodeOutputsParams {
  compiler: WasmModule;
  inputMode: InputMode;
  source: string;
  options: CompilerOptions;
  baseOutput: CompileResult | null;
  baseSfcResult: SfcCompileResult | null;
}

export async function compileCodeOutputs({
  compiler,
  inputMode,
  source,
  options,
  baseOutput,
  baseSfcResult,
}: CompileCodeOutputsParams): Promise<CodeOutputs> {
  const outputs = createEmptyCodeOutputs();

  if (inputMode === "sfc") {
    if (baseSfcResult) {
      outputs.dom = await buildSfcVariant(baseSfcResult);
    }

    const [ssr, vapor, vaporSsr] = await Promise.all([
      compileSfcVariant(compiler, source, options, "ssr"),
      compileSfcVariant(compiler, source, options, "vapor"),
      compileSfcVariant(compiler, source, options, "vapor-ssr"),
    ]);

    outputs.ssr = ssr;
    outputs.vapor = vapor;
    outputs["vapor-ssr"] = vaporSsr;
    return outputs;
  }

  if (baseOutput) {
    outputs.dom = await buildTemplateVariant(baseOutput);
  }

  const [ssr, vapor, vaporSsr] = await Promise.all([
    compileTemplateVariant(compiler, source, options, "ssr"),
    compileTemplateVariant(compiler, source, options, "vapor"),
    compileTemplateVariant(compiler, source, options, "vapor-ssr"),
  ]);

  outputs.ssr = ssr;
  outputs.vapor = vapor;
  outputs["vapor-ssr"] = vaporSsr;
  return outputs;
}
