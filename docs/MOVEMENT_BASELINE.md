# Movement baseline

Baseline Git checkpoint: `09c84f5` (working v1). Existing user edits in Roadmap.md and SILAKKA54_SETUP.md are retained.

The unchanged movement implementation is preserved as the test-only `movement_baseline.rs`. Regression sequences use explicit monotonic seconds, a fixed seed, 20,000 ticks, directional additions/reversals/opposites, Precision/Turbo changes and resets. Every input displacement and tick result is compared, rather than just total distance.

Defaults: Normal 40–1400 px/s, 900 ms ramp, exponent 1.6; Precision 8–80 px/s, 700 ms ramp, exponent 1.8, 160 ms hold delay, 1 pixel tap; Turbo 900–4000 px/s, 350 ms ramp, exponent 1.2. Scroll repeat is enabled, delay 300 ms, interval 70 ms, amount 120. Grid Ctrl+Alt+G, depth 3. All twelve F13–F24 bindings are occupied.

Calculation path: Raw Input → InputState → Movement::input (precision taps) / Movement::tick (velocity and fractions) → Runtime::move_by → cursor_delta → actual GetCursorPos plus SetCursorPos. One worker owns continuous displacement and remainders.

The established no-noticeable-latency behavior is user-observed. Hardware latency, idle/active CPU, memory, traversal accuracy and correction counts have not been measured here. Pure calculation timings cannot certify input-to-cursor latency.
