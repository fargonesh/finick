import { useEffect, useLayoutEffect, useRef, useState } from "react"
import gsap from "gsap"
import { ScrollTrigger } from "gsap/ScrollTrigger"
import {
  Wifi, Monitor, HardDrive, Settings2, Folder, Search, Layers, Copy, Check, ArrowRight, Github, Sun, Moon, Box
} from "lucide-react"
import Docs from "./docs/Docs.jsx"

gsap.registerPlugin(ScrollTrigger)

const ACCENTS = [
  { name: "Indigo", hex: "#5B5FE9" },
  { name: "Coral", hex: "#FF6952" },
  { name: "Amber", hex: "#E3A23D" },
  { name: "Teal", hex: "#2CA6A0" },
  { name: "Rose", hex: "#E85A88" },
  { name: "Slate", hex: "#7B7F87" },
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
  const [hash, setHash] = useState(() => window.location.hash || "")

  useEffect(() => {
    const onHash = () => setHash(window.location.hash || "")
    window.addEventListener("hashchange", onHash)
    return () => window.removeEventListener("hashchange", onHash)
  }, [])

  const isDocs = hash.startsWith("#/docs")
  const docSlug = isDocs ? (hash.replace("#/docs/", "").split(/[?#]/)[0] || "intro") : "intro"

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme)
    localStorage.setItem("finick-theme", theme)
  }, [theme])

  useEffect(() => {
    document.documentElement.style.setProperty("--accent", accent)
    localStorage.setItem("finick-accent", accent)
  }, [accent])

  useLayoutEffect(() => {
    if (isDocs) return
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches
    if (reduce) return
    const ctx = gsap.context(() => {
      gsap.from(".hero-eyebrow", { y: 16, opacity: 0, duration: 0.6, ease: "power3.out" })
      gsap.from(".hero-title", { y: 30, opacity: 0, duration: 0.8, ease: "power3.out", delay: 0.08 })
      gsap.from(".hero-lead", { y: 16, opacity: 0, duration: 0.6, ease: "power3.out", delay: 0.18 })
      gsap.from(".hero-cta .magnet", { y: 12, opacity: 0, duration: 0.5, stagger: 0.08, delay: 0.28, ease: "back.out(1.4)" })
      gsap.from(".hero-visual", { y: 20, opacity: 0, duration: 0.8, ease: "power3.out", delay: 0.2 })
      gsap.from("nav", { y: -16, opacity: 0, duration: 0.6, ease: "power3.out" })

      gsap.to(".progress", {
        scaleX: 1, ease: "none",
        scrollTrigger: { trigger: document.body, start: "top top", end: "bottom bottom", scrub: 0.3 }
      })

      gsap.from(".bento-card", {
        y: 28, opacity: 0, duration: 0.6, stagger: 0.08, ease: "power3.out",
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
          gsap.to(card, { rotationY: x * 4, rotationX: -y * 4, transformPerspective: 800, duration: 0.4, ease: "power2.out" })
        })
        card.addEventListener("mouseleave", () => {
          gsap.to(card, { rotationY: 0, rotationX: 0, duration: 0.6, ease: "power3.out" })
        })
      })

      const glow = document.querySelector(".cursor-glow")
      if (glow) {
        const xTo = gsap.quickTo(glow, "x", { duration: 0.6, ease: "power3" })
        const yTo = gsap.quickTo(glow, "y", { duration: 0.6, ease: "power3" })
        const onMove = (e) => {
          xTo(e.clientX)
          yTo(e.clientY)
          gsap.to(glow, { opacity: 0.18, duration: 0.3 })
        }
        const onLeave = () => gsap.to(glow, { opacity: 0, duration: 0.4 })
        window.addEventListener("mousemove", onMove)
        window.addEventListener("mouseleave", onLeave)
        return () => {
          window.removeEventListener("mousemove", onMove)
          window.removeEventListener("mouseleave", onLeave)
        }
      }

      gsap.utils.toArray(".magnet").forEach((el) => {
        const onMove = (e) => {
          const r = el.getBoundingClientRect()
          const x = e.clientX - r.left - r.width / 2
          const y = e.clientY - r.top - r.height / 2
          gsap.to(el, { x: x * 0.18, y: y * 0.18, duration: 0.3, ease: "power2.out" })
        }
        const onLeave = () => gsap.to(el, { x: 0, y: 0, duration: 0.4, ease: "power3.out" })
        el.addEventListener("mousemove", onMove)
        el.addEventListener("mouseleave", onLeave)
      })

    }, rootRef)
    return () => ctx.revert()
  }, [isDocs])

  const copy = async (text, id) => {
    await navigator.clipboard.writeText(text)
    setCopied(id)
    setTimeout(() => setCopied(null), 1400)
  }

  return (
    <div ref={rootRef} className="relative flex min-h-screen flex-col">
      <div className="progress fixed left-0 top-0 z-[30] h-[2px] w-full origin-left scale-x-0" style={{ background: "var(--accent)" }} />
      <div className="cursor-glow pointer-events-none fixed left-0 top-0 z-0 h-[420px] w-[420px] -translate-x-1/2 -translate-y-1/2 rounded-full opacity-0 blur-[70px]" style={{ background: "radial-gradient(circle at center, var(--accent) 0%, transparent 70%)" }} />
      <div className="pointer-events-none fixed inset-0 opacity-[0.22]" style={{
        backgroundImage: `linear-gradient(var(--border) 1px, transparent 1px), linear-gradient(90deg, var(--border) 1px, transparent 1px)`,
        backgroundSize: "56px 56px",
        maskImage: "radial-gradient(800px 600px at 50% -10%, #000 60%, transparent 85%)"
      }} />
      <div className="pointer-events-none fixed -left-[180px] -top-[120px] h-[720px] w-[720px] rounded-full blur-[90px] opacity-[0.18]" style={{ background: "#5B5FE9" }} />
      <div className="pointer-events-none fixed -right-[200px] top-[200px] h-[720px] w-[720px] rounded-full blur-[90px] opacity-[0.14]" style={{ background: "#FF6952" }} />

      <nav className="sticky top-0 z-20 border-b backdrop-blur-[16px]" style={{ background: "color-mix(in srgb, var(--bg) 72%, transparent)", borderColor: "var(--border)" }}>
        <div className="mx-auto flex max-w-[1120px] items-center justify-between gap-4 px-5 py-[10px]">
          <a href="#" className="flex items-center gap-2.5">
            <span className="grid h-8 w-8 place-items-center rounded-full border text-[13px] font-bold" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--accent)" }}>F</span>
            <span className="font-bold tracking-[-0.02em]">finick</span>
            <span className="rounded-full border px-2 py-0.5 text-xs" style={{ background: "var(--panel)", borderColor: "var(--border)", color: "var(--text-dim)" }}>for Hyprland</span>
          </a>
          <div className="hidden items-center gap-[18px] text-[13px] md:flex" style={{ color: "var(--text-dim)" }}>
            <a href="#apps" className="hover:text-[var(--text)]">Apps</a>
            <a href="#system" className="hover:text-[var(--text)]">Included</a>
            <a href="#integrations" className="hover:text-[var(--text)]">Hardware</a>
            <a href="#install" className="hover:text-[var(--text)]">Install</a>
            <a href="#/docs/intro" className="hover:text-[var(--text)]">Docs</a>
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

      {/* HERO: asymmetric split, 50/50, real visual, <20 words, no scroll cue */}
      <main className="h-full flex-1">
      {!isDocs ? (<>
      <header className="mx-auto max-w-[1120px] px-5 pb-6 pt-12 md:pt-16">
        <div className="grid items-center gap-8 md:grid-cols-[1.05fr_0.95fr]">
          <div className="text-left">
            <div className="hero-eyebrow inline-flex items-center gap-2 rounded-full border px-2.5 py-1.5 text-[11px] uppercase tracking-[0.08em]" style={{ background: "var(--panel)", borderColor: "var(--border)", color: "var(--text-dim)" }}>
              <span className="h-2 w-2 animate-pulse rounded-full" style={{ background: "var(--accent)", boxShadow: "0 0 0 6px color-mix(in srgb, var(--accent) 18%, transparent)" }} />
              Hyprland on NixOS
            </div>
            <h1 className="hero-title mt-[18px] text-[42px] font-extrabold leading-[0.92] tracking-[-0.04em] md:text-[52px]">
              The missing desktop shell<br /><span style={{ color: "var(--accent)" }}>for Hyprland</span>
            </h1>
            <p className="hero-lead mt-4 max-w-[480px] text-[17px] leading-relaxed" style={{ color: "var(--text-dim)" }}>
              Settings, files, search, and a shop for apps that all feel like one desktop. Hyprland finally has the calm setup it was missing.
            </p>
            <div className="hero-cta mt-6 flex flex-wrap gap-2.5">
              <a href="#install" className="magnet inline-flex items-center gap-2 rounded-full px-[18px] py-[11px] text-sm font-semibold text-white" style={{ background: "var(--accent)" }}>
                Install on NixOS <ArrowRight size={16} />
              </a>
              <a href="#apps" className="magnet inline-flex items-center gap-2 rounded-full border px-[18px] py-[11px] text-sm font-semibold" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}>
                See what's inside
              </a>
            </div>
          </div>
          <div className="hero-visual relative overflow-hidden rounded-[20px] border" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <img
              src="/shots/hero-desktop.jpg"
              alt="Settings overview on Hyprland with the finick top bar"
              width={1600} height={1120}
              className="aspect-[10/7] w-full object-cover"
              loading="eager"
            />
            <div className="absolute bottom-0 left-0 right-0 flex items-center justify-between border-t px-3 py-2.5 text-xs backdrop-blur" style={{ background: "color-mix(in srgb, var(--panel) 88%, transparent)", borderColor: "var(--border)", color: "var(--text-dim)" }}>
              <span className="flex items-center gap-2"><span className="h-2 w-2 rounded-full" style={{ background: "var(--accent)" }} /> finick running on Hyprland</span>
              <span>real screenshot</span>
            </div>
          </div>
        </div>
      </header>

      {/* STAT STRIP moved out of hero, per hero stack discipline */}
      <div className="mx-auto max-w-[1120px] px-5">
        <div className="flex justify-start gap-10 border-y py-4 md:justify-center" style={{ borderColor: "var(--border)" }}>
          <div className="leading-none"><strong className="block text-lg">6</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>apps that match each other</span></div>
          <div className="leading-none"><strong className="block text-lg">22</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>settings pages, one search</span></div>
          <div className="leading-none"><strong className="block text-lg">6</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>colors, light and dark</span></div>
        </div>
      </div>

      <section id="apps" className="mx-auto max-w-[1120px] px-5 py-9">
        <div className="mb-5 max-w-[720px]">
          <h2 className="text-[34px] font-bold leading-[0.95] tracking-[-0.03em]">One desktop,<br /><span className="font-normal" style={{ color: "var(--text-dim)" }}>everything matches.</span></h2>
          <p className="mt-3 max-w-[560px] text-sm" style={{ color: "var(--text-dim)" }}>Settings, files, search, and the app shop share the same look and the same theme. Pick a color once; try it up top.</p>
        </div>

        <div className="bento grid gap-3.5 md:grid-cols-3">
          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 overflow-hidden rounded-[20px] border md:row-span-2" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <img src="/shots/settings.jpg" alt="Settings app showing network and display controls" className="h-[160px] w-full object-cover" loading="lazy" />
            <div className="flex flex-col gap-2.5 p-[18px] pt-3">
              <div className="flex items-center justify-between">
                <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #5B5FE9 16%, var(--panel))", borderColor: "color-mix(in srgb, #5B5FE9 18%, var(--border))", color: "#5B5FE9" }}><Settings2 size={16} /></span>
                <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>App</span>
              </div>
              <h3 className="text-lg font-semibold tracking-[-0.02em]">Settings</h3>
              <p className="text-[13px] leading-[1.55]" style={{ color: "var(--text-dim)" }}>Wi-Fi, sound, displays, battery, and more in one calm place with a search that actually finds things.</p>
              <div className="flex flex-wrap gap-1.5">
                {["Wi-Fi", "Bluetooth", "Sound", "Displays", "Battery", "Appearance"].map(s => (
                  <span key={s} className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>{s}</span>
                ))}
              </div>
              <div className="mt-auto flex items-center justify-between border-t pt-2.5 text-xs" style={{ borderColor: "var(--border)" }}><span style={{ color: "var(--text-dim)" }}>22 pages, one search</span></div>
            </div>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #2CA6A0 16%, var(--panel))", borderColor: "color-mix(in srgb, #2CA6A0 18%, var(--border))", color: "#2CA6A0" }}><Folder size={16} /></span>
              <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>App</span>
            </div>
            <h3 className="text-lg font-semibold">Files</h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Your folders, favorites, and previews in a quick window; search keeps up while you type.</p>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 overflow-hidden rounded-[20px] border" style={{ background: "var(--sidebar)", borderColor: "var(--border)" }}>
            <img src="/shots/topbar.jpg" alt="Top bar overlay on Hyprland desktop" className="h-[132px] w-full object-cover" loading="lazy" />
            <div className="flex flex-col gap-2 p-[18px] pt-3">
              <div className="flex items-center justify-between">
                <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #FF6952 16%, var(--panel))", borderColor: "color-mix(in srgb, #FF6952 18%, var(--border))", color: "#FF6952" }}><Layers size={16} /></span>
                <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel)", borderColor: "var(--border)", color: "var(--text-dim)" }}>Built in</span>
              </div>
              <h3 className="text-lg font-semibold">Top bar</h3>
              <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Sound, Wi-Fi, battery, clipboard, and notifications in one strip; quick settings sit one click away.</p>
            </div>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #E3A23D 16%, var(--panel))", borderColor: "color-mix(in srgb, #E3A23D 18%, var(--border))", color: "#E3A23D" }}><Box size={16} /></span>
                <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>Built in</span>
              </div>
              <h3 className="text-lg font-semibold">Always in sync</h3>
              <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Nudge the volume or swap the theme and every window follows right away.</p>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #E85A88 16%, var(--panel))", borderColor: "color-mix(in srgb, #E85A88 18%, var(--border))", color: "#E85A88" }}><Search size={16} /></span>
                <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>Built in</span>
              </div>
              <h3 className="text-lg font-semibold">Fast search</h3>
              <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Apps and files appear while you type; picks and recent files wait on an empty search.</p>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #5B5FE9 16%, var(--panel))", borderColor: "color-mix(in srgb, #5B5FE9 18%, var(--border))", color: "#5B5FE9" }}><Box size={16} /></span>
              <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>App</span>
            </div>
            <h3 className="text-lg font-semibold">App shop</h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Add and remove apps in a friendly window, and decide what each one may touch.</p>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
              <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #E3A23D 16%, var(--panel))", borderColor: "color-mix(in srgb, #E3A23D 18%, var(--border))", color: "#E3A23D" }}><Search size={16} /></span>
              <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>App</span>
            </div>
            <h3 className="text-lg font-semibold">Search</h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>One spot to find apps and files, run quick math, and hop straight to what you need.</p>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <div className="flex items-center justify-between">
                <span className="float-icon grid h-9 w-9 place-items-center rounded-[10px] border" style={{ background: "color-mix(in srgb, #64748B 16%, var(--panel))", borderColor: "color-mix(in srgb, #64748B 18%, var(--border))", color: "var(--text-dim)" }}><Moon size={16} /></span>
              <span className="rounded-full border px-2 py-1 text-[11px]" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}>App</span>
            </div>
            <h3 className="text-lg font-semibold">Lock screen</h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Your wallpaper and clock on a quiet screen that wakes the moment you sign back in.</p>
          </article>

          <article className="bento-card flex min-h-[220px] flex-col gap-2.5 rounded-[20px] border p-[18px]" style={{ background: `linear-gradient(180deg, color-mix(in srgb, var(--accent) 14%, var(--panel)), var(--panel))`, borderColor: "var(--border)" }}>
            <h3 className="text-lg font-semibold">One look <span style={{ color: "var(--accent)" }}>everywhere</span></h3>
            <p className="text-[13px]" style={{ color: "var(--text-dim)" }}>Light, dark, and six colors to make it yours; every window keeps the same rounded style.</p>
          </article>
        </div>
      </section>

      <section id="system" className="mx-auto max-w-[1120px] px-5">
        <div className="rounded-[20px] border p-5 md:p-[22px]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
          <div className="mb-5 max-w-[640px]">
            <h2 className="text-[30px] font-bold leading-[0.95] tracking-[-0.03em]">One install, everything included.</h2>
            <p className="mt-3 max-w-[560px] text-sm" style={{ color: "var(--text-dim)" }}>Settings, files, top bar, search, lock screen, and the app shop arrive together and update with the rest of your setup.</p>
          </div>

          <div className="arch grid gap-3.5 md:grid-cols-3">
            <div className="arch-col">
              <h4 className="mb-2.5 text-xs font-semibold" style={{ color: "var(--text-dim)" }}>Your apps</h4>
              <div className="flex flex-col gap-2.5">
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Settings</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Wi-Fi, sound, displays, battery, and more, with search</span></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Files</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Folders, favorites, and quick previews</span></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">App shop</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Add apps and decide what each may touch; extras stay in step with your setup</span></div>
              </div>
            </div>
            <div className="arch-col">
              <h4 className="mb-2.5 text-xs font-semibold" style={{ color: "var(--text-dim)" }}>Always with you</h4>
              <div className="flex flex-col gap-2.5">
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Top bar</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Quick settings, clipboard, screenshots, and notifications</span></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Search</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Apps, files, and quick math in one spot</span></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Lock screen</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Wallpaper, clock, and a fast way back in</span></div>
              </div>
            </div>
            <div className="arch-col">
              <h4 className="mb-2.5 text-xs font-semibold" style={{ color: "var(--text-dim)" }}>Your style</h4>
              <div className="flex flex-col gap-2.5">
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Light and dark</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Every window follows the same theme, right away</span></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Six colors</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Try them in the bar above; the whole page follows</span></div>
                <div className="rounded-2xl border p-3" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Focus and quiet</strong><span className="block text-xs" style={{ color: "var(--text-dim)" }}>Do-not-disturb hours, night light, and calm alerts</span></div>
              </div>
            </div>
          </div>

          <div className="mt-3.5 grid gap-3.5 md:grid-cols-2">
            <div className="rounded-2xl border p-3" style={{ background: "var(--bg)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Opens fast, stays light</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>Windows pop open without a wait, even on older laptops, since search keeps its own small index.</span></div>
            <div className="rounded-2xl border p-3" style={{ background: "var(--bg)", borderColor: "var(--border)" }}><strong className="block text-[13px]">Updates with your system</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>One rebuild brings the desktop along; no separate downloads to babysit.</span></div>
          </div>
        </div>
      </section>

      <section id="integrations" className="mx-auto max-w-[1120px] px-5 py-9">
        <div className="mb-5 max-w-[720px]">
          <h2 className="text-[34px] font-bold leading-[0.95] tracking-[-0.03em]">Works with your hardware.</h2>
          <p className="mt-3 max-w-[560px] text-sm" style={{ color: "var(--text-dim)" }}>Wi-Fi, sound, displays, battery, and time each get a plain window with sliders and switches that do what they say.</p>
        </div>

        <div className="integrations grid gap-3.5 md:grid-cols-4">
          <div className="int-card rounded-[20px] border p-4" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <h4 className="mb-2 flex items-center gap-2 text-[13px] font-semibold"><Wifi size={14} style={{ color: "var(--accent)" }} /> Internet and devices</h4>
            <p className="text-[13px] leading-relaxed" style={{ color: "var(--text-dim)" }}>Join Wi-Fi and connect headphones from one window; saved networks reconnect on their own.</p>
          </div>
          <div className="int-card rounded-[20px] border p-4" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <h4 className="mb-2 flex items-center gap-2 text-[13px] font-semibold"><Monitor size={14} style={{ color: "var(--accent)" }} /> Sound and screen</h4>
            <p className="text-[13px] leading-relaxed" style={{ color: "var(--text-dim)" }}>Volume, brightness, and night light sliders; pick speakers or headphones without digging.</p>
          </div>
          <div className="int-card rounded-[20px] border p-4" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <h4 className="mb-2 flex items-center gap-2 text-[13px] font-semibold"><HardDrive size={14} style={{ color: "var(--accent)" }} /> Power and space</h4>
            <p className="text-[13px] leading-relaxed" style={{ color: "var(--text-dim)" }}>Battery, storage, and time stay correct while you work, with warnings before things run out.</p>
          </div>
          <div className="int-card rounded-[20px] border p-4" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
            <h4 className="mb-2 flex items-center gap-2 text-[13px] font-semibold"><Settings2 size={14} style={{ color: "var(--accent)" }} /> Printing and language</h4>
            <p className="text-[13px] leading-relaxed" style={{ color: "var(--text-dim)" }}>Add a printer, pick a language, and set shortcuts that stick around after a reboot.</p>
          </div>
        </div>
      </section>

      <section id="install" className="mx-auto max-w-[1120px] px-5 pb-9">
        <div className="install grid gap-[18px] rounded-[20px] border p-[18px] md:grid-cols-[0.9fr_1.1fr]" style={{ background: "var(--panel)", borderColor: "var(--border)" }}>
          <div>
            <h2 className="text-[30px] font-bold leading-[0.95] tracking-[-0.03em]">Install it with<br />the rest of your system.</h2>
            <p className="mt-2 text-[13px]" style={{ color: "var(--text-dim)" }}>Add finick to your setup, rebuild, and log back in. Settings, top bar, files, search, and the shop land together.</p>
            <div className="mt-3.5 flex flex-wrap gap-2.5">
              <a href="https://github.com/fargonesh/finick" target="_blank" className="inline-flex items-center gap-2 rounded-full px-4 py-2.5 text-sm font-semibold text-white" style={{ background: "var(--accent)" }}>View on GitHub <ArrowRight size={14} /></a>
              <button onClick={() => copy('inputs.finick.url = "github:fargonesh/finick";', "run")} className="inline-flex items-center gap-1.5 rounded-full border px-4 py-2.5 text-sm font-semibold" style={{ background: "var(--panel-raised)", borderColor: "var(--border)" }}>
                {copied === "run" ? <Check size={14} /> : <Copy size={14} />} {copied === "run" ? "Copied" : "Copy flake input"}
              </button>
            </div>
          </div>
          <div className="install-card overflow-hidden rounded-2xl border" style={{ background: "var(--bg)", borderColor: "var(--border)" }}>
            <div className="flex h-9 items-center gap-2 border-b px-3 text-xs" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--text-dim)" }}><span className="h-2.5 w-2.5 rounded-full" style={{ background: "var(--accent)" }} /> flake.nix plus configuration.nix</div>
            <pre className="overflow-auto p-3.5 text-[12.5px] leading-6" style={{ color: "var(--text-dim)" }}><code>{`{
  inputs.finick.url = "github:fargonesh/finick";
  outputs = { finick, nixpkgs, ... }: {
    nixosConfigurations.yourHost = nixpkgs.lib.nixosSystem {
      modules = [
        finick.nixosModules.default
        { programs.finick.enable = true; }
      ];
    };
  };
}
# then
$ nixos-rebuild switch --flake .#yourHost`}</code></pre>
          </div>
        </div>
      </section>
      </>) : (<Docs slug={docSlug} />)}
      </main>

      <footer className="border-t" style={{ borderColor: "var(--border)", background: "color-mix(in srgb, var(--panel) 60%, transparent)" }}>
        <div className="mx-auto flex max-w-[1120px] flex-wrap items-center justify-between gap-3 px-5 py-[18px]">
          <div className="flex items-center gap-2.5">
            <span className="grid h-8 w-8 place-items-center rounded-full border text-[13px] font-bold" style={{ background: "var(--panel-raised)", borderColor: "var(--border)", color: "var(--accent)" }}>F</span>
            <div><strong className="block text-sm">finick</strong><span className="text-xs" style={{ color: "var(--text-dim)" }}>A calm desktop for Hyprland on NixOS</span></div>
          </div>
          <div className="flex items-center gap-3.5 text-[13px]" style={{ color: "var(--text-dim)" }}>
            <a href="https://github.com/fargonesh/finick" className="hover:text-[var(--text)]">GitHub</a>
            <a href="#/docs/intro" className="hover:text-[var(--text)]">Docs</a>
            <a href="#apps" className="hover:text-[var(--text)]">Apps</a>
            <a href="#system" className="hover:text-[var(--text)]">What's inside</a>
            <span>2026 fargone</span>
          </div>
        </div>
      </footer>
    </div>
  )
}
