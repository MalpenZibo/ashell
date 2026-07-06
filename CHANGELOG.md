# Changelog

## [0.9.0] - 2026-06-08

Here we are with a new release! Thanks, everyone, for the support! 🎉

A brief recap of the major changes:
- 🔔 Notification system: optionally works as a notification daemon
- 📺 OSD system: optionally show OSD information through the new IPC system
- 🔌 IPC over a Unix socket: a way to control ashell externally
- 🌍 Internationalization: the foundation for localized formatting and translations
- ✨ Experimental animations: work-in-progress animation support
- ⚙️ Official iced: thanks to `iced_layershell`, we now use the official upstream iced instead of the custom pop-os fork

### ⚠️ Important license change (packagers!) https://github.com/MalpenZibo/ashell/issues/763

Relicensed from MIT to GPL-3.0-or-later. Please update your license metadata accordingly.

### Changes

### 💥 Breaking changes

- feat(settings)!: optional hibernate\_cmd with empty string handling [@Scott-Nx](https://github.com/Scott-Nx) ([#706](https://github.com/MalpenZibo/ashell/issues/706))

### 🚀 Features

- feat(osd): scale volume bar to max\_volume so overdrive level is visible [@MalpenZibo](https://github.com/MalpenZibo) ([#799](https://github.com/MalpenZibo/ashell/issues/799))
- feat(settings): fallback text in empty tooltip popups [@MalpenZibo](https://github.com/MalpenZibo) ([#796](https://github.com/MalpenZibo/ashell/issues/796))
- feat(power): add UPower charge limit toggle to quick settings [@Scott-Nx](https://github.com/Scott-Nx) ([#779](https://github.com/MalpenZibo/ashell/issues/779))
- feat(audio): configurable volume\_step and max\_volume with overdrive i… [@sudo-Tiz](https://github.com/sudo-Tiz) ([#774](https://github.com/MalpenZibo/ashell/issues/774))
- add configurable wind speed unit [@romanstingler](https://github.com/romanstingler) ([#788](https://github.com/MalpenZibo/ashell/issues/788))
- feat(tray): support checkbox submenus [@Lykathia](https://github.com/Lykathia) ([#787](https://github.com/MalpenZibo/ashell/issues/787))
- docs: update documentation for configuration options and new features [@romanstingler](https://github.com/romanstingler) ([#761](https://github.com/MalpenZibo/ashell/issues/761))
- feat(workspaces): highlight workspaces with urgent windows [@kiryl](https://github.com/kiryl) ([#742](https://github.com/MalpenZibo/ashell/issues/742))
- implement tooltip menus for quick settings indicators [@romanstingler](https://github.com/romanstingler) ([#733](https://github.com/MalpenZibo/ashell/issues/733))
- add help feature to clap dependency [@romanstingler](https://github.com/romanstingler) ([#770](https://github.com/MalpenZibo/ashell/issues/770))
- add typed temperature sensor configuration with auto-detection [@romanstingler](https://github.com/romanstingler) ([#668](https://github.com/MalpenZibo/ashell/issues/668))
- tempo: add click-to-toggle location visibility for screenshot protection [@romanstingler](https://github.com/romanstingler) ([#719](https://github.com/MalpenZibo/ashell/issues/719))
- feat(tray): left/right click behavior [@alexandre-abrioux](https://github.com/alexandre-abrioux) ([#729](https://github.com/MalpenZibo/ashell/issues/729))
- feat(i18n): add French (fr-FR) translation [@noirbizarre](https://github.com/noirbizarre) ([#756](https://github.com/MalpenZibo/ashell/issues/756))
- feat(settings): animate quick-setting toggle color transitions [@MalpenZibo](https://github.com/MalpenZibo) ([#745](https://github.com/MalpenZibo/ashell/issues/745))
- Feat/animate centerbox position [@MalpenZibo](https://github.com/MalpenZibo) ([#743](https://github.com/MalpenZibo/ashell/issues/743))
- feat: add width animations for bar modules and workspace buttons [@MalpenZibo](https://github.com/MalpenZibo) ([#682](https://github.com/MalpenZibo/ashell/issues/682))
- feat(tempo): display weather update time in current timezone [@alexandre-abrioux](https://github.com/alexandre-abrioux) ([#720](https://github.com/MalpenZibo/ashell/issues/720))
- system\_info add mounts option to filter disk display [@romanstingler](https://github.com/romanstingler) ([#707](https://github.com/MalpenZibo/ashell/issues/707))
- Feat/i18n translate system info [@MalpenZibo](https://github.com/MalpenZibo) ([#715](https://github.com/MalpenZibo/ashell/issues/715))
- Feat/i18n translate notifications [@MalpenZibo](https://github.com/MalpenZibo) ([#711](https://github.com/MalpenZibo/ashell/issues/711))
- Feat/i18n translate tempo [@MalpenZibo](https://github.com/MalpenZibo) ([#712](https://github.com/MalpenZibo/ashell/issues/712))
- Feat/i18n translate settings [@MalpenZibo](https://github.com/MalpenZibo) ([#713](https://github.com/MalpenZibo/ashell/issues/713))
- Feat/i18n translate osd [@MalpenZibo](https://github.com/MalpenZibo) ([#714](https://github.com/MalpenZibo/ashell/issues/714))
- Feat/i18n translate password dialog [@MalpenZibo](https://github.com/MalpenZibo) ([#716](https://github.com/MalpenZibo/ashell/issues/716))
- Feat/i18n translate media player [@MalpenZibo](https://github.com/MalpenZibo) ([#717](https://github.com/MalpenZibo/ashell/issues/717))
- feat - i18n Updates module [@MalpenZibo](https://github.com/MalpenZibo) ([#696](https://github.com/MalpenZibo/ashell/issues/696))
- fix(logging): added file size of 10MB as an additional criterion for … [@MustafaAamir](https://github.com/MustafaAamir) ([#709](https://github.com/MalpenZibo/ashell/issues/709))
- feat(settings)!: optional hibernate\_cmd with empty string handling [@Scott-Nx](https://github.com/Scott-Nx) ([#706](https://github.com/MalpenZibo/ashell/issues/706))
- docs: update troubleshooting guide for rendering backends [@romanstingler](https://github.com/romanstingler) ([#595](https://github.com/MalpenZibo/ashell/issues/595))
- Make airplane mode and bluetooth icons match their state [@whynotofu](https://github.com/whynotofu) ([#702](https://github.com/MalpenZibo/ashell/issues/702))
- Feat/i18n units [@MalpenZibo](https://github.com/MalpenZibo) ([#695](https://github.com/MalpenZibo/ashell/issues/695))
- feature: idle inhibitor ipc, osd + 3 related fixes [@whynotofu](https://github.com/whynotofu) ([#684](https://github.com/MalpenZibo/ashell/issues/684))
- feat - Add experimental animation support [@MalpenZibo](https://github.com/MalpenZibo) ([#681](https://github.com/MalpenZibo/ashell/issues/681))
- add option to select between Celsius and Fahrenheit in Tempo module [@romanstingler](https://github.com/romanstingler) ([#669](https://github.com/MalpenZibo/ashell/issues/669))
- Feat/i18n foundations [@MalpenZibo](https://github.com/MalpenZibo) ([#676](https://github.com/MalpenZibo/ashell/issues/676))
- feature: microphone ipc, osd [@whynotofu](https://github.com/whynotofu) ([#677](https://github.com/MalpenZibo/ashell/issues/677))
- Add two more configuration options for battery display [@CodedNil](https://github.com/CodedNil) ([#665](https://github.com/MalpenZibo/ashell/issues/665))
- Add tray blocklist [@CodedNil](https://github.com/CodedNil) ([#674](https://github.com/MalpenZibo/ashell/issues/674))
- Feat/osd audio, brightness, airplane control [@MalpenZibo](https://github.com/MalpenZibo) ([#657](https://github.com/MalpenZibo/ashell/issues/657))
- Feat/ipc socket [@MalpenZibo](https://github.com/MalpenZibo) ([#653](https://github.com/MalpenZibo/ashell/issues/653))
- Add configurable workspace scrolling direction [@lkcv](https://github.com/lkcv) ([#622](https://github.com/MalpenZibo/ashell/issues/622))
- Notifications blacklist [@denqxotl](https://github.com/denqxotl) ([#637](https://github.com/MalpenZibo/ashell/issues/637))
- Feat notification urgency [@MalpenZibo](https://github.com/MalpenZibo) ([#638](https://github.com/MalpenZibo/ashell/issues/638))
- feat(notifications): show single-notification groups with normal card [@MalpenZibo](https://github.com/MalpenZibo) ([#631](https://github.com/MalpenZibo/ashell/issues/631))
- create a built-in Notifications module for ashell  [@Yoimiya-Naganohara](https://github.com/Yoimiya-Naganohara) ([#456](https://github.com/MalpenZibo/ashell/issues/456))
- l10n [@juvilius](https://github.com/juvilius) ([#576](https://github.com/MalpenZibo/ashell/issues/576))
- Add different display units for systeminfo [@sulabi](https://github.com/sulabi) ([#541](https://github.com/MalpenZibo/ashell/issues/541))

### 🐞 Bug fixes

- fix: Use XDG\_RUNTIME\_DIR to store logs [@francoisfreitag](https://github.com/francoisfreitag) ([#760](https://github.com/MalpenZibo/ashell/issues/760))
- fix: replace panicking code paths with graceful error handling [@dacrab](https://github.com/dacrab) ([#766](https://github.com/MalpenZibo/ashell/issues/766))
- CLIPPY fix formatting warnings [@romanstingler](https://github.com/romanstingler) ([#783](https://github.com/MalpenZibo/ashell/issues/783))
- fix(tempo): align daily forecast rows and pair weather icon with temp [@MalpenZibo](https://github.com/MalpenZibo) ([#791](https://github.com/MalpenZibo/ashell/issues/791))
- tempo: fix hourly forecast and current weather time to use location timezone [@romanstingler](https://github.com/romanstingler) ([#782](https://github.com/MalpenZibo/ashell/issues/782))
- fix(workspaces): let named workspace buttons grow to fit their name [@MalpenZibo](https://github.com/MalpenZibo) ([#790](https://github.com/MalpenZibo/ashell/issues/790))
- ci: upgrade to Node.js 24 and fix GitHub Pages deployment [@dacrab](https://github.com/dacrab) ([#775](https://github.com/MalpenZibo/ashell/issues/775))
- fix: simplify COPR workflow and fix make install [@dacrab](https://github.com/dacrab) ([#764](https://github.com/MalpenZibo/ashell/issues/764))
- fix(notifications): show toast/OSD on focused output with correct scaling [@MalpenZibo](https://github.com/MalpenZibo) ([#730](https://github.com/MalpenZibo/ashell/issues/730))
- partial fix(notifications): clip toast card to respect padding when body overflows [@MalpenZibo](https://github.com/MalpenZibo) ([#759](https://github.com/MalpenZibo/ashell/issues/759))
- fix(hyprland): support Lua dispatch protocol on 0.55+ with Lua config [@romanstingler](https://github.com/romanstingler) ([#757](https://github.com/MalpenZibo/ashell/issues/757))
- fix(tempo): swap temperature order to max/min in daily forecast [@romanstingler](https://github.com/romanstingler) ([#753](https://github.com/MalpenZibo/ashell/issues/753))
- fix(i18n): honor config region in unit-system resolution [@MalpenZibo](https://github.com/MalpenZibo) ([#752](https://github.com/MalpenZibo/ashell/issues/752))
- Fix Tray: prevent icon duplication [@SimoneFelici](https://github.com/SimoneFelici) ([#731](https://github.com/MalpenZibo/ashell/issues/731))
- fix(tray): keep menu open when toggling a checkbox item [@Lykathia](https://github.com/Lykathia) ([#697](https://github.com/MalpenZibo/ashell/issues/697))
- build(deps): Use hyprland-rs master branch to fix crashes [@romanstingler](https://github.com/romanstingler) ([#721](https://github.com/MalpenZibo/ashell/issues/721))
- fix: replace unwrap on connection id lookup with safe chaining [@romanstingler](https://github.com/romanstingler) ([#723](https://github.com/MalpenZibo/ashell/issues/723))
- tempo: add missing round for `feels like` temperature value [@romanstingler](https://github.com/romanstingler) ([#718](https://github.com/MalpenZibo/ashell/issues/718))
- Fix output fuzzy matching [@CodedNil](https://github.com/CodedNil) ([#710](https://github.com/MalpenZibo/ashell/issues/710))
- fix: improve text truncation accuracy using grapheme clusters [@Scott-Nx](https://github.com/Scott-Nx) ([#694](https://github.com/MalpenZibo/ashell/issues/694))
- revert: disk deduplication ([#679](https://github.com/MalpenZibo/ashell/issues/679)) [@MalpenZibo](https://github.com/MalpenZibo) ([#698](https://github.com/MalpenZibo/ashell/issues/698))
- feat(i18n): respect LC\_MEASUREMENT for unit system [@clotodex](https://github.com/clotodex) ([#704](https://github.com/MalpenZibo/ashell/issues/704))
- Fix 652 Ashell appears twice [@MalpenZibo](https://github.com/MalpenZibo) ([#680](https://github.com/MalpenZibo/ashell/issues/680))
- feature: idle inhibitor ipc, osd + 3 related fixes [@whynotofu](https://github.com/whynotofu) ([#684](https://github.com/MalpenZibo/ashell/issues/684))
- Fix fallback lang locale [@MalpenZibo](https://github.com/MalpenZibo) ([#699](https://github.com/MalpenZibo/ashell/issues/699))
- fix(tray): drop malformed icon pixmaps to avoid panic [@Lykathia](https://github.com/Lykathia) ([#693](https://github.com/MalpenZibo/ashell/issues/693))
- standardize error handling for PulseAudio object creation [@romanstingler](https://github.com/romanstingler) ([#690](https://github.com/MalpenZibo/ashell/issues/690))
- Disk deduplication [@CodedNil](https://github.com/CodedNil) ([#679](https://github.com/MalpenZibo/ashell/issues/679))
- (fix) ipc second instance hijack [@MalpenZibo](https://github.com/MalpenZibo) ([#678](https://github.com/MalpenZibo/ashell/issues/678))
- fix: recover wgpu surface on NVIDIA + Niri (frozen bar) [@MalpenZibo](https://github.com/MalpenZibo) ([#671](https://github.com/MalpenZibo/ashell/issues/671))
- Fix notifications daemon [@denqxotl](https://github.com/denqxotl) ([#660](https://github.com/MalpenZibo/ashell/issues/660))
- Use canonicalize path for watching config file [@boerngen-schmidt](https://github.com/boerngen-schmidt) ([#658](https://github.com/MalpenZibo/ashell/issues/658))
- fix workspace pixels scroll direction [@MalpenZibo](https://github.com/MalpenZibo) ([#654](https://github.com/MalpenZibo/ashell/issues/654))
- Fix/media player cover [@MalpenZibo](https://github.com/MalpenZibo) ([#644](https://github.com/MalpenZibo/ashell/issues/644))
- fix(ci): restore autolabeler broken by release-drafter v7 upgrade [@MalpenZibo](https://github.com/MalpenZibo) ([#647](https://github.com/MalpenZibo/ashell/issues/647))
- fix tray discovery on restart [@MalpenZibo](https://github.com/MalpenZibo) ([#646](https://github.com/MalpenZibo/ashell/issues/646))
- correct timezone index boundary check [@romanstingler](https://github.com/romanstingler) ([#645](https://github.com/MalpenZibo/ashell/issues/645))
- fix(idle\_inhibitor): map surface via layer-shell for niri compatibility [@MalpenZibo](https://github.com/MalpenZibo) ([#639](https://github.com/MalpenZibo/ashell/issues/639))
- fix(workspaces): scroll in visual order on Niri [@MalpenZibo](https://github.com/MalpenZibo) ([#636](https://github.com/MalpenZibo/ashell/issues/636))
- (fix) Touch input and natural scroll [@MalpenZibo](https://github.com/MalpenZibo) ([#635](https://github.com/MalpenZibo/ashell/issues/635))
- fix(workspaces): stabilize monitor group order on Niri [@MalpenZibo](https://github.com/MalpenZibo) ([#634](https://github.com/MalpenZibo/ashell/issues/634))
- (fix) Niri Workspace filling [@MalpenZibo](https://github.com/MalpenZibo) ([#632](https://github.com/MalpenZibo/ashell/issues/632))
- [Regression] Audio devices not shown [@denqxotl](https://github.com/denqxotl) ([#617](https://github.com/MalpenZibo/ashell/issues/617))
- fix: prevent retrying failed MPRIS cover art downloads [@romanstingler](https://github.com/romanstingler) ([#575](https://github.com/MalpenZibo/ashell/issues/575))
- fix(network): reconnect after D-Bus error [@romanstingler](https://github.com/romanstingler) ([#601](https://github.com/MalpenZibo/ashell/issues/601))
- tray: skip empty icon name lookup [@SimoneFelici](https://github.com/SimoneFelici) ([#600](https://github.com/MalpenZibo/ashell/issues/600))
- fix: replace logger init unwrap with stderr fallback [@romanstingler](https://github.com/romanstingler) ([#604](https://github.com/MalpenZibo/ashell/issues/604))
- Fix: validate Temperature [@MalpenZibo](https://github.com/MalpenZibo) ([#608](https://github.com/MalpenZibo/ashell/issues/608))
- fix/config threshold validation [@romanstingler](https://github.com/romanstingler) ([#603](https://github.com/MalpenZibo/ashell/issues/603))
- Fix SystemInfo bugs and clean up display format code [@MalpenZibo](https://github.com/MalpenZibo) ([#607](https://github.com/MalpenZibo/ashell/issues/607))
- fix(ui): remove magic number 8 in height calculation, use theme space.xs instead [@romanstingler](https://github.com/romanstingler) ([#565](https://github.com/MalpenZibo/ashell/issues/565))

### 📚 Documentation

- docs: fill gaps in website config reference [@MalpenZibo](https://github.com/MalpenZibo) ([#800](https://github.com/MalpenZibo/ashell/issues/800))
- document listen\_cmd requires compact JSON due to line-by-line parsing [@romanstingler](https://github.com/romanstingler) ([#792](https://github.com/MalpenZibo/ashell/issues/792))
- docs: update documentation for configuration options and new features [@romanstingler](https://github.com/romanstingler) ([#761](https://github.com/MalpenZibo/ashell/issues/761))
- docs: add Gentoo installation instructions [@kakoed337](https://github.com/kakoed337) ([#772](https://github.com/MalpenZibo/ashell/issues/772))
- docs: update troubleshooting guide for rendering backends [@romanstingler](https://github.com/romanstingler) ([#595](https://github.com/MalpenZibo/ashell/issues/595))
- docs: update for iced\_layershell migration and add notifications [@MalpenZibo](https://github.com/MalpenZibo) ([#649](https://github.com/MalpenZibo/ashell/issues/649))
- docs: add Matrix community channel link [@MalpenZibo](https://github.com/MalpenZibo) ([#648](https://github.com/MalpenZibo/ashell/issues/648))
- Add developer guide, publish to website, and update README [@MalpenZibo](https://github.com/MalpenZibo) ([#560](https://github.com/MalpenZibo/ashell/issues/560))

### 🧰 Maintenance

- chore: relicense from MIT to GPL-3.0-or-later [@MalpenZibo](https://github.com/MalpenZibo) ([#789](https://github.com/MalpenZibo/ashell/issues/789))
- chore - Update dependencies  [@MalpenZibo](https://github.com/MalpenZibo) ([#794](https://github.com/MalpenZibo/ashell/issues/794))
- Chore/upower cleanup [@romanstingler](https://github.com/romanstingler) ([#785](https://github.com/MalpenZibo/ashell/issues/785))
- refactor: code cleanup and streamlining [@dacrab](https://github.com/dacrab) ([#767](https://github.com/MalpenZibo/ashell/issues/767))
- docs: update documentation for configuration options and new features [@romanstingler](https://github.com/romanstingler) ([#761](https://github.com/MalpenZibo/ashell/issues/761))
- ci: upgrade dorny/paths-filter v3 → v4 (node24) [@dacrab](https://github.com/dacrab) ([#777](https://github.com/MalpenZibo/ashell/issues/777))
- Replace generic `unwrap()` with `expect()` in network backends [@romanstingler](https://github.com/romanstingler) ([#722](https://github.com/MalpenZibo/ashell/issues/722))
- chore(deps): bump the cargo group across 1 directory with 2 updates @[dependabot[bot]](https://github.com/apps/dependabot) ([#675](https://github.com/MalpenZibo/ashell/issues/675))
- Calendar show days without leading zeros [@whynotofu](https://github.com/whynotofu) ([#687](https://github.com/MalpenZibo/ashell/issues/687))
- chore: remove redundant temperature conversion function [@romanstingler](https://github.com/romanstingler) ([#691](https://github.com/MalpenZibo/ashell/issues/691))
- chore: document safety of .expect() on config path parent [@romanstingler](https://github.com/romanstingler) ([#689](https://github.com/MalpenZibo/ashell/issues/689))
- refactor(theme): access AshellTheme via thread\_local global [@MalpenZibo](https://github.com/MalpenZibo) ([#672](https://github.com/MalpenZibo/ashell/issues/672))
- document optional runtime dependencies and package requirements [@romanstingler](https://github.com/romanstingler) ([#666](https://github.com/MalpenZibo/ashell/issues/666))
- Chore/menu surface on demand [@MalpenZibo](https://github.com/MalpenZibo) ([#656](https://github.com/MalpenZibo/ashell/issues/656))
- (chore) Refactor/shared components [@MalpenZibo](https://github.com/MalpenZibo) ([#641](https://github.com/MalpenZibo/ashell/issues/641))
- ci(release-drafter): fix autolabeler for fork PRs [@MalpenZibo](https://github.com/MalpenZibo) ([#651](https://github.com/MalpenZibo/ashell/issues/651))
- de-duplicate launcher code [@romanstingler](https://github.com/romanstingler) ([#640](https://github.com/MalpenZibo/ashell/issues/640))
- (Chore) Notification refactor [@MalpenZibo](https://github.com/MalpenZibo) ([#625](https://github.com/MalpenZibo/ashell/issues/625))
- Refactor palette config [@MalpenZibo](https://github.com/MalpenZibo) ([#574](https://github.com/MalpenZibo/ashell/issues/574))
- refactor(icons): extract duplicate battery icon logic  [@romanstingler](https://github.com/romanstingler) ([#611](https://github.com/MalpenZibo/ashell/issues/611))
- (chore): remove deprecated Clock module [@romanstingler](https://github.com/romanstingler) ([#602](https://github.com/MalpenZibo/ashell/issues/602))
- chore: speed up CI [@MalpenZibo](https://github.com/MalpenZibo) ([#609](https://github.com/MalpenZibo/ashell/issues/609))
- chore(deps): bump allsorts from 0.15.1 to 0.16.1 @[dependabot[bot]](https://github.com/apps/dependabot) ([#584](https://github.com/MalpenZibo/ashell/issues/584))
- chore(deps): bump zbus from 5.13.2 to 5.14.0 @[dependabot[bot]](https://github.com/apps/dependabot) ([#589](https://github.com/MalpenZibo/ashell/issues/589))
- chore(deps): bump anyhow from 1.0.101 to 1.0.102 @[dependabot[bot]](https://github.com/apps/dependabot) ([#585](https://github.com/MalpenZibo/ashell/issues/585))
- chore(deps): bump inotify from 0.11.0 to 0.11.1 @[dependabot[bot]](https://github.com/apps/dependabot) ([#586](https://github.com/MalpenZibo/ashell/issues/586))
- chore(deps): bump chrono from 0.4.43 to 0.4.44 @[dependabot[bot]](https://github.com/apps/dependabot) ([#588](https://github.com/MalpenZibo/ashell/issues/588))
- chore(deps-dev): bump typescript from 5.9.3 to 6.0.2 in /website @[dependabot[bot]](https://github.com/apps/dependabot) ([#590](https://github.com/MalpenZibo/ashell/issues/590))
- chore(deps): bump pnpm/action-setup from 4 to 5 @[dependabot[bot]](https://github.com/apps/dependabot) ([#579](https://github.com/MalpenZibo/ashell/issues/579))
- docs(system\_info): document display format options and fix threshold behavior [@MalpenZibo](https://github.com/MalpenZibo) ([#606](https://github.com/MalpenZibo/ashell/issues/606))
- fix: correct deploy working-directory and release-drafter config [@MalpenZibo](https://github.com/MalpenZibo) ([#592](https://github.com/MalpenZibo/ashell/issues/592))
- chore(deps): bump actions/download-artifact from 7 to 8 @[dependabot[bot]](https://github.com/apps/dependabot) ([#580](https://github.com/MalpenZibo/ashell/issues/580))
- chore(deps): bump actions/upload-artifact from 6 to 7 @[dependabot[bot]](https://github.com/apps/dependabot) ([#581](https://github.com/MalpenZibo/ashell/issues/581))
- chore(deps): bump nix-community/cache-nix-action from 6 to 7 @[dependabot[bot]](https://github.com/apps/dependabot) ([#582](https://github.com/MalpenZibo/ashell/issues/582))
- chore(deps): bump release-drafter/release-drafter from 6 to 7 @[dependabot[bot]](https://github.com/apps/dependabot) ([#583](https://github.com/MalpenZibo/ashell/issues/583))
- chore: remove unused dependencies uuid and parking\_lot [@romanstingler](https://github.com/romanstingler) ([#564](https://github.com/MalpenZibo/ashell/issues/564))
- fix(ui): remove magic number 8 in height calculation, use theme space.xs instead [@romanstingler](https://github.com/romanstingler) ([#565](https://github.com/MalpenZibo/ashell/issues/565))
- fix: rename refesh\_config to refresh\_config in app.rs [@romanstingler](https://github.com/romanstingler) ([#562](https://github.com/MalpenZibo/ashell/issues/562))

### 🔧 Dependency updates

- chore(deps): bump the cargo group across 1 directory with 2 updates @[dependabot[bot]](https://github.com/apps/dependabot) ([#675](https://github.com/MalpenZibo/ashell/issues/675))
- chore(deps): bump the cargo group across 1 directory with 3 updates @[dependabot[bot]](https://github.com/apps/dependabot) ([#624](https://github.com/MalpenZibo/ashell/issues/624))
- chore(deps): bump allsorts from 0.15.1 to 0.16.1 @[dependabot[bot]](https://github.com/apps/dependabot) ([#584](https://github.com/MalpenZibo/ashell/issues/584))
- chore(deps): bump zbus from 5.13.2 to 5.14.0 @[dependabot[bot]](https://github.com/apps/dependabot) ([#589](https://github.com/MalpenZibo/ashell/issues/589))
- chore(deps): bump anyhow from 1.0.101 to 1.0.102 @[dependabot[bot]](https://github.com/apps/dependabot) ([#585](https://github.com/MalpenZibo/ashell/issues/585))
- chore(deps): bump inotify from 0.11.0 to 0.11.1 @[dependabot[bot]](https://github.com/apps/dependabot) ([#586](https://github.com/MalpenZibo/ashell/issues/586))
- chore(deps): bump chrono from 0.4.43 to 0.4.44 @[dependabot[bot]](https://github.com/apps/dependabot) ([#588](https://github.com/MalpenZibo/ashell/issues/588))
- chore(deps-dev): bump typescript from 5.9.3 to 6.0.2 in /website @[dependabot[bot]](https://github.com/apps/dependabot) ([#590](https://github.com/MalpenZibo/ashell/issues/590))
- chore(deps): bump pnpm/action-setup from 4 to 5 @[dependabot[bot]](https://github.com/apps/dependabot) ([#579](https://github.com/MalpenZibo/ashell/issues/579))
- Switch from pop-os iced fork to iced\_layershell [@MalpenZibo](https://github.com/MalpenZibo) ([#578](https://github.com/MalpenZibo/ashell/issues/578))
- chore(deps): bump actions/download-artifact from 7 to 8 @[dependabot[bot]](https://github.com/apps/dependabot) ([#580](https://github.com/MalpenZibo/ashell/issues/580))
- chore(deps): bump actions/upload-artifact from 6 to 7 @[dependabot[bot]](https://github.com/apps/dependabot) ([#581](https://github.com/MalpenZibo/ashell/issues/581))
- chore(deps): bump nix-community/cache-nix-action from 6 to 7 @[dependabot[bot]](https://github.com/apps/dependabot) ([#582](https://github.com/MalpenZibo/ashell/issues/582))
- chore(deps): bump release-drafter/release-drafter from 6 to 7 @[dependabot[bot]](https://github.com/apps/dependabot) ([#583](https://github.com/MalpenZibo/ashell/issues/583))

### Contributors

❤️ A big thanks to [@CodedNil](https://github.com/CodedNil), [@Lykathia](https://github.com/Lykathia), [@MustafaAamir](https://github.com/MustafaAamir), [@Scott-Nx](https://github.com/Scott-Nx), [@SimoneFelici](https://github.com/SimoneFelici), [@Yoimiya-Naganohara](https://github.com/Yoimiya-Naganohara), [@alexandre-abrioux](https://github.com/alexandre-abrioux), [@boerngen-schmidt](https://github.com/boerngen-schmidt), [@clotodex](https://github.com/clotodex), [@dacrab](https://github.com/dacrab), [@denqxotl](https://github.com/denqxotl), [@francoisfreitag](https://github.com/francoisfreitag), [@juvilius](https://github.com/juvilius), [@kakoed337](https://github.com/kakoed337), [@kiryl](https://github.com/kiryl), [@lkcv](https://github.com/lkcv), [@mustafaa2](https://github.com/mustafaa2), [@noirbizarre](https://github.com/noirbizarre), [@romanstingler](https://github.com/romanstingler), [@sudo-Tiz](https://github.com/sudo-Tiz), [@sulabi](https://github.com/sulabi), [@whynotofu](https://github.com/whynotofu) and sudo-Tiz

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
