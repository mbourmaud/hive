import type mermaidApi from "mermaid";

let mermaidInstance: typeof mermaidApi | null = null;
let initPromise: Promise<typeof mermaidApi> | null = null;
let idCounter = 0;

function getMermaidTheme(): "dark" | "default" {
  const attr = document.documentElement.getAttribute("data-theme");
  return attr === "light" ? "default" : "dark";
}

async function getMermaid(): Promise<typeof mermaidApi> {
  if (mermaidInstance) return mermaidInstance;
  if (initPromise) return initPromise;

  initPromise = (async () => {
    const { default: mermaid } = await import("mermaid");
    mermaid.initialize({
      startOnLoad: false,
      theme: getMermaidTheme(),
      securityLevel: "strict",
    });
    mermaidInstance = mermaid;
    return mermaid;
  })();

  return initPromise;
}

/**
 * Post-pass: find `<pre><code class="language-mermaid">` blocks,
 * render them as SVG, and replace the `<pre>` with a diagram container.
 * Must be called BEFORE `wrapCodeBlocks` so mermaid `<pre>` are removed
 * before copy buttons are added.
 */
export async function renderMermaidBlocks(container: HTMLElement): Promise<void> {
  const codeEls = container.querySelectorAll("pre > code.language-mermaid");
  if (codeEls.length === 0) return;

  const mermaid = await getMermaid();
  // Re-initialize with current theme on each render pass
  mermaid.initialize({ startOnLoad: false, theme: getMermaidTheme(), securityLevel: "strict" });

  for (const codeEl of codeEls) {
    const pre = codeEl.parentElement;
    if (!pre) continue;
    // Skip already rendered or errored blocks
    if (pre.getAttribute("data-mermaid") === "rendered") continue;
    if (pre.getAttribute("data-mermaid") === "error") continue;

    const definition = codeEl.textContent?.trim() ?? "";
    if (!definition) continue;

    try {
      const id = `mermaid-${++idCounter}`;
      const { svg } = await mermaid.render(id, definition);
      const wrapper = document.createElement("div");
      wrapper.setAttribute("data-slot", "mermaid-diagram");
      wrapper.setAttribute("data-mermaid", "rendered");
      wrapper.innerHTML = svg;
      pre.parentNode?.replaceChild(wrapper, pre);
    } catch {
      // Mark as error so we don't retry — raw code block stays as fallback
      pre.setAttribute("data-mermaid", "error");
    }
  }
}
