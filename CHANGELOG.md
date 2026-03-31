# Changelog

## [0.8.0] - 2026-03-27

Here we are!! A lot of new things and fixes, and a lot of active contributors.

Also, say hello to the new Tempo module!!

 ## 🚀 Features

  - feat(system_info): add configurable polling interval for updates [@romanstingler](https://github.com/romanstingler) ([#549](https://github.com/MalpenZibo/ashell/issues/549))
  - docs: add missing features, update full_config [@romanstingler](https://github.com/romanstingler) ([#547](https://github.com/MalpenZibo/ashell/issues/547))
  - Add coordinates-based weather location and improve location name formatting [@romanstingler](https://github.com/romanstingler) ([#532](https://github.com/MalpenZibo/ashell/issues/532))
  - Audio Port Icons [@levitatingpineapple](https://github.com/levitatingpineapple) ([#512](https://github.com/MalpenZibo/ashell/issues/512))
  - Tempo: Add clock format cycling feature [@romanstingler](https://github.com/romanstingler) ([#361](https://github.com/MalpenZibo/ashell/issues/361))
  - 2| Feature: open network warning [@romanstingler](https://github.com/romanstingler) ([#499](https://github.com/MalpenZibo/ashell/issues/499))
  - 1| feat: add password visibility toggle to WiFi password dialog [@romanstingler](https://github.com/romanstingler) ([#497](https://github.com/MalpenZibo/ashell/issues/497))
  - Add SIGUSR1 signal handling for visibility toggle [@romanstingler](https://github.com/romanstingler) ([#417](https://github.com/MalpenZibo/ashell/issues/417))
  - Feature: Add Tempo module [@MalpenZibo](https://github.com/MalpenZibo) ([#279](https://github.com/MalpenZibo/ashell/issues/279))
  - Dynamic menu wrapper [@MalpenZibo](https://github.com/MalpenZibo) ([#323](https://github.com/MalpenZibo/ashell/issues/323))
  - [Status: 2/2] Add format options and indicator support for brightness [@romanstingler](https://github.com/romanstingler) ([#418](https://github.com/MalpenZibo/ashell/issues/418))
  - [Status: 1/2] Add format options for audio, network, and bluetooth indicators [@romanstingler](https://github.com/romanstingler) ([#396](https://github.com/MalpenZibo/ashell/issues/396))
  - Add support for initialTitle and initialClass for Window Title Mode [@lkcv](https://github.com/lkcv) ([#430](https://github.com/MalpenZibo/ashell/issues/430))
  - [Audio 1/2] add microphone indicator to settings module [@romanstingler](https://github.com/romanstingler) ([#419](https://github.com/MalpenZibo/ashell/issues/419))
  - Add indicator_format option to MediaPlayer module [@gwiazdorrr](https://github.com/gwiazdorrr) ([#433](https://github.com/MalpenZibo/ashell/issues/433))
  - Add Time and IconAndTime battery format options [@romanstingler](https://github.com/romanstingler) ([#438](https://github.com/MalpenZibo/ashell/issues/438))
  - Add git commit hash to version output [@romanstingler](https://github.com/romanstingler) ([#434](https://github.com/MalpenZibo/ashell/issues/434))
  - [1/2 Custom] Add Text type to CustomModule and make command optional [@romanstingler](https://github.com/romanstingler) ([#422](https://github.com/MalpenZibo/ashell/issues/422))
  - Add right-click support to quick settings buttons for opening more commands [@romanstingler](https://github.com/romanstingler) ([#412](https://github.com/MalpenZibo/ashell/issues/412))
  - feat(updates): add configurable polling interval for update checks [@romanstingler](https://github.com/romanstingler) ([#444](https://github.com/MalpenZibo/ashell/issues/444))
  - [Tray: 2/2] Implement proactive StatusNotifierItem discovery [@romanstingler](https://github.com/romanstingler) ([#408](https://github.com/MalpenZibo/ashell/issues/408))
  - [Tray: 1/2] Improve tray icon lookup with fuzzy matching and system icon indexing [@romanstingler](https://github.com/romanstingler) ([#407](https://github.com/MalpenZibo/ashell/issues/407))
  - feat: Add support for optional disk name in indicator configuration [@kazie](https://github.com/kazie) ([#403](https://github.com/MalpenZibo/ashell/issues/403))
  - Fix WiFi Network Detection When No Networks Available at Startup [@sudo-Tiz](https://github.com/sudo-Tiz) ([#405](https://github.com/MalpenZibo/ashell/issues/405))
  - Add logind service to handle resume from sleep events [@romanstingler](https://github.com/romanstingler) ([#404](https://github.com/MalpenZibo/ashell/issues/404))
  - Add Nord theme to documentation [@romanstingler](https://github.com/romanstingler) ([#409](https://github.com/MalpenZibo/ashell/issues/409))
  - add scroll support for brightness slider and fix UI sync issues [@romanstingler](https://github.com/romanstingler) ([#374](https://github.com/MalpenZibo/ashell/issues/374))
  - add configurable Wayland layer support  [@romanstingler](https://github.com/romanstingler) ([#362](https://github.com/MalpenZibo/ashell/issues/362))
  - tempo: timezones in menu [@1randomguy](https://github.com/1randomguy) ([#521](https://github.com/MalpenZibo/ashell/issues/521))
  - Brightness Slider [@levitatingpineapple](https://github.com/levitatingpineapple) ([#539](https://github.com/MalpenZibo/ashell/issues/539))
  - SystemInfo: Detect bridge interfaces (ex: br0) [@TheGreatMcPain](https://github.com/TheGreatMcPain) ([#530](https://github.com/MalpenZibo/ashell/issues/530))
  - Tempo weather additional config [@1randomguy](https://github.com/1randomguy) ([#519](https://github.com/MalpenZibo/ashell/issues/519))
  - Responsive Sliders [@levitatingpineapple](https://github.com/levitatingpineapple) ([#522](https://github.com/MalpenZibo/ashell/issues/522))
  - AudioService improvements [@levitatingpineapple](https://github.com/levitatingpineapple) ([#449](https://github.com/MalpenZibo/ashell/issues/449))
  - Allow configuration of ashell on top-layer [@1randomguy](https://github.com/1randomguy) ([#494](https://github.com/MalpenZibo/ashell/issues/494))
  - Show Update button only when updates available [@denqxotl](https://github.com/denqxotl) ([#458](https://github.com/MalpenZibo/ashell/issues/458))
  - Nicer player metadata display [@jazzpi](https://github.com/jazzpi) ([#319](https://github.com/MalpenZibo/ashell/issues/319))
  - Dynamic menu wrapper [@MalpenZibo](https://github.com/MalpenZibo) ([#462](https://github.com/MalpenZibo/ashell/issues/462))

 ## 🐞 Bug fixes

  - Fix/remove clone [@romanstingler](https://github.com/romanstingler) ([#556](https://github.com/MalpenZibo/ashell/issues/556))
  - docs(config): update NVIDIA troubleshooting guidance [@romanstingler](https://github.com/romanstingler) ([#555](https://github.com/MalpenZibo/ashell/issues/555))
  - Fix/upower [@romanstingler](https://github.com/romanstingler) ([#554](https://github.com/MalpenZibo/ashell/issues/554))
  - fix(brightness): sync slider with actual brightness on menu open [@MalpenZibo](https://github.com/MalpenZibo) ([#546](https://github.com/MalpenZibo/ashell/issues/546))
  - Use UPower Percentage for single battery systems [@MalpenZibo](https://github.com/MalpenZibo) ([#543](https://github.com/MalpenZibo/ashell/issues/543))
  - Improve WiFi scanning reliability for both NetworkManager and iwd backends [@romanstingler](https://github.com/romanstingler) ([#486](https://github.com/MalpenZibo/ashell/issues/486))
  - Current day tz fix [@1randomguy](https://github.com/1randomguy) ([#538](https://github.com/MalpenZibo/ashell/issues/538))
  - Tray icon fixes: added visible option + not displaying  [@SimoneFelici](https://github.com/SimoneFelici) ([#533](https://github.com/MalpenZibo/ashell/issues/533))
  - Filter out stopped players from MPRIS player list [@romanstingler](https://github.com/romanstingler) ([#526](https://github.com/MalpenZibo/ashell/issues/526))
  - refactor: improve IWD RSSI to signal strength mapping [@romanstingler](https://github.com/romanstingler) ([#502](https://github.com/MalpenZibo/ashell/issues/502))
  - Fix NFS disk indicator support [@romanstingler](https://github.com/romanstingler) ([#517](https://github.com/MalpenZibo/ashell/issues/517))
  - Fix workspace scroll [@megabyte6](https://github.com/megabyte6) ([#468](https://github.com/MalpenZibo/ashell/issues/468))
  - fix(iwd): properly toggle WiFi by controlling all adapters [@romanstingler](https://github.com/romanstingler) ([#501](https://github.com/MalpenZibo/ashell/issues/501))
  - Fix weather indicator and menu having inconsistent data checks [@MalpenZibo](https://github.com/MalpenZibo) ([#516](https://github.com/MalpenZibo/ashell/issues/516))
  - fix: enforce 2048 character hard limit for window titles [@romanstingler](https://github.com/romanstingler) ([#506](https://github.com/MalpenZibo/ashell/issues/506))
  - Fix typo in status bar style description [@pxy1337](https://github.com/pxy1337) ([#504](https://github.com/MalpenZibo/ashell/issues/504))
  - Fix tempo calendar weekday alignment [@romanstingler](https://github.com/romanstingler) ([#484](https://github.com/MalpenZibo/ashell/issues/484))
  - Fix: wheater retry [@MalpenZibo](https://github.com/MalpenZibo) ([#480](https://github.com/MalpenZibo/ashell/issues/480))
  - Fix/tempo module width + build fix [@MalpenZibo](https://github.com/MalpenZibo) ([#467](https://github.com/MalpenZibo/ashell/issues/467))
  - [Audio 2/2]Fix/volume icon logic [@romanstingler](https://github.com/romanstingler) ([#453](https://github.com/MalpenZibo/ashell/issues/453))
  - [2/2 Custom] Fix/cleanup custom module [@romanstingler](https://github.com/romanstingler) ([#443](https://github.com/MalpenZibo/ashell/issues/443))
  - fix: Sort workspaces by index and monitor order instead of alphabetically [@romanstingler](https://github.com/romanstingler) ([#442](https://github.com/MalpenZibo/ashell/issues/442))
  - Fix workspaces on multi-monitor Niri setups [@jazzpi](https://github.com/jazzpi) ([#392](https://github.com/MalpenZibo/ashell/issues/392))
  - Fix/network settings [@romanstingler](https://github.com/romanstingler) ([#381](https://github.com/MalpenZibo/ashell/issues/381))
  - Fix WiFi Network Detection When No Networks Available at Startup [@sudo-Tiz](https://github.com/sudo-Tiz) ([#405](https://github.com/MalpenZibo/ashell/issues/405))
  - Fix VPN list when there are too many VPNs [@kushagharahi](https://github.com/kushagharahi) ([#370](https://github.com/MalpenZibo/ashell/issues/370))
  - FIX Tray: revert(tray): use find() instead of split_once() for parsing servicenames [@romanstingler](https://github.com/romanstingler) ([#431](https://github.com/MalpenZibo/ashell/issues/431))
  - Remove unnecessary spawn and re-check updates after applying them [@romanstingler](https://github.com/romanstingler) ([#380](https://github.com/MalpenZibo/ashell/issues/380))
  - Revert "Fix text color not applied to workspace numbers, Wi-Fi widget, and power widget" [@MalpenZibo](https://github.com/MalpenZibo) ([#421](https://github.com/MalpenZibo/ashell/issues/421))
  - add scroll support for brightness slider and fix UI sync issues [@romanstingler](https://github.com/romanstingler) ([#374](https://github.com/MalpenZibo/ashell/issues/374))
  - replace map().unwrap_or* with map_or_else and add inline attributes [@romanstingler](https://github.com/romanstingler) ([#372](https://github.com/MalpenZibo/ashell/issues/372))
  - Cleanup code docs [@romanstingler](https://github.com/romanstingler) ([#371](https://github.com/MalpenZibo/ashell/issues/371))
  - Fix text color not applied to workspace numbers, Wi-Fi widget, and power widget [@romanstingler](https://github.com/romanstingler) ([#368](https://github.com/MalpenZibo/ashell/issues/368))
  - Add keyboard layout change handler for Hyprland [@romanstingler](https://github.com/romanstingler) ([#367](https://github.com/MalpenZibo/ashell/issues/367))
  - Better logging when search for target output fails. [@SGumbles](https://github.com/SGumbles) ([#518](https://github.com/MalpenZibo/ashell/issues/518))
  - Handle missing rfkill binary and /dev/rfkill gracefully [@romanstingler](https://github.com/romanstingler) ([#466](https://github.com/MalpenZibo/ashell/issues/466))
  - [HF] Layer is missing [@denqxotl](https://github.com/denqxotl) ([#413](https://github.com/MalpenZibo/ashell/issues/413))

 ## 📚 Documentation

  - docs(config): update NVIDIA troubleshooting guidance [@romanstingler](https://github.com/romanstingler) ([#555](https://github.com/MalpenZibo/ashell/issues/555))
  - docs: fix grammar and wording in configuration docs [@romanstingler](https://github.com/romanstingler) ([#551](https://github.com/MalpenZibo/ashell/issues/551))
  - docs: update features in README to reflect Tempo changes [@romanstingler](https://github.com/romanstingler) ([#548](https://github.com/MalpenZibo/ashell/issues/548))
  - docs: add missing features, update full_config [@romanstingler](https://github.com/romanstingler) ([#547](https://github.com/MalpenZibo/ashell/issues/547))
  - docs: reorder module documentation sidebar positions [@romanstingler](https://github.com/romanstingler) ([#488](https://github.com/MalpenZibo/ashell/issues/488))
  - docs: add troubleshooting guide with common issues and solutions [@romanstingler](https://github.com/romanstingler) ([#477](https://github.com/MalpenZibo/ashell/issues/477))
  - docs: add Tempo module documentation and update sidebar positions [@romanstingler](https://github.com/romanstingler) ([#448](https://github.com/MalpenZibo/ashell/issues/448))
  - Add right-click command support documentation for settings indicators [@romanstingler](https://github.com/romanstingler) ([#455](https://github.com/MalpenZibo/ashell/issues/455))
  - docs: document media player indicator format configuration option [@romanstingler](https://github.com/romanstingler) ([#436](https://github.com/MalpenZibo/ashell/issues/436))
  - Docs: Update custom module documentation with detailed usage guidelines [@romanstingler](https://github.com/romanstingler) ([#426](https://github.com/MalpenZibo/ashell/issues/426))
  - docs: improve module documentation and examples [@romanstingler](https://github.com/romanstingler) ([#432](https://github.com/MalpenZibo/ashell/issues/432))
  - Add documentation for workspace grouping by monitor [@romanstingler](https://github.com/romanstingler) ([#423](https://github.com/MalpenZibo/ashell/issues/423))
  - Update window_title module documentation  [@romanstingler](https://github.com/romanstingler) ([#411](https://github.com/MalpenZibo/ashell/issues/411))

 ## 🧰 Maintenance

  - chore: update iced to latest master [@MalpenZibo](https://github.com/MalpenZibo) ([#550](https://github.com/MalpenZibo/ashell/issues/550))
  - Refactor NetworkDialogState construction with helper methods [@romanstingler](https://github.com/romanstingler) ([#536](https://github.com/MalpenZibo/ashell/issues/536))
  - Optimize tray icon name handling with OsString and Cow [@romanstingler](https://github.com/romanstingler) ([#537](https://github.com/MalpenZibo/ashell/issues/537))
  - clippy [@romanstingler](https://github.com/romanstingler) ([#544](https://github.com/MalpenZibo/ashell/issues/544))
  - Optimize [@romanstingler](https://github.com/romanstingler) ([#545](https://github.com/MalpenZibo/ashell/issues/545))
  - Clippy Refactor: Use sort_by_key and pattern guards for cleaner code [@romanstingler](https://github.com/romanstingler) ([#535](https://github.com/MalpenZibo/ashell/issues/535))
  - reduce unnecessary allocations [@Follpvosten](https://github.com/Follpvosten) ([#470](https://github.com/MalpenZibo/ashell/issues/470))
  - Remove once_cell dependency and migrate to std::sync::LazyLock [@romanstingler](https://github.com/romanstingler) ([#490](https://github.com/MalpenZibo/ashell/issues/490))
  - Add deprecation warning to Clock module [@romanstingler](https://github.com/romanstingler) ([#463](https://github.com/MalpenZibo/ashell/issues/463))
  - Improve battery state handling and refactor peripheral icon selection [@romanstingler](https://github.com/romanstingler) ([#439](https://github.com/MalpenZibo/ashell/issues/439))
  - Remove deprecated AppLauncher and Clipboard modules [@romanstingler](https://github.com/romanstingler) ([#401](https://github.com/MalpenZibo/ashell/issues/401))
  - refactor: simplify code and improve readability [@romanstingler](https://github.com/romanstingler) ([#373](https://github.com/MalpenZibo/ashell/issues/373))

  🔧 Dependency updates

  - Bump wayland-protocols from 0.32.9 to 0.32.10 [@https](https://github.com/https)://github.com/apps/dependabot ([#391](https://github.com/MalpenZibo/ashell/issues/391))
  - Bump uuid from 1.18.1 to 1.19.0 [@https](https://github.com/https)://github.com/apps/dependabot ([#390](https://github.com/MalpenZibo/ashell/issues/390))
  - Bump wayland-client from 0.31.11 to 0.31.12 [@https](https://github.com/https)://github.com/apps/dependabot ([#389](https://github.com/MalpenZibo/ashell/issues/389))
  - Bump toml from 0.9.8 to 0.9.10+spec-1.1.0 [@https](https://github.com/https)://github.com/apps/dependabot ([#387](https://github.com/MalpenZibo/ashell/issues/387))
  - Bump serde_json from 1.0.145 to 1.0.148 [@https](https://github.com/https)://github.com/apps/dependabot ([#386](https://github.com/MalpenZibo/ashell/issues/386))
  - Bump actions/download-artifact from 4 to 7 [@https](https://github.com/https)://github.com/apps/dependabot ([#385](https://github.com/MalpenZibo/ashell/issues/385))
  - Bump actions/upload-artifact from 4 to 6 [@https](https://github.com/https)://github.com/apps/dependabot ([#384](https://github.com/MalpenZibo/ashell/issues/384))
  - Bump actions/checkout from 4 to 6 [@https](https://github.com/https)://github.com/apps/dependabot ([#383](https://github.com/MalpenZibo/ashell/issues/383))
  - Bump actions/setup-node from 5 to 6 [@https](https://github.com/https)://github.com/apps/dependabot ([#382](https://github.com/MalpenZibo/ashell/issues/382))

 ## Contributors

  ❤️ A big thanks to [@1randomguy](https://github.com/1randomguy), [@Follpvosten](https://github.com/Follpvosten), [@SGumbles](https://github.com/SGumbles), [@SimoneFelici](https://github.com/SimoneFelici), [@TheGreatMcPain](https://github.com/TheGreatMcPain), [@clotodex](https://github.com/clotodex), [@denqxotl](https://github.com/denqxotl), [@gwiazdorrr](https://github.com/gwiazdorrr), [@jazzpi](https://github.com/jazzpi), [@kazie](https://github.com/kazie),
  [@kushagharahi](https://github.com/kushagharahi), [@levitatingpineapple](https://github.com/levitatingpineapple), [@lkcv](https://github.com/lkcv), [@megabyte6](https://github.com/megabyte6), [@pxy1337](https://github.com/pxy1337), [@romanstingler](https://github.com/romanstingler), [@sudo-Tiz](https://github.com/sudo-Tiz) and Benedikt von Blomberg

## [0.7.0] - 2025-12-22

It’s been a long time coming, but a new release is finally here! 

Hopefully, the CI has correctly included everyone who contributed. 

Thanks to everyone for the support!

### 💥 Breaking changes

- Icons refactor. Include only a Nerdfont subset instead of the entire font [@MalpenZibo](https://github.com/MalpenZibo) ([#269](https://github.com/MalpenZibo/ashell/issues/269))

### 🚀 Features

- niri-support [@clotodex](https://github.com/clotodex) ([#328](https://github.com/MalpenZibo/ashell/issues/328))
- Allow hiding special workspaces [@fdev31](https://github.com/fdev31) ([#332](https://github.com/MalpenZibo/ashell/issues/332))
- Improve vpn button [@matuck](https://github.com/matuck) ([#307](https://github.com/MalpenZibo/ashell/issues/307))
- Feature: Mouse Scrolling [@EdgesFTW](https://github.com/EdgesFTW) ([#308](https://github.com/MalpenZibo/ashell/issues/308))
- Feature: multi-monitor visible indicator [@EdgesFTW](https://github.com/EdgesFTW) ([#306](https://github.com/MalpenZibo/ashell/issues/306))
- Add support for virtual desktops [@emarforio](https://github.com/emarforio) ([#214](https://github.com/MalpenZibo/ashell/issues/214))
- feat(bluetooth): change indicator icon on connected status [@sudo-Tiz](https://github.com/sudo-Tiz) ([#288](https://github.com/MalpenZibo/ashell/issues/288))
- Feat: Add MonitorSpecificExclusive visibility mode [@MalpenZibo](https://github.com/MalpenZibo) ([#287](https://github.com/MalpenZibo/ashell/issues/287))
- Feat: add custom button to settings panel [@sudo-Tiz](https://github.com/sudo-Tiz) ([#233](https://github.com/MalpenZibo/ashell/issues/233))
- Feat: Support bluetooth device management [@sudo-Tiz](https://github.com/sudo-Tiz) ([#277](https://github.com/MalpenZibo/ashell/issues/277))
- Feature peripheral battery levels [@MalpenZibo](https://github.com/MalpenZibo) ([#266](https://github.com/MalpenZibo/ashell/issues/266))
- Feat: bluetooth indicator and indicators order [@sudo-Tiz](https://github.com/sudo-Tiz) ([#276](https://github.com/MalpenZibo/ashell/issues/276))
- feat: add hibernate option to power settings [@sudo-Tiz](https://github.com/sudo-Tiz) ([#278](https://github.com/MalpenZibo/ashell/issues/278))
- feat: add temperature sensor configuration option [@sudo-Tiz](https://github.com/sudo-Tiz) ([#254](https://github.com/MalpenZibo/ashell/issues/254))
- Fuzzy search output names from config [@CodedNil](https://github.com/CodedNil) ([#312](https://github.com/MalpenZibo/ashell/issues/312))

### 🐞 Bug fixes

- Fix the reported SystemBattery percentage. [@kiryl](https://github.com/kiryl) ([#364](https://github.com/MalpenZibo/ashell/issues/364))
- Fix scroll direction + scroll touchpad sensibility [@MalpenZibo](https://github.com/MalpenZibo) ([#366](https://github.com/MalpenZibo/ashell/issues/366))
- chore: fix clippy [@MalpenZibo](https://github.com/MalpenZibo) ([#357](https://github.com/MalpenZibo/ashell/issues/357))
- Fix: Tray missing icons + Tray svg icon size [@MalpenZibo](https://github.com/MalpenZibo) ([#353](https://github.com/MalpenZibo/ashell/issues/353))
- Fix the logic of the previous PR [@fdev31](https://github.com/fdev31) ([#344](https://github.com/MalpenZibo/ashell/issues/344))
- Fix scale factor lag [@MalpenZibo](https://github.com/MalpenZibo) ([#340](https://github.com/MalpenZibo/ashell/issues/340))
- Fix: Use a fixed rev in iced dep + fix lag issue [@MalpenZibo](https://github.com/MalpenZibo) ([#337](https://github.com/MalpenZibo/ashell/issues/337))
- Fix regression [#312](https://github.com/MalpenZibo/ashell/issues/312), WorkspaceVisibilityMode doesn't work anymore [@MalpenZibo](https://github.com/MalpenZibo) ([#331](https://github.com/MalpenZibo/ashell/issues/331))
- Fix: Update menu scroll padding [@MalpenZibo](https://github.com/MalpenZibo) ([#309](https://github.com/MalpenZibo/ashell/issues/309))
- Chore: Minor bluetooth submenu UI fixes  [@MalpenZibo](https://github.com/MalpenZibo) ([#293](https://github.com/MalpenZibo/ashell/issues/293))
- fix(config) Make Default and Deserialize more in sync [@Siprj](https://github.com/Siprj) ([#294](https://github.com/MalpenZibo/ashell/issues/294))
- Fix: typo on Makefile [@sudo-Tiz](https://github.com/sudo-Tiz) ([#275](https://github.com/MalpenZibo/ashell/issues/275))
- Pipewire boot check [@chazfg](https://github.com/chazfg) ([#349](https://github.com/MalpenZibo/ashell/issues/349))
- Make system\_info network selection deterministic [@kylesferrazza](https://github.com/kylesferrazza) ([#315](https://github.com/MalpenZibo/ashell/issues/315))

### 📚 Documentation

- docs: improve temperature sensor configuration documentation [@romanstingler](https://github.com/romanstingler) ([#363](https://github.com/MalpenZibo/ashell/issues/363))
- Update Docs to add Niri support [@MalpenZibo](https://github.com/MalpenZibo) ([#352](https://github.com/MalpenZibo/ashell/issues/352))
- docs(appearance): font configuration cannot be hot-reloaded [@tank-bohr](https://github.com/tank-bohr) ([#290](https://github.com/MalpenZibo/ashell/issues/290))
- feat: add hibernate option to power settings [@sudo-Tiz](https://github.com/sudo-Tiz) ([#278](https://github.com/MalpenZibo/ashell/issues/278))

### 🧰 Maintenance

- chore: fix clippy [@MalpenZibo](https://github.com/MalpenZibo) ([#357](https://github.com/MalpenZibo/ashell/issues/357))
- Chore: Update website deps [@MalpenZibo](https://github.com/MalpenZibo) ([#336](https://github.com/MalpenZibo/ashell/issues/336))
- Fix VPN button capitalization [@jazzpi](https://github.com/jazzpi) ([#330](https://github.com/MalpenZibo/ashell/issues/330))
- Chore: Improvement on release workflow. Add binary, deb and rpm assets  [@MalpenZibo](https://github.com/MalpenZibo) ([#300](https://github.com/MalpenZibo/ashell/issues/300))
- CI: Copr automation + Nix build fix + Wayland compatibility [@dacrab](https://github.com/dacrab) ([#297](https://github.com/MalpenZibo/ashell/issues/297))
- Chore: Minor bluetooth submenu UI fixes  [@MalpenZibo](https://github.com/MalpenZibo) ([#293](https://github.com/MalpenZibo/ashell/issues/293))
- Chore: Icon font improvement [@MalpenZibo](https://github.com/MalpenZibo) ([#292](https://github.com/MalpenZibo/ashell/issues/292))
- Chore: Upd depbot interval + autolabel fixes [@MalpenZibo](https://github.com/MalpenZibo) ([#281](https://github.com/MalpenZibo/ashell/issues/281))
- Chore: upd rust min version + remove codegen-units = 1 [@MalpenZibo](https://github.com/MalpenZibo) ([#280](https://github.com/MalpenZibo/ashell/issues/280))
- chore: Optimize binary size [@MalpenZibo](https://github.com/MalpenZibo) ([#270](https://github.com/MalpenZibo/ashell/issues/270))
- New release system [@MalpenZibo](https://github.com/MalpenZibo) ([#261](https://github.com/MalpenZibo/ashell/issues/261))
- Suggest installation path as /usr/local/bin [@jennydaman](https://github.com/jennydaman) ([#355](https://github.com/MalpenZibo/ashell/issues/355))
- nix fmt flake.nix [@kylesferrazza](https://github.com/kylesferrazza) ([#320](https://github.com/MalpenZibo/ashell/issues/320))
- Remove flake-utils [@kylesferrazza](https://github.com/kylesferrazza) ([#316](https://github.com/MalpenZibo/ashell/issues/316))
- add rust-analyzer to devshell [@kylesferrazza](https://github.com/kylesferrazza) ([#314](https://github.com/MalpenZibo/ashell/issues/314))

### 🔧 Dependency updates

- Bump mdast-util-to-hast from 13.2.0 to 13.2.1 in /website in the npm\_and\_yarn group across 1 directory @[dependabot[bot]](https://github.com/apps/dependabot) ([#339](https://github.com/MalpenZibo/ashell/issues/339))
- Bump the npm\_and\_yarn group across 1 directory with 3 updates @[dependabot[bot]](https://github.com/apps/dependabot) ([#338](https://github.com/MalpenZibo/ashell/issues/338))
- Bump clap from 4.5.48 to 4.5.49 @[dependabot[bot]](https://github.com/apps/dependabot) ([#271](https://github.com/MalpenZibo/ashell/issues/271))
- Bump zbus from 5.11.0 to 5.12.0 @[dependabot[bot]](https://github.com/apps/dependabot) ([#285](https://github.com/MalpenZibo/ashell/issues/285))
- Bump sysinfo from 0.36.1 to 0.37.2 @[dependabot[bot]](https://github.com/apps/dependabot) ([#284](https://github.com/MalpenZibo/ashell/issues/284))
- Bump actions/checkout from 4 to 5 @[dependabot[bot]](https://github.com/apps/dependabot) ([#282](https://github.com/MalpenZibo/ashell/issues/282))
- Bump actions/setup-node from 5 to 6 @[dependabot[bot]](https://github.com/apps/dependabot) ([#283](https://github.com/MalpenZibo/ashell/issues/283))
- Bump regex from 1.11.3 to 1.12.2 @[dependabot[bot]](https://github.com/apps/dependabot) ([#272](https://github.com/MalpenZibo/ashell/issues/272))
- Bump actions/checkout from 4 to 5 @[dependabot[bot]](https://github.com/apps/dependabot) ([#264](https://github.com/MalpenZibo/ashell/issues/264))
- Update pipewire crate [@MalpenZibo](https://github.com/MalpenZibo) ([#286](https://github.com/MalpenZibo/ashell/issues/286))

### Contributors

❤️ A big thanks to [@CodedNil](https://github.com/CodedNil), [@EdgesFTW](https://github.com/EdgesFTW), [@Siprj](https://github.com/Siprj), [@chazfg](https://github.com/chazfg), [@clotodex](https://github.com/clotodex), [@dacrab](https://github.com/dacrab), [@emarforio](https://github.com/emarforio), [@fdev31](https://github.com/fdev31), [@jazzpi](https://github.com/jazzpi), [@jennydaman](https://github.com/jennydaman), [@kiryl](https://github.com/kiryl), [@kylesferrazza](https://github.com/kylesferrazza), [@matuck](https://github.com/matuck), [@romanstingler](https://github.com/romanstingler), [@sudo-Tiz](https://github.com/sudo-Tiz) and [@tank-bohr](https://github.com/tank-bohr)

## [0.6.0] - 2025-10-06

### WARNING BREAKING CHANGES

The `truncate_title_after_length` configuration has been moved
inside the `window_title` configuration section. [WindowTitle](https://malpenzibo.github.io/ashell/docs/configuration/modules/window_title)

The `system` configuration section has been renamed into `system_info`. [SystemInfo](https://malpenzibo.github.io/ashell/docs/configuration/modules/system_info)

### Added

- Add option to remove the airplane button
- Add window title configuration
- Add modes to window title module.
- Add a optional command line parameter (`config-path`) to specify
  the path to the configuration file
- Add `scale_factor` configuration to change the scaling factor of the status bar
- Add custom commands for power menu actions
- Add `enable_esc_key` configuration to close the menu with the ESC key
- Support for custom workspace naming via the `workspace_names` config option.
- Add `remove_idle_btn` to disable the idle inhibitor button from settings menu

### Changed

- Move `truncate_title_after_length` to the window_title configuration

### Fixed

- Bluetooth: use alias instead of name for device name
- Airplane button fail when the `rfkill` returns an error or is not present
- Reduced wifi rescan requests

### Thanks

A big thanks to @ineu, @tqwewe, @beeender, @Pebor, @CodedNil, @GabMus, @repomaa, @adamm-xyz, @sudo-Tiz

## [0.5.0] - 2025-05-20

### WARNING BREAKING CHANGES

The configuration switch from `yaml` to `toml` format.
The configuration file must be updated to adapt to the new format.
The `camelCase` format has been removed in favor of `snake_case`,
which better aligns with the `toml` syntax.

You could use an online tool like: <https://transform.tools/yaml-to-toml>
but remember to change the `camelCase` to `snake_case` format.

Now the configuration file is located in `~/.config/ashell/config.toml`

### Added

- Add font name configuration
- Add main bar solid and gradient style
- Add main bar opacity settings
- Add menu opacity and backdrop settings
- Add experimental IWD support as fallback for the network module
- Handle system with multiple battery
- Allow to specify custom labels for keyboard layouts
- Allow to always show a specific number of workspaces,
  whether they have windows or not
- Added custom modules and their ability to receive events from external commands

### Changed

- Change configuration file format
- Enhance the system info module adding network and disk usage
- Simplify style of "expand" button on wifi/bluetooth buttons
- Allow to specify custom labels for keyboard layouts
- Removed background on power info in menu

### Fixed

- Fix missing tray icons
- Fix hide vpn button when no vpn is configured

### Thanks

- @JumpIn-Git for fixing nix flake instruction
- @ineu for the custom labels for keyboard layouts, the `max_workspaces` settings and for fixing some bugs
- @rahatarmanahmed for the new settings button style, the new battery info style and for fixing some bugs
- Huge thanks to @clotodex for the `iwd` network support and the custom modules
- @tqwewe for fixing the audio sink menu position with bottom bar

## [0.4.1] - 2025-03-16

### Added

- Media player module

### Fixed

- Fix bluetooth service in NixOS systems
- Fix brightness wrong value in some situations
- Fix settings menu not resetting it's state when closed
- Fix bluetooth service crash during listing of devices without battery info
- Fix centerbox children positioning

### Thanks

- Huge thanks to @mazei513 for the implementation of the media player module

## [0.4.0] - 2025-01-19

A big update with new features and new configurations!

The configuration file must be updated to adapt to the new stuff.

### Added

- Multi monitor support
- Tray module
- Dynamic modules system configuration
- New workspace module configuration

### Changed

- Update to pop-os Iced 14.0-dev
- Dynamic menu positioning

### Thanks

- @fiersik for participating in the discussions
- @ReshetnikovPavel for the proposal of the new dynamic modules system configuration

## [0.3.1] - 2024-12-13

### Fixed

- Fix upower service startup fail in case of missing `org.freedesktop.UPower.PowerProfiles` dbus interface

## [0.3.0] - 2024-11-26

A small release with some new Hyprland related modules

Thanks @fiersik for the new modules and contributions to the project

### Added

- Hyprland Keyboard Layout module
- Hyprland Keyboard Submap module

### Changed

- Change main surface layer from Top to Bottom

## [0.2.0] - 2024-11-08

### Added

- Support for special workspaces

### Fixed

- Ashell crash when the title module try to split a multi-byte character
- Removed fixed monitor name in the workspace module
- Fix privacy webcam usage check during initialization
- Fix microphone selection
- Fix sink and source slider toggle button state
- Fix brightness initial value

### Thanks

- @fiersik for all the feedback
- @leftas for the PRs to fix the special workspace crash and the title module

## [0.1.5] - 2024-11-04

### Added

- Added a clipboard button

### Fixed

- Fix workspace indicator foreground color selection

### Changed

- Nerd fonts are now included in the binary
- Workspace indicator now has an hover state

### Thanks

- @fiersik for the clipboard button and the ALT Linux package

## [0.1.4] - 2024-10-23

### Fixed

- bluetooth quick toggle button no longer appear when no bluetooth device is available
- rfkill absence doesn't cause an error during network service initialization
- rfkill is launched using absolute path to avoid issues with $PATH
- webcam absence doesn't cause an error during privacy service initialization

### Changed

- added more logging to the services in case of errors

## [0.1.3] - 2024-10-22

### Fixed

- resolved problem with `cargo vendor` command

## [0.1.2] - 2024-10-17

### Added

- Privacy module: webcam usage indicator

### Changed

- Reduced clock refresh rate to 5 sec
- Increased update check frequency to 3600 sec

### Removed

- Privacy module: removed privacy sub-menu

### Fixed

- Improve wifi indicator

## [0.1.1] - 2024-10-03

### Fixed

- re-added vpn toggle functionality that was removed during the services refactor

## [0.1.0] - 2024-09-30

### Added

- First release
- Configuration system
- Lancher button
- OS Updates indicator
- Hyprland Active Window
- Hyprland Workspaces
- System Information (CPU, RAM, Temperature)
- Date time
- Settings panel
  - Power menu
  - Battery information
  - Audio sources and sinks
  - Screen brightness
  - Network stuff
  - VPN
  - Bluetooth
  - Power profiles
  - Idle inhibitor
  - Airplane mode
