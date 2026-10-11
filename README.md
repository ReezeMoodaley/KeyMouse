# KeyMouse

KeyMouse is a Windows desktop utility that turns dedicated keyboard controls into precise cursor movement, clicks, dragging, scrolling and hierarchical grid navigation. It is designed for the Silakka54's QMK/Vial mouse layer. Firmware changes are unnecessary.

The backend owns input and configuration. React is a settings interface; it never drives cursor movement. No telemetry, accounts, cloud storage, typed-text recording or global keyboard suppression is used.

The Retro Terminal interface includes **Theme Studio** with four presets, custom themes, phosphor glow and adjustable CRT lettering. Themes synchronize with the tray-accessible **Mini Control Panel** and overlays. See [the interface and theme guide](docs/RETRO_TERMINAL.md) for customization, architecture and validation details.

Open **Precision guide** in the settings sidebar for feature explanations, starting values, suggested combinations and a calibration routine. The same guide is available as [PRECISION_GUIDE.md](docs/PRECISION_GUIDE.md). Both versions share `src/guide.json`; regenerate the document after edits with `node scripts/export-guide.mjs`.

## Requirements

Windows 10 1809 or newer / Windows 11, x64, Microsoft Edge WebView2 and a keyboard capable of sending F13–F24. A physical mouse is recommended during initial setup. Development needs Node 22.20+ (or a version supported by the locked Vite release), stable Rust **MSVC**, and Visual Studio 2022 C++ Build Tools with a Windows SDK.

## Install and run

Build the installer with the commands below, then run `src-tauri\target\release\bundle\nsis\KeyMouse_0.1.0_x64-setup.exe`. The installer is per-user and bootstraps WebView2 if it is missing (internet required). No administrator privileges are requested by KeyMouse. Windows may warn because the installer is unsigned.

For development, open a Developer PowerShell for VS 2022:

```powershell
cd C:\Users\reeze\Documents\Development\KeyMouse\KeyMouse
npm ci
rustup default stable-x86_64-pc-windows-msvc
rustup component add rustfmt clippy
npm run desktop
```

`npm run dev` alone previews the interface in a browser; it cannot access native input. The UI explicitly reports this rather than simulating an engine.

In this implementation workspace, Rust was installed locally in the parent `.toolchain` directory. Activate it before the standard commands:

```powershell
. .\scripts\use-local-toolchain.ps1
npm.cmd run desktop
```

## How movement works

The `keymouse-input-engine` Rust thread receives keyboard make/break events through Raw Input and a hidden Win32 receiver. It ignores repeated keydowns and tracks simultaneous controls per keyboard device. Active movement uses a one-shot Windows waitable timer targeting 4 ms intervals (250 Hz). Idle operation waits for messages, without a frontend movement loop.

For elapsed hold time `t`, ramp duration `r`, and exponent `e`, speed is `min + (max - min) * clamp(t/r, 0, 1)^e`. The normalized direction, speed and actual elapsed tick time produce pixel deltas. Fractional remainders preserve slow movement. Reversal and stopping clear affected remainders. Opposite inputs cancel on their axis.

Cursor movement adds deltas to the current `GetCursorPos` and sends exact physical positions through absolute virtual-desktop `SendInput`. Button and wheel events carry the current cursor position in the same input stream. Grid, bookmark and history jumps use `SetCursorPos`. A worker stall over 100 ms cancels held controls and releases owned buttons. Release and press the control again to resume. No Windows pointer-speed or acceleration settings are changed.

| Mode | Min / max px/s | Ramp | Exponent | Behaviour |
| --- | --- | --- | --- | --- |
| Normal | 40 / 1400 | 900 ms | 1.6 | Gentle start, predictable acceleration |
| Precision | 8 / 80 | 700 ms | 1.8 | One pixel per new direction press; 160 ms hold delay |
| Turbo | 900 / 4000 | 350 ms | 1.2 | Rapid display traversal |

Precision overrides turbo; both modifiers are momentary. A precision diagonal tap steps one pixel on each axis when both keys are pressed, resulting in `(±1, ±1)`. Continuous diagonals are normalized. Entering precision while a direction is already held does not add a tap. Step size is configurable from 1–10 pixels; exact one-pixel behaviour refers to the default.

## Default controls

| Key | Action |
| --- | --- |
| F13 | Up |
| F14 | Down |
| F15 | Left |
| F16 | Right |
| F17 | Left button (hold to drag) |
| F18 | Right button |
| F19 | Middle button |
| F20 | Scroll positive (up / right) |
| F21 | Scroll negative (down / left) |
| F22 | Precision |
| F23 | Turbo |
| F24 | Horizontal-scroll modifier |

Reassign any action among F13–F24 in settings. Every action must have a unique key. Rust deliberately rejects ordinary typing keys, unsupported values and conflicts. F13–F24 are not suppressed and may reach other applications.

See [Silakka54 setup](docs/SILAKKA54_SETUP.md) for the Vial procedure and rollback steps.

## Grid navigation

Ctrl+Alt+G opens a transparent overlay on a monitor. Choose regions with **1–9**, laid out left to right, top to bottom. Each choice narrows the region. At the configured depth (default three), the cursor moves to its center. Enter confirms early, Escape cancels, Backspace restarts and Tab cycles monitors. The activation key and depth are configurable.

The MVP deliberately uses **focused selection**. The overlay takes keyboard focus so numbers are not delivered to the previously active app. Switching to another app cancels it. This is the permitted focus-based fallback, not a non-activating global overlay. Cursor controls pause while the grid is open. Monitor coordinates and overlay sizing use physical pixels under Per-Monitor-V2 DPI awareness; hardware validation is still required for mixed-DPI arrangements.

## Tray, startup and emergency disable

Closing settings hides them to the tray; input continues. Double-click the tray icon or choose Open settings to return. The tray also enables/disables the engine, opens grid navigation and quits. Quit stops and joins the worker. Only one instance runs, using Tauri's single-instance plugin.

**Ctrl+Alt+Shift+K** disables KeyMouse independently of the mouse layer, releases KeyMouse-owned buttons and cancels the grid. Enabled state is persisted. Re-enable through settings or the tray. Keep this shortcut and a physical mouse available while testing. If either reserved shortcut cannot register at startup, initialization fails visibly rather than running without the escape mechanism.

Cleanup covers normal exit, disabling, configuration replacement, keyboard removal, session lock/disconnection and suspend. Lost break events are reconciled against `GetAsyncKeyState` for selected controls only. A removed keyboard conservatively cancels all controls. Fatal process termination, power failure or OS failure cannot guarantee button release; physically press and release affected buttons after restarting.

If Windows makes the input desktop inaccessible before a session notification can release a button, ownership is retained and release is retried on resume. A visible diagnostic identifies deferred cleanup. A startup API/shortcut error opens settings with an unavailable-engine diagnostic; settings can be corrected and saved for a restart.

## Configuration

Rust stores schema version 1 JSON in Tauri's per-user application configuration directory, normally `%APPDATA%\dev.keymouse.desktop\config.json`. Writes flush a sibling temporary file and replace the destination with `MoveFileExW` using replace and write-through flags. `config.last-good.json` provides recovery from invalid or truncated primary files. Unknown versions and invalid numbers fall back with a visible warning; original corrupt files are preserved until a user saves.

Changes are validated, applied to the worker and persisted through Tauri commands. A failed save attempts to restore the preceding running configuration and startup registration. Reset restores and persists defaults. General settings control theme, minimized launch and Windows login registration. No frontend file access is granted.

## Troubleshooting and Windows limitations

- Exit code `0xc0000139` (`STATUS_ENTRYPOINT_NOT_FOUND`): rebuild or reinstall the corrected package. The Windows manifest must include `Microsoft.Windows.Common-Controls` version `6.0.0.0` for the UI's `TaskDialogIndirect` import. Both development and release builds embed this dependency.
- No movement: verify settings show a running enabled engine, assign the exact F13–F24 Vial keycodes, and inspect the last recognized action in Diagnostics. Ordinary F1–F12 are different keys.
- Stopped worker: read its diagnostic error, quit and relaunch. API failures are surfaced; the worker does not pretend to operate.
- Buttons or scrolling rejected: Windows UIPI prevents an unelevated app injecting into elevated applications. KeyMouse intentionally runs asInvoker. Secure desktop, UAC prompts, lock screens and some games are unsupported.
- Shortcut conflict: release the shortcut in other software or edit the saved grid key (Ctrl+Alt with A–Z/F1–F12). The emergency shortcut is fixed.
- After a stall, resume by releasing and pressing controls. After suspend/lock, no held controls are replayed.
- Fine wheel deltas may be ignored or aggregated by applications. Set amount to 120 for a standard notch.
- Physical and generated button presses share Windows state. Avoid simultaneous physical/button-key dragging; KeyMouse can track its own events but cannot separate Windows' combined button state.
- Input-device count lists connected keyboard-class Raw Input devices, not specifically a Silakka54.
- Start-at-login registers the executable's current path. Enable it from the installed application; a moved development binary leaves a stale startup path.
- The grid has no click-through/non-activating selection mode. Focus is intentional and may affect the previous application's active window state.

## Build and test

Optional movement refinements, native Precision magnifier, cursor history, bookmarks, application profiles and explicit calibration sessions are available in their settings pages. All nine toggles default off. Save a toggle to apply it; parameters remain saved when disabled. See [movement responsibilities and metrics](docs/MOVEMENT_REFINEMENTS.md), [implementation matrix](docs/IMPLEMENTATION_MATRIX.md) and [baseline comparison exercises](docs/MOVEMENT_ACCEPTANCE.md).

```powershell
npm run typecheck
npm run lint
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run package
```

Release binaries are in `src-tauri\target\release`; the NSIS installer is in `bundle\nsis`. The icon is committed; `python scripts/make-icons.py` regenerates it. The checked-in manifest requests PerMonitorV2 and asInvoker.

Automated tests exercise acceleration, diagonals, precision taps and delays, fractions, stalls, input ownership, device removal, cancellation, randomized transitions, configuration recovery, grid boundaries and frontend IPC errors. They do not replace physical keyboard, monitor, suspend or endurance testing. See [validation procedure](docs/VALIDATION.md) and [architecture](docs/ARCHITECTURE.md). Implementation and measured verification status are recorded in [handover](docs/HANDOVER.md).

## Known issues and possible extensions

The investigation document was absent. Assumptions: precision diagonals step once per axis, key remapping is restricted to F13–F24 for safety, and grid navigation uses focused interaction. No latency, idle CPU or memory-leak target is claimed without measurement. Hardware tests are required before relying on this as your only pointer device.

Potential extensions are signed installers, per-device allowlists, additional validated profiles and richer performance capture. UI Automation, AI prediction, drivers, firmware changes, telemetry, cloud services and automatic updates are excluded.
