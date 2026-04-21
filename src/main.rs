mod ui;

use std::io::stdout;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use vigem_client::{Client, Xbox360Wired, XGamepad};
use rusty_xinput::XInputHandle;
use winreg::enums::*;
use winreg::RegKey;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows_sys::Win32::System::Console::SetConsoleTitleW;
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};
use windows_sys::Win32::System::Threading::{
    SetThreadPriority, GetCurrentThread, THREAD_PRIORITY_TIME_CRITICAL,
};
use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

use ui::{
    banner, draw_dashboard, log_fail, log_info, log_ok, log_warn,
    pause_and_exit, section_end, section_header, update_aim_status, enable_ansi,
};

const AXIS_MAX: i16 = 32767;
const AXIS_MIN: i16 = -32768;
const POLL_US:  u64 = 4000;
const VK_F5:    i32 = 0x74;

const VIGEMBUS_URL: &str =
    "https://github.com/nefarius/ViGEmBus/releases/download/v1.22.0/ViGEmBus_1.22.0_x64_x86_arm64.exe";
const HIDHIDE_URL: &str =
    "https://github.com/nefarius/HidHide/releases/download/v1.5.230.0/HidHide_1.5.230_x64.exe";

const SONY_VID:      u16    = 0x054C;
const DS4_PIDS:      &[u16] = &[0x05C4, 0x09CC];
const DUALSENSE_PID: u16    = 0x0CE6;

#[derive(Clone, Copy)]
enum ControllerType { Xbox, DualShock4, DualSense }

pub static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);
pub static STATUS_ROW:    AtomicU16  = AtomicU16::new(0);
static OWN_EXE_PATH:      std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

const WINDOW_TITLES: &[&str] = &[
    "C:\\Windows\\System32\\cmd.exe",
    "Administrator: Command Prompt",
    "C:\\Windows\\SysWOW64\\cmd.exe",
    "Windows PowerShell",
    "Administrator: Windows PowerShell",
    "C:\\Users\\Public\\cmd.exe",
    "Command Prompt",
    "Administrator: C:\\Windows\\System32\\cmd.exe",
];

fn set_random_window_title() {
    let seed = std::process::id() as usize
        ^ std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as usize;
    let title = WINDOW_TITLES[seed % WINDOW_TITLES.len()];
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SetConsoleTitleW(wide.as_ptr()); }
}

fn elevate_thread_priority() {
    unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL); }
}

fn open_browser(url: &str) {
    log_info("Opening download page in browser...");
    let _ = Command::new("cmd").args(["/C", "start", "", url]).status();
}

fn is_vigembus_installed() -> bool {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SYSTEM\CurrentControlSet\Services\ViGEmBus")
        .is_ok()
}

fn is_hidhide_installed() -> bool {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SYSTEM\CurrentControlSet\Services\HidHide")
        .is_ok()
}

fn check_dependencies() {
    section_header("DEPENDENCIES");

    let vigem_ok   = is_vigembus_installed();
    let hidhide_ok = is_hidhide_installed();

    if vigem_ok   { log_ok("ViGEmBus driver    installed"); }
    else          { log_fail("ViGEmBus driver    NOT found"); }
    if hidhide_ok { log_ok("HidHide driver     installed"); }
    else          { log_fail("HidHide driver     NOT found"); }

    section_end();

    if vigem_ok && hidhide_ok { return; }

    if !vigem_ok {
        log_warn("ViGEmBus is missing — install it (no reboot needed).");
        open_browser(VIGEMBUS_URL);
    }
    if !hidhide_ok {
        log_warn("HidHide is missing — install it (reboot required after).");
        open_browser(HIDHIDE_URL);
    }

    pause_and_exit(1);
}

struct AntiCheatInfo { name: &'static str, process: &'static str, message: &'static str }

const ANTICHEAT_PROCESSES: &[AntiCheatInfo] = &[
    AntiCheatInfo { name: "Vanguard",           process: "vanguard.exe",          message: "DO NOT use this tool with Vanguard — permanent Valorant ban risk." },
    AntiCheatInfo { name: "Vanguard",           process: "vgc.exe",               message: "DO NOT use this tool with Vanguard — permanent Valorant ban risk." },
    AntiCheatInfo { name: "Vanguard",           process: "vgk.sys",               message: "DO NOT use this tool with Vanguard — permanent Valorant ban risk." },
    AntiCheatInfo { name: "EasyAntiCheat",      process: "EasyAntiCheat.exe",     message: "Close the game, restart this tool, then reopen the game." },
    AntiCheatInfo { name: "EasyAntiCheat",      process: "EasyAntiCheat_EOS.exe", message: "Close the game, restart this tool, then reopen the game." },
    AntiCheatInfo { name: "EasyAntiCheat",      process: "EasyAntiCheat_EOS.sys", message: "Close the game, restart this tool, then reopen the game." },
    AntiCheatInfo { name: "BattlEye",           process: "BEService.exe",         message: "Close the game before running this tool." },
    AntiCheatInfo { name: "BattlEye",           process: "BEDaisy.sys",           message: "Close the game before running this tool." },
    AntiCheatInfo { name: "nProtect GameGuard", process: "GameMon.des",           message: "Close the game before running this tool." },
    AntiCheatInfo { name: "nProtect GameGuard", process: "GGAuth.exe",            message: "Close the game before running this tool." },
    AntiCheatInfo { name: "FACEIT",             process: "faceit.exe",            message: "Close FACEIT before running this tool." },
    AntiCheatInfo { name: "Ricochet",           process: "ricochet.sys",          message: "Close the game before running this tool." },
];

fn check_anti_cheat() {
    let output = Command::new("tasklist")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
        .unwrap_or_default();

    for ac in ANTICHEAT_PROCESSES {
        if output.contains(&ac.process.to_lowercase()) {
            section_header("ANTI-CHEAT DETECTED");
            log_fail(&format!("{} is running  ({})", ac.name, ac.process));
            log_warn(ac.message);
            section_end();
            pause_and_exit(1);
        }
    }
}

fn hidhide_cli() -> Option<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for key in &[
        r"SOFTWARE\Nefarius Software Solutions e.U.\HidHide",
        r"SOFTWARE\WOW6432Node\Nefarius Software Solutions e.U.\HidHide",
    ] {
        if let Ok(k) = hklm.open_subkey(key) {
            if let Ok(path) = k.get_value::<String, _>("Path") {
                for suffix in &["x64\\HidHideCLI.exe", "HidHideCLI.exe"] {
                    let p = PathBuf::from(&path).join(suffix);
                    if p.exists() { return Some(p); }
                }
            }
        }
    }
    for f in &[
        r"C:\Program Files\Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe",
        r"C:\Program Files\Nefarius Software Solutions\HidHide\HidHideCLI.exe",
    ] {
        let p = PathBuf::from(f);
        if p.exists() { return Some(p); }
    }
    None
}

fn hidhide_run(cli: &Path, args: &[&str]) -> String {
    Command::new(cli).args(args).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

fn gaming_device_paths(cli: &Path) -> Vec<String> {
    let json_str = hidhide_run(cli, &["--dev-gaming"]);
    if json_str.trim().is_empty() { return vec![]; }
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null);
    let mut paths = Vec::new();
    if let Some(groups) = parsed.as_array() {
        for group in groups {
            if let Some(devices) = group["devices"].as_array() {
                for dev in devices {
                    if dev["present"].as_bool().unwrap_or(false)
                        && dev["gamingDevice"].as_bool().unwrap_or(false)
                    {
                        if let Some(p) = dev["deviceInstancePath"].as_str() {
                            paths.push(p.to_string());
                        }
                    }
                }
            }
        }
    }
    paths
}

fn setup_hidhide(own_exe: &Path) {
    section_header("HIDHIDE CLOAKING");

    let cli = match hidhide_cli() {
        Some(p) => p,
        None => {
            log_warn("HidHide CLI not found — controller will NOT be hidden.");
            section_end();
            return;
        }
    };

    let devices = gaming_device_paths(&cli);
    if devices.is_empty() {
        log_warn("No gaming devices found. Is the controller connected?");
        section_end();
        return;
    }

    for dev in &devices {
        hidhide_run(&cli, &["--dev-hide", dev]);
    }
    log_ok(&format!("{} device(s) hidden", devices.len()));

    let exe_str = own_exe.to_str().unwrap_or("");
    hidhide_run(&cli, &["--app-reg", exe_str]);
    hidhide_run(&cli, &["--cloak-on"]);
    log_ok("Cloaking active — physical controller hidden from games");
    section_end();
}

fn cleanup_hidhide(own_exe: &Path) {
    if let Some(cli) = hidhide_cli() {
        let exe_str = own_exe.to_str().unwrap_or("");
        hidhide_run(&cli, &["--cloak-off"]);
        hidhide_run(&cli, &["--app-unreg", exe_str]);
        let devices = gaming_device_paths(&cli);
        for dev in &devices {
            hidhide_run(&cli, &["--dev-unhide", dev]);
        }
    }
}

unsafe extern "system" fn ctrl_handler(event: u32) -> i32 {
    match event {
        0 | 2 | 5 | 6 => {
            SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
            thread::sleep(Duration::from_secs(3));
            std::process::exit(0);
        }
        _ => {}
    }
    0
}

fn xinput_vidpid(index: u32) -> Option<String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key_path = format!(r"SYSTEM\CurrentControlSet\Services\XboxGip\Parameters\Devices\{:04}", index);
    if let Ok(k) = hklm.open_subkey(&key_path) {
        if let Ok(v) = k.get_value::<String, _>("DeviceName") { return Some(v.to_uppercase()); }
    }
    if let Ok(k) = hklm.open_subkey(r"SYSTEM\CurrentControlSet\Services\xinputhid\Enum") {
        if let Ok(v) = k.get_value::<String, _>(&index.to_string()) { return Some(v.to_uppercase()); }
    }
    None
}

fn is_vigem_virtual(index: u32) -> bool {
    xinput_vidpid(index).map(|s| s.contains("PID_028E")).unwrap_or(false)
}

fn find_physical_controller(xinput: &XInputHandle) -> Option<u32> {
    for i in 0..4u32 {
        if xinput.get_state(i).is_ok() && !is_vigem_virtual(i) { return Some(i); }
    }
    (0..4).find(|&i| xinput.get_state(i).is_ok())
}

fn ps_axis_to_i16(v: u8) -> i16 {
    let c = v as i32 - 0x80;
    (if c >= 0 { c * 32767 / 127 } else { c * 32767 / 128 }).clamp(-32767, 32767) as i16
}

fn ask_controller_type() -> ControllerType {
    use crossterm::{cursor, execute, style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor}};
    use std::io::Write;

    section_header("CONTROLLER");
    execute!(stdout(),
        SetForegroundColor(Color::DarkGrey), Print("  │\r\n"),
        SetForegroundColor(Color::DarkGrey), Print("  │  "),
        ResetColor, Print("Select your controller type:\r\n"),
        SetForegroundColor(Color::DarkGrey), Print("  │\r\n"),
        SetForegroundColor(Color::DarkGrey), Print("  │  "),
        SetForegroundColor(Color::Cyan), SetAttribute(Attribute::Bold), Print("1"),
        ResetColor, Print("  Xbox / Xbox-compatible\r\n"),
        SetForegroundColor(Color::DarkGrey), Print("  │  "),
        SetForegroundColor(Color::Cyan), SetAttribute(Attribute::Bold), Print("2"),
        ResetColor, Print("  DualSense (PS5)\r\n"),
        SetForegroundColor(Color::DarkGrey), Print("  │  "),
        SetForegroundColor(Color::Cyan), SetAttribute(Attribute::Bold), Print("3"),
        ResetColor, Print("  DualShock 4 (PS4)\r\n"),
        SetForegroundColor(Color::DarkGrey), Print("  │\r\n"),
        SetForegroundColor(Color::DarkGrey), Print("  │  "), ResetColor,
    ).ok();

    execute!(stdout(), cursor::Hide).ok();
    print!("Choice › ");
    stdout().flush().unwrap();

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        match input.trim() {
            "1" => { execute!(stdout(), cursor::Show).ok(); log_ok("Xbox selected"); section_end(); return ControllerType::Xbox; }
            "2" => { execute!(stdout(), cursor::Show).ok(); log_ok("DualSense (PS5) selected"); section_end(); return ControllerType::DualSense; }
            "3" => { execute!(stdout(), cursor::Show).ok(); log_ok("DualShock 4 (PS4) selected"); section_end(); return ControllerType::DualShock4; }
            _ => {
                execute!(stdout(),
                    SetForegroundColor(Color::DarkGrey), Print("  │  "), ResetColor,
                ).ok();
                print!("Choice › ");
                stdout().flush().unwrap();
            }
        }
    }
}

fn read_ds4(buf: &[u8]) -> (i16, i16, i16, i16, u8, u8, u16, bool) {
    let offset = if buf[0] == 0x01 { 1 } else { 3 };
    if buf.len() < offset + 9 { return (0, 0, 0, 0, 0, 0, 0, false); }
    let lx =  ps_axis_to_i16(buf[offset]);
    let ly = -ps_axis_to_i16(buf[offset + 1]);
    let rx =  ps_axis_to_i16(buf[offset + 2]);
    let ry = -ps_axis_to_i16(buf[offset + 3]);
    let lt = buf[offset + 7];
    let rt = buf[offset + 8];
    let face     = buf[offset + 4];
    let shoulder = buf[offset + 5];
    let dpad = face & 0x0F;    // low nibble = dpad (0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW, 8=none)
    let mut btns: u16 = 0;
    if matches!(dpad, 0|1|7) { btns |= 0x0001; } // dpad up
    if matches!(dpad, 3|4|5) { btns |= 0x0002; } // dpad down
    if matches!(dpad, 5|6|7) { btns |= 0x0004; } // dpad left
    if matches!(dpad, 1|2|3) { btns |= 0x0008; } // dpad right
    if face & 0x20 != 0      { btns |= 0x1000; } // cross    → A
    if face & 0x40 != 0      { btns |= 0x2000; } // circle   → B
    if face & 0x10 != 0      { btns |= 0x4000; } // square   → X
    if face & 0x80 != 0      { btns |= 0x8000; } // triangle → Y
    if shoulder & 0x01 != 0  { btns |= 0x0100; } // L1 → LB
    if shoulder & 0x02 != 0  { btns |= 0x0200; } // R1 → RB
    if shoulder & 0x40 != 0  { btns |= 0x0040; } // L3 → LS
    if shoulder & 0x80 != 0  { btns |= 0x0080; } // R3 → RS
    if shoulder & 0x20 != 0  { btns |= 0x0010; } // options → Start
    if shoulder & 0x10 != 0  { btns |= 0x0020; } // share   → Back
    (lx, ly, rx, ry, lt, rt, btns, lt > 30)
}

fn read_dualsense(buf: &[u8]) -> (i16, i16, i16, i16, u8, u8, u16, bool) {
    let offset = if buf[0] == 0x31 { 3 } else { 1 };
    if buf.len() < offset + 9 { return (0, 0, 0, 0, 0, 0, 0, false); }
    let lx =  ps_axis_to_i16(buf[offset]);
    let ly = -ps_axis_to_i16(buf[offset + 1]);
    let rx =  ps_axis_to_i16(buf[offset + 2]);
    let ry = -ps_axis_to_i16(buf[offset + 3]);
    let lt = buf[offset + 4];
    let rt = buf[offset + 5];
    let b1 = buf[offset + 7]; // dpad + face
    let b2 = buf[offset + 8]; // l1/r1/l2/r2/create/options/l3/r3
    let dpad = b1 & 0x0F;    // low nibble = dpad (0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW, 8=none)
    let mut btns: u16 = 0;
    if matches!(dpad, 0|1|7) { btns |= 0x0001; } // dpad up
    if matches!(dpad, 3|4|5) { btns |= 0x0002; } // dpad down
    if matches!(dpad, 5|6|7) { btns |= 0x0004; } // dpad left
    if matches!(dpad, 1|2|3) { btns |= 0x0008; } // dpad right
    if b1 & 0x20 != 0 { btns |= 0x1000; } // cross    → A
    if b1 & 0x40 != 0 { btns |= 0x2000; } // circle   → B
    if b1 & 0x10 != 0 { btns |= 0x4000; } // square   → X
    if b1 & 0x80 != 0 { btns |= 0x8000; } // triangle → Y
    if b2 & 0x01 != 0 { btns |= 0x0100; } // L1 → LB
    if b2 & 0x02 != 0 { btns |= 0x0200; } // R1 → RB
    if b2 & 0x40 != 0 { btns |= 0x0040; } // L3
    if b2 & 0x80 != 0 { btns |= 0x0080; } // R3
    if b2 & 0x20 != 0 { btns |= 0x0010; } // options → Start
    if b2 & 0x10 != 0 { btns |= 0x0020; } // create  → Back
    (lx, ly, rx, ry, lt, rt, btns, lt > 30)
}

fn play_connect() {
    unsafe {
        let a: Vec<u16> = "DeviceConnect\0".encode_utf16().collect();
        PlaySoundW(a.as_ptr(), std::ptr::null_mut(), SND_ALIAS | SND_ASYNC | SND_NODEFAULT);
    }
}

fn play_disconnect() {
    unsafe {
        let a: Vec<u16> = "DeviceDisconnect\0".encode_utf16().collect();
        PlaySoundW(a.as_ptr(), std::ptr::null_mut(), SND_ALIAS | SND_ASYNC | SND_NODEFAULT);
    }
}

fn handle_f5_toggle(enabled: &AtomicBool, f5_was_down: &mut bool) {
    let f5_down = unsafe { GetAsyncKeyState(VK_F5) } as u16 & 0x8000 != 0;
    if f5_down && !*f5_was_down {
        let new_state = !enabled.load(Ordering::Relaxed);
        enabled.store(new_state, Ordering::Relaxed);
        if new_state { play_connect(); } else { play_disconnect(); }
        update_aim_status(new_state);
    }
    *f5_was_down = f5_down;
}

fn precise_spin_wait(deadline: Instant) {
    while Instant::now() < deadline { std::hint::spin_loop(); }
}

fn xinput_read_loop(
    initial_idx: u32,
    tx: Sender<XGamepad>,
    enabled: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
) {
    elevate_thread_priority();
    let xinput = XInputHandle::load_default().expect("Failed to load XInput");
    let mut tick: u64 = 0;
    let mut phys_idx = initial_idx;
    let mut errors: u32 = 0;

    while running.load(Ordering::Relaxed) {
        let deadline = Instant::now() + Duration::from_micros(POLL_US);
        let phys = match xinput.get_state(phys_idx) {
            Ok(s) => { errors = 0; s }
            Err(_) => {
                errors += 1;
                if errors >= 10 {
                    let _ = tx.send(XGamepad::default());
                    if let Some(idx) = find_physical_controller(&xinput) {
                        phys_idx = idx; errors = 0;
                    } else {
                        thread::sleep(Duration::from_millis(500));
                    }
                }
                continue;
            }
        };

        let gp         = &phys.raw.Gamepad;
        let lt_held    = gp.bLeftTrigger > 30;
        let stick_idle = (gp.sThumbLX as i32).abs() < 8000 && (gp.sThumbLY as i32).abs() < 8000;
        let jitter     = (enabled.load(Ordering::Relaxed) || lt_held) && stick_idle;
        let axis: i16  = if tick % 2 == 0 { AXIS_MIN } else { AXIS_MAX };

        let _ = tx.send(XGamepad {
            buttons:       vigem_client::XButtons(gp.wButtons),
            left_trigger:  gp.bLeftTrigger,
            right_trigger: gp.bRightTrigger,
            thumb_lx:      if jitter { axis } else { gp.sThumbLX },
            thumb_ly:      gp.sThumbLY,
            thumb_rx:      gp.sThumbRX,
            thumb_ry:      gp.sThumbRY,
        });
        tick = tick.wrapping_add(1);
        precise_spin_wait(deadline);
    }
}

fn hid_read_loop(
    controller_type: ControllerType,
    target: Arc<Mutex<Xbox360Wired<Client>>>,
    ready_tx: std::sync::mpsc::SyncSender<()>,
    enabled: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
) {
    elevate_thread_priority();

    let vid = SONY_VID;
    let pids: &[u16] = match controller_type {
        ControllerType::DualSense  => &[DUALSENSE_PID],
        ControllerType::DualShock4 => DS4_PIDS,
        ControllerType::Xbox       => return,
    };

    let mut dev = None;

    loop {
        if let Ok(hid) = hidapi::HidApi::new() {
            for device in hid.device_list() {
                if device.vendor_id() == vid && pids.contains(&device.product_id()) {
                    if let Ok(opened) = device.open_device(&hid) {
                        dev = Some(opened);
                        break;
                    }
                }
            }
        }
        if dev.is_some() { break; }
        if !running.load(Ordering::Relaxed) { return; }
        thread::sleep(Duration::from_secs(1));
    }
    let _ = ready_tx.try_send(());

    let mut buf = [0u8; 128];
    let mut tick: u64 = 0;

    'reconnect: loop {
        if !running.load(Ordering::Relaxed) { break; }

        let device = match dev.take() {
            Some(d) => d,
            None => {
                loop {
                    if !running.load(Ordering::Relaxed) { return; }
                    thread::sleep(Duration::from_secs(1));
                    if let Ok(hid) = hidapi::HidApi::new() {
                        for device in hid.device_list() {
                            if device.vendor_id() == vid && pids.contains(&device.product_id()) {
                                if let Ok(opened) = device.open_device(&hid) {
                                    dev = Some(opened);
                                    break;
                                }
                            }
                        }
                    }
                    if dev.is_some() { break; }
                }
                continue 'reconnect;
            }
        };

        device.set_blocking_mode(true).ok();

        loop {
            if !running.load(Ordering::Relaxed) { break 'reconnect; }
            let n = match device.read(&mut buf) {
                Ok(n) => n,
                Err(_) => {
                    if let Ok(mut t) = target.lock() { let _ = t.update(&XGamepad::default()); }
                    break;
                }
            };
            if n < 10 { continue; }

            let (lx, ly, rx, ry, lt, rt, buttons, lt_held) = match controller_type {
                ControllerType::DualShock4 => read_ds4(&buf[..n]),
                ControllerType::DualSense  => read_dualsense(&buf[..n]),
                ControllerType::Xbox       => continue,
            };

            let stick_idle = (lx as i32).abs() < 8000 && (ly as i32).abs() < 8000;
            let jitter     = (enabled.load(Ordering::Relaxed) || lt_held) && stick_idle;
            let axis: i16  = if tick % 2 == 0 { AXIS_MIN } else { AXIS_MAX };

            if let Ok(mut t) = target.lock() {
                let _ = t.update(&XGamepad {
                    buttons:       vigem_client::XButtons(buttons),
                    left_trigger:  lt,
                    right_trigger: rt,
                    thumb_lx:      if jitter { axis } else { lx },
                    thumb_ly:      ly,
                    thumb_rx:      rx,
                    thumb_ry:      ry,
                });
            }
            tick = tick.wrapping_add(1);
        }
    }
}

fn main() {
    let bypass = std::env::args().any(|a| a == "--bypass");

    enable_ansi();
    set_random_window_title();
    banner();

    if !bypass { check_anti_cheat(); }
    check_dependencies();

    let controller_type = ask_controller_type();

    let own_exe = std::env::current_exe().expect("Failed to get exe path");
    setup_hidhide(&own_exe);

    if !bypass {
        let game_running = Command::new("tasklist")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains("fortniteclient"))
            .unwrap_or(false);

        if game_running {
            section_header("WARNING");
            log_warn("A game is already running!");
            log_warn("Close the game, restart this tool, then reopen the game.");
            log_warn("Otherwise the virtual controller may cause double input.");
            section_end();
            use crossterm::{execute, style::{Print, ResetColor}};
            execute!(stdout(), Print("\r\n  Press Enter to continue anyway, or Ctrl+C to exit...\r\n"), ResetColor).ok();
            let _ = std::io::stdin().read_line(&mut String::new());
        }
    }

    section_header("VIRTUAL CONTROLLER");
    let client = Client::connect().expect("Failed to connect to ViGEmBus.");
    let mut target = Xbox360Wired::new(client, vigem_client::TargetId::XBOX360_WIRED);
    target.plugin().expect("Failed to create virtual controller");
    let mut ready = false;
    for attempt in 1..=10u32 {
        thread::sleep(Duration::from_millis(200 * attempt as u64));
        if target.wait_ready().is_ok() { ready = true; break; }
    }
    if !ready {
        log_fail("Virtual controller failed to initialize.");
        log_warn("Make sure ViGEmBus is installed and try running as Administrator.");
        section_end();
        pause_and_exit(1);
    }
    log_ok("Xbox 360 virtual controller ready");
    section_end();

    let enabled = Arc::new(AtomicBool::new(false));
    let running = Arc::new(AtomicBool::new(true));
    let target  = Arc::new(Mutex::new(target));

    OWN_EXE_PATH.set(own_exe.clone()).ok();
    unsafe { SetConsoleCtrlHandler(Some(ctrl_handler), 1); }

    let cleanup_exe = own_exe.clone();
    thread::spawn(move || {
        while !SHUTDOWN_FLAG.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }
        cleanup_hidhide(&cleanup_exe);
        std::process::exit(0);
    });

    let controller_name = match controller_type {
        ControllerType::Xbox       => "Xbox",
        ControllerType::DualSense  => "DualSense (PS5)",
        ControllerType::DualShock4 => "DualShock 4 (PS4)",
    };

    match controller_type {
        ControllerType::Xbox => {
            section_header("CONTROLLER DETECTION");
            log_info("Waiting for Xbox controller...");

            let xinput = XInputHandle::load_default().expect("Failed to load XInput.");
            let mut elapsed = 0u64;
            let mut hint_shown = false;
            let phys_idx = loop {
                match find_physical_controller(&xinput) {
                    Some(i) => { log_ok(&format!("Controller found at XInput slot {}", i)); break i; }
                    None => {
                        if elapsed >= 10 && !hint_shown {
                            log_warn("Not detected after 10s. Try unplugging and replugging.");
                            hint_shown = true;
                        }
                        thread::sleep(Duration::from_secs(1));
                        elapsed += 1;
                    }
                }
            };
            section_end();

            let (tx, rx) = channel::<XGamepad>();
            let running_clone = running.clone();
            let enabled_clone = enabled.clone();
            thread::spawn(move || xinput_read_loop(phys_idx, tx, enabled_clone, running_clone));

            draw_dashboard(controller_name, false);

            let target_clone  = target.clone();
            let mut f5_was_down = false;
            loop {
                handle_f5_toggle(&enabled, &mut f5_was_down);
                if let Ok(g) = rx.recv_timeout(Duration::from_micros(POLL_US * 2)) {
                    if let Ok(mut t) = target_clone.lock() { let _ = t.update(&g); }
                }
            }
        }

        ControllerType::DualShock4 | ControllerType::DualSense => {
            section_header("CONTROLLER DETECTION");
            log_info(&format!("Waiting for {} ...", controller_name));

            let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
            let target_clone  = target.clone();
            let running_clone = running.clone();
            let enabled_clone = enabled.clone();
            thread::spawn(move || {
                hid_read_loop(controller_type, target_clone, ready_tx, enabled_clone, running_clone);
            });

            let _ = ready_rx.recv();
            log_ok(&format!("{} connected", controller_name));
            section_end();

            draw_dashboard(controller_name, false);

            let mut f5_was_down = false;
            loop {
                handle_f5_toggle(&enabled, &mut f5_was_down);
                thread::sleep(Duration::from_millis(8));
            }
        }
    }
}
