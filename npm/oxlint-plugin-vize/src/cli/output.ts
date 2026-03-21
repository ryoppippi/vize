export function rewriteReportedPaths(
  output: string,
  replacements: ReadonlyMap<string, string>,
): string {
  let rewritten = output;

  for (const [from, to] of replacements) {
    rewritten = rewritten.split(from).join(to);
  }

  return rewritten;
}
