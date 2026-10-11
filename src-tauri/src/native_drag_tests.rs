//! Desktop-only regression: inject clicks only into a disposable window owned by this test.
use super::*;
use crate::config::Action;
use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};

unsafe extern "system" fn target_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

fn pump_for(duration: Duration) {
    let until = Instant::now() + duration;
    while Instant::now() < until {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

struct Target {
    hwnd: HWND,
    original: (i32, i32),
    foreground: HWND,
    module: HINSTANCE,
}
impl Drop for Target {
    fn drop(&mut self) {
        // The test owns the only injected hold; always release it even after assertion failure.
        let _ = button(0, false);
        unsafe {
            let _ = DestroyWindow(self.hwnd);
            let _ = UnregisterClassW(w!("KeyMouseDragRegression"), Some(self.module));
            let _ = SetForegroundWindow(self.foreground);
        }
        let _ = cursor_at(self.original.0, self.original.1);
    }
}

fn move_while_held(mode: Mode) {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let mut c = Config::default();
    c.normal.min_speed = 300.;
    c.normal.max_speed = 300.;
    c.turbo.min_speed = 300.;
    c.turbo.max_speed = 300.;
    c.precision.curve.min_speed = 300.;
    c.precision.curve.max_speed = 300.;
    let mut s = InputState::default();
    let mut m = Movement::default();
    let clock = Instant::now();
    s.transition(1, c.bindings[&Action::LeftButton], true, &c);
    for (i, down) in s.reconcile_buttons() {
        button(i, down).unwrap();
        s.commit_button(i, down);
    }
    // Also cover holding the button before beginning movement.
    std::thread::sleep(Duration::from_millis(180));
    if let Some(action) = match mode {
        Mode::Normal => None,
        Mode::Precision => Some(Action::Precision),
        Mode::Turbo => Some(Action::Turbo),
    } {
        s.transition(1, c.bindings[&action], true, &c);
        m.input(&s, &c, clock.elapsed().as_secs_f64(), None);
    }
    s.transition(1, c.bindings[&Action::Right], true, &c);
    let p = m.input(&s, &c, clock.elapsed().as_secs_f64(), Some(Action::Right));
    cursor_delta(p.0, p.1).unwrap();
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(4));
        let p = m.tick(&c, clock.elapsed().as_secs_f64()).unwrap();
        cursor_delta(p.0, p.1).unwrap();
        assert!(s.owned[0]);
    }
    s.transition(1, c.bindings[&Action::LeftButton], false, &c);
    for (i, down) in s.reconcile_buttons() {
        button(i, down).unwrap();
        s.commit_button(i, down);
    }
}

#[test]
#[ignore = "Requires an interactive Windows desktop; temporarily activates an owned test window"]
fn native_drag_selects_text_and_moves_window_in_all_modes() {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(
            GetAsyncKeyState(1) >= 0,
            "Release the physical left mouse button before testing"
        );
        let module = GetModuleHandleW(None).unwrap();
        let original = cursor_position().unwrap();
        let foreground = GetForegroundWindow();
        let class = WNDCLASSW {
            lpfnWndProc: Some(target_proc),
            hInstance: module.into(),
            lpszClassName: w!("KeyMouseDragRegression"),
            ..Default::default()
        };
        assert_ne!(RegisterClassW(&class), 0);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("KeyMouseDragRegression"),
            w!("KeyMouse disposable drag regression"),
            WS_OVERLAPPEDWINDOW,
            120,
            120,
            600,
            260,
            None,
            None,
            Some(module.into()),
            None,
        )
        .unwrap();
        let target = Target {
            hwnd,
            original,
            foreground,
            module: module.into(),
        };
        let edit = CreateWindowExW(
            WINDOW_EX_STYLE::default(), w!("EDIT"),
            w!("KeyMouse drag regression selects this disposable text in all three movement modes."),
            WS_CHILD | WS_VISIBLE | WS_BORDER, 20, 30, 530, 40,
            Some(hwnd), None, Some(module.into()), None,
        ).unwrap();
        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
        pump_for(Duration::from_millis(150));
        if GetForegroundWindow() != hwnd {
            // Foreground lock can reject a newly launched test process. Activate by
            // clicking only the verified, topmost client area of our own window.
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
            .unwrap();
            let mut bounds = windows::Win32::Foundation::RECT::default();
            GetWindowRect(hwnd, &mut bounds).unwrap();
            let point = POINT {
                x: bounds.left + 30,
                y: bounds.top + 200,
            };
            cursor_at(point.x, point.y).unwrap();
            assert_eq!(GetAncestor(WindowFromPoint(point), GA_ROOT), hwnd);
            button(0, true).unwrap();
            pump_for(Duration::from_millis(30));
            button(0, false).unwrap();
            pump_for(Duration::from_millis(100));
        }
        assert_eq!(
            GetForegroundWindow(),
            hwnd,
            "Test window must own the foreground before injecting a click"
        );
        for mode in [Mode::Normal, Mode::Turbo, Mode::Precision] {
            for _ in 0..3 {
                assert_eq!(
                    GetForegroundWindow(),
                    hwnd,
                    "Another window took focus during the native drag test"
                );
                let mut rect = windows::Win32::Foundation::RECT::default();
                GetWindowRect(edit, &mut rect).unwrap();
                cursor_at(rect.left + 8, rect.top + 12).unwrap();
                SendMessageW(edit, 0x00b1, Some(WPARAM(0)), Some(LPARAM(0))); // EM_SETSEL
                let worker = std::thread::spawn(move || move_while_held(mode));
                while !worker.is_finished() {
                    pump_for(Duration::from_millis(5));
                }
                worker.join().unwrap();
                pump_for(Duration::from_millis(50));
                let mut start = 0u32;
                let mut end = 0u32;
                SendMessageW(
                    edit,
                    0x00b0,
                    Some(WPARAM((&mut start as *mut u32) as usize)),
                    Some(LPARAM((&mut end as *mut u32) as isize)),
                ); // EM_GETSEL
                println!("{mode:?}: selected {} characters", end.abs_diff(start));
                assert!(
                    end.abs_diff(start) >= 3,
                    "{mode:?}: held button did not select text"
                );
            }
            let mut before = windows::Win32::Foundation::RECT::default();
            GetWindowRect(hwnd, &mut before).unwrap();
            cursor_at(before.left + 180, before.top + 12).unwrap();
            let worker = std::thread::spawn(move || move_while_held(mode));
            while !worker.is_finished() {
                pump_for(Duration::from_millis(5));
            }
            worker.join().unwrap();
            pump_for(Duration::from_millis(50));
            let mut after = windows::Win32::Foundation::RECT::default();
            GetWindowRect(hwnd, &mut after).unwrap();
            println!("{mode:?}: window moved {} pixels", after.left - before.left);
            assert!(
                after.left - before.left >= 20,
                "{mode:?}: title-bar drag did not move window"
            );
        }
        drop(target);
    }
}
