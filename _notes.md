# General
- [x] settings-daemon should allow things (topbar, settings-app) to connect to an ipc similar to a websocket meaning changes like overall font-size or app-specific settings can be stored and update live :) 

# Overlay/Topbar
- [x] Topbar was renamed to overlay (as it does more than a top bar now), but both folders exist. Reconsile.
- [x] Clicking the toggle does not always toggle, sometimes takes too long, and sometimes spawn the window on the wrong monitor or in the middle of the screen
- [x] Connected text should not be a pill, should be much smaller
- [x] Panel should be about 15% wider
- [x] Brightness panel shows even when display doesn't support dimming
- [x] Settings button does not work
- [x] Notifications not showing on screen or in control panel

# Settings
- [x] Remove "Theme: X" and Colour pills from topbar
- [x] Remove "Studio connected" from top left
- [x] Top left user section should go to accounts on click
  - [x] Should also use current user icon if one exists
- [x] **Overview Page**
  - [x] Appearence colours + Desktop Background Selections show outdated fields (See Appearence page)
  - [x] Focus Settings (Off, Work, Personal, Sleep) should be side-by-side, not on top of each other
- [x] **Wi-Fi Page**
  - [x] Not showing nearby networks
- [x] **Bluetooth Page**
  - [x] Remove Disconnect button
  - [x] Move connected into where the MAC address is currently (Show Connected (dot) MAC:ADDRESS)
  - [x] Add hover BG and on click Connect/Pair or Disconnect
  - [x] Nearby devices not showing up
  - [x] All devices are currently showing up as mac adresses
- [x] **Appearence Page**
  - [x] Remove 'Material You'
  - [x] Merge both picked colours and default accent colours into one row, last one should be a '+' which initiates colour picker (remove pick colour button)
  - [x] Colours are not being picked from wallpaper. Please wire up. (Use pywal or mutagen or wtvr)
  - [x] Remove the full path of a custom background
  - [x] Interface settings do nothing, should update all finick apps (UI lib should have hooks for this i guess)
- [x] **Display Page**
  - [x] Remove 'True Tone' (Does nothing)
  - [x] Wire up Night Shift to do things (Either with hyprctl or through some trickery with [overlay](./apps/overlay/))
- [x] **Sound Page**
  - [x] Input bar is horribly slow
  - [x] None of the dropdowns work (No pointer change on hover, dropdown doesn't open on click, things change completely unexpectedly on click) 
- [x] **Screen Time**
  - [x] Wire up tracking into [daemon](./services/settings-daemon/). Leave the rest for now
- [x] **Desktop Page**
  - [x] Remove workspaces, dock panes
  - [x] Remove 'Gaps are applied live...' text
  - [x] Add another slider for inner gaps
  - [x] Control Panel + Top-bar text do not change things in the overlay view. Wire this up via IPC
  - [x] Remove top-bar text preview
- [x] **Storage Page**
  - [x] Wire up file type size discovery via [index service](./services/index/)
  - [x] Remove 'Save new files to Cloud'
  - [x] Remove 'Review large files' button
- [x] **Battery Page**
  - [x] Replace power mode thing with actual picker like light/dark switch. Make it change things via powermanagement
- [x] **Accessibility Page**
  - [x] Wire up accessibility settings to make changes to UI.
- [x] **Accounts Page**
  - [x] Allow changing the users icon via picking an image and using imagemagick

# Files
- [x] Sidebar is so poorly arranged lol (also cant close???)
- [x] Not actually opening things on double-click or 'Open File'
- [x] Right-clicking a file should select it
- [x] Cant drag-select
- [x] Current location input is gone? Or invisible?
- [x] UI things disappear weirdly on window not being wide enough
- [x] 'Add Place' modal is off the bottom of the screen lol