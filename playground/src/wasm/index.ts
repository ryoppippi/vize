// WASM module loader for vize
// Re-exports all types and provides the WASM loading API

export type {
  CompilerOptions,
  CompileResult,
  SfcBlock,
  SfcScriptBlock,
  SfcStyleBlock,
  SfcDescriptor,
  SfcCompileResult,
  CssCompileOptions,
  CssCompileResult,
  ArtParseOptions,
  ArtMetadata,
  ArtVariant,
  ArtStyleBlock,
  ArtDescriptor,
  CsfOutput,
  LintOptions,
  LocaleInfo,
  LintDiagnostic,
  LintResult,
  LintRule,
  FormatOptions,
  FormatResult,
  CroquisOptions,
  BindingSource,
  BindingMetadata,
  BindingDisplay,
  ScopeKind,
  ScopeDisplay,
  MacroDisplay,
  TypeExportDisplay,
  InvalidExportDisplay,
  PropDisplay,
  EmitDisplay,
  ProvideKey,
  ProvideDisplay,
  InjectPattern,
  InjectDisplay,
  CssDisplay,
  CroquisStats,
  CroquisDiagnostic,
  Croquis,
  CroquisResult,
  TypeCheckOptions,
  TypeCheckRelatedLocation,
  TypeCheckDiagnostic,
  TypeCheckResult,
  TypeCheckCapability,
  TypeCheckCapabilities,
  CrossFileOptions,
  CrossFileDiagnostic,
  CrossFileStats,
  CrossFileResult,
  CrossFileInput,
  WasmModule,
} from './types';

import type { WasmModule } from './types';
import { createMockModule } from './mock-module';
import { createTransformAnalyzeSfc } from './wasm-transform';

let wasmModule: WasmModule | null = null;
let loadPromise: Promise<WasmModule> | null = null;
let usingMock = false;

export async function loadWasm(): Promise<WasmModule> {
  if (wasmModule) {
    return wasmModule;
  }

  if (loadPromise) {
    return loadPromise;
  }

  loadPromise = (async () => {
    try {
      // Try to load the actual WASM module
      // For --target web, we need to explicitly call init() before using exports
      const wasm = await import('./vize_vitrine.js');

      // Initialize WASM - required for --target web
      if (wasm.default) {
        await wasm.default();
      }

      // Get mock module to fill in any missing functions
      const mock = createMockModule();

      // Wrapper to transform WASM analyzeSfc output to expected TypeScript format
      const transformAnalyzeSfc = createTransformAnalyzeSfc(
        wasm.analyzeSfc,
        mock.analyzeSfc,
      );

      // Merge WASM module with mock fallbacks for missing functions
      const module: WasmModule = {
        compile: wasm.compile || mock.compile,
        compileVapor: wasm.compileVapor || mock.compileVapor,
        compileCss: wasm.compileCss || mock.compileCss,
        parseTemplate: wasm.parseTemplate || mock.parseTemplate,
        parseSfc: wasm.parseSfc || mock.parseSfc,
        compileSfc: wasm.compileSfc || mock.compileSfc,
        // Analysis functions - use WASM croquis analyzer
        // Note: Scope spans from WASM may be 0 (not tracked during analyze_script yet)
        // but macros, props, emits, bindings are properly extracted
        analyzeSfc: transformAnalyzeSfc,
        // Cross-file analysis - use Rust CrossFileAnalyzer
        analyzeCrossFile: wasm.analyzeCrossFile || mock.analyzeCrossFile,
        // Musea functions
        parseArt: wasm.parseArt || mock.parseArt,
        artToCsf: wasm.artToCsf || mock.artToCsf,
        // Patina (Linter) functions - may not be in WASM yet
        lintTemplate: wasm.lintTemplate || mock.lintTemplate,
        lintSfc: wasm.lintSfc || mock.lintSfc,
        getLintRules: wasm.getLintRules || mock.getLintRules,
        getLocales: wasm.getLocales || mock.getLocales,
        // Glyph (Formatter) functions - may not be in WASM yet
        formatSfc: wasm.formatSfc || mock.formatSfc,
        formatTemplate: wasm.formatTemplate || mock.formatTemplate,
        formatScript: wasm.formatScript || mock.formatScript,
        // Canon (TypeCheck) functions
        typeCheck: wasm.typeCheck || mock.typeCheck,
        getTypeCheckCapabilities: wasm.getTypeCheckCapabilities || mock.getTypeCheckCapabilities,
        Compiler: (wasm.Compiler || mock.Compiler) as unknown as WasmModule['Compiler'],
      };
      wasmModule = module;
      usingMock = false;
      return module;
    } catch (e) {
      console.warn('WASM module not found, using mock compiler:', e);
      // Return mock module if WASM is not available
      const mockModule = createMockModule();
      wasmModule = mockModule;
      usingMock = true;
      return mockModule;
    }
  })();

  return loadPromise!;
}

export function isWasmLoaded(): boolean {
  return wasmModule !== null && !usingMock;
}

export function isUsingMock(): boolean {
  return usingMock;
}

// Get the current WASM module (if loaded)
export function getWasm(): WasmModule | null {
  return wasmModule;
}
