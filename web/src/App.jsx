import { useEffect, useLayoutEffect, useRef, useState } from "react"
import gsap from "gsap"
import { ScrollTrigger } from "gsap/ScrollTrigger"
import {
  Wifi, Bluetooth, Sun, Monitor, Volume2, HardDrive, Battery,
  Settings2, Folder, Search, Bell, Moon, Zap, Box, Layers, Terminal, Copy, Check, ArrowRight, Github, SunDim
} from "lucide-react"

gsap.registerPlugin(ScrollTrigger)

const ACCENTS = [
  { name: "Indigo", hex: "#5B5FE9" },
  { name: "Coral", hex: "#FF6952" },
  { name: "Amber", hex: "#E3A23D" },
  { name: "Teal", hex: "#2CA6A0" },
  { name: "Rose", hex: "#E85A88" },
  { name: "Slate", hex: "#64748B" },
]

export default function App() {
  const [theme, setTheme] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("finick-theme") || "light"
    }
    return "light"
  })
  const [accent, setAccent] = useState(() => localStorage.getItem("finick-accent") || "#5B5FE9")
  const [copied, setCopied] = useState(null)
  const rootRef = useRef(null)

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme)
    localStorage.setItem("finick-theme", theme)
  }, [theme])

  useEffect(() => {
    document.documentElement.style.setProperty("--accent", accent)
    localStorage.setItem("finick-accent", accent)
  }, [accent])

  useLayoutEffect(() => {
    const ctx = gsap.context(() => {
      gsap.from(".hero-eyebrow", { y: 16, opacity: 0, duration: 0.6, ease: "power3.out" })
      gsap.from(".hero-title", { y: 30, opacity: 0, duration: 0.8, ease: "power3.out", delay: 0.08 })
      gsap.from(".hero-lead", { y: 16, opacity: 0, duration: 0.6, ease: "power3.out", delay: 0.18 })
      gsap.from(".hero-cta .magnet", { y: 12, opacity: 0, duration: 0.5, stagger: 0.08, delay: 0.28, ease: "back.out(1.4)" })
      gsap.from(".hero-meta", { opacity: 0, duration: 0.5, delay: 0.4 })

      gsap.from("nav", { y: -16, opacity: 0, duration: 0.6, ease: "power3.out" })

      gsap.to(".progress", {
        scaleX: 1, ease: "none",
        scrollTrigger: { trigger: document.body, start: "top top", end: "bottom bottom", scrub: 0.3 }
      })

      gsap.to(".bg-glow--indigo", {
        y: 120, ease: "none",
        scrollTrigger: { trigger: "body", start: "top top", end: "bottom top", scrub: 1 }
      })
      gsap.to(".bg-glow--coral", {
        y: -80, ease: "none",
        scrollTrigger: { trigger: "body", start: "top top", end: "bottom top", scrub: 1 }
      })

      gsap.from(".bento-card", {
        y: 28, opacity: 0, rotation: 0.6, duration: 0.6, stagger: 0.08, ease: "power3.out",
        scrollTrigger: { trigger: ".bento", start: "top 82%" }
      })

      gsap.from(".arch-col", {
        y: 24, opacity: 0, duration: 0.6, stagger: 0.12, ease: "power3.out",
        scrollTrigger: { trigger: ".arch", start: "top 84%" }
      })

      gsap.from(".int-card", {
        y: 20, opacity: 0, duration: 0.5, stagger: 0.07, ease: "power2.out",
        scrollTrigger: { trigger: ".integrations", start: "top 84%" }
      })

      gsap.from(".install-card", {
        y: 24, opacity: 0, duration: 0.7, ease: "power3.out",
        scrollTrigger: { trigger: ".install", start: "top 85%" }
      })

      gsap.to(".float-icon", {
        y: -6, duration: 2.2, repeat: -1, yoyo: true, ease: "sine.inOut",
        stagger: { each: 0.18, from: "random" }
      })

      gsap.utils.toArray(".bento-card").forEach((card) => {
        card.addEventListener("mousemove", (e) => {
          const rect = card.getBoundingClientRect()
          const x = (e.clientX - rect.left) / rect.width - 0.5
          const y = (e.clientY - rect.top) / rect.height - 0.5
          gsap.to(card, { rotationY: x * 6, rotationX: -y * 6, transformPerspective: 800, duration: 0.4, ease: "power2.out" })
        })
        card.addEventListener("mouseleave", () => {
          gsap.to(card, { rotationY: 0, rotationX: 0, duration: 0.6, ease: "power3.out" })
        })
      })

      const glow = document.querySelector(".cursor-glow")
      if (glow) {
        const xTo = gsap.quickTo(glow, "x", { duration: 0.6, ease: "power3" })
        const yTo = gsap.quickTo(glow, "y", { duration: 0.6, ease: "power3" })
        window.addEventListener("mousemove", (e) => {
          xTo(e.clientX)
          yTo(e.clientY)
          gsap.to(glow, { opacity: 0.18, duration: 0.3 })
        })
        window.addEventListener("mouseleave", () => gsap.to(glow, { opacity: 0, duration: 0.4 }))
      }

      gsap.utils.toArray(".magnet").forEach((el) => {
        el.addEventListener("mousemove", (e) => {
          const r = el.getBoundingClientRect()
          const x = e.clientX - r.left - r.width / 2
          const y = e.clientY - r.top - r.height / 2
          gsap.to(el, { x: x * 0.18, y: y * 0.18, duration: 0.3, ease: "power2.out" })
        })
        el.addEventListener("mouseleave", () => gsap.to(el, { x: 0, y: 0, duration: 0.4, ease: "power3.out" }))
      })

      gsap.utils.toArray("section h2").forEach((h) => {
        gsap.from(h, {
          y: 18, opacity: 0, duration: 0.6, ease: "power3.out",
          scrollTrigger: { trigger: h, start: "top 88%" }
        })
      })

    }, rootRef)
    return () => ctx.revert()
  }, [])

  const copy = async (text, id) => {
    await navigator.clipboard.writeText(text)
    setCopied(id)
    setTimeout(() => setCopied(null), 1400)
  }

  return (
    <div ref={rootRef} className="relative">
      <div className="progress fixed left-0 top-0 z-[30] h-[2px] w-full origin-left scale-x-0" style={{ background: "var(--accent)" }} />
      <div className="cursor-glow pointer-events-none fixed left-0 top-0 z-0 h-[420px] w-[420px] -translate-x-1/2 -translate-y-1/2 rounded-full opacity-0 blur-[70px]" style={{ background: "radial-gradient(circle at center, var(--accent) 0%, transparent 70%)" }} />
      <div className="pointer-events-none fixed inset-0 opacity-[0.22]" style={{
        backgroundImage: `linear-gradient(var(--border) 1px, transparent 1px), linear-gradient(90deg, var(--border) 1px, transparent 1px)`,
        backgroundSize: "56px 56px",
        maskImage: "radial-gradient(800px 600px at 50% -10%, #000 60%, transparent 85%)"
      }} />
      <div className="bg-glow--indigo pointer-events-none fixed -left-[180px] -top-[120px] h-[720px] w-[720px] rounded-full blur-[90px] opacity-[0.18]" style={{ background: "#5B5FE9" }} />
      <div className="bg-glow--coral pointer-events-none fixed -right-[200px] top-[200px] h-[720px] w-[720px] rounded-full blur-[90px] opacity-[0.14]" style={{ background: "#FF6952" }} />

      <nav className="sticky top-0 z-20 border-b backdrop-blur-[16px]" style={{ background: "color-mix(in srgb, var(--bg) 72%, transparent)", borderColor: "var(--border)" }}>
        <div className="mx-auto flex max-w-[1120px] items-center justify-between gap-4 px-5 py-[10px]">
          <a href="#" className="flex items-center gap-2.5">
            <span className="grid h-8 w-8 place-items-center rounded-full border text-[13px] font-bold" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--accent)" }}>F</span>
            <span className="font-bold tracking-[-0.02em]">finick</span>
            <span className="rounded-full border px-2 py-0.5 text-xs" style={{ background: "var(--panel)", borderColor: "var(--border)", color: "var(--text-dim)" }}>desktop shell</span>
          </a>
          <div className="hidden items-center gap-[18px] text-[13px] md:flex" style={{ color: "var(--text-dim)" }}>
            <a href="#apps" className="hover:text-[var(--text)]">Apps</a>
            <a href="#system" className="hover:text-[var(--text)]">System</a>
            <a href="#integrations" className="hover:text-[var(--text)]">Integrations</a>
            <a href="#install" className="hover:text-[var(--text)]">Install</a>
          </div>
          <div className="flex items-center gap-3">
            <div className="hidden items-center gap-1.5 rounded-full border p-1 sm:flex" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
              {ACCENTS.map(a => (
                <button key={a.hex} onClick={() => setAccent(a.hex)} aria-label={a.name}
                  className="h-[18px] w-[18px] rounded-full border-2 transition"
                  style={{ background: a.hex, borderColor: accent === a.hex ? "var(--text)" : "transparent", opacity: accent === a.hex ? 1 : 0.7, boxShadow: accent === a.hex ? `0 0 0 2px color-mix(in srgb, ${a.hex} 30%, transparent)` : "none" }} />
              ))}
            </div>
            <button onClick={() => setTheme(theme === "dark" ? "light" : "dark")} aria-label="Toggle theme"
              className="relative flex h-7 w-[56px] items-center justify-between rounded-full border p-0.5" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
              <span className="z-10 grid w-[22px] place-items-center text-xs opacity-70"><Sun size={12} /></span>
              <span className="z-10 grid w-[22px] place-items-center text-xs opacity-70"><Moon size={12} /></span>
              <span className="absolute left-0.5 top-0.5 h-[22px] w-[22px] rounded-full transition-transform duration-300" style={{ background: "var(--accent)", transform: theme === "light" ? "translateX(28px)" : "translateX(0)" }} />
            </button>
            <a href="https://github.com/fargonesh/finick" target="_blank" rel="noreferrer"
              className="hidden items-center gap-1.5 rounded-full px-3.5 py-2 text-[13px] font-semibold text-white sm:inline-flex" style={{ background: "var(--accent)" }}>
              <Github size={14} /> GitHub
            </a>
          </div>
        </div>
      </nav>

      <header className="mx-auto max-w-[1120px] px-5 pb-6 pt-12 md:pt-16">
        <div className="mx-auto max-w-[720px] text-center">
          <div className="hero-eyebrow inline-flex items-center gap-2 rounded-full border px-2.5 py-1.5 text-[11px] uppercase tracking-[0.08em]" style={{ background: "var(--panel)", borderColor: "var(--border)", color: "var(--text-dim)" }}>
            <span className="h-2 w-2 animate-pulse rounded-full" style={{ background: "var(--accent)", boxShadow: "0 0 0 6px color-mix(in srgb, var(--accent) 18%, transparent)" }} />
            Hyprland · NixOS · Rust · Freya
          </div>
          <h1 className="hero-title mt-[18px] text-[42px] font-extrabold leading-[0.92] tracking-[-0.04em] md:text-[58px]">
            The missing desktop shell<br /><em className="not-italic" style={{ color: "var(--accent)" }}>for Hyprland</em>
          </h1>
          <p className="hero-lead mx-auto mt-4 max-w-[560px] text-[17px] leading-relaxed" style={{ color: "var(--text-dim)" }}>
            Hyprland handles window management; finick provides the controls and utilities. Instead of stitching together independent Python scripts and rofi menus, you get a cohesive settings manager, file browser, and status bar built in Rust.
          </p>
          <div className="hero-cta mt-6 flex flex-wrap justify-center gap-2.5">
            <a href="#install" className="magnet inline-flex items-center gap-2 rounded-full px-[18px] py-[11px] text-sm font-semibold text-white" style={{ background: "var(--accent)" }}>
              Add to Flake <ArrowRight size={16} />
            </a>
            <a href="#apps" className="magnet inline-flex items-center gap-2 rounded-full border px-[18px] py-[11px] text-sm font-semibold" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}>
              Explore Architecture
            </a>
          </div>
          <div className="hero-meta mx-auto mt-6 flex max-w-[400px] justify-center gap-8 border-t pt-4" style={{ borderColor: "var(--border)" }}>
            <div className="leading-none text-center"><strong className="block text-lg">3</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>desktop apps</span></div>
            <div className="leading-none text-center"><strong className="block text-lg">2</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>daemons</span></div>
            <div className="leading-none text-center"><strong className="block text-lg">6</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>theme palettes</span></div>
          </div>

        </div>
      </header>

      <section id="apps" className="mx-auto max-w-[1120px] px-5 py-9">
        <div className="mb-5 grid items-end gap-6 md:grid-cols-[1.1fr_0.9fr]">
          <div>
            <div className="mb-2 text-[11px] uppercase tracking-[0.12em]" style={{ color: "var(--text-dim)" }}>Core Utilities</div>
            <h2 className="text-[34px] font-bold leading-[0.95] tracking-[-0.03em]">Unified by Freya,<br /><span className="font-normal" style={{ color: "var(--text-dim)" }}>not shell scripts.</span></h2>
          </div>
          <p className="max-w-[420px] text-sm md:justify-self-end" style={{ color: "var(--text-dim)" }}>All applications share a single component library and communicate with local daemons over typed IPC. Changing your theme mode or accent propagates immediately across every window.</p>
        </div>

        <div className="bento grid gap-3.5 md:grid-cols-3">
          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px] md:row-span-2" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #5B5FE9 16%, var(--panel))", borderColor: "color-mix(in srgb, #5B5FE9 18%, var(--border))", color: "#5B5FE9" }}><Settings2 size={16} /></span>
              <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>App · Freya</span>
            </div>
            <h3 className="text-lg font-semibold tracking-[-0.02em]">Settings</h3>
            <p className="text-[13px] leading-[1.55]" style={{ color: "var(--text-dim)" }}>18 configuration views covering network, displays, audio devices, and power profiles, directly reading hardware status through system backends.</p>
            <div className="mt-1.5 flex flex-wrap gap-1.5">
              {["Wi-Fi", "Bluetooth", "Sound", "Displays", "Storage", "Power", "Printers", "Privacy"].map(s => (
                <span key={s} className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>{s}</span>
              ))}
            </div>
            <div className="mt-auto flex items-center justify-between border-t pt-2.5 text-xs" style={{ borderColor: "var(--border)" }}><span style={{ color: "var(--text-dim)" }}>apps/settings</span><span className="font-semibold" style={{ color: "var(--text-dim)" }}>18 pages</span></div>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #2CA6A0 16%, var(--panel))", borderColor: "color-mix(in srgb, #2CA6A0 18%, var(--border))", color: "#2CA6A0" }}><Folder size={16} /></span>
              <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>App · Freya</span>
            </div>
            <h3 className="text-lg font-semibold">Files</h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Directory bookmarks, instant search, and a metadata preview drawer; loads folders without GTK or Qt runtime dependencies.</p>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--sidebar)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #FF6952 16%, var(--panel))", borderColor: "color-mix(in srgb, #FF6952 18%, var(--border))", color: "#FF6952" }}><Layers size={16} /></span>
              <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel)", borderColor: "var(--border)", color: "var(--text-dim)" }}>Shell · Overlay</span>
            </div>
            <h3 className="text-lg font-semibold">Top Bar</h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>A Wayland overlay surface anchored to the screen edge; quick toggles expand into dedicated control panels beneath the clock.</p>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #E3A23D 16%, var(--panel))", borderColor: "color-mix(in srgb, #E3A23D 18%, var(--border))", color: "#E3A23D" }}><Box size={16} /></span>
              <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>Service</span>
            </div>
            <h3 className="text-lg font-semibold">Daemon</h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Synchronizes environment state and coordinates desktop notifications via ipsea over unix domain sockets.</p>
            <code className="rounded-lg border p-2 text-xs" style={{ background: "var(--bg)", borderColor: "var(--border)" }}>finickctl · ipsea</code>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #E85A88 16%, var(--panel))", borderColor: "color-mix(in srgb, #E85A88 18%, var(--border))", color: "#E85A88" }}><Search size={16} /></span>
              <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>Service</span>
            </div>
            <h3 className="text-lg font-semibold">Index</h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>In-memory file indexing that supports instant type-ahead filtering without running heavy background crawlers.</p>
            <code className="rounded-lg border p-2 text-xs" style={{ background: "var(--bg)", borderColor: "var(--border)" }}>type-ahead search</code>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: `linear-gradient(180deg, color-mix(in srgb, var(--accent) 14%, var(--panel)), var(--panel))`, borderColor: "var(--border)" }}>
            <h3 className="text-lg font-semibold"><span style={{ color: "var(--accent)" }}>libs/ui</span>: shared system</h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Shared design system: cards, switches, sliders, and navigation sidebars with six selectable accent colors.</p>
          </article>
        </div>
      </section>

      <section id="system" className="mx-auto max-w-[1120px] px-5">
        <div className="rounded-[20px] border p-5 md:p-[22px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
          <div className="mb-5 grid items-end gap-6 md:grid-cols-[1.1fr_0.9fr]">
            <div>
              <div className="mb-2 text-[11px] uppercase tracking-[0.12em]" style={{ color: "var(--text-dim)" }}>How it's built</div>
              <h2 className="text-[30px] font-bold leading-[0.95] tracking-[-0.03em]">A clean Cargo workspace.</h2>
            </div>
            <p className="max-w-[420px] text-sm md:justify-self-end" style={{ color: "var(--text-dim)" }}>Split into independent libraries, services, and apps. You can inspect or extend any part without navigating complex foreign-function layers.</p>
          </div>

          <div className="arch grid gap-3.5 md:grid-cols-3">
            <div className="arch-col">
              <h4 className="mb-2.5 text-xs uppercase tracking-[0.08em]" style={{ color: "var(--text-dim)" }}>Libs</h4>
              <div className="flex flex-col gap-2.5">
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">ui</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Tiles, sidebar, switches: 20px radius, gap 14</span><code className="text-[11px]" style={{ color: "var(--text-dim)" }}>libs/ui</code></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">ipc</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Typed IPC: settings, notifications</span><code className="text-[11px]" style={{ color: "var(--text-dim)" }}>libs/ipc</code></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">system</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>SystemBackend · HyprlandBackend</span><code className="text-[11px]" style={{ color: "var(--text-dim)" }}>libs/system</code></div>
              </div>
            </div>
            <div className="arch-col">
              <h4 className="mb-2.5 text-xs uppercase tracking-[0.08em]" style={{ color: "var(--text-dim)" }}>Services</h4>
              <div className="flex flex-col gap-2.5">
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">settings-daemon</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>State tick loop · notifications</span><code className="text-[11px]" style={{ color: "var(--text-dim)" }}>services/settings-daemon</code></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">index</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>ListDir / Search</span><code className="text-[11px]" style={{ color: "var(--text-dim)" }}>services/index</code></div>
              </div>
            </div>
            <div className="arch-col">
              <h4 className="mb-2.5 text-xs uppercase tracking-[0.08em]" style={{ color: "var(--text-dim)" }}>Apps &amp; Shell</h4>
              <div className="flex flex-col gap-2.5">
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">settings</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Overview + 18 detail pages</span><code className="text-[11px]" style={{ color: "var(--text-dim)" }}>apps/settings</code></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">files</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Places, grid/list, preview drawer</span><code className="text-[11px]" style={{ color: "var(--text-dim)" }}>apps/files</code></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">overlay</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Top Bar + Control Panel</span><code className="text-[11px]" style={{ color: "var(--text-dim)" }}>apps/overlay</code></div>
              </div>
            </div>
          </div>

          <div className="mt-3.5 grid gap-3.5 md:grid-cols-3">
            <div className="rounded-2xl border p-3" style={{ background: "var(--bg)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Explicit layout</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>Built on Freya's flex engine so interface elements reflow predictably across display scales.</span></div>
            <div className="rounded-2xl border p-3" style={{ background: "var(--bg)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Declarative Nix</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>Exported as a Nix flake module, enabling complete desktop reproducibility across machines.</span></div>
            <div className="rounded-2xl border p-3" style={{ background: "var(--bg)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Direct host hooks</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>Queries existing Linux subsystems directly rather than running duplicate hardware abstractions.</span></div>
          </div>
        </div>
      </section>

      <section id="integrations" className="mx-auto max-w-[1120px] px-5 py-9">
        <div className="mb-5 grid items-end gap-6 md:grid-cols-[1.1fr_0.9fr]">
          <div>
            <div className="mb-2 text-[11px] uppercase tracking-[0.12em]" style={{ color: "var(--text-dim)" }}>Under the hood</div>
            <h2 className="text-[34px] font-bold leading-[0.95] tracking-[-0.03em]">Standard Linux plumbing.</h2>
          </div>
          <p className="max-w-[420px] text-sm md:justify-self-end" style={{ color: "var(--text-dim)" }}>Instead of maintaining private daemons, finick exposes clear controls over the services already active on your machine:</p>
        </div>

        <div className="integrations grid gap-3.5 md:grid-cols-4">
          <div className="int-card rounded-[20px] border p-4" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <h4 className="mb-2 flex items-center gap-2 text-[13px] font-semibold"><Wifi size={14} style={{ color: "var(--accent)" }} /> Network &amp; Bluetooth</h4>
            <p className="text-[13px] leading-relaxed" style={{ color: "var(--text-dim)" }}>Manages connections and pairing directly through NetworkManager and BlueZ.</p>
          </div>
          <div className="int-card rounded-[20px] border p-4" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <h4 className="mb-2 flex items-center gap-2 text-[13px] font-semibold"><Monitor size={14} style={{ color: "var(--accent)" }} /> Audio &amp; Displays</h4>
            <p className="text-[13px] leading-relaxed" style={{ color: "var(--text-dim)" }}>Adjusts WirePlumber sinks, monitor layouts via Hyprland IPC, and backlight levels.</p>
          </div>
          <div className="int-card rounded-[20px] border p-4" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <h4 className="mb-2 flex items-center gap-2 text-[13px] font-semibold"><Terminal size={14} style={{ color: "var(--accent)" }} /> Host Metrics</h4>
            <p className="text-[13px] leading-relaxed" style={{ color: "var(--text-dim)" }}>Inspects UPower battery state, drive mount utilization, and systemd time synchronization.</p>
          </div>
          <div className="int-card rounded-[20px] border p-4" style={{ background: "var(--sidebar)", borderColor: "var(--border)" }}>
            <h4 className="mb-2 text-[13px] font-semibold">Implementation Status</h4>
            <div className="flex flex-col gap-2">
              <span className="flex items-center gap-2 text-xs"><i className="h-2 w-2 rounded-full bg-[#2CA6A0]" /> Wi-Fi, Bluetooth, Sound <em className="ml-auto text-[11px] not-italic" style={{ color: "var(--text-dim)" }}>solid</em></span>
              <span className="flex items-center gap-2 text-xs"><i className="h-2 w-2 rounded-full bg-[#2CA6A0]" /> Storage, Displays, Power <em className="ml-auto text-[11px] not-italic" style={{ color: "var(--text-dim)" }}>solid</em></span>
              <span className="flex items-center gap-2 text-xs"><i className="h-2 w-2 rounded-full bg-[#E3A23D]" /> Appearance <em className="ml-auto text-[11px] not-italic" style={{ color: "var(--text-dim)" }}>in progress</em></span>
              <span className="flex items-center gap-2 text-xs"><i className="h-2 w-2 rounded-full bg-[var(--text-dim)]" /> Focus, Notifications <em className="ml-auto text-[11px] not-italic" style={{ color: "var(--text-dim)" }}>next</em></span>
            </div>
          </div>
        </div>
      </section>

      <section id="install" className="mx-auto max-w-[1120px] px-5 pb-9">
        <div className="install grid gap-[18px] rounded-[20px] border p-[18px] md:grid-cols-[0.9fr_1.1fr]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
          <div>
            <div className="mb-2 text-[11px] uppercase tracking-[0.12em]" style={{ color: "var(--text-dim)" }}>Install</div>
            <h2 className="text-[30px] font-bold leading-[0.95] tracking-[-0.03em]">Add to your NixOS<br />configuration.</h2>
            <p className="mt-2 text-[13px]" style={{ color: "var(--text-dim)" }}>Include the flake input, enable the module, and run your switch rebuild. The background daemon, overlay bar, and client applications deploy together.</p>
            <div className="mt-3.5 flex flex-wrap gap-2.5">
              <a href="https://github.com/fargonesh/finick" target="_blank" className="inline-flex items-center gap-2 rounded-full px-4 py-2.5 text-sm font-semibold text-white" style={{ background: "var(--accent)" }}>View on GitHub <ArrowRight size={14} /></a>
              <button onClick={() => copy('inputs.finick.url = "github:fargonesh/finick";', "run")} className="inline-flex items-center gap-1.5 rounded-full border px-4 py-2.5 text-sm font-semibold" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}>
                {copied === "run" ? <Check size={14} /> : <Copy size={14} />} {copied === "run" ? "Copied" : "Copy flake input"}
              </button>
            </div>
          </div>
          <div className="install-card overflow-hidden rounded-2xl border" style={{ background: "var(--bg)", borderColor: "var(--border)" }}>
            <div className="flex h-9 items-center gap-2 border-b px-3 text-xs" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}><span className="h-2.5 w-2.5 rounded-full" style={{ background: "var(--accent)" }} /> flake.nix + configuration.nix</div>
            <pre className="overflow-auto p-3.5 text-[12.5px] leading-6" style={{ color: "var(--text-dim)" }}><code>{`{
  inputs.finick.url = "github:fargonesh/finick";
  outputs = { finick, nixpkgs, ... }: {
    nixosConfigurations.yourHost = nixpkgs.lib.nixosSystem {
      modules = [
        finick.nixosModules.default
        { programs.finick.settings.enable = true; }
      ];
    };
  };
}
# then
$ nixos-rebuild switch --flake .#yourHost`}</code></pre>
          </div>
        </div>
      </section>

      <footer className="border-t" style={{ borderColor: "var(--border)", background: "color-mix(in srgb, var(--panel) 60%, transparent)" }}>
        <div className="mx-auto flex max-w-[1120px] flex-wrap items-center justify-between gap-3 px-5 py-[18px]">
          <div className="flex items-center gap-2.5">
            <span className="grid h-8 w-8 place-items-center rounded-full border text-[13px] font-bold" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--accent)" }}>F</span>
            <div><strong className="block text-sm">finick</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>Native desktop shell for Hyprland on NixOS</span></div>
          </div>
          <div className="flex items-center gap-3.5 text-[13px]" style={{ color: "var(--text-dim)" }}>
            <a href="https://github.com/fargonesh/finick" className="hover:text-[var(--text)]">GitHub</a>
            <a href="#apps" className="hover:text-[var(--text)]">Apps</a>
            <a href="#system" className="hover:text-[var(--text)]">Architecture</a>
            <span>© 2026 fargone</span>
          </div>
        </div>
      </footer>
    </div>
  )
}
