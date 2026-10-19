# finickctl

`finickctl -p <program> [-s DATA] [-j]`. Add `-j` for JSON where supported.

## index

```sh
finickctl -p index -s "report"
finickctl -p index -j -s "report"
```

Queries `services/index`. Suffixes: `/` directory, `@` desktop entry, `*` executable.

## accent / border

```sh
finickctl -p accent -s Teal
finickctl -p accent -s "#5B5FE9"
finickctl -p border -s 2
finickctl -p border -s "0xff5B5FE9"
```

Accent names (`Indigo`, `Coral`, `Amber`, `Teal`, `Rose`, `Slate`) or hex. Accent also pushes `general:col.active_border` via `hyprctl` and writes `SettingKey::AccentColor` through `finickd`.

## settings

```sh
finickctl -p settings -s "accent_color"          # get
finickctl -p settings -s "accent_color=Teal"     # set via finickd
```

Key parsing uses `SettingKey`; accent sets also update the Hyprland border.

## notify

```sh
finickctl -p notify -s "Finick|Test Notification"
```

Sends over DBus (`org.freedesktop.Notifications`) via `busctl`. Fails loudly if the daemon isn't running.

## modal

```sh
finickctl -p modal -s "pam:Unlock to continue"
finickctl -p modal -s "wifi:MySSID,WPA2"
finickctl -p modal -s "bt:AA:BB:CC:DD:EE:FF,Headphones"
```

`pam:`, `wifi:ssid,security`, `bt:mac,name` → `ModalRequest`, response printed (JSON with `-j`).

## launcher

```sh
finickctl -p launcher            # start (or dispatch via hyprctl)
finickctl -p launcher -s toggle  # toggle via /tmp/finick-launcher.lock
```

Hyprland hint: `bind = SUPER, SPACE, exec, launcher`.

## apps

See [App shop](./using-store.md) for the full `search | list | install-nix | remove-nix | install-flatpak | remove-flatpak | link | apply` flow.
