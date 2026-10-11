#![cfg(windows)]
use crate::{
    config::Config,
    engine::{Command, Status},
    input::InputState,
    movement::{Mode, Movement},
    scroll::ScrollScheduler,
};
use std::{
    cell::RefCell,
    collections::{BTreeSet, VecDeque},
    mem::size_of,
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::Manager;
use windows::{
    core::w,
    Win32::{
        Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WAIT_FAILED, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW,
            RemoteDesktop::{
                WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
                NOTIFY_FOR_THIS_SESSION,
            },
            Threading::{CreateWaitableTimerExW, SetWaitableTimer, INFINITE, TIMER_ALL_ACCESS},
        },
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, RegisterHotKey, SendInput, UnregisterHotKey, INPUT, INPUT_0,
                INPUT_MOUSE, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOUSEEVENTF_HWHEEL,
                MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
                MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
                MOUSEEVENTF_WHEEL, MOUSEINPUT, MOUSE_EVENT_FLAGS,
            },
            Input::{
                GetRawInputData, GetRawInputDeviceList, RegisterRawInputDevices, HRAWINPUT,
                RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICELIST, RAWINPUTHEADER, RIDEV_DEVNOTIFY,
                RIDEV_INPUTSINK, RIDEV_REMOVE, RID_INPUT, RIM_TYPEKEYBOARD,
            },
            WindowsAndMessaging::*,
        },
    },
};

enum Event {
    Key(usize, u16, bool),
    Removed(usize),
    Devices,
    Cancel,
    Session(bool),
    Emergency,
    Grid,
}
thread_local! {static EVENTS:RefCell<Vec<Event>>=const {RefCell::new(Vec::new())};}
fn enqueue(e: Event) {
    EVENTS.with(|q| q.borrow_mut().push(e));
}
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: this callback is installed only for our worker-owned window. Raw input is
    // copied while its handle is valid; only selected control VKs are retained.
    unsafe {
        match msg {
            WM_INPUT => {
                let mut raw = RAWINPUT::default();
                let mut bytes = size_of::<RAWINPUT>() as u32;
                let read = GetRawInputData(
                    HRAWINPUT(lp.0 as *mut _),
                    RID_INPUT,
                    Some((&mut raw as *mut RAWINPUT).cast()),
                    &mut bytes,
                    size_of::<RAWINPUTHEADER>() as u32,
                );
                if read != u32::MAX
                    && read >= size_of::<RAWINPUTHEADER>() as u32
                    && raw.header.dwType == RIM_TYPEKEYBOARD.0
                {
                    let k = raw.data.keyboard;
                    if (0x7c..=0x87).contains(&k.VKey) {
                        enqueue(Event::Key(
                            raw.header.hDevice.0 as usize,
                            k.VKey,
                            k.Flags & 1 == 0,
                        ));
                    }
                }
                return DefWindowProcW(hwnd, msg, wp, lp);
            }
            WM_INPUT_DEVICE_CHANGE => {
                if wp.0 == 2 {
                    enqueue(Event::Removed(lp.0 as usize));
                }
                enqueue(Event::Devices);
            }
            WM_HOTKEY => match wp.0 {
                1 => enqueue(Event::Emergency),
                2 => enqueue(Event::Grid),
                _ => {}
            },
            WM_WTSSESSION_CHANGE => {
                if [2, 4, 7].contains(&wp.0) {
                    enqueue(Event::Session(true));
                } else if [1, 3, 8].contains(&wp.0) {
                    enqueue(Event::Session(false));
                }
            }
            WM_POWERBROADCAST => {
                if wp.0 == 4 {
                    enqueue(Event::Session(true));
                } else if [6, 7, 18].contains(&wp.0) {
                    enqueue(Event::Session(false));
                }
            }
            WM_ENDSESSION if wp.0 != 0 => {
                enqueue(Event::Cancel);
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}
pub fn wake(hwnd: usize) {
    if hwnd == 0 {
        return;
    }
    // SAFETY: HWND is worker-owned; posting to an already-destroyed window simply fails.
    unsafe {
        let _ = PostMessageW(Some(HWND(hwnd as *mut _)), WM_APP, WPARAM(0), LPARAM(0));
    }
}
fn win<T>(r: windows::core::Result<T>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}
pub fn cursor_delta(dx: i32, dy: i32) -> Result<(), String> {
    if dx == 0 && dy == 0 {
        return Ok(());
    } // SAFETY: stack POINT is valid; signed coordinates preserve negative monitor positions.
    unsafe {
        let mut p = POINT::default();
        win(GetCursorPos(&mut p))?;
        win(SetCursorPos(p.x.saturating_add(dx), p.y.saturating_add(dy)))
    }
}
pub fn cursor_at(x: i32, y: i32) -> Result<(), String> {
    // SAFETY: signed physical screen coordinates are value arguments; Windows clamps desktop bounds.
    unsafe { win(SetCursorPos(x, y)) }
}
pub fn startup_error(message: &str) {
    use windows::core::PCWSTR;
    let text:Vec<u16>=format!("KeyMouse could not start.\n\n{message}\n\nCheck WebView2 and application data directory access.").encode_utf16().chain(Some(0)).collect();
    // SAFETY: the terminated message stays alive for the modal call.
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            w!("KeyMouse startup error"),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn mouse(flags: MOUSE_EVENT_FLAGS, data: i32) -> Result<(), String> {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                mouseData: data as u32,
                ..Default::default()
            },
        },
    };
    // SAFETY: the INPUT union contains MOUSEINPUT and remains valid throughout the call.
    let n = unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };
    if n == 1 {
        Ok(())
    } else {
        Err(
            "SendInput was rejected. Check session access and elevated target applications (UIPI)."
                .into(),
        )
    }
}
fn button(i: usize, down: bool) -> Result<(), String> {
    let flags = match (i, down) {
        (0, true) => MOUSEEVENTF_LEFTDOWN,
        (0, false) => MOUSEEVENTF_LEFTUP,
        (1, true) => MOUSEEVENTF_RIGHTDOWN,
        (1, false) => MOUSEEVENTF_RIGHTUP,
        (_, true) => MOUSEEVENTF_MIDDLEDOWN,
        (_, false) => MOUSEEVENTF_MIDDLEUP,
    };
    mouse(flags, 0)
}
fn device_count() -> usize {
    unsafe {
        let mut count = 0;
        let cb = size_of::<RAWINPUTDEVICELIST>() as u32;
        if GetRawInputDeviceList(None, &mut count, cb) == u32::MAX {
            return 0;
        }
        let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
        if GetRawInputDeviceList(Some(list.as_mut_ptr()), &mut count, cb) == u32::MAX {
            return 0;
        }
        list.iter()
            .take(count as usize)
            .filter(|d| d.dwType == RIM_TYPEKEYBOARD)
            .count()
    }
}
struct Resources {
    hwnd: HWND,
    timer: HANDLE,
    session: bool,
    registered: bool,
    class: bool,
    module: HINSTANCE,
}
impl Drop for Resources {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterHotKey(Some(self.hwnd), 1);
            let _ = UnregisterHotKey(Some(self.hwnd), 2);
            if self.session {
                let _ = WTSUnRegisterSessionNotification(self.hwnd);
            }
            if self.registered {
                let _ = RegisterRawInputDevices(
                    &[RAWINPUTDEVICE {
                        usUsagePage: 1,
                        usUsage: 6,
                        dwFlags: RIDEV_REMOVE,
                        hwndTarget: HWND::default(),
                    }],
                    size_of::<RAWINPUTDEVICE>() as u32,
                );
            }
            if !self.hwnd.is_invalid() {
                let _ = DestroyWindow(self.hwnd);
            }
            if !self.timer.is_invalid() {
                let _ = windows::Win32::Foundation::CloseHandle(self.timer);
            }
            if self.class {
                let _ = UnregisterClassW(w!("KeyMouseInputReceiver"), Some(self.module));
            }
        }
    }
}
struct Runtime {
    input: InputState,
    movement: Movement,
    scroll: ScrollScheduler,
    config: Config,
    paused: bool,
    suspended: bool,
    dispatch: VecDeque<f64>,
    pending_input: Option<Instant>,
}
impl Runtime {
    fn cleanup(&mut self, now: f64) -> Result<(), String> {
        self.input.cancel();
        self.movement.reset(now);
        self.scroll.reset();
        self.sync_buttons()
    }
    fn sync_buttons(&mut self) -> Result<(), String> {
        for (i, down) in self.input.reconcile_buttons() {
            button(i, down)?;
            self.input.commit_button(i, down);
        }
        Ok(())
    }
    fn move_by(&mut self, p: (i32, i32)) -> Result<(), String> {
        if p != (0, 0) {
            cursor_delta(p.0, p.1)?;
            if let Some(t) = self.pending_input.take() {
                if self.dispatch.len() == 512 {
                    self.dispatch.pop_front();
                }
                self.dispatch.push_back(t.elapsed().as_secs_f64() * 1000.);
            }
        }
        Ok(())
    }
    fn scroll(&mut self, now: f64) -> Result<(), String> {
        for wheel in self.scroll.update(&self.input, &self.config.scroll, now) {
            mouse(
                if wheel.horizontal {
                    MOUSEEVENTF_HWHEEL
                } else {
                    MOUSEEVENTF_WHEEL
                },
                wheel.amount,
            )?;
        }
        Ok(())
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.input.cancel();
        for _ in 0..3 {
            if self.sync_buttons().is_ok() {
                break;
            }
        }
    }
}
pub fn run(
    config: Config,
    app: tauri::AppHandle,
    rx: mpsc::Receiver<Command>,
    status: Arc<Mutex<Status>>,
    ready: mpsc::Sender<Result<usize, String>>,
) -> Result<(), String> {
    let result = run_inner(config, app, rx, status, &ready);
    if let Err(e) = &result {
        let _ = ready.send(Err(e.clone()));
    }
    result
}
fn run_inner(
    config: Config,
    app: tauri::AppHandle,
    rx: mpsc::Receiver<Command>,
    status: Arc<Mutex<Status>>,
    ready: &mpsc::Sender<Result<usize, String>>,
) -> Result<(), String> {
    let mut res = Resources {
        hwnd: HWND::default(),
        timer: HANDLE::default(),
        session: false,
        registered: false,
        class: false,
        module: HINSTANCE::default(),
    };
    // SAFETY: all Win32 handles below are confined to this thread and owned by Resources.
    unsafe {
        let module = win(GetModuleHandleW(None))?;
        res.module = module.into();
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: module.into(),
            lpszClassName: w!("KeyMouseInputReceiver"),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(windows::core::Error::from_win32().to_string());
        }
        res.class = true;
        res.hwnd = win(CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("KeyMouseInputReceiver"),
            w!("KeyMouse input"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(module.into()),
            None,
        ))?;
        win(RegisterRawInputDevices(
            &[RAWINPUTDEVICE {
                usUsagePage: 1,
                usUsage: 6,
                dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
                hwndTarget: res.hwnd,
            }],
            size_of::<RAWINPUTDEVICE>() as u32,
        ))?;
        res.registered = true;
        win(WTSRegisterSessionNotification(
            res.hwnd,
            NOTIFY_FOR_THIS_SESSION,
        ))?;
        res.session = true;
        win(RegisterHotKey(
            Some(res.hwnd),
            1,
            MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_NOREPEAT,
            0x4b,
        ))?;
        win(RegisterHotKey(
            Some(res.hwnd),
            2,
            MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
            config.grid.key as u32,
        ))?;
        // High-resolution one-shot timer on supported Windows; ordinary waitable timer otherwise.
        res.timer = CreateWaitableTimerExW(None, None, 0x2, TIMER_ALL_ACCESS.0)
            .or_else(|_| CreateWaitableTimerExW(None, None, 0, TIMER_ALL_ACCESS.0))
            .map_err(|e| e.to_string())?;
    }
    let clock = Instant::now();
    let mut runtime = Runtime {
        input: InputState::default(),
        movement: Movement::default(),
        scroll: ScrollScheduler::default(),
        config,
        paused: false,
        suspended: false,
        dispatch: VecDeque::new(),
        pending_input: None,
    };
    let mut next_tick = Instant::now();
    let mut last_reconcile = Instant::now();
    let mut last_status = Instant::now();
    let mut last_loop = Instant::now();
    let mut devices = BTreeSet::new();
    {
        let mut s = status.lock().unwrap_or_else(|p| p.into_inner());
        s.running = true;
        s.enabled = runtime.config.enabled;
        s.devices = device_count();
        s.mode = "Normal".into();
    }
    ready
        .send(Ok(res.hwnd.0 as usize))
        .map_err(|e| e.to_string())?;
    loop {
        let now = clock.elapsed().as_secs_f64();
        if runtime.input.active() && last_loop.elapsed() > Duration::from_millis(100) {
            runtime.cleanup(now)?;
            status.lock().unwrap_or_else(|p| p.into_inner()).stalls += 1;
        }
        last_loop = Instant::now();
        for command in rx.try_iter() {
            match command {
                Command::Stop => {
                    runtime.cleanup(now)?;
                    return Ok(());
                }
                Command::Pause(p, ack) => {
                    let result = runtime.cleanup(now);
                    if !runtime.suspended {
                        if let Err(e) = result {
                            let _ = ack.send(Err(e.clone()));
                            return Err(e);
                        }
                    }
                    runtime.paused = p;
                    let _ = ack.send(Ok(()));
                }
                Command::Apply(c, ack) => {
                    let mut fatal = None;
                    let result = (|| {
                        c.validate().map_err(|e| e.to_string())?;
                        runtime.cleanup(now)?;
                        if c.grid.key != runtime.config.grid.key {
                            unsafe {
                                win(UnregisterHotKey(Some(res.hwnd), 2))?;
                                if let Err(e) = RegisterHotKey(
                                    Some(res.hwnd),
                                    2,
                                    MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                                    c.grid.key as u32,
                                ) {
                                    if let Err(restore_error) = RegisterHotKey(
                                        Some(res.hwnd),
                                        2,
                                        MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                                        runtime.config.grid.key as u32,
                                    ) {
                                        let error=format!("Grid shortcut could not be restored: {restore_error}. Restart KeyMouse.");
                                        fatal = Some(error.clone());
                                        return Err(error);
                                    }
                                    return Err(e.to_string());
                                }
                            }
                        }
                        runtime.config = *c;
                        status.lock().unwrap_or_else(|p| p.into_inner()).enabled =
                            runtime.config.enabled;
                        Ok(())
                    })();
                    let _ = ack.send(result);
                    if let Some(error) = fatal {
                        return Err(error);
                    }
                }
            }
        }
        // Bound message batches so a busy keyboard cannot starve movement scheduling.
        unsafe {
            let mut msg = MSG::default();
            for _ in 0..256 {
                if !PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                if msg.message == WM_QUIT {
                    runtime.cleanup(now)?;
                    return Ok(());
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        let events = EVENTS.with(|q| std::mem::take(&mut *q.borrow_mut()));
        for event in events {
            let now = clock.elapsed().as_secs_f64();
            match event {
                Event::Key(device, key, down) => {
                    devices.insert(device);
                    if runtime.paused || runtime.suspended {
                        runtime.input.suppress(device, key, down);
                        continue;
                    }
                    if let Some((action, make)) =
                        runtime.input.transition(device, key, down, &runtime.config)
                    {
                        status.lock().unwrap_or_else(|p| p.into_inner()).last_action = Some(
                            format!("{action:?} {}", if make { "pressed" } else { "released" }),
                        );
                        if action.direction() && make {
                            runtime.pending_input = Some(Instant::now());
                        }
                        let p = runtime.movement.input(
                            &runtime.input,
                            &runtime.config,
                            now,
                            if action.direction() && make {
                                Some(action)
                            } else {
                                None
                            },
                        );
                        runtime.move_by(p)?;
                        runtime.sync_buttons()?;
                        runtime.scroll(now)?;
                    }
                }
                Event::Removed(d) => {
                    devices.remove(&d);
                    runtime.cleanup(now)?;
                    runtime.input.remove_device(d);
                }
                Event::Devices => {
                    status.lock().unwrap_or_else(|p| p.into_inner()).devices = device_count();
                }
                Event::Cancel => {
                    runtime.cleanup(now)?;
                    crate::grid::cancel_from_native(&app);
                }
                Event::Session(suspended) => {
                    runtime.suspended = suspended;
                    if let Err(e) = runtime.cleanup(now) {
                        if suspended {
                            status.lock().unwrap_or_else(|p| p.into_inner()).error = Some(format!(
                                "Button release deferred until the session is accessible: {e}"
                            ));
                        } else {
                            return Err(e);
                        }
                    } else if !suspended {
                        status.lock().unwrap_or_else(|p| p.into_inner()).error = None;
                    }
                    crate::grid::cancel_from_native(&app);
                }
                Event::Emergency => {
                    runtime.cleanup(now)?;
                    runtime.config.enabled = false;
                    status.lock().unwrap_or_else(|p| p.into_inner()).enabled = false;
                    crate::grid::cancel_from_native(&app);
                    let handle = app.clone();
                    std::thread::spawn(move || crate::emergency_disabled(&handle));
                }
                Event::Grid => {
                    if runtime.config.enabled && !runtime.paused && !runtime.suspended {
                        runtime.cleanup(now)?;
                        let handle = app.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = crate::grid::open(&handle) {
                                let state = handle.state::<crate::AppState>();
                                state
                                    .engine
                                    .status
                                    .lock()
                                    .unwrap_or_else(|p| p.into_inner())
                                    .error = Some(e);
                            }
                        });
                    }
                }
            }
        }
        let instant = Instant::now();
        if runtime.input.active() && instant >= next_tick {
            let now = clock.elapsed().as_secs_f64();
            match runtime.movement.tick(&runtime.config, now) {
                Ok(p) => runtime.move_by(p)?,
                Err(()) => {
                    runtime.cleanup(now)?;
                    status.lock().unwrap_or_else(|p| p.into_inner()).stalls += 1;
                }
            }
            runtime.scroll(now)?;
            runtime.sync_buttons()?;
            {
                let mut s = status.lock().unwrap_or_else(|p| p.into_inner());
                s.ticks += 1;
                s.max_tick_ms = s
                    .max_tick_ms
                    .max((instant.duration_since(next_tick).as_secs_f64() * 1000.) + 4.);
            }
            next_tick = instant + Duration::from_millis(4);
        } else if !runtime.input.active() {
            runtime.movement.reset(clock.elapsed().as_secs_f64());
            next_tick = instant + Duration::from_millis(4);
        }
        // Poll only selected held/blocked control VKs, never ordinary keyboard activity.
        if !runtime.suspended && instant.duration_since(last_reconcile) >= Duration::from_millis(50)
        {
            for key in runtime.input.keys() {
                if unsafe { GetAsyncKeyState(key as i32) } >= 0 {
                    runtime.input.release_key(key);
                }
            }
            runtime.movement.input(
                &runtime.input,
                &runtime.config,
                clock.elapsed().as_secs_f64(),
                None,
            );
            runtime.sync_buttons()?;
            last_reconcile = instant;
        }
        {
            let mut s = status.lock().unwrap_or_else(|p| p.into_inner());
            s.enabled = runtime.config.enabled;
            s.moving = crate::movement::direction(&runtime.input) != (0, 0);
            s.mode = match runtime.movement.mode {
                Mode::Normal => "Normal",
                Mode::Precision => "Precision",
                Mode::Turbo => "Turbo",
            }
            .into();
        }
        if instant.duration_since(last_status) >= Duration::from_millis(250) {
            let mut s = status.lock().unwrap_or_else(|p| p.into_inner());
            let mut samples: Vec<f64> = runtime.dispatch.iter().copied().collect();
            samples.sort_by(f64::total_cmp);
            s.dispatch_samples = samples.len();
            s.dispatch_p95_ms = if samples.is_empty() {
                None
            } else {
                Some(samples[((samples.len() - 1) as f64 * 0.95).ceil() as usize])
            };
            last_status = instant;
        }
        let active =
            !runtime.suspended && (runtime.input.active() || !runtime.input.keys().is_empty());
        unsafe {
            let waited = if active {
                let remaining = next_tick.saturating_duration_since(Instant::now());
                let due = -((remaining.as_nanos() / 100).max(1) as i64);
                win(SetWaitableTimer(res.timer, &due, 0, None, None, false))?;
                MsgWaitForMultipleObjectsEx(
                    Some(&[res.timer]),
                    INFINITE,
                    QS_ALLINPUT,
                    MWMO_INPUTAVAILABLE,
                )
            } else {
                MsgWaitForMultipleObjectsEx(None, INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE)
            };
            if waited == WAIT_FAILED {
                return Err(windows::core::Error::from_win32().to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::Threading::WaitForSingleObject,
        UI::Input::GetRegisteredRawInputDevices,
    };
    #[test]
    fn native_waitable_timer_and_device_enumeration() {
        // No generated input or cursor movement: exercise the actual scheduling boundary.
        unsafe {
            let timer = CreateWaitableTimerExW(None, None, 0, TIMER_ALL_ACCESS.0).unwrap();
            let set = SetWaitableTimer(timer, &-10_000, 0, None, None, false);
            let waited = WaitForSingleObject(timer, 1000);
            CloseHandle(timer).unwrap();
            set.unwrap();
            assert_eq!(waited, WAIT_OBJECT_0);
            let mut count = 0;
            assert_ne!(
                GetRawInputDeviceList(None, &mut count, size_of::<RAWINPUTDEVICELIST>() as u32),
                u32::MAX
            );
        }
    }
    #[test]
    fn native_raw_receiver_registration_and_cleanup() {
        // This process owns the registration; no actual physical keys are required.
        unsafe {
            let module = GetModuleHandleW(None).unwrap();
            let class = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: module.into(),
                lpszClassName: w!("KeyMouseInputReceiver"),
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&class), 0);
            let mut guard = Resources {
                hwnd: HWND::default(),
                timer: HANDLE::default(),
                session: false,
                registered: false,
                class: true,
                module: module.into(),
            };
            guard.hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("KeyMouseInputReceiver"),
                w!("KeyMouse test receiver"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(module.into()),
                None,
            )
            .unwrap();
            let device = RAWINPUTDEVICE {
                usUsagePage: 1,
                usUsage: 6,
                dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
                hwndTarget: guard.hwnd,
            };
            RegisterRawInputDevices(&[device], size_of::<RAWINPUTDEVICE>() as u32).unwrap();
            guard.registered = true;
            let mut count = 0;
            assert_ne!(
                GetRegisteredRawInputDevices(None, &mut count, size_of::<RAWINPUTDEVICE>() as u32),
                u32::MAX
            );
            let mut list = vec![RAWINPUTDEVICE::default(); count as usize];
            assert_ne!(
                GetRegisteredRawInputDevices(
                    Some(list.as_mut_ptr()),
                    &mut count,
                    size_of::<RAWINPUTDEVICE>() as u32
                ),
                u32::MAX
            );
            assert!(list
                .iter()
                .any(|d| d.hwndTarget == guard.hwnd && d.usUsagePage == 1 && d.usUsage == 6));
            drop(guard);
            count = 0;
            assert_ne!(
                GetRegisteredRawInputDevices(None, &mut count, size_of::<RAWINPUTDEVICE>() as u32),
                u32::MAX
            );
            assert_eq!(count, 0);
        }
    }
}
