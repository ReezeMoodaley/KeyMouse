# Movement refinements and verification

All nine features have saved independent toggles, default **off**. Old version-1 JSON loads with these defaults. Disabling preserves parameters, profiles and bookmarks. The working baseline checkpoint and deterministic oracle are described in [MOVEMENT_BASELINE.md](MOVEMENT_BASELINE.md).

## Authoritative pipeline

Raw Input determines held controls. `Movement::input` interprets a new directional make as a Precision tap or optional Normal/Turbo tap. `Movement::tick` selects the acceleration strategy, optionally computes per-axis velocities, applies the braking constraint, and performs the sole continuous integration into fractional pixels. `Runtime::move_by` applies the integer displacement to the latest actual cursor position. Feature modules do not issue competing movement updates.

The phase enum exposes Idle, InitialTap, WaitingForHold, ContinuousHold, Precision, Turbo and CorrectiveBraking. Mode and phase are separate because Precision/Turbo priority applies throughout a tap/hold episode.

| Transition | Clock / velocity / remainder behavior |
| --- | --- |
| Idle → direction | Shared ramp begins now; changed axes get new clocks; no old remainder on those axes |
| Directional make in Precision | Existing single Precision step implementation; hold clock restarts; no optional tap |
| Discrete Tap | Immediate configured step; intentional hold threshold; continuous integration excludes the waiting interval |
| Responsive Continuous | Immediate configured step plus continuous integration from press time; short taps can exceed the step; no intentional waiting interval |
| Same-sign axis added | Original shared ramp is retained; per-axis mode starts the new axis separately |
| Axis reversal | Its remainder and axis clock reset; no velocity may continue in the old sign |
| Last direction released | Direction and velocity become zero immediately; remainders are discarded; recent effective direction/time/speed are retained only for correction detection |
| Precision / Turbo / Normal change | Original shared ramp and appropriate remainders reset; per-axis clocks restart; braking history is canceled; entering Precision does not emit a synthetic tap |
| Feature/config/profile update | Applied on the worker command boundary; held controls and unrelated button ownership remain; no tap is emitted; affected braking/axis state is reset or migrated; fractions are preserved |
| Explicit grid / bookmark / history move | Worker cancels and blocks held directions until release, clears ramp/velocity/fractions and pending latency sample, restores held modifier mode, then positions the cursor; unrelated held buttons remain owned |
| Disable / bindings change / stall / session cleanup | Existing full cancellation and owned-button cleanup; fresh presses are required |

Discrete Tap tracks each directional axis's press time independently: a newly added axis emits its step and waits for its own threshold while an existing held axis continues. Each component integrates only the portion of a frame after its threshold. Per-axis acceleration uses separate ramp clocks, so changing that feature's parameters cannot replay or reset another axis's tap. Precision retains its original shared hold-delay rules.

## Feature responsibilities

**Acceleration strategy:** optional Power, Linear or Smoothstep interpolation between each mode's configured minimum/maximum over its ramp duration. Disabled is the original power formula, including floating-point integration order.

**Adaptive Braking:** compares the dot product of the previous effective direction and the request. A negative dot product qualifies as an opposite correction; zero qualifies only with the separately enabled perpendicular option. Previous speed must reach the minimum threshold. A release/re-press must fall within the correction window; a direct direction change is current. Any simultaneously held opposing controls invalidate correction history. Precision bypasses braking. The corrective cap starts at the configured bound and recovers to the mode maximum using smoothstep over the configured recovery time. This is a velocity bound without inertia; a deliberate reversal can require an immediate speed reduction. Release/mode changes cancel the constraint.

**Per-axis acceleration:** X and Y use independent elapsed hold times. A newly introduced/reversed axis begins at its configured minimum, then ramps. Candidate velocities are limited uniformly by vector magnitude, interpolated from the previous velocity using `dt / transitionDuration`, and limited again to the active mode's maximum. Zero or reversed requested components cannot carry old velocity. An existing axis changes gradually during diagonal additions, except that lowering the configured maximum requires an immediate cap. Shared-vector normalization is used only when per-axis mode is off.

**Magnifier:** native windowed Magnification API, approximately 30 Hz on a separate desktop-services thread, visible while Precision is held. Its layered host is topmost, transparent to clicks, a tool window and non-activating. The host is excluded from capture, the child control is automatically excluded, and cleanup destroys the HWND hierarchy and uninitializes the API on the owning thread. Per-monitor-aware thread coordinates and source/host rectangles are physical pixels. Disabling destroys resources. Failures appear in settings; failed initialization waits for a settings refresh rather than retrying each frame. Rendering does not alter movement calculations.

The implementation choice follows Microsoft's [Magnification API overview](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/magapi/magapi-intro), [window filter documentation](https://learn.microsoft.com/en-us/windows/win32/api/magnification/nf-magnification-magsetwindowfilterlist), and [windowed sample](https://github.com/microsoft/Windows-classic-samples/blob/main/Samples/Magnification/cpp/Windowed/MagnifierSample.cpp). [DXGI Desktop Duplication](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api) was considered: it supplies capture frames but requires additional graphics/rendering ownership for a lens. The documented windowed magnifier already provides that rendering. WDDM support is required; no custom capture fallback is presented as implemented.

**History:** at most 64 session locations; directional episode starts and confirmed Raw Input final-direction releases, and before/after grid/bookmark teleports are meaningful events. It does not sample every timer tick. Duplicate current positions are omitted; a new event after Back replaces the forward branch. Navigation changes the index only after positioning succeeds and does not record itself. Disabled history stops recording/navigation and retains its session entries. Safety cancellation/lost-break recovery is not independently added as a history event.

**Bookmarks:** at most 64 persisted named positions containing original virtual-desktop coordinates, display identity/rectangle and normalized monitor coordinates. Capture runs after a three-second settings countdown, off the worker. Resolution follows a uniquely matching display identity across position/resolution changes. An unnamed display requires its original rectangle to match. Missing/ambiguous displays produce an error; positions do not silently map to another display. Navigation goes through the worker. Settings provide capture, Go and Delete.

**Application profiles:** at most 32 exact, case-insensitive absolute executable paths. Precedence is global feature availability → active permitted overrides → remaining global settings → deserialization defaults. Overrides are limited to movement curves and the four movement refinements. Profiles cannot change bindings, button ownership, global availability, magnifier, calibration or login registration. The service polls foreground process identity about every 200 ms and atomically applies changed effective configurations. An inaccessible foreground process falls back to global settings. Switching away removes overrides; global profile disable restores global settings. The service parks when profiles/magnification are unavailable, with no periodic wakeup solely for disabled features.

## Calibration metrics

Availability is separate from measurement. Save the calibration toggle, target size, distance and trial count, then explicitly begin Baseline or Experimental. Click the start circle to begin the monotonic timer, move using your controls, and click the destination circle. Trials alternate start/destination centers. Target dimensions and distances are physical pixels; a session rejects distances that cannot fit safely on its display. Cancel, calibration disable, KeyMouse disable, configuration changes, lock/suspend or emergency disable terminate measurement. Grid/bookmark/history positioning marks an in-progress trial as teleported.

Baseline disables the four optional movement refinements while retaining the same global curves and Precision settings. Experimental uses saved global movement settings. Foreground profiles are held out during sessions to keep comparisons repeatable. Completion/cancel restores current effective settings. Up to 200 trial results remain in memory; opening settings or merely enabling calibration collects no movement measurements.

- Elapsed time: start-target confirmation to destination confirmation, measured with `Instant`; includes command dispatch and the user's click time. It is not input-to-cursor latency.
- Generated path: sum of Euclidean distances between actual cursor positions read immediately before and after successful KeyMouse displacement calls.
- Straight distance: Euclidean distance from actual trial start to actual destination confirmation.
- Efficiency: straight distance / generated path, limited to 1; omitted for ambiguous movement, teleports, dropped samples or zero generated path. Teleport distances never enter generated path.
- Corrections: successive nonzero generated displacement vectors with a negative dot product. This is an operational definition, not a claim of detected human intent.
- Overshoots: generated segments crossing beyond the target-center plane in the initial start-to-target direction. Target-edge misses are not separately inferred.
- Ambiguity: an observed pre-update cursor position differs from the last recorded position, or the destination confirmation differs from the last sample. Physical motion concurrent with the API read/update sequence cannot always be distinguished. No reliable physical-mouse attribution is claimed.
- Sample gap: largest monotonic interval between recorded generated updates; includes idle/hold-delay time and is not a timer precision measurement.
- Dropped samples: measurement data lock was busy; worker sampling uses `try_lock` and reports loss instead of waiting. Each sample updates counters without allocating.

Results keep their effective configuration snapshot. Compare multiple matched trials in both orders. A single successful trial does not prove a configuration is superior. No unrelated typing, window titles, screen images or application-activity history is collected.

## Bindings and verification

The twelve F13–F24 assignments and Ctrl+Alt+G / Ctrl+Alt+Shift+K remain intact. New tools use settings and the explicit calibration overlay. No new global shortcuts or suppression hooks were added; configuring extra chords remains a future requirement. This avoids assuming unused F-keys or silently remapping the Silakka54.

The optional `diagnostics` Cargo feature builds a separate `keymouse-diagnostics` executable for native checks. It uses an isolated settings/WebView directory, registers no global hotkeys, and ignores physical keyboard events. Internal synthetic direction/modifier commands exercise the real worker and cursor adapter. Left-button presses are additionally allowed only while the diagnostic process owns the foreground; the harness uses a disposable browser text field. Releases remain available after focus changes for cleanup. Those commands, the extra executable and its isolation mode are absent from normal builds/installers. The harness restores cursor coordinates and stops its own process. It does not modify a running user's KeyMouse instance or login registration.

Run from the project with the local Rust environment available:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --lib calculation_timing -- --ignored --nocapture
cargo build --manifest-path src-tauri/Cargo.toml --features diagnostics --bin keymouse-diagnostics
# A Vite server must already be listening on port 1420:
npm.cmd run dev
# In another shell with normal Windows desktop access:
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/native-smoke.ps1
```

Displayed magnifier and cursor adapter checks require normal desktop access; the restricted sandbox can deny these APIs. Native synthetic checks do not certify physical keyboard timing, click-through behavior in every application, mixed-DPI layouts or long-term reliability. See [MOVEMENT_ACCEPTANCE.md](MOVEMENT_ACCEPTANCE.md).
