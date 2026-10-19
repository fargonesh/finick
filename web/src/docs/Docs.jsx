import { useEffect, useMemo, useState } from "react"
import { BookOpen, Search, ArrowLeft, ArrowRight } from "lucide-react"
import { SECTIONS, ALL_PAGES } from "./manifest.js"
import { CONTENT } from "./content.js"
import { renderMarkdown } from "./markdown.js"
import "./docs.css"

export default function Docs({ slug }) {
  const [query, setQuery] = useState("")
  const page = ALL_PAGES.find((p) => p.slug === slug) || ALL_PAGES[0]
  const idx = ALL_PAGES.findIndex((p) => p.slug === page.slug)
  const prev = ALL_PAGES[idx - 1]
  const next = ALL_PAGES[idx + 1]

  const results = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return null
    return ALL_PAGES.filter(
      (p) =>
        p.title.toLowerCase().includes(q) ||
        (CONTENT[p.slug] || "").toLowerCase().includes(q)
    ).slice(0, 12)
  }, [query])

  useEffect(() => {
    setQuery("")
    document.querySelector(".docs-content")?.scrollTo({ top: 0 })
    window.scrollTo({ top: 0 })
  }, [slug])

  const html = useMemo(
    () => renderMarkdown(CONTENT[page.slug] || "# Missing\nNo content."),
    [page.slug]
  )

  return (
    <div className="docs mx-auto grid max-w-[1120px] gap-4 px-5 py-8 md:grid-cols-[240px_1fr]">
      <aside className="docs-sidebar rounded-[20px] border p-3">
        <div className="mb-2 flex items-center gap-2 px-2 text-sm font-semibold">
          <BookOpen size={14} style={{ color: "var(--accent)" }} /> Docs
        </div>
        <label className="mb-3 flex items-center gap-2 rounded-full border px-3 py-2 text-[13px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}>
          <Search size={13} style={{ color: "var(--text-dim)" }} />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search docs…"
            className="w-full bg-transparent outline-none"
          />
        </label>
        {results ? (
          <div className="flex flex-col gap-1">
            {results.length === 0 && (
              <p className="px-2 py-3 text-[13px]" style={{ color: "var(--text-dim)" }}>No matches. Try fewer words.</p>
            )}
            {results.map((p) => (
              <a
                key={p.slug}
                href={`#/docs/${p.slug}`}
                className={`rounded-xl px-2.5 py-2 text-[13px] ${p.slug === page.slug ? "font-semibold" : ""}`}
                style={p.slug === page.slug
                  ? { background: "color-mix(in srgb, var(--accent) 14%, transparent)", color: "var(--text)" }
                  : { color: "var(--text-dim)" }}
              >
                {p.title}
              </a>
            ))}
          </div>
        ) : (
          SECTIONS.map((s) => (
            <div key={s.title} className="mb-3">
              <h4 className="px-2 pb-1 text-[11px] font-semibold uppercase tracking-[0.08em]" style={{ color: "var(--text-dim)" }}>{s.title}</h4>
              <div className="flex flex-col gap-0.5">
                {s.pages.map((p) => (
                  <a
                    key={p.slug}
                    href={`#/docs/${p.slug}`}
                    className={`rounded-xl px-2.5 py-[7px] text-[13px] ${p.slug === page.slug ? "font-semibold" : ""}`}
                    style={p.slug === page.slug
                      ? { background: "color-mix(in srgb, var(--accent) 14%, transparent)", color: "var(--text)" }
                      : { color: "var(--text-dim)" }}
                  >
                    {p.title}
                  </a>
                ))}
              </div>
            </div>
          ))
        )}
      </aside>

      <article className="min-w-0 rounded-[20px] border p-5 md:p-7" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
        <p className="mb-2 text-xs" style={{ color: "var(--text-dim)" }}>{page.section}</p>
        <div className="docs-content" dangerouslySetInnerHTML={{ __html: html }} />
        <div className="mt-8 flex items-center justify-between gap-3 border-t pt-4 text-[13px]" style={{ borderColor: "var(--border)" }}>
          {prev
            ? <a href={`#/docs/${prev.slug}`} className="inline-flex items-center gap-1.5 font-medium"><ArrowLeft size={14} /> {prev.title}</a>
            : <span />}
          {next
            ? <a href={`#/docs/${next.slug}`} className="inline-flex items-center gap-1.5 font-medium">{next.title} <ArrowRight size={14} /></a>
            : <span />}
        </div>
      </article>
    </div>
  )
}
