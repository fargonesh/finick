// Raw doc sources, synced from docs/ by scripts/sync-docs.mjs.
import intro from "./content/intro.md?raw"
import install from "./content/install.md?raw"
import nixosModule from "./content/nixos-module.md?raw"
import usingSettings from "./content/using-settings.md?raw"
import usingFiles from "./content/using-files.md?raw"
import usingLauncher from "./content/using-launcher.md?raw"
import usingTopbar from "./content/using-topbar.md?raw"
import usingStore from "./content/using-store.md?raw"
import usingLocker from "./content/using-locker.md?raw"
import finickctl from "./content/finickctl.md?raw"
import troubleshooting from "./content/troubleshooting.md?raw"
import architecture from "./content/architecture.md?raw"
import designSystem from "./content/design_system.md?raw"
import logicalSystem from "./content/logical_system.md?raw"

export const CONTENT = {
  intro,
  install,
  "nixos-module": nixosModule,
  "using-settings": usingSettings,
  "using-files": usingFiles,
  "using-launcher": usingLauncher,
  "using-topbar": usingTopbar,
  "using-store": usingStore,
  "using-locker": usingLocker,
  finickctl,
  troubleshooting,
  architecture,
  "design-system": designSystem,
  "logical-system": logicalSystem,
}
