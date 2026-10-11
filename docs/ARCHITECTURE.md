# Architecture

The supplied investigation document was unavailable. The implementation follows the request's fixed architecture. Precision diagonal taps are `(±step, ±step)` from two direction transitions; normal continuous movement uses a unit direction vector. Bindings are restricted to the twelve dedicated F13–F24 keys. Grid selection uses the explicitly permitted focused fallback.

## Threads and ownership

Tauri's application thread owns the tray and webview lifecycle. One named `keymouse-input-engine` worker owns a hidden top-level Win32 receiver, Raw Input keyboard registration, hotkeys, session notifications, waitable timer, pressed states, cursor calculations and generated button ownership. The single-instance plugin prevents duplicate processes. The engine handle sends channel commands and posts `WM_APP` to wake the receiver. Configuration application has an acknowledgment; shutdown sends Stop and joins the thread.

Grid and settings operations are serialized using short mutex-protected state and a configuration-operation lock. The engine never waits for webview rendering or emits per-tick IPC. Grid opening runs outside the input loop and uses a separate Tauri window. Settings poll a coarse status snapshot once per second. The pure movement and input-state modules do not call native APIs.

## Input processing

`RegisterRawInputDevices` registers keyboard usage page 1, usage 6 with `RIDEV_INPUTSINK | RIDEV_DEVNOTIFY`. A dedicated message loop receives make and break events even when settings are hidden or unfocused. `WM_INPUT` is decoded while its `HRAWINPUT` is valid and passed to `DefWindowProcW` for Windows cleanup. Only F13–F24 are retained. Ordinary keys are immediately discarded; no scan-to-text conversion exists.

The state maps `(device, virtual key)` to action. Additional make events do not repeat an action. Multiple keyboards can contribute to one action; it releases only after the last contributing key releases. Cancellation moves held keys to a blocked set; make events remain blocked until a break or native release reconciliation. Device removal cancels all controls conservatively. Selected held controls alone are checked against `GetAsyncKeyState` every 50 ms when active. Windows' async state is global; identical simultaneous keys across devices cannot be individually recovered if their break events are lost.

## Calculations and scheduling

The monotonic clock is `Instant`. The curve is `min + (max-min) * clamp(elapsed/ramp,0,1)^exponent`. Each tick scales the normalized direction by the speed and actual delta time. Per-axis fractional remainders accumulate until an integer pixel is available. Direction changes clear changed axes; a reversal, initial movement, stop or mode change resets acceleration. Precision takes priority over turbo, and mode transitions reset timing immediately.

Precision emits a configured integer step on a fresh non-conflicting direction make, provided precision was already active. Held movement waits for the precision delay, then ramps. Mode entry while a direction is held produces no step. A tick over 100 ms is rejected; an independent worker-loop delay check cancels controls before processing queued input after a stall. This prevents replaying old held directions after an unsafe delay.

High-resolution waitable timers are attempted with `CreateWaitableTimerExW`; older Windows fall back to a normal timer. Active ticks target a one-shot 4 ms interval using `SetWaitableTimer` and `MsgWaitForMultipleObjectsEx`, which also wakes for input and commands. The scheduler does not busy-wait or request global timer-resolution changes. Inactivity has no periodic movement wakeup. Tick scheduling is unrelated to webview animation or monitor refresh.

## Native output and resources

Cursor deltas are added to the actual current signed position from `GetCursorPos`, then sent as absolute `SendInput` movement over the entire virtual desktop. Pixel-center normalization preserves exact physical pixels and negative monitor origins without applying Windows pointer acceleration. Buttons and wheels carry the current absolute position in the same event, so a click is anchored to the visible cursor even after a grid/bookmark/history `SetCursorPos` teleport. Each button transition is emitted once; held-button movement never repeats a down event. A PMv2 manifest avoids DPI virtualization. Horizontal wheel data uses signed values represented as the API's `u32` field. Output failure terminates the worker with an explicit diagnostic error rather than silently ignoring it.

Button ownership commits only after successful dispatch. Cleanup releases only KeyMouse-owned buttons, with bounded retry during unwind. A `Resources` guard unregisters hotkeys, session notifications and Raw Input, destroys the hidden window, closes the timer and unregisters its window class. The window callback retains no pointers to Rust state, so window destruction cannot access dropped runtime data. Unsafe code is confined to checked Windows API boundaries and configuration file replacement. Worker panic unwinding retains both guards.

## Rust-owned configuration

Schema v1 uses Serde, rejects unknown fields, validates finite numerical bounds, all bindings and distinct controls before application. The store is rooted in Tauri's per-user config path. Temporary-file flush and Windows replace/write-through form the atomic persistence boundary. A separate validated last-good snapshot is tried if the primary is corrupt. Configuration transactions reserve a changed grid hotkey, cancel held states and replace runtime configuration. Failed application/persistence attempts restore the preceding engine configuration and startup registration; errors remain visible. Autostart is implemented through Tauri's supported plugin.

## Grid subsystem

A session stores a monitor rectangle, current region, depth and refinement level. Rectangles use half-open bounds. Integer division partitions a region into nine disjoint cells, preserving odd-sized edges and negative monitor origins. Empty subpixel cells are rejected; Enter can confirm the current region. Selection either refines, confirms at final depth, restarts or cycles monitors. The final `SetCursorPos` uses the cell center. Rendering is an SVG in a transparent always-on-top webview. The backend supplies region snapshots only on changes.

Grid interaction takes focus deliberately; ordinary digits cannot be intercepted reliably by Raw Input without passing through to another application. No suppression hook is installed. Focus loss, Escape, lifecycle cancellation and settings changes close the grid and resume continuous control. Previously held controls remain canceled until released. This MVP therefore makes an explicit focus change on activation, documented in the UI.

## Lifecycle and privacy

Disabling, live configuration changes, grid activation, keyboard removal, session lock/disconnection, system suspend, explicit quit and normal worker unwinding cancel states and release generated buttons. Emergency Ctrl+Alt+Shift+K is registered independently with `RegisterHotKey` and persists disabled state off the input thread. Settings close hides the window. The worker stays alive until explicit quit. Catastrophic termination cannot guarantee cleanup.

Diagnostics expose only current engine state, latest recognized action and aggregate timer/first-pixel timing. A bounded 512-sample ring contains durations, not keys or text. First-pixel p95 includes intentional fractional/precision delay; it must not be misinterpreted as measured hardware receipt-to-API latency.

Native contracts were checked against [Microsoft Raw Input documentation](https://learn.microsoft.com/en-us/windows/win32/inputdev/using-raw-input), [WM_INPUT cleanup](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-input), and [Tauri's single-instance plugin](https://v2.tauri.app/plugin/single-instance/). Generated `windows` crate bindings are the compile-time API source.
