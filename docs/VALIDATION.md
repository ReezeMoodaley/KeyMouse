# Validation procedure

Do not interpret successful builds and pure tests as physical acceptance. Use a physical Silakka54 and at least two monitors (including a negative desktop origin and different DPI scales). Keep the emergency shortcut and physical mouse available.

## Automated checks

Run the commands in the README. Pure Rust tests never move the physical cursor. Frontend tests use mocked command boundaries only; these mocks are absent from the production app. Grid geometry tests partition odd rectangles and exercise one-pixel limits. Configuration tests corrupt a stored primary and confirm validated last-good recovery.

## Windows and keyboard acceptance

| Procedure | Expected result | Status before hardware testing |
| --- | --- | --- |
| Hold F22 and tap each direction 100 times | Exactly one physical pixel per tap; no drift | Unverified |
| Tap two precision directions together | One pixel per axis, no repeated make steps | Unverified |
| Hold direction, add/remove precision and turbo | Immediate priority; no surprise mode-entry step | Unverified |
| Hold normal direction for several seconds | Monotonic ramp; stop immediately on break | Unverified |
| Compare cardinal / diagonal distance at constant speed | Equal vector speed, within rounding error | Unverified |
| Simultaneous opposite directions | Cancel opposing axis; resume when one releases | Unverified |
| Drag, then disable / exit / unplug / lock / suspend | Generated button releases; held controls do not resume | Unverified |
| Artificially stall the worker >100 ms in a debugger | No jump; controls require release and fresh press | Unverified |
| Close settings and switch to another app | Continuous Raw Input still operates | Unverified |
| Launch a second process | Existing settings appear, one worker remains | Unverified |
| Use physical mouse during keyboard motion | Cursor updates from latest actual position | Unverified |
| Grid each monitor at 100%, 150%, 200% DPI | Correct regions and physical center, negative origin handled | Unverified |
| Grid Escape / Enter / Tab / Backspace / focus switch | Clean cancellation or selection, controls resume | Unverified |
| Ctrl+Alt+Shift+K outside mouse layer | Stops movement, releases generated buttons, persists disabled | Unverified |
| Click an elevated app or secure desktop | Explicit limitation / error, no escalation | Unverified |
| Login startup / start minimized / tray Quit | Correct visibility, shutdown joins worker | Unverified |

## Performance measurements

Use Windows Performance Recorder/Analyzer or a debugger/profiler in a disposable test session. For receipt-to-dispatch acceptance, timestamp `WM_INPUT` receipt and the corresponding first movement API dispatch using `Instant` (or instrument with ETW). Use **precision taps** with F22 already held to avoid intentional normal-mode subpixel accumulation. Collect at least 1,000 physical taps, report median, p95 and max with machine, keyboard polling, CPU load, OS, display/DPI layout and build version. Target p95 approximately 8 ms. Diagnostics' first-pixel ring begins at processed direction transitions and includes curve delay; it is not equivalent to receipt-to-API certification.

Measure idle CPU with settings hidden for ten minutes and compare to a baseline. Record actual active timer intervals and variance under normal desktop load. Run an eight-hour session with repeated enable/disable, window hide/show, monitor changes, grid sessions and device reconnects. Compare private bytes, handles and thread counts at intervals; inspect held-state and generated-button cleanup. Do not call these targets passed without recordings.

Record results, hardware and any failures in `docs/HANDOVER.md`. Installer execution, interactive cleanup and physical keyboard/DPI tests must be performed in the user's desktop session; automated tests alone cannot certify them.
