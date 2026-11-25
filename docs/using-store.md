# App shop

`apps/store`: add/remove apps in a friendly window with per-app permission decisions. Nix and build logs are summarized to one honest line with at most ~30 detail lines behind a Details toggle, plus Copy log / Save log and Retry — recovery stays inside the window.

## Desired state, not one-click install

The shop is declarative. Picking packages stages them into the generated `apps.nix`; **Apply** rebuilds via home-manager. Removing a package removes the declaration — run Apply to rebuild.

- `programs.finick.store.extraPackages = [ "firefox" "vlc" ]` — nixpkgs attrs
- `programs.finick.store.flatpaks = [ "com.spotify.Client" ]` — flatpak IDs

## Headless flow (`finickctl -p apps`)

```sh
finickctl -p apps -s "search firefox"
finickctl -p apps -s "list"
finickctl -p apps -s "install-nix firefox"
finickctl -p apps -s "remove-nix firefox"
finickctl -p apps -s "install-flatpak com.spotify.Client"
finickctl -p apps -s "remove-flatpak com.spotify.Client"
finickctl -p apps -s "link"    # ensure the generated file is imported
finickctl -p apps -s "apply"   # rebuild home-manager
```

Add `-j` for JSON on `search` / `list`. Remove dialogs show package name, source + short id, and timing ("Removes now" vs "Removes the declaration, run Apply to rebuild").
