### finick environment
builds on top of hyperland to provide things every system needs in a pretty + nice to use way yaaay :D


### things
| Thing       | Category | Path                                                    |
| ----------- | -------- | ------------------------------------------------------- |
| Settings    | App      | [apps/settings](./apps/settings/)                       |
| ⤷ Daemon    | Service  | [services/finickd](./services/finickd/) |
| Files       | App      | [apps/files](./apps/files/)                             |
| ⤷ Index     | Service  | [services/index](./services/index/)                     |
| Top Bar     | Shell    | [apps/topbar](./apps/topbar/)                           |
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
