// Mirrors docs/SUMMARY.md. `file` is the basename synced into content/.
export const SECTIONS = [
  {
    title: "Getting started",
    pages: [
      { slug: "intro", title: "Introduction", file: "intro.md" },
      { slug: "install", title: "Install", file: "install.md" },
      { slug: "nixos-module", title: "NixOS / home-manager module", file: "nixos-module.md" },
    ],
  },
  {
    title: "Using finick",
    pages: [
      { slug: "using-settings", title: "Settings", file: "using-settings.md" },
      { slug: "using-files", title: "Files", file: "using-files.md" },
      { slug: "using-launcher", title: "Launcher", file: "using-launcher.md" },
      { slug: "using-topbar", title: "Top bar and overlay", file: "using-topbar.md" },
      { slug: "using-store", title: "App shop", file: "using-store.md" },
      { slug: "using-locker", title: "Lock screen", file: "using-locker.md" },
    ],
  },
  {
    title: "Reference",
    pages: [
      { slug: "finickctl", title: "finickctl", file: "finickctl.md" },
      { slug: "troubleshooting", title: "Troubleshooting", file: "troubleshooting.md" },
    ],
  },
  {
    title: "Contributing",
    pages: [
      { slug: "architecture", title: "Architecture", file: "architecture.md" },
      { slug: "design-system", title: "Design system", file: "design_system.md" },
      { slug: "logical-system", title: "Logical system", file: "logical_system.md" },
    ],
  },
]

export const ALL_PAGES = SECTIONS.flatMap((s) =>
  s.pages.map((p) => ({ ...p, section: s.title }))
)

export const slugForFile = (name) =>
  ALL_PAGES.find((p) => p.file === name || p.file === `${name}.md`)?.slug
