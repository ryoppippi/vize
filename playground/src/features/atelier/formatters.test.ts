import { describe, expect, it } from "vitest";
import { formatCode } from "./formatters";

describe("formatCode", () => {
  it("formats SSR template literal output", async () => {
    const code =
      "import { ssrInterpolate as _ssrInterpolate, ssrRenderAttr as _ssrRenderAttr } from 'vue/server-renderer'\nfunction ssrRender(_ctx, _push, _parent, _attrs) { _push(`<div class=\"card\"><h2>${_ssrInterpolate(name)}</h2><p>Count: ${_ssrInterpolate(count)} (doubled: ${_ssrInterpolate(doubled.value)})</p><p>Items: ${_ssrInterpolate(itemCount.value)}</p><button${_ssrRenderAttr('disabled', disabled)}>Increment</button></div>`) }";

    await expect(formatCode(code, "babel")).resolves.toMatchInlineSnapshot(`
      "import {
        ssrInterpolate as _ssrInterpolate,
        ssrRenderAttr as _ssrRenderAttr,
      } from 'vue/server-renderer'
      function ssrRender(_ctx, _push, _parent, _attrs) {
        _push(\`<div class="card">
        <h2>\${_ssrInterpolate(name)}</h2>
        <p>Count: \${_ssrInterpolate(count)} (doubled: \${_ssrInterpolate(doubled.value)})</p>
        <p>Items: \${_ssrInterpolate(itemCount.value)}</p>
        <button\${_ssrRenderAttr('disabled', disabled)}>Increment</button>
      </div>\`)
      }
      "
    `);
  });
});
