# Silakka54 setup in Vial

KeyMouse needs standard **F13–F24 keyboard usages**, not QMK mouse keycodes. Nothing in this application changes firmware or applies a Vial layout automatically.

1. Connect both keyboard halves normally and open the Vial application that already recognizes your Silakka54. Confirm the correct keyboard is selected. Do not flash firmware for this setup.
2. Back up your current layout with Vial's **File → Save current layout** command (`Ctrl+S`). Save the `.vil` file somewhere you can find it, and retain a copy of any original firmware/layout files. Take a screenshot of the mouse layer if helpful. The save/load menu names are also documented in the [splitkb Vial usage guide](https://docs.splitkb.com/resources/firmware/vial/usage-guide).
3. Select your dedicated mouse layer. Keep the existing `MO(layer)`/layer-access key so you can enter and exit it. Leave normal typing layers intact. Make sure modifier keys needed for the emergency shortcut remain reachable outside the mouse layer.
4. Select each target position and assign the standard function key from Vial's keycode picker. Look for the function-key group; search for `F13` if your release provides search. These are `KC_F13` through `KC_F24` in QMK notation. If a key is absent from the picker, choose **Basic → Any**, or double-click the target position, and enter its QMK name such as `KC_F13`. Vial documents this [Any key editor](https://get.vial.today/manual/any-key.html). Use the named keycode; Windows virtual-key numbers used in KeyMouse's JSON are different from QMK keycodes. Do not use macros that emit repeated taps: KeyMouse needs ordinary held make/break transitions.

| Existing mouse-layer role | Vial key | QMK name | KeyMouse action |
| --- | --- | --- | --- |
| Cursor up / `MS_UP` | F13 | `KC_F13` | Up |
| Cursor down / `MS_DOWN` | F14 | `KC_F14` | Down |
| Cursor left / `MS_LEFT` | F15 | `KC_F15` | Left |
| Cursor right / `MS_RIGHT` | F16 | `KC_F16` | Right |
| Button 1 | F17 | `KC_F17` | Left button |
| Button 2 | F18 | `KC_F18` | Right button |
| Button 3 | F19 | `KC_F19` | Middle button |
| Wheel up | F20 | `KC_F20` | Scroll positive |
| Wheel down | F21 | `KC_F21` | Scroll negative |
| Fine movement modifier | F22 | `KC_F22` | Precision |
| Fast movement modifier | F23 | `KC_F23` | Turbo |
| Horizontal wheel modifier | F24 | `KC_F24` | Horizontal scroll |

5. Place F22 and F23 where they can be held comfortably while pressing directions. They are momentary keys, not toggles. Place F24 near wheel controls. Holding F24 with F20 scrolls right; with F21 it scrolls left. Keep Ctrl, Alt, Shift and K accessible for emergency disable.
6. Remove old QMK cursor/button/wheel keycodes from the positions now assigned to these controls. Otherwise firmware and KeyMouse can both move the cursor or produce duplicate events. Retain a separate firmware mouse layer if you want a fallback when KeyMouse is not running; use different layer access so both systems are not active together.
7. Launch KeyMouse, enable it, and open Diagnostics. Hold one control at a time and check the last recognized action. F13 must show `Up pressed`, followed by `Up released`. This panel stores only the latest control, never a typed-key history. Check F14–F24 the same way. If nothing is recognized, verify the layer and exact function-key number.
8. Hold F22 and tap a cardinal direction: the default precision step must be one physical pixel. Test a short hold, then a hold longer than 160 ms. Hold F23 to test turbo. Test buttons and drag on an expendable window or test page; do not start with destructive application controls.
9. Test **Ctrl+Alt+Shift+K** using ordinary keyboard access. The cursor should stop and the tray/settings should show disabled. Re-enable from the physical mouse or settings keyboard navigation. Keep your physical mouse connected until all acceptance tests pass.
10. Save the new layout separately (for example `silakka54-keymouse.vil`). To roll back, disable/quit KeyMouse, use **File → Load saved layout** (`Ctrl+O`) in Vial to load the original `.vil`, and verify the restored mouse layer. Vial normally applies edits to the keyboard immediately; saving a file is your backup, rather than an application step. Restore keys individually from your screenshot if necessary. See [Vial's description of live configuration](https://get.vial.today/).

If you change bindings in KeyMouse, make corresponding assignments in Vial. The application validates that all twelve controls are distinct F13–F24 keys. QMK/Vial firmware support and UI vary; this guide assumes your existing firmware already works with Vial. It does not prescribe a firmware upgrade.
