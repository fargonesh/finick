# Finick agent guide
- Follow `docs/design_system.md` + `docs/logical_system.md`, as well as the skills in `.agents/skills/` (`freya`, `finick`, `anti-ai-slop-writing`).
- Relavent github project: https://github.com/orgs/fargonesh/projects/1/views/1
- Don't touch `libs/freya` without explicit consent from the develoepr
- When editing `web/`, use the screenshot cli options to generate new photos for the product shots
- The same `--screenshot` flags are the GUI debugging harness (not just product shots): `--page <slug>` deep-links to the surface, `--window-size <WxH>` pins main-window geometry (settings/files/store/launcher), `--screenshot-virtual` runs headless in an isolated Xvfb display. Full reference: `docs/architecture.md` → Dev screenshots. 