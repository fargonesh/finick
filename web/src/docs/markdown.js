// Minimal markdown -> HTML for the docs reader. Covers what docs/ uses:
// headings, fenced code, lists, bold, inline code, links. No dependency.
import { slugForFile } from "./manifest.js"

const esc = (s) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")

function inline(src) {
  let out = esc(src)
  out = out.replace(/`([^`]+)`/g, "<code>$1</code>")
  out = out.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
  out = out.replace(
    /\[([^\]]+)\]\(([^)]+)\)/g,
    (_, text, href) => {
      const md = href.match(/\.?\/?([^/#)]+)\.md(#.*)?$/)
      if (md) {
        const slug = slugForFile(md[1])
        if (slug) return `<a href="#/docs/${slug}">${text}</a>`
      }
      return `<a href="${href}">${text}</a>`
    }
  )
  return out
}

export function renderMarkdown(src) {
  const blocks = []
  const text = src.replace(
    /```(\w*)\n([\s\S]*?)```/g,
    (_, lang, code) => {
      blocks.push(
        `<pre><code${lang ? ` data-lang="${esc(lang)}"` : ""}>${esc(code.replace(/\n$/, ""))}</code></pre>`
      )
      return `\u0000${blocks.length - 1}\u0000`
    }
  )
  const html = []
  let list = null
  const closeList = () => {
    if (list) {
      html.push(list === "ul" ? "</ul>" : "</ol>")
      list = null
    }
  }
  for (const raw of text.split("\n")) {
    const line = raw.trimEnd()
    if (/^\u0000\d+\u0000$/.test(line.trim())) {
      closeList()
      html.push(blocks[Number(line.trim().slice(1, -1))])
      continue
    }
    const h = line.match(/^(#{1,3})\s+(.*)/)
    if (h) {
      closeList()
      const level = h[1].length
      const id = h[2].toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "")
      html.push(`<h${level} id="${id}">${inline(h[2])}</h${level}>`)
      continue
    }
    if (/^---+$/.test(line.trim())) {
      closeList()
      html.push("<hr />")
      continue
    }
    const ul = line.match(/^\s*[-*]\s+(.*)/)
    if (ul) {
      if (list !== "ul") {
        closeList()
        html.push("<ul>")
        list = "ul"
      }
      html.push(`<li>${inline(ul[1])}</li>`)
      continue
    }
    const ol = line.match(/^\s*\d+\.\s+(.*)/)
    if (ol) {
      if (list !== "ol") {
        closeList()
        html.push("<ol>")
        list = "ol"
      }
      html.push(`<li>${inline(ol[1])}</li>`)
      continue
    }
    if (!line.trim()) {
      closeList()
      continue
    }
    closeList()
    html.push(`<p>${inline(line.trim())}</p>`)
  }
  closeList()
  return html.join("\n")
}
