### finick environment
builds on top of hyperland to provide things every system needs in a pretty + nice to use way yaaay :D


### things
| Thing       | Category | Path                                                    |
| ----------- | -------- | ------------------------------------------------------- |
| Settings    | App      | [apps/settings](./apps/settings/)                       |
| \ (D)       | Service  | [services/settings-daemon](./services/settings-daemon/) |
| Files       | App      | [apps/files](./apps/files/)                             |
| \ Index     | Service  | [services/index](./services/index/)                     |
| Top Bar     | Shell    | n/a                                                     |
| UI          | Lib      | [libs/ui](./libs/ui)                                    |
| IPC         | Lib      | [libs/ipc](./libs/ipc/)                                 |
| Config      | Lib      | [libs/config](./libs/config/)                           |
| System Info | Lib      | [libs/system](./libs/system/)                           |

### Settings (App + Daemon)
Settings for the device + the environment, and the daemon that monitors stuff. Also provides notifications + notification control.

### Files (App + Index)
File browser + indexing service

### Top Bar
Gnome-style top bar with control panel (Settings(D) via CLI)

#### todo 
- [ ] Wire up `ipc` & `config` in all the apps
- [ ] Finish implimenting all the settings in `settings`
- [ ] Think about how to impliment changes that could break a nix flake (can we lock settings that are restricted?)
- [ ] Start work on the top bar app with a control panel using `ipc` to talk to `settings-daemon`\
- [ ] Impliment new UI changes into `files`
- [ ] Get nix flake working with all the apps and setup all the xdg-open things (file picker, etc.)
- [ ] Think about more apps to add for a full environment that existing solutions look or feel bad