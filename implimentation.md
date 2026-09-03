## Plan: Finick Full Environment Roadmap

Recommended approach: deliver a production-ready v1 in dependency order (foundation -> settings platform -> files platform -> shell -> nix rollout), while deferring stretch apps until core app/service reliability and reproducible deployment are complete.

**Steps**
1. [x] Phase 0 - Foundation and quality gates
- [x] 1.1. Normalize workspace health so later work is not blocked by lint noise or fragile IO paths.
- [x] 1.2. Fix core warnings and baseline reliability in [libs/config/src/lib.rs](libs/config/src/lib.rs), [services/index/src/lib.rs](services/index/src/lib.rs), [apps/settings/src/detail_pages.rs](apps/settings/src/detail_pages.rs), and [libs/ui/src/theme.rs](libs/ui/src/theme.rs).
- [x] 1.3. Define acceptance gates for each phase: compiles cleanly, key user flows work, no critical unwrap panic paths in service hot paths.
- [x] 1.4. Depends on nothing; blocks all later phases.

2. [x] Phase 1 - Shared contracts and persistence model
- [x] 2.1. Finalize canonical settings schema and ownership boundaries: app handles UX, daemon owns persistence and system application.
- [x] 2.2. Extend app identity and persistence support in [libs/config/src/ty.rs](libs/config/src/ty.rs) and [libs/config/src/lib.rs](libs/config/src/lib.rs) for Settings-centric config payloads.
- [x] 2.3. Define Settings IPC request/response/event contract within [libs/ipc/src/lib.rs](libs/ipc/src/lib.rs) module structure (or sibling module under same directory), reusing existing stream patterns already used by Index.
- [x] 2.4. Add explicit setting lock metadata for Nix-restricted keys so UI can mark keys read-only before backend apply.
- [x] 2.5. Depends on Phase 0; blocks Phases 2, 3, and 4.

3. [x] Phase 2 - Settings daemon implementation (core backend)
- [x] 3.1. Replace daemon stub loop with real IPC server and request handling in [services/settings-daemon/src/main.rs](services/settings-daemon/src/main.rs).
- [x] 3.2. Implement startup load, get/set/apply operations, and persisted writes through config library.
- [x] 3.3. Add event subscription channel for changed settings and daemon-originated alerts.
- [x] 3.4. Route system-change execution through backend abstractions in [libs/system/src/lib.rs](libs/system/src/lib.rs), moving direct imperative side effects out of UI where practical.
- [x] 3.5. Add coarse policy checks for dangerous settings changes, including lock enforcement and human-readable denial reasons.
- [x] 3.6. Depends on Phase 1; blocks Phase 3 and Phase 4 integration.

4. [x] Phase 3 - Settings app wiring and completion
- [x] 4.1. Integrate daemon client lifecycle in [apps/settings/src/main.rs](apps/settings/src/main.rs): initial load, optimistic updates, rollback on backend rejection.
- [x] 4.2. Incrementally wire page groups in [apps/settings/src/pages](apps/settings/src/pages) and [apps/settings/src/detail_pages.rs](apps/settings/src/detail_pages.rs): Connectivity, Personalization, System.
- [x] 4.3. For each page, replace in-memory-only state with persisted state and daemon apply flow.
- [x] 4.4. Add lock-state visuals and disabled controls for Nix-restricted settings.
- [x] 4.5. Preserve Freya layout constraints documented in [AGENTS.md](AGENTS.md) and [memories/repo/layout-conventions.md](memories/repo/layout-conventions.md).
- [x] 4.6. Depends on Phase 2. Can run partially in parallel by page area once core app client scaffolding is complete.

5. Phase 4 - Files and Index production hardening
5.1. Stabilize index-service panic paths and error surfaces in [services/index/src/lib.rs](services/index/src/lib.rs).
5.2. Add missing essential file-manager workflows in [apps/files/src/main.rs](apps/files/src/main.rs): rename, safer delete strategy, multi-select/bulk actions, hidden file toggle, keyboard shortcuts.
5.3. Add user preference persistence for files UX via config library and route search/list behavior through existing IPC reliably.
5.4. Improve watch consistency for move/rename/reindex recovery and add manual rebuild command integration via [finickctl/src/main.rs](finickctl/src/main.rs).
5.5. Depends on Phase 1 for config conventions. Most of this phase is parallel with Phase 3 after Phase 2 is stable.

6. Phase 5 - Top bar shell and control panel
6.1. Create Top Bar app package under existing apps workspace and reuse shared UI patterns from [libs/ui/src/components](libs/ui/src/components), especially [libs/ui/src/components/topbar.rs](libs/ui/src/components/topbar.rs).
6.2. Implement quick settings control panel using daemon IPC: wifi, bluetooth, sound, brightness, notifications/focus, power actions.
6.3. Add live state subscriptions so top bar reflects daemon updates without polling.
6.4. Start with Freya app delivery for v1 speed; plan a later migration path to deeper layer-shell semantics if needed.
6.5. Depends on Phase 2; can overlap final part of Phase 4.

7. Phase 6 - Nix and deployment integration
7.1. Align settings module behavior and real binaries in [apps/settings/nix/settings-module.nix](apps/settings/nix/settings-module.nix).
7.2. Create reproducible package outputs for apps/services and validate service definitions from module assumptions.
7.3. Wire xdg-open and MIME defaults so Files app is first-class for file handling.
7.4. Ensure devenv process model in [devenv.nix](devenv.nix) matches deployment service topology.
7.5. Depends on Phases 2, 4, and 5 for realistic service/app names and startup contracts.

8. Phase 7 - End-to-end environment validation and launch readiness
8.1. Run integrated workflow tests: boot daemon, launch settings/files/topbar, mutate settings, verify persistence and system effect.
8.2. Validate failure modes: daemon unavailable, stale index DB, invalid setting payload, locked-setting mutation attempt.
8.3. Publish v1 readiness checklist and known limitations; only then branch into additional new apps from overview backlog.
8.4. Depends on all prior phases.

**Relevant files**
- [overview.md](overview.md) - source scope for environment goals and todo alignment.
- [apps/settings/src/main.rs](apps/settings/src/main.rs) - app composition and global state wiring to daemon.
- [apps/settings/src/detail_pages.rs](apps/settings/src/detail_pages.rs) - large settings page logic currently containing many in-memory flows.
- [apps/settings/src/pages](apps/settings/src/pages) - page-level integration points for persisted settings.
- [services/settings-daemon/src/main.rs](services/settings-daemon/src/main.rs) - currently stubbed daemon entrypoint to replace.
- [libs/ipc/src/lib.rs](libs/ipc/src/lib.rs) - reusable IPC transport and stream protocol patterns.
- [libs/config/src/lib.rs](libs/config/src/lib.rs) - persistence read/write behavior and file open semantics.
- [libs/config/src/ty.rs](libs/config/src/ty.rs) - app identity types for config targeting.
- [libs/system/src/lib.rs](libs/system/src/lib.rs) - backend boundary for system application logic.
- [apps/files/src/main.rs](apps/files/src/main.rs) - files UX and index IPC client behavior.
- [services/index/src/lib.rs](services/index/src/lib.rs) - indexing/search/list/watch reliability and performance.
- [finickctl/src/main.rs](finickctl/src/main.rs) - operational tooling surface for scan/rebuild commands.
- [libs/ui/src/components/topbar.rs](libs/ui/src/components/topbar.rs) - base shell UI primitive for top bar app.
- [apps/settings/nix/settings-module.nix](apps/settings/nix/settings-module.nix) - NixOS module integration for settings daemon/app.
- [devenv.nix](devenv.nix) - local process orchestration baseline.
- [AGENTS.md](AGENTS.md) - Freya layout rule required during UI changes.

**Verification**
1. Workspace quality gate: cargo build --workspace and cargo clippy --workspace pass at end of each phase.
2. Settings contract tests: daemon request/response integration tests validate get/set/apply/subscription paths with persisted restarts.
3. Settings UX tests: user changes survive restart and reflect actual system state for representative features (wifi, bluetooth, sound, timezone, appearance).
4. Files regression tests: list/search performance, rename/delete safety, fallback behavior when index service is unavailable.
5. Top bar integration tests: toggles mutate daemon state and render live updates.
6. Nix validation: module options apply cleanly, services start, and xdg-open routes to Files app.
7. End-to-end smoke: fresh session startup reaches usable desktop flow without manual service patching.

**Decisions**
- Assumption: roadmap targets production-ready v1 first, then optional expansion.
- Assumption: top bar ships first as Freya-based implementation for velocity; layer-shell-level behavior is a post-v1 enhancement path.
- Assumption: Nix rollout prioritizes NixOS module plus local devenv alignment before broad packaging complexity.
- Included scope: Settings app/daemon, Files/index, top bar shell, IPC/config wiring, Nix integration, reliability gates.
- Excluded until post-v1: additional new environment apps not listed as core dependencies for current overview deliverable.

**Further Considerations**
1. Settings schema versioning policy should be finalized early (simple integer migration chain is recommended) to avoid breakage after first release.
2. Permission model depth can start with lock/deny at key level and evolve later to role-based controls if multi-user admin workflows become required.
3. Top bar migration checkpoint: define explicit criteria that justify moving from Freya-only shell behavior to layer-shell integration.