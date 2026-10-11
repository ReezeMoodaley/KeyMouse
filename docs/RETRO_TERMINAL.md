# Retro Terminal interface and Theme Studio

KeyMouse now has a responsive terminal shell, a shared semantic theme system, configurable phosphor glow and CRT lettering, and a tray-accessible Mini Control Panel. Movement preferences and theme preferences are independent. The Rust movement engine, Raw Input processing, button ownership and cursor injection are unchanged by this redesign.

## Feature audit and screen coverage

The implementation audit found Normal / Precision / Turbo, Power / Linear / Smoothstep acceleration strategies, Tap versus Hold, Adaptive Braking, Per-Axis Acceleration, calibration, grid navigation, application profiles, precision magnification, cursor history and bookmarks. There is no separate Three-Phase Acceleration implementation; the interface does not advertise one.

| Screen | Desktop layout and behavior |
| --- | --- |
| Dashboard | Engine / quick controls 8–4; acceleration / readout 8–4; observed events / health 7–5. Backend status supplies every live value. |
| Movement | Full-width mode selector; curve / profile 8–4; editable response settings / feature switches 8–4; contextual movement explanation. Graph uses the selected base acceleration strategy. |
| Keymapping | Manual source toolbar; 54-position split keyboard template / selected action 8–4; binding matrix / conflicts and recognized input 8–4. Diagram and matrix edit the same draft. Firmware positions are not detected. |
| Calibration | Setup / canvas preview / live session 3–6–3; performance summary / A–B comparison 8–4; full-width session history. Real measurements remain on the existing native overlay. |
| Profiles | List / editor / sampled effective configuration 3–6–3; full-width executable matching diagnostics. Existing override capture and precedence are preserved. |
| Diagnostics | Engine / configuration / window DPI 4–4–4; performance / devices 8–4; observed events / recovery 8–4. No invented hardware latency or instantaneous speed. |
| Theme Studio | Presets; token groups / miniature preview / color or geometry editor 3–5–4; CRT / theme management 7–5; Apply / Cancel / Reset. |
| General | Preferences / utility actions 7–5; configuration / safety and recovery 7–5. Appearance opens Theme Studio. |
| About | Product / technical information 8–4; documentation links full width. |
| Other existing screens | Refinements, Cursor tools, Mouse & scrolling, Grid navigation and Precision guide remain available and use the same tokens and shell. |
| Mini Control Panel | Tray and dashboard access; engine toggle, sampled foreground profile, saved speed summaries, immediate feature toggles, full app and Theme Studio links, explicit calibration start and setup link. |

The reference shell uses a 224 px sidebar, 56 px header, 30 px bottom strip, 24 px content padding, 16 px panel gap and 4 px corners. Only the content pane scrolls. Below 1400 px the sidebar and padding become compact; below 1100 px navigation collapses to labeled icon buttons and panels reflow; below 900 px panels stack. Calibration and theme previews receive priority at narrow widths. Natural content growth and scrolling take precedence over fixed panel heights.

## Using Theme Studio

1. Select **Phosphor Green**, **Amber Terminal**, **Cyber Cyan** or **Monochrome**. This updates the draft preview.
2. Choose a token group, select a token, and use its color picker. Geometry offers panel radius, interface density and base font size.
3. Adjust effects while comparing Clean Typography, Phosphor Glow and Glow + CRT Distortion. Preview controls are illustrative, not engine controls.
4. **Apply** persists and activates the draft in every supported window. **Cancel** returns the preview to the currently active theme. **Reset** restores the default in the preview; Apply makes it active.
5. Enter a name and **Save Custom Theme** to retain a reusable copy without activating it. **Duplicate** always creates a new identifier. **Rename** and **Delete** are available only for custom themes. Deleting the active custom theme activates the safe default. Built-in presets remain available and unchanged.
6. **Export JSON** writes the complete versioned theme to the JSON text area. Copy that text to a file to share it. Paste a theme into the same area and use **Import JSON** to load a draft. Save Custom Theme or Apply retains an import. These actions operate on JSON text; they do not open native file dialogs.

Contrast ratios and low-contrast warnings help evaluate edited text colors. Arbitrary custom palettes can still reduce contrast; use Reset if an applied palette is difficult to read. Reset and all controls are keyboard reachable. Collapsed navigation retains accessible names and tooltips.

## Glow and CRT effects

**Phosphor Glow** has Off / Subtle / Standard / Strong levels. Subtle is the default. Headings receive a tight halo; larger active readouts receive a slightly wider bloom. Body copy, editable numbers, error messages and calibration geometry remain sharp.

**CRT Distortion** is enabled at 20% by default. Layered CSS `text-shadow` adds a tight illumination layer, a restrained bloom, a faint offset ghost and optional secondary-color fringe. Original glyphs and element geometry stay in place. No text copies, character-level elements, transforms, canvas rendering, screen capture or full-screen distortion filters are involved.

Master intensity multiplies every CRT-specific layer. At zero or with distortion disabled, ghost and channel offsets and CRT bloom become zero and CRT shadows become transparent. Ordinary glow is independent. Advanced ghost strength, displacement, channel misalignment and bloom refine the master setting; displacement is bounded to one pixel, and the Mini Control Panel multiplies displacement by 0.65. Recommended advanced defaults are ghost 20%, displacement 0.6 px, misalignment 8%, bloom 25%, flicker off.

Every shadow color comes from the active semantic palette. Amber produces warm shadows; cyan may fringe using its secondary accent; Monochrome uses neutral colors. No fixed RGB separation is used.

Scanlines, screen noise and flicker are independent switches. Scanlines and noise use static low-opacity CSS patterns whose pseudo-elements have `pointer-events: none`; disabled patterns have `display: none`. Noise is a static dither texture, not changing random grain. Flicker gently changes the opacity of selected lettering and is disabled under `prefers-reduced-motion`. Native visibility events and document visibility pause animations in hidden windows. No JavaScript animation loop runs.

## Architecture and storage

| Source | Responsibility |
| --- | --- |
| `src/theme-default.json` | Canonical default palette and effects, also embedded by Rust. |
| `src/themes.ts` | Typed version 1 schema, immutable-in-use preset definitions, strict validation, JSON import/export, contrast calculation and derived CSS variables. |
| `src/ThemeProvider.tsx` | Shared frontend context, initial native load, event subscription, revision ordering, persistence commands and reduced-motion/visibility handling. Every window mounts this provider. |
| `src/terminal.css` | Semantic colors, typography, spacing, geometry, responsive shell, reusable CRT utilities and static decorative layers. Existing functional controls consume aliases into the same tokens. |
| `src/Terminal.tsx` | TerminalPanel, MetricReadout and CRTText primitives. |
| `src/ThemeStudio.tsx` | Isolated draft editor, miniature interface, comparison preview, effects and custom-theme actions. |
| `src/Presentation.tsx` | Read-only presentation snapshot, observed action changes, profile readouts, diagnostics and keymap workspace. |
| `src/MiniControlPanel.tsx` | Compact real controls, fresh configuration reads before feature changes, navigation and explicit calibration start. |
| `src-tauri/src/themes.rs` | One authoritative theme store; structural/range validation, atomic persistence, revision conflict rejection and event broadcast. |
| `src-tauri/src/presentation.rs` | Read-only foreground/display information and window routing. Mini WebView creation uses an asynchronous command to avoid blocking Windows IPC. |

Colors use `--km-bg-*`, `--km-text-*`, `--km-accent-*`, `--km-border-*`, `--km-status-*` and `--km-control-*`. Geometry/density/typography and effects use corresponding `--km-*` variables. Components consume tokens rather than checking a theme name. The previous system/light/dark configuration field remains readable for compatibility but no longer controls appearance.

`themes.json` sits beside the existing movement `config.json` in Tauri's application configuration directory. One native state owns the active theme and up to 100 custom themes. Writes validate, flush and atomically replace the file before broadcasting `theme-state`. Revision checks reject stale writes. Windows subscribe before loading state and ignore older events. Invalid saved themes fall back to safe defaults and report the error; the damaged file remains untouched until an explicit save. Imports accept only expected fields, bounded numbers, supported enums and six-digit hex colors; arbitrary CSS is rejected.

## Validation and limits

Frontend tests cover presets, color editing, isolated drafts, Apply / Cancel / Reset, custom creation/duplication/rename/delete, JSON round trips and rejection, geometry persistence, simulated cross-window events, CRT scaling and independent effects, accessible text, reduced-motion contracts and Mini Control Panel command boundaries. Existing movement-settings, keybinding, refinement, calibration, profile and grid tests remain in the suite.

Rust tests add invalid-theme, atomic-persistence/restart, corrupt-file preservation and stale-revision coverage. Existing movement/input/backend regression tests remain unchanged. The calculation timing and interactive native drag tests stay explicitly excluded from the default test run.

`scripts/retro-smoke.ps1` launches an isolated diagnostic instance with a complete configuration that disables movement before startup. It verifies native WebView page rendering, real layered shadows, sharp input text, noninteractive decoration, responsive layouts, main/mini event synchronization, reduced motion, zero distortion and theme persistence after a second native launch. It saves screenshots and JSON reports under the printed `src-tauri/target/retro-smoke-*` directory. No cursor movement or mouse-button injection is performed.

Performance review: all default effects are static; optional flicker uses CSS only; disabled overlays are removed from painting; hidden-window animation pauses. Status polling remains low frequency (1 s), and presentation information is sampled at 1.5 s outside the input worker. This is an architectural and rendering check, not a hardware latency or GPU performance certification.

Known limits: the keyboard diagram is a manual template; foreground profile information is sampled independently of the worker; event history records observed action changes and can miss rapid transitions; display DPI reports the settings window's scale, not independently measured DPI for every monitor; the acceleration graph shows the base curve and does not simulate braking/axis transitions; calibration canvas is an explanatory preview of the native measurement overlay; calibration history remains process-local. No new Three-Phase Acceleration feature is implied.

Windows functions not re-certified by this redesign: physical Silakka54 firmware/input, mixed-DPI cursor accuracy, native text selection/window dragging, magnifier behavior in third-party apps, actual tray-menu clicks, Windows login registration and installer installation/uninstallation. Existing regression tests run, and native theme/window IPC is checked; the installer is built but is not installed over a user's working installation during validation.

### Completed checks — 11 October 2026

| Check | Result |
| --- | --- |
| TypeScript | Passed; also run by the production build. |
| Frontend | 48 tests passed across eight files, including the original regression suite. |
| ESLint | Passed. |
| Rust tests | 47 passed, two existing explicit exclusions. The native magnifier test passed with desktop access. |
| Clippy | All targets passed with warnings denied. |
| Rust formatting | Applied and checked. |
| Native WebView smoke | Passed all fourteen pages at desktop width, all nine redesigned workspaces at 1200 / 1000 / 800 px, actual text-shadow rendering, input sharpness, pointer transparency, initial hidden state, reduced motion, main/mini synchronization, compact offsets, navigation and restart persistence. |
| Color audit | No literal theme colors remain in screen components or stylesheets. Color values live in the default theme JSON and preset definitions. |
| Production frontend | Built: approximately 317.33 kB JavaScript (98.43 kB gzip), 25.45 kB CSS (5.69 kB gzip). |
| Windows package | Production executable and NSIS installer built successfully; installer size 2.16 MiB. |

Native evidence is in `src-tauri/target/retro-smoke-185ed13071f949f890be8e8f86ed264f`: `ui-report.json`, `restart-report.json`, `default-dashboard.png`, `dashboard.png`, `theme-studio.png` and `mini.png`. These disposable artifacts are ignored by Git.

To repeat the native check, use the local Rust toolchain, build `cargo build --manifest-path src-tauri/Cargo.toml --features diagnostics --bin keymouse-diagnostics`, start `npm run dev` in another terminal, then run `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/retro-smoke.ps1`. The diagnostics feature is excluded from the production installer.

### Distortion visibility correction

The initial shadows were effectively invisible: recommended ghost settings produced only 1.4% opacity and a 0.12 px horizontal offset at 20% master intensity. Ghosting now uses stronger opacity and a small diagonal displacement, with a restrained intensity curve for illumination, bloom and theme-colored fringes. The glyph remains sharp, offsets stay bounded, and ordinary glow remains independent. Advanced ghost, bloom and misalignment controls still suppress their respective layers at zero.

The native smoke check now captures the same wordmark at 0%, 20%, 50% and 100%, decodes the screenshots and asserts increasing rendered pixel differences against zero. Evidence for this correction is in `src-tauri/target/retro-smoke-dde2abca660b46748879c4ee02252d2a`, including `distortion-*.png` and `distortion-visibility.json`. The 48 frontend tests and native synchronization/restart checks pass with this correction.
