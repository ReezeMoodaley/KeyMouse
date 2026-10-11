import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { join } from "node:path";
const port = Number(process.argv[2] ?? 9232),
  directory = process.argv[3],
  second = process.argv[4] === "1";
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const clients = [];
async function connect(fragment = "") {
  let target;
  for (let i = 0; i < 100; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
      target = list.find(
        (t) =>
          t.type === "page" &&
          (fragment
            ? t.url.includes(fragment)
            : !t.url.includes("mini=") &&
              !t.url.includes("grid=") &&
              !t.url.includes("calibration=")),
      );
      if (target) break;
    } catch {
      /* Window is starting. */
    }
    await pause(100);
  }
  assert(target, "WebView target unavailable");
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
      pending.get(m.id)?.(m);
      pending.delete(m.id);
    }
  });
  const call = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const id = ++sequence;
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(Error(`${method} timed out`));
      }, 10000);
      pending.set(id, (m) => {
        clearTimeout(timer);
        if (m.error) reject(Error(JSON.stringify(m.error)));
        else resolve(m.result);
      });
      socket.send(JSON.stringify({ id, method, params }));
    });
  const evaluate = async (expression) => {
    const r = await call("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (r.exceptionDetails) throw Error(JSON.stringify(r.exceptionDetails));
    return r.result.value;
  };
  const invoke = (command, args = {}) =>
    evaluate(
      `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`,
    );
  const client = { call, evaluate, invoke, socket };
  clients.push(client);
  return client;
}
const checks = [];
const passed = (name) => {
  checks.push(name);
  console.log(`PASS ${name}`);
};
let main;
try {
  main = await connect();
  for (let i = 0; i < 100; i++) {
    if (await main.evaluate("Boolean(document.querySelector('.dashboard'))"))
      break;
    await pause(100);
  }
  assert(await main.evaluate("Boolean(document.querySelector('.dashboard'))"));
  const snapshot = await main.invoke("get_snapshot");
  assert.equal(snapshot.config.enabled, false);
  passed("isolated engine remains disabled");
  let state = await main.invoke("theme_load");
  if (second) {
    assert.equal(state.active.name, "Native UI custom");
    assert.equal(state.active.effects.intensity, 80);
    assert.equal(state.customs.length, 1);
    assert.equal(
      await main.evaluate(
        "getComputedStyle(document.documentElement).getPropertyValue('--km-accent-primary').trim()",
      ),
      "#78DEEE",
    );
    passed(
      "active theme, advanced settings and custom theme survive native restart",
    );
  } else {
    await main.call("Emulation.setDeviceMetricsOverride", {
      width: 1440,
      height: 900,
      deviceScaleFactor: 1,
      mobile: false,
    });
    assert.equal(
      await main.evaluate("document.documentElement.dataset.hidden"),
      "true",
    );
    passed("initial hidden native window pauses decorative animation");
    const defaultShot = await main.call("Page.captureScreenshot", {
      format: "png",
    });
    await writeFile(
      join(directory, "default-dashboard.png"),
      Buffer.from(defaultShot.data, "base64"),
    );
    const originalTheme = structuredClone(state.active);
    const clip = await main.evaluate(`(() => {
      const r = document.querySelector('.sidebar .crt-text').getBoundingClientRect();
      return {x:Math.floor(r.x)-12,y:Math.floor(r.y)-12,width:Math.ceil(r.width)+24,height:Math.ceil(r.height)+24,scale:1};
    })()`);
    const visibility = [];
    for (const intensity of [0, 20, 50, 100]) {
      const draft = structuredClone(state);
      draft.active.effects.intensity = intensity;
      state = await main.invoke("theme_save", { state: draft });
      await pause(150);
      const shot = await main.call("Page.captureScreenshot", {
        format: "png",
        clip,
      });
      await writeFile(
        join(directory, `distortion-${intensity}.png`),
        Buffer.from(shot.data, "base64"),
      );
      const difference = await main.evaluate(`(async () => {
        const img = new Image(); img.src = 'data:image/png;base64,${shot.data}'; await img.decode();
        const canvas = document.createElement('canvas'); canvas.width=img.width;canvas.height=img.height;
        const ctx=canvas.getContext('2d');ctx.drawImage(img,0,0);
        const pixels=ctx.getImageData(0,0,img.width,img.height).data;
        if (!window.__crtBaseline) {window.__crtBaseline=Array.from(pixels);return 0;}
        let difference=0;for(let i=0;i<pixels.length;i++) if(i%4!==3) difference+=Math.abs(pixels[i]-window.__crtBaseline[i]);
        return difference;
      })()`);
      visibility.push({ intensity, difference });
    }
    assert(
      visibility[1].difference > 100,
      "default distortion must visibly alter rendered pixels",
    );
    assert(visibility[2].difference > visibility[1].difference);
    assert(visibility[3].difference > visibility[2].difference);
    await writeFile(
      join(directory, "distortion-visibility.json"),
      JSON.stringify(visibility, null, 2),
    );
    state.active = originalTheme;
    state = await main.invoke("theme_save", { state });
    passed(
      "rendered CRT pixels visibly increase at 20, 50 and 100 percent versus zero",
    );
    for (const page of [
      "Dashboard",
      "Movement",
      "Keymapping",
      "Calibration",
      "Profiles",
      "Diagnostics",
      "General",
      "About",
      "Refinements",
      "Cursor tools",
      "Precision guide",
      "Mouse & scrolling",
      "Grid navigation",
      "Theme Studio",
    ]) {
      await main.evaluate(
        `document.querySelector('nav button[aria-label="${page}"]').click()`,
      );
      await pause(180);
      const layout = await main.evaluate(
        `({heading:document.querySelector('h1').textContent,overflow:document.querySelector('.content').scrollWidth-document.querySelector('.content').clientWidth})`,
      );
      assert.equal(layout.heading, page);
      assert(layout.overflow <= 1, `${page} content overflows at 1440`);
    }
    passed(
      "all fourteen pages render at desktop width without horizontal overflow",
    );
    await main.evaluate(
      "document.querySelector('.preset-selector button:nth-child(3)').click()",
    );
    await pause(50);
    assert.equal((await main.invoke("theme_load")).active.id, "phosphor-green");
    passed("draft preset selection does not mutate native active theme");
    const style = await main.evaluate(
      "({input:getComputedStyle(document.querySelector('[aria-label=\"Preview value\"]')).textShadow,word:getComputedStyle(document.querySelector('.preview-interface .crt-text')).textShadow,pointer:getComputedStyle(document.querySelector('.preview-interface'),'::before').pointerEvents})",
    );
    assert.equal(style.input, "none");
    assert.notEqual(style.word, "none");
    assert.equal(style.pointer, "none");
    passed(
      "native WebView renders layered shadows with sharp inputs and noninteractive decoration",
    );
    await main.invoke("open_mini");
    const mini = await connect("mini=");
    await pause(400);
    const next = structuredClone(state);
    next.active.colors["accent-primary"] = "#78DEEE";
    next.active.name = "Native UI custom";
    next.active.id = "custom-native-ui";
    next.active.effects.intensity = 80;
    next.active.effects.ghost = 65;
    next.active.effects.displacement = 0.9;
    next.customs = [structuredClone(next.active)];
    await main.invoke("theme_save", { state: next });
    await pause(150);
    assert.equal(
      await mini.evaluate(
        "getComputedStyle(document.documentElement).getPropertyValue('--km-accent-primary').trim()",
      ),
      "#78DEEE",
    );
    assert.equal(
      await main.evaluate(
        "getComputedStyle(document.documentElement).getPropertyValue('--km-accent-primary').trim()",
      ),
      "#78DEEE",
    );
    passed(
      "main and Mini Control Panel synchronize through native theme events",
    );
    const miniOffset = await mini.evaluate(
      "parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--km-crt-ghost-offset'))",
    );
    assert(miniOffset < 0.8 * 0.9);
    passed("mini CRT displacement is scaled down");
    const miniShot = await mini.call("Page.captureScreenshot", {
      format: "png",
    });
    await writeFile(
      join(directory, "mini.png"),
      Buffer.from(miniShot.data, "base64"),
    );
    // Native navigation must deliver the destination back to the full app.
    await mini.invoke("open_page", { page: "Theme Studio" });
    await pause(100);
    assert.equal(
      await main.evaluate("document.querySelector('h1').textContent"),
      "Theme Studio",
    );
    passed("mini routes Theme Studio to full app");
    for (const width of [1200, 1000, 800]) {
      await main.call("Emulation.setDeviceMetricsOverride", {
        width,
        height: 900,
        deviceScaleFactor: 1,
        mobile: false,
      });
      for (const page of [
        "Dashboard",
        "Movement",
        "Keymapping",
        "Calibration",
        "Profiles",
        "Diagnostics",
        "General",
        "About",
        "Theme Studio",
      ]) {
        await main.evaluate(
          `document.querySelector('nav button[aria-label="${page}"]').click()`,
        );
        await pause(100);
        assert(
          await main.evaluate(
            "document.querySelector('.content').scrollWidth<=document.querySelector('.content').clientWidth+1",
          ),
          `${page} overflows at ${width}`,
        );
      }
    }
    passed(
      "all nine redesigned workspaces reflow at every responsive breakpoint",
    );
    await main.call("Emulation.setDeviceMetricsOverride", {
      width: 1440,
      height: 900,
      deviceScaleFactor: 1,
      mobile: false,
    });
    await main.evaluate("document.querySelector('.content').scrollTop=0");
    const shot = await main.call("Page.captureScreenshot", { format: "png" });
    await writeFile(
      join(directory, "theme-studio.png"),
      Buffer.from(shot.data, "base64"),
    );
    await main.evaluate(
      "document.querySelector('nav button[aria-label=Dashboard]').click()",
    );
    await pause(100);
    const dashboard = await main.call("Page.captureScreenshot", {
      format: "png",
    });
    await writeFile(
      join(directory, "dashboard.png"),
      Buffer.from(dashboard.data, "base64"),
    );
    // Actual reduced-motion rendering and live effect changes.
    let active = await main.invoke("theme_load");
    active.active.effects.flicker = true;
    active.active.effects.scanlines = true;
    active.active.effects.noise = true;
    await main.invoke("theme_save", { state: active });
    await main.call("Emulation.setEmulatedMedia", {
      features: [{ name: "prefers-reduced-motion", value: "reduce" }],
    });
    await pause(100);
    assert.equal(
      await main.evaluate(
        "getComputedStyle(document.querySelector('.crt-readout')).animationName",
      ),
      "none",
    );
    passed("native reduced-motion preference stops flicker");
    active = await main.invoke("theme_load");
    active.active.effects.intensity = 0;
    await main.invoke("theme_save", { state: active });
    await pause(100);
    assert.equal(
      await main.evaluate(
        "getComputedStyle(document.documentElement).getPropertyValue('--km-crt-ghost-offset').trim()",
      ),
      "0px",
    );
    assert.equal(
      await main.evaluate(
        "getComputedStyle(document.documentElement).getPropertyValue('--km-crt-channel-offset').trim()",
      ),
      "0px",
    );
    passed("zero distortion clears native ghost and channel offsets");
    active = await main.invoke("theme_load");
    active.active.effects.intensity = 80;
    active.active.effects.flicker = false;
    active.active.effects.scanlines = false;
    active.active.effects.noise = false;
    await main.invoke("theme_save", { state: active });
  }
  await writeFile(
    join(directory, second ? "restart-report.json" : "ui-report.json"),
    JSON.stringify(
      {
        checks,
        notes:
          "No cursor movement or button injection. Desktop WebView rendering and native theme/window IPC only.",
      },
      null,
      2,
    ),
  );
} finally {
  if (main)
    try {
      await main.invoke("diagnostic", {
        action: "quit",
        x: null,
        y: null,
        key: null,
        down: null,
      });
    } catch {
      /* Process may already be closing. */
    }
  clients.forEach((c) => c.socket.close());
}
