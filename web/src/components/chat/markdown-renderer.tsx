import "./markdown-renderer.css"

import { useEffect, useRef } from "react"
import { marked } from "marked"
import DOMPurify from "dompurify"
import morphdom from "morphdom"

// ---------------------------------------------------------------------------
// LRU cache — max 200 entries, simple Map eviction (oldest first)
// ---------------------------------------------------------------------------
const CACHE_MAX = 200
const htmlCache = new Map<string, string>()

function getCachedHtml(key: string, raw: string): string {
  const cached = htmlCache.get(key)
  if (cached !== undefined) return cached

  const html = renderMarkdown(raw)

  if (htmlCache.size >= CACHE_MAX) {
    // evict oldest entry (first key)
    const firstKey = htmlCache.keys().next().value
    if (firstKey !== undefined) htmlCache.delete(firstKey)
  }
  htmlCache.set(key, html)
  return html
}

// ---------------------------------------------------------------------------
// Markdown → sanitised HTML
// ---------------------------------------------------------------------------
const PURIFY_CONFIG = {
  USE_PROFILES: { html: true, mathMl: true },
  SANITIZE_NAMED_PROPS: true,
  FORBID_TAGS: ["style"],
  FORBID_CONTENTS: ["style", "script"],
}

function renderMarkdown(text: string): string {
  const raw = marked.parse(text, { async: false }) as string
  return DOMPurify.sanitize(raw, PURIFY_CONFIG)
}

// ---------------------------------------------------------------------------
// SVG icon strings for copy / check (inline, no JSX dep for morphdom compat)
// ---------------------------------------------------------------------------
const ICON_COPY = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>`
const ICON_CHECK = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>`

// ---------------------------------------------------------------------------
// DOM helpers — build code-block wrapper + copy button (pure DOM, not JSX)
// ---------------------------------------------------------------------------
function createCopyButton(preEl: HTMLPreElement): HTMLButtonElement {
  const btn = document.createElement("button")
  btn.className = "copy-button"
  btn.setAttribute("aria-label", "Copy code")
  btn.innerHTML = ICON_COPY

  btn.addEventListener("click", () => {
    const code = preEl.textContent ?? ""
    navigator.clipboard.writeText(code).then(() => {
      btn.innerHTML = ICON_CHECK
      setTimeout(() => {
        btn.innerHTML = ICON_COPY
      }, 2000)
    })
  })

  return btn
}

function wrapCodeBlocks(container: HTMLElement): void {
  const pres = container.querySelectorAll("pre")
  pres.forEach((pre) => {
    // skip if already wrapped
    const parent = pre.parentElement
    if (parent?.getAttribute("data-component") === "markdown-code") return

    const wrapper = document.createElement("div")
    wrapper.setAttribute("data-component", "markdown-code")
    pre.parentNode?.insertBefore(wrapper, pre)
    wrapper.appendChild(pre)
    wrapper.appendChild(createCopyButton(pre))
  })
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
interface MarkdownRendererProps {
  text: string
  cacheKey?: string
  className?: string
}

export function MarkdownRenderer({
  text,
  cacheKey,
  className,
}: MarkdownRendererProps) {
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return

    const key = cacheKey ?? text
    const html = getCachedHtml(key, text)

    // Wrap in the root data-component div for morphdom diffing
    const target = `<div data-component="markdown">${html}</div>`

    if (!el.firstElementChild) {
      // First render — set innerHTML directly
      el.innerHTML = target
    } else {
      // Incremental update via morphdom
      morphdom(el.firstElementChild, target, {
        onBeforeElUpdated(fromEl, toEl) {
          // Skip update if DOM is already equal
          if (fromEl.isEqualNode(toEl)) return false
          // Preserve code-block wrappers built by wrapCodeBlocks
          if (
            fromEl.getAttribute("data-component") === "markdown-code"
          ) {
            return false
          }
          return true
        },
        onBeforeNodeDiscarded(node) {
          // Preserve copy buttons added by wrapCodeBlocks
          if (
            node instanceof HTMLElement &&
            node.classList.contains("copy-button")
          ) {
            return false
          }
          return true
        },
      })
    }

    // Post-pass: wrap bare <pre> elements and add copy buttons
    wrapCodeBlocks(el)
  }, [text, cacheKey])

  return <div ref={containerRef} className={className} />
}
