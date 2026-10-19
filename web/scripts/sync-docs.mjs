// Copies ../docs/*.md into src/docs/content/ so `docs/` stays the source of
// truth and `web/` builds hermetically with `?raw` imports.
import { copyFileSync, mkdirSync, readdirSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const from = join(root, "..", "docs")
const to = join(root, "src", "docs", "content")
mkdirSync(to, { recursive: true })

for (const f of readdirSync(from)) {
  if (f.endsWith(".md")) copyFileSync(join(from, f), join(to, f))
}
console.log(`synced docs -> web/src/docs/content (${readdirSync(to).length} files)`)
