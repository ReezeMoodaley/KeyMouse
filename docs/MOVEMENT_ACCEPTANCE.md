# Baseline versus experimental acceptance

Repeat each exercise at least ten times with identical curves, desktop layout, target distance/size and keyboard. First disable all four movement refinements, then enable one at a time, then the intended combination. Alternate the testing order. Record configuration snapshots, successes, elapsed time, overshoot/correction counts and errors. These exercises are instructions, not claims that physical testing was performed.

| Exercise | Baseline expectation | Experimental comparison |
| --- | --- | --- |
| Move exactly 10 pixels | Hold Precision, ten taps; exactly 10 physical pixels | Same with every feature combination; optional tap cannot add Precision displacement |
| Move 100 pixels and stop | Stops on final release; fractional remainder cannot drift | Compare short-move accuracy and release response for both tap strategies |
| Move 500 pixels | Record traversal time and actual stopping position | Compare overshoot/correction counts with braking off/on |
| Cross a 1920-pixel display | Record traversal time in Normal and Turbo | Compare strategies/axes without exceeding mode maximum |
| Cross a 3840-pixel display | Repeat on an actual 4K or matching desktop path | Keep curves/mode fixed; document monitor and DPI layout |
| Select a 12-pixel target | Calibration target size 12, repeated trials | Compare both orders with at least 10 trials per group |
| Select a 20-pixel target | Calibration target size 20, same distance/trial count | Compare trial distributions, not just a best result |
| Reverse after sustained movement | Immediate requested sign, no inertia | Braking cap and smooth recovery; late re-press should not brake |
| Hold opposing keys | Opposing axis cancels | Must not be counted as an intentional correction |
| Add perpendicular direction at high speed | Shared ramp with normalized diagonal vector | Existing axis transitions smoothly; total vector speed remains bounded |
| Remove a diagonal axis | Cardinal direction remains active | Removed axis stops immediately; retained axis clock is preserved |
| Normal ↔ Precision ↔ Turbo | Original modifier priority and mode-entry behavior | Braking cancels; Precision tap remains singular; magnifier does not take focus |
| Toggle each feature while moving | Direction remains held across a settings update | No cursor jump, stale delay or residual braking; held drag button stays owned |
| Change foreground application while moving | Global settings remain valid | Matching profile activates within service polling interval; unmatched/disabled profile restores global settings; drag ownership remains |
| Grid/bookmark/history while refinements enabled | Explicit position replaces continuous episode | Fractions/timestamps reset, directions require release, history does not recurse, calibration marks teleportation |
| Missing/rearranged monitor | Saved known display follows its new rectangle | Removed/ambiguous display reports an error without moving elsewhere |
| Disable magnifier / calibration mid-session | No remaining resources or measurement | Lens disappears, overlay hides, trial ends and current effective settings restore |

## Performance worksheet

| Metric | Baseline | Experimental | Method / qualification |
| --- | --- | --- | --- |
| Physical input-to-cursor median/p95/max | Unmeasured | Unmeasured | At least 1,000 physical Precision taps; timestamp receipt and first native dispatch, include machine/polling/load |
| Movement update interval/jitter | Unmeasured | Unmeasured | ETW/WPR trace under fixed desktop load; diagnostics maxima are not a complete timing distribution |
| Active CPU | Unmeasured | Unmeasured | Same sustained held direction, settings hidden, include Rust host and WebView child processes |
| Idle CPU | Unmeasured | Unmeasured | Ten-minute hidden-settings sessions; disabled service should park |
| Private bytes/handles/threads | Unmeasured | Unmeasured | Start, after repeated toggles, and after eight hours |
| Short movement error | Manual exercise required | Manual exercise required | Actual physical coordinates after 10/100-pixel targets |
| Long traversal time | Manual exercise required | Manual exercise required | 500/1920/3840-pixel exercises |
| Overshooting/corrections | Session trials required | Session trials required | Use documented calibration definitions; exclude ambiguous/teleported trials |
| Pure movement calculation cost | Frozen v1 oracle | All features disabled | Explicit debug-build timing test; excludes OS, scheduling, UI, input and physical hardware |

Do not translate a pure tick microbenchmark into a hardware-latency or whole-process CPU claim. Native scripted Precision checks validate actual pixel positions with internal synthetic controls; they do not measure Silakka54 performance.
