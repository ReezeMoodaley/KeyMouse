// Runs against the optional, isolated diagnostic binary, never a user's normal instance.
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
const port = Number(process.argv[2] ?? 9231);
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let target;
for (let i = 0; i < 100; i++) {
  try {
    const list = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
    target = list.find(
      (t) =>
        t.type === "page" &&
        !t.url.includes("grid=") &&
        !t.url.includes("calibration="),
    );
    if (target) break;
  } catch {
    /* The isolated WebView's local port is not ready yet. */
  }
  await pause(200);
}
assert(target, "Diagnostic WebView did not start");
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});
let sequence = 0;
const pending = new Map();
socket.addEventListener("message", (e) => {
  const m = JSON.parse(e.data);
  if (m.id) {
    const callback = pending.get(m.id);
    pending.delete(m.id);
    callback?.(m);
  }
});
async function evaluate(expression) {
  const id = ++sequence;
  const reply = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(Error("Native command timed out"));
    }, 10000);
    pending.set(id, (m) => {
      clearTimeout(timer);
      resolve(m);
    });
    socket.send(
      JSON.stringify({
        id,
        method: "Runtime.evaluate",
        params: { expression, awaitPromise: true, returnByValue: true },
      }),
    );
  });
  if (reply.error || reply.result.exceptionDetails)
    throw Error(JSON.stringify(reply.error ?? reply.result.exceptionDetails));
  return reply.result.result.value;
}
const invoke = (command, args = {}) =>
  evaluate(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`,
  );
const diagnostic = (action, args = {}) =>
  invoke("diagnostic", {
    action,
    x: null,
    y: null,
    key: null,
    down: null,
    ...args,
  });
const key = (key, down) => diagnostic("key", { key, down });
let ready = false;
for (let i = 0; i < 100; i++) {
  try {
    ready = await evaluate("Boolean(window.__TAURI_INTERNALS__)");
    if (ready) break;
  } catch {
    /* Navigation can replace the execution context before IPC initializes. */
  }
  await pause(100);
}
assert(ready, "Diagnostic Tauri IPC did not initialize");
const report = {
  checks: [],
  nativeInput:
    "Synthetic internal controls; no physical hardware certification",
};
const passed = (name) => {
  report.checks.push(name);
  console.log(`PASS ${name}`);
};
let original;
try {
  const initial = await invoke("get_snapshot");
  assert(initial.status.running, initial.status.error);
  original = await diagnostic("position");
  let c = structuredClone(initial.config);
  assert(
    Object.values(c.features).every((v) =>
      typeof v === "boolean" ? !v : !v.enabled,
    ),
  );
  passed("all nine features default disabled; native worker running");
  const dragConfig = structuredClone(c);
  for (const mode of ["normal", "turbo", "precision"]) {
    c[mode].min_speed = 200;
    c[mode].max_speed = 200;
  }
  await invoke("save_config", { config: c });
  const origin = await diagnostic("focus");
  await pause(200);
  const field = await evaluate(`(() => {
    const host = document.createElement('div');
    host.id = 'drag-fixture';
    host.style.cssText = 'position:fixed;inset:0;z-index:2147483647;background:white';
    const field = document.createElement('textarea');
    field.id = 'drag-field';
    field.value = 'Disposable KeyMouse text selection regression. '.repeat(8);
    field.style.cssText = 'position:absolute;left:50px;top:100px;width:600px;height:120px;font:20px monospace;color:black;background:white';
    host.append(field); document.body.append(host);
    window.dragEvents = [];
    window.dragListener = e => window.dragEvents.push({type:e.type, buttons:e.buttons});
    for (const type of ['mousedown', 'mouseup', 'mousemove']) {
      // Selection can capture across the textarea boundary; release belongs to
      // the document even when the final cursor is outside the original field.
      document.addEventListener(type, window.dragListener, true);
    }
    const r = field.getBoundingClientRect();
    return [(r.left+10)*devicePixelRatio, (r.top+15)*devicePixelRatio];
  })()`);
  for (const [mode, modifier] of [
    ["Normal", null],
    ["Turbo", 0x86],
    ["Precision", 0x85],
  ]) {
    for (let repeat = 0; repeat < 3; repeat++) {
      await diagnostic("reposition", {
        x: origin[0] + Math.round(field[0]),
        y: origin[1] + Math.round(field[1]),
      });
      const dragStart = await diagnostic("position");
      await evaluate(
        "document.getElementById('drag-field').setSelectionRange(0,0); window.dragEvents=[]",
      );
      if (modifier) await key(modifier, true);
      await key(0x80, true);
      await pause(180);
      const heldStatus = (await invoke("get_snapshot")).status;
      const heldPosition = await diagnostic("position");
      await key(0x7f, true);
      await pause(400);
      await key(0x7f, false);
      await key(0x80, false);
      if (modifier) await key(modifier, false);
      await pause(50);
      const result = await evaluate(`(() => {
        const f=document.getElementById('drag-field');
        return {selected:Math.abs(f.selectionEnd-f.selectionStart), events:window.dragEvents};
      })()`);
      assert(
        result.selected > 0,
        `${mode} did not select browser text: ${JSON.stringify({ result, dragStart, heldPosition, heldStatus, finalStatus: (await invoke("get_snapshot")).status, position: await diagnostic("position"), origin, field })}`,
      );
      assert.equal(
        result.events.filter((e) => e.type === "mousedown").length,
        1,
      );
      assert.equal(
        result.events.filter((e) => e.type === "mouseup").length,
        1,
        JSON.stringify({
          mode,
          result,
          dragStart,
          heldPosition,
          position: await diagnostic("position"),
          origin,
          field,
        }),
      );
      assert(
        result.events.some((e) => e.type === "mousemove" && e.buttons & 1),
      );
    }
    passed(
      `native browser text dragging: ${mode}, three repetitions with a held left button`,
    );
  }
  await evaluate(
    "for (const type of ['mousedown','mouseup','mousemove']) document.removeEventListener(type, window.dragListener, true); document.getElementById('drag-fixture').remove()",
  );
  c = dragConfig;
  await invoke("save_config", { config: c });
  await diagnostic("reposition", { x: original[0], y: original[1] });
  c.features.history_enabled = true;
  c.features.bookmarks_enabled = true;
  await invoke("save_config", { config: c });
  await invoke("cursor_tool", {
    action: "capture",
    index: null,
    name: "Original cursor",
  });
  c = (await invoke("get_snapshot")).config;
  assert.equal(c.bookmarks.length, 1);
  // Capture deliberately samples after a countdown; its location can differ from
  // the cursor saved before the browser fixture (focus/physical input can move it).
  const capturedBookmark = [c.bookmarks[0].x, c.bookmarks[0].y];
  passed("bookmark capture persisted in isolated settings");
  await invoke("open_grid");
  await invoke("grid_command", { action: "confirm", region: null });
  const center = await diagnostic("position");
  assert.notDeepEqual(center, original);
  passed("grid reposition coordinated through worker");
  await key(0x85, true);
  for (let i = 0; i < 10; i++) {
    await key(0x7f, true);
    await pause(4);
    await key(0x7f, false);
    await pause(4);
  }
  const precise = await diagnostic("position");
  assert.deepEqual(precise, [center[0] + 10, center[1]]);
  await key(0x85, false);
  passed("native Precision: ten taps move exactly ten physical pixels");
  c.normal = { min_speed: 100, max_speed: 100, ramp_ms: 100, exponent: 1 };
  c.features.tap = {
    enabled: true,
    strategy: "responsive",
    step: 3,
    hold_ms: 150,
  };
  c.features.axes.enabled = true;
  c.features.acceleration.enabled = true;
  c.features.acceleration.strategy = "linear";
  c.features.braking.enabled = true;
  c.features.magnifier.enabled = true;
  await invoke("save_config", { config: c });
  const before = await diagnostic("position");
  await key(0x7f, true);
  await pause(80);
  const active = await diagnostic("position");
  assert(active[0] > before[0]);
  for (const feature of ["tap", "axes", "acceleration", "braking"])
    c.features[feature].enabled = false;
  await invoke("save_config", { config: c });
  await pause(50);
  const changed = await diagnostic("position");
  assert(changed[0] > active[0]);
  await key(0x7f, false);
  await pause(10);
  const stopped = await diagnostic("position");
  await pause(50);
  assert.deepEqual(await diagnostic("position"), stopped);
  passed(
    "live feature deactivation preserves held direction; release stops immediately",
  );
  c.features.tap = {
    enabled: true,
    strategy: "discrete",
    step: 1,
    hold_ms: 150,
  };
  await invoke("save_config", { config: c });
  await key(0x7f, true);
  await pause(190);
  const cardinal = await diagnostic("position");
  await key(0x7d, true);
  await pause(5);
  const perpendicularTap = await diagnostic("position");
  assert.equal(perpendicularTap[1], cardinal[1] + 1);
  await pause(55);
  const waitingAxis = await diagnostic("position");
  assert.equal(waitingAxis[1], perpendicularTap[1]);
  assert(waitingAxis[0] > perpendicularTap[0]);
  await key(0x7d, false);
  await key(0x7f, false);
  c.features.tap = {
    enabled: false,
    strategy: "responsive",
    step: 3,
    hold_ms: 150,
  };
  await invoke("save_config", { config: c });
  passed(
    "native Discrete Tap: new axis waits independently while the existing axis continues",
  );
  await key(0x85, true);
  await pause(150);
  assert.equal((await invoke("get_snapshot")).service_error, null);
  await key(0x85, false);
  passed("magnifier service activates without a runtime error");
  await invoke("cursor_tool", { action: "bookmark", index: 0, name: null });
  assert.deepEqual(await diagnostic("position"), capturedBookmark);
  const h = await invoke("cursor_tools_snapshot");
  assert(h.history.length >= 2);
  await invoke("cursor_tool", { action: "back", index: null, name: null });
  assert.equal(
    (await invoke("cursor_tools_snapshot")).history.length,
    h.history.length,
  );
  passed(
    "bookmark restores native position; history navigation does not recurse",
  );
  const exe = await diagnostic("foreground");
  c.profiles = [
    {
      name: "Native smoke",
      executable: exe,
      enabled: true,
      normal: { ...c.normal, max_speed: 123 },
      precision: null,
      turbo: null,
      tap: null,
      axes: null,
      acceleration: null,
      braking: null,
    },
  ];
  c.features.profiles_enabled = true;
  await invoke("save_config", { config: c });
  await pause(300);
  assert.equal((await invoke("get_snapshot")).status.normal_max_speed, 123);
  c.features.profiles_enabled = false;
  await invoke("save_config", { config: c });
  await pause(250);
  assert.equal((await invoke("get_snapshot")).status.normal_max_speed, 100);
  passed("foreground profile applies and disables without stale overrides");
  c.features.calibration = {
    enabled: true,
    target_size: 20,
    distance: 20,
    trials: 1,
  };
  for (const f of ["tap", "axes", "acceleration", "braking"])
    c.features[f].enabled = true;
  await invoke("save_config", { config: c });
  let session = (await invoke("calibration_command", { action: "baseline" }))
    .session;
  assert(session);
  await diagnostic("reposition", { x: session.start[0], y: session.start[1] });
  await invoke("calibration_command", { action: "target" });
  await key(0x85, true);
  for (let i = 0; i < 20; i++) {
    await key(0x7f, true);
    await pause(3);
    await key(0x7f, false);
    await pause(3);
  }
  await key(0x85, false);
  let measurement = await invoke("calibration_command", { action: "target" });
  assert.equal(measurement.results.at(-1).samples, 20);
  assert.equal(measurement.results.at(-1).generated_path, 20);
  assert.equal(measurement.results.at(-1).efficiency, 1);
  passed(
    "calibration baseline records actual generated positions and monotonic trial metrics",
  );
  session = (await invoke("calibration_command", { action: "experimental" }))
    .session;
  await diagnostic("reposition", { x: session.start[0], y: session.start[1] });
  await invoke("calibration_command", { action: "target" });
  await diagnostic("reposition", {
    x: session.target[0],
    y: session.target[1],
  });
  measurement = await invoke("calibration_command", { action: "target" });
  assert.equal(measurement.results.at(-1).teleports, 1);
  assert.equal(measurement.results.at(-1).efficiency, null);
  passed("calibration excludes teleportation from efficiency");
  for (const f of [
    "tap",
    "axes",
    "acceleration",
    "braking",
    "magnifier",
    "calibration",
  ])
    c.features[f].enabled = false;
  c.features.history_enabled = false;
  c.features.bookmarks_enabled = false;
  c.features.profiles_enabled = false;
  await invoke("save_config", { config: c });
  assert.equal((await invoke("get_snapshot")).config.features.tap.step, 3);
  assert.equal((await invoke("get_snapshot")).config.bookmarks.length, 1);
  await assert.rejects(() =>
    invoke("cursor_tool", { action: "bookmark", index: 0, name: null }),
  );
  await assert.rejects(() =>
    invoke("calibration_command", { action: "baseline" }),
  );
  passed("disabled subsystems reject activation and retain saved settings");
  report.completed = new Date().toISOString();
  await writeFile(
    "src-tauri/target/native-smoke-report.json",
    JSON.stringify(report, null, 2) + "\n",
  );
} finally {
  try {
    for (const k of [0x7c, 0x7d, 0x7e, 0x7f, 0x80, 0x85, 0x86])
      await key(k, false);
    if (original)
      await diagnostic("reposition", { x: original[0], y: original[1] });
    await diagnostic("quit");
  } finally {
    socket.close();
  }
}
