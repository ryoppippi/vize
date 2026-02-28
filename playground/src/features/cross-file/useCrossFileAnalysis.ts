import type {
  CroquisResult,
  CrossFileResult,
  CrossFileInput,
  CrossFileOptions as WasmCrossFileOptions,
} from "../../wasm/index";
import { getWasm } from "../../wasm/index";
import type { CrossFileIssue, AnalyzerContext } from "./types";
import type { Ref, ComputedRef } from "vue";
import {
  resetIssueIdCounter,
  offsetToLineColumn,
  buildSuppressionMap,
  filterSuppressedIssues,
} from "./utils";
import { analyzeProvideInject } from "./analyzers/analyzeProvideInject";
import { analyzeComponentEmits } from "./analyzers/analyzeComponentEmits";
import { analyzeFallthroughAttrs } from "./analyzers/analyzeFallthroughAttrs";
import { analyzeReactivity } from "./analyzers/analyzeReactivity";
import { analyzeUniqueIds, analyzeSSRBoundary } from "./analyzers/analyzeOther";

interface UseCrossFileAnalysisOptions {
  files: Ref<Record<string, string>>;
  croquisResults: Ref<Record<string, CroquisResult | null>>;
  crossFileIssues: Ref<CrossFileIssue[]>;
  analysisTime: Ref<number>;
  isAnalyzing: Ref<boolean>;
  dependencyGraph: ComputedRef<Record<string, string[]>>;
  options: Ref<{
    provideInject: boolean;
    componentEmits: boolean;
    fallthroughAttrs: boolean;
    reactivityTracking: boolean;
    uniqueIds: boolean;
    serverClientBoundary: boolean;
  }>;
}

export function useCrossFileAnalysis({
  files,
  croquisResults,
  crossFileIssues,
  analysisTime,
  isAnalyzing,
  dependencyGraph,
  options,
}: UseCrossFileAnalysisOptions) {
  let issueIdCounter = 0;

  function buildContext(): AnalyzerContext {
    return {
      files: files.value,
      croquisResults: croquisResults.value,
      dependencyGraph: dependencyGraph.value,
    };
  }

  async function analyzeAll() {
    const compiler = getWasm();
    if (!compiler) return;

    isAnalyzing.value = true;
    const startTime = performance.now();
    resetIssueIdCounter();
    issueIdCounter = 0;

    const results: Record<string, CroquisResult | null> = {};
    for (const [filename, source] of Object.entries(files.value)) {
      try {
        results[filename] = compiler.analyzeSfc(source, { filename });
      } catch {
        results[filename] = null;
      }
    }
    croquisResults.value = results;

    const crossFileInputs: CrossFileInput[] = Object.entries(files.value).map(
      ([path, source]) => ({
        path,
        source,
      }),
    );

    const wasmOptions: WasmCrossFileOptions = {
      all: true,
      provideInject: options.value.provideInject,
      componentEmits: options.value.componentEmits,
      fallthroughAttrs: options.value.fallthroughAttrs,
      reactivityTracking: options.value.reactivityTracking,
      uniqueIds: options.value.uniqueIds,
      serverClientBoundary: options.value.serverClientBoundary,
    };

    let issues: CrossFileIssue[] = [];

    try {
      if (compiler.analyzeCrossFile) {
        const crossFileResult: CrossFileResult = compiler.analyzeCrossFile(
          crossFileInputs,
          wasmOptions,
        );

        for (const diag of crossFileResult.diagnostics) {
          const source = files.value[diag.file] || "";
          const loc = offsetToLineColumn(source, diag.offset);
          const endLoc = offsetToLineColumn(source, diag.endOffset);

          issues.push({
            id: `issue-${++issueIdCounter}`,
            type: diag.type,
            code: diag.code,
            severity: diag.severity === "hint" ? "info" : diag.severity,
            message: diag.message,
            file: diag.file,
            line: loc.line,
            column: loc.column,
            endLine: endLoc.line,
            endColumn: endLoc.column,
            relatedLocations: diag.relatedLocations?.map((rel) => {
              const relSource = files.value[rel.file] || "";
              const relLoc = offsetToLineColumn(relSource, rel.offset);
              return {
                file: rel.file,
                line: relLoc.line,
                column: relLoc.column,
                message: rel.message,
              };
            }),
            suggestion: diag.suggestion,
          });
        }

        analysisTime.value = crossFileResult.stats.analysisTimeMs;
      } else {
        issues = fallbackAnalysis();
        analysisTime.value = performance.now() - startTime;
      }
    } catch (e) {
      console.warn("WASM cross-file analysis failed, using fallback:", e);
      issues = fallbackAnalysis();
      analysisTime.value = performance.now() - startTime;
    }

    const suppressionMap = buildSuppressionMap(files.value);
    const filteredIssues = filterSuppressedIssues(issues, suppressionMap);

    crossFileIssues.value = filteredIssues;
    isAnalyzing.value = false;
  }

  function fallbackAnalysis(): CrossFileIssue[] {
    const ctx = buildContext();
    const issues: CrossFileIssue[] = [];

    if (options.value.provideInject) {
      issues.push(...analyzeProvideInject(ctx));
    }
    if (options.value.componentEmits) {
      issues.push(...analyzeComponentEmits(ctx));
    }
    if (options.value.fallthroughAttrs) {
      issues.push(...analyzeFallthroughAttrs(ctx));
    }
    if (options.value.reactivityTracking) {
      issues.push(...analyzeReactivity(ctx));
    }
    if (options.value.uniqueIds) {
      issues.push(...analyzeUniqueIds(ctx));
    }
    if (options.value.serverClientBoundary) {
      issues.push(...analyzeSSRBoundary(ctx));
    }

    return issues;
  }

  return {
    analyzeAll,
  };
}
