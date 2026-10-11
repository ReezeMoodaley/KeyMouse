# Implementation handover

**Final classification: Implemented but requires Windows/hardware validation.**

Movement refinement extension: all nine requested features are connected to Rust, saved toggles and settings UI, with disabled behavior checked against the frozen v1 engine. See [implementation matrix](IMPLEMENTATION_MATRIX.md) for the current test counts/native checks, [movement design and calibration metrics](MOVEMENT_REFINEMENTS.md), and [comparison exercises](MOVEMENT_ACCEPTANCE.md). The verification table below records the original v1 handover; the linked matrix records the extension.

The application compiles on Windows x64 and produces a release executable and an unsigned NSIS installer. Automated tests include real Win32 boundary calls. Physical Silakka54 input, interactive tray/startup/grid operation, mixed-DPI monitors, suspend/lock cleanup and measured performance/endurance have not been certified. Installer execution has not been tested in a separate clean Windows installation.

## Implemented

- Tauri 2 shell, React/TypeScript/Vite settings UI, light/dark/system themes and native diagnostics.
- Dedicated named Raw Input worker, repeated-make filtering, simultaneous per-device controls, reconnect/removal handling and selected-key lost-break recovery.
- Pure time-based movement with normalized diagonals, fractional accumulation, reversal/stop reset, precision taps/delay, turbo and modifier precedence.
- Windows cursor APIs, mouse button ownership, dragging, vertical/horizontal wheel events and tested repeat scheduling without catch-up bursts.
- Waitable-timer/message scheduling, stale-loop cancellation, cleanup guards, session/suspend handling and an independent emergency disable shortcut.
- Versioned Rust-owned JSON, validation, atomic Windows replacement, last-good recovery, live updates and defaults.
- Tray lifecycle, minimized launch, per-user login startup and Tauri single-instance enforcement.
- Focused transparent multi-monitor grid with nested refinement, early confirm, cancel, restart and monitor cycling.
- App icon, per-user NSIS/WebView2 packaging and an embedded asInvoker / PerMonitorV2 manifest.
- README, architecture, Vial setup and hardware validation instructions.

## Verification

| Check | Result |
| --- | --- |
| Rust formatting | Passed |
| Clippy, all targets, warnings denied | Passed |
| Rust tests | 20 passed |
| Real Win32 timer/device enumeration test | Passed, included above |
| Real hidden Raw Input receiver registration and cleanup test | Passed, included above |
| TypeScript check | Passed |
| ESLint | Passed |
| Vitest | 11 passed across settings and grid tests |
| npm dependency audit | Zero reported vulnerabilities |
| Frontend production build | Passed |
| Rust MSVC production build | Passed |
| NSIS installer generation | Passed |
| Embedded manifest extraction using Windows SDK `mt.exe` | Verified asInvoker, PerMonitorV2 and Common Controls v6 |
| Debug and packaged release process startup | Passed on Windows build 26100; both created a window and remained running for 8 seconds |
| Physical input, mixed-DPI, latency, idle CPU and endurance | Unverified; use VALIDATION.md |
| Installer installation/uninstallation and login launch | Unverified |

Initial failures were corrected: missing toolchains were installed, sandbox path-resolution checks were rerun with the needed access, frontend text matching was fixed, vulnerable test dependencies were upgraded, and Clippy findings were resolved. No warnings were disabled to obtain passing checks. Reviews covered Windows handle ownership, unsafe boundaries, thread/UI lock scopes, per-device transitions, configuration rollback, grid event ordering, disabled-held controls and deferred inaccessible-session button cleanup.

Build environment: Node 22.20.0, stable Rust MSVC 1.99.0, Visual Studio 2022 C++ Build Tools (MSVC 14.44) and Windows SDK 10.0.26100.0. Lockfiles are included. Rust is local to the workspace's parent `.toolchain`; the Microsoft C++ tools were installed for native compilation.

Startup correction (2026-10-11): reproduced exit `0xc0000139` before application initialization. The custom manifest omitted the Common Controls v6 assembly dependency while the executable imported `TaskDialogIndirect`, which the default System32 ComCtl32 did not export. Added the dependency, rebuilt debug/release and regenerated the NSIS installer. Actual launch checks passed with normal per-user directory access; the release process loaded ComCtl32 v6 from Windows WinSxS. Test processes were stopped afterward. These startup checks do not certify hardware behavior or installer installation/uninstallation.

## Run and install

From this workspace:

```powershell
cd C:\Users\reeze\Documents\Development\KeyMouse\KeyMouse
. .\scripts\use-local-toolchain.ps1
npm.cmd ci
npm.cmd run desktop
```

To build and install:

```powershell
npm.cmd run package
& .\src-tauri\target\release\bundle\nsis\KeyMouse_0.1.0_x64-setup.exe
```

The portable release executable is `src-tauri\target\release\keymouse.exe`. Use the installed application when enabling login startup. The installer bootstraps WebView2 when necessary; this may require internet. No signing certificate was available, so Windows may show an unsigned-app warning.

## Silakka54 mapping

F13/F14/F15/F16 = up/down/left/right. F17/F18/F19 = left/right/middle button. F20/F21 = positive/negative scrolling. F22 = precision, F23 = turbo, F24 = horizontal scrolling. Assign standard `KC_F13`–`KC_F24` keycodes in Vial. Firmware and Vial configuration were not modified.

Emergency disable: **Ctrl+Alt+Shift+K**. Grid: **Ctrl+Alt+G** by default, then 1–9 to refine, Enter to confirm, Escape to cancel, Backspace to restart, Tab to change monitor.

## Assumptions and limitations

The technical investigation was not present. Precision diagonal taps step one pixel per axis by default. Remapping is limited to distinct F13–F24 controls to avoid ordinary typing/system bindings. Grid navigation intentionally takes focus; no global suppression hook or non-activating interception is implemented.

UIPI, secure desktops, elevated targets and some games restrict generated input. If the input desktop is inaccessible during lock/disconnection, owned-button releases may be deferred until resume, with a diagnostic. Catastrophic termination cannot guarantee cleanup. Physical and generated button events share Windows' combined state. Raw Input device counts are keyboard-class devices rather than positive Silakka54 identification. Timing diagnostics report processed-transition-to-first-pixel durations and include intentional fractional/precision delay; they are not certified hardware receipt-to-API measurements.

Potential future work: code signing, a per-device allowlist, additional validated profiles and ETW latency/endurance capture. Excluded features remain excluded: firmware changes, drivers, UI Automation targeting, AI prediction, telemetry, cloud services and automatic updates.
