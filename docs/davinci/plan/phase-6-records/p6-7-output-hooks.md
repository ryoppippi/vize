# P6-7: checked formatter and output hooks

The native output hook family consumes an actual `CompileResult` after the
compiler finishes. `compileWithOutputPlugins` compiles VDom or Vapor and applies
the hooks; `applyOutputPlugins` composes the same boundary with another native
compile result, including the S2 transform hook's `result`.

Each plugin receives one serialized batch containing the preceding compiler
result, its family, schema 1, and `offsetEncoding: "utf8"`. Each subsequent
plugin sees the validated output of its predecessor. Hooks run outside the
fused walks and cannot substitute generated JavaScript or a source map.

- `formatter` returns ordered `{start, end, text}` edits in generated UTF-8 byte
  offsets. Both the removed text and replacement must be ASCII whitespace.
  The host parses the preceding and edited JavaScript and compares their
  normalized generated programs and original comments. ASI changes, string or
  template literal edits, malformed programs, overlaps and split characters
  are rejected.
- `output` returns `{placement: "prepend" | "append", comment}` additions. The
  host inserts unlinked block comments, rejects closing delimiters and map or
  optimization directives, and checks
  that the resulting JavaScript program retains its meaning. Arbitrary target
  backends and executable output replacement are outside this contract.

Existing single-source native v3 map anchors are decoded and checked against
the generated and authored text, then carried into the shared `EmitDocument`.
Edits rebase every generated anchor and its authored symbol before the normal
v3 writer runs. Authored source content and metadata survive; inserted trivia
claims no new authored provenance. The public map has anchors, so this bridge
does not reconstruct full internal generated ranges from them. A compile that
did not request maps still returns no map.

Each plugin reports host time, callback time, batch size, operation count and
cache status. Cached misses always audit two calls; uncached runs audit by default. Invalid
or inconsistent results never enter the cache. The content key covers the entire
preceding compile result, compiler configuration, host build, feature flags,
audit mode, plugin family/name/version/code fingerprint and its declared input
values. The process cache keeps at most 64 results and 4 MiB of serialized
result/response bytes; hits skip the JS join. `cacheDir` adds atomic disk reuse.
Disk records carry the exact key and operation response; fresh processes repeat
native edit, semantic and map checks before accepting them. Torn, mismatched,
invalid and oversized records are misses. A memory hit can populate a new
directory without calling JavaScript.

The pure Rust contracts exercise real native Vapor results, every decoded
authored anchor, Unicode, no-map output, ASI/literal/comment rejection,
determinism, declared cache inputs and preceding-output invalidation. The napi
fixture executes a formatted VDom render and event handler, composes output
hooks, observes cache skips across fresh Node processes, repairs invalid disk
entries, and checks Vapor template/map preservation.

This closes the bounded native formatter/output consumer portion of P6-7.
The SDK package, sandbox, all fact groups and the phase's full acceptance matrix
remain separate obligations and must pass together before P6-7 is complete.
