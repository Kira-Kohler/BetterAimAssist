use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::thread;
use vigem_client::{Client, Xbox360Wired, XGamepad};
use rusty_xinput::XInputHandle;
use winreg::enums::*;
use winreg::RegKey;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows_sys::Win32::System::Console::{
    SetConsoleTitleW, GetStdHandle, SetConsoleMode, GetConsoleMode,
    STD_OUTPUT_HANDLE, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
};
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};

const AXIS_MAX: i16 = 32767;
const AXIS_MIN: i16 = -32768;
const POLL_MS: u64 = 16;
const VK_F5: i32 = 0x74;

const VIGEMBUS_URL: &str =
    "https://github.com/nefarius/ViGEmBus/releases/download/v1.22.0/ViGEmBus_1.22.0_x64_x86_arm64.exe";
const HIDHIDE_URL: &str =
    "https://github.com/nefarius/HidHide/releases/download/v1.5.230.0/HidHide_1.5.230_x64.exe";

const W: usize = 60;

const RESET:  &str = "\x1b[0m";
const GREEN:  &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED:    &str = "\x1b[31m";
const CYAN:   &str = "\x1b[36m";
const BOLD:   &str = "\x1b[1m";
const DIM:    &str = "\x1b[2m";

const SONY_VID: u16 = 0x054C;
const DS4_PIDS: &[u16] = &[0x05C4, 0x09CC];
const DUALSENSE_PID: u16 = 0x0CE6;

#[derive(Clone, Copy)]
enum ControllerType { Xbox, DualShock4, DualSense }

struct PhysicalState {
    lx: i16,
    ly: i16,
    rx: i16,
    ry: i16,
    lt: u8,
    rt: u8,
    buttons: u16,
    lt_held: bool,
}

fn enable_ansi() {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0u32;
        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

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
    let idx = seed % WINDOW_TITLES.len();
    let title = WINDOW_TITLES[idx];
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SetConsoleTitleW(wide.as_ptr()); }
}

fn banner() {
    let title = "BETTER AIM-ASSIST by Kira Kohler";
    let pad = (W - 2 - title.len()) / 2;
    println!("{}{}", CYAN, BOLD);
    println!("╔{}╗", "═".repeat(W - 2));
    println!("║{}{}{} ║", " ".repeat(pad), title, " ".repeat(W - 3 - pad - title.len()));
    println!("╚{}╝{}", "═".repeat(W - 2), RESET);
    println!();
}

fn section(label: &str) {
    println!("\n{}{DIM}┌─{RESET} {BOLD}{}{RESET} {DIM}{}{RESET}",
        "", label, "─".repeat(W.saturating_sub(4 + label.len())),
        DIM=DIM, BOLD=BOLD, RESET=RESET);
}

fn ok(msg: &str)   { println!("{}│{}  {}[✓]{} {}", DIM, RESET, GREEN,  RESET, msg); }
fn info(msg: &str) { println!("{}│{}  {}[·]{} {}", DIM, RESET, CYAN,   RESET, msg); }
fn warn(msg: &str) { println!("{}│{}  {}[!]{} {}", DIM, RESET, YELLOW, RESET, msg); }
fn fail(msg: &str) { println!("{}│{}  {}[✗]{} {}", DIM, RESET, RED,    RESET, msg); }
fn sep()           { println!("{DIM}└{}{RESET}", "─".repeat(W - 1), DIM=DIM, RESET=RESET); }

fn open_browser(url: &str) {
    print!("{}│{}  Opening browser in", DIM, RESET);
    std::io::stdout().flush().unwrap();
    for i in (1..=3).rev() {
        print!(" {}...", i);
        std::io::stdout().flush().unwrap();
        thread::sleep(Duration::from_secs(1));
    }
    println!();
    let _ = Command::new("cmd").args(["/C", "start", "", url]).status();
}

fn pause_and_exit(code: i32) {
    println!("\nPress Enter to exit...");
    let _ = std::io::stdin().read_line(&mut String::new());
    std::process::exit(code);
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
    section("DEPENDENCIES");

    let vigem_ok = is_vigembus_installed();
    let hidhide_ok = is_hidhide_installed();

    if vigem_ok  { ok("ViGEmBus driver   — installed"); } else { fail("ViGEmBus driver   — NOT found"); }
    if hidhide_ok { ok("HidHide driver    — installed"); } else { fail("HidHide driver    — NOT found"); }

    sep();

    if vigem_ok && hidhide_ok { return; }

    println!();
    if !vigem_ok {
        warn("ViGEmBus is missing — download and install it (no reboot needed).");
        info(&format!("URL: {}", VIGEMBUS_URL));
        open_browser(VIGEMBUS_URL);
        println!("{}│{}", DIM, RESET);
    }
    if !hidhide_ok {
        warn("HidHide is missing — download and install it (reboot required after).");
        info(&format!("URL: {}", HIDHIDE_URL));
        open_browser(HIDHIDE_URL);
        println!("{}│{}", DIM, RESET);
    }

    if !vigem_ok && !hidhide_ok {
        warn("Install both drivers, reboot, then re-run this program.");
    } else if !hidhide_ok {
        warn("Install HidHide, reboot, then re-run this program.");
    } else {
        warn("Install ViGEmBus, then re-run this program (no reboot needed).");
    }

    sep();
    println!();
    pause_and_exit(1);
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
    let out = Command::new(cli).args(args).output()
        .unwrap_or_else(|e| panic!("Failed to run HidHideCLI: {}", e));
    String::from_utf8_lossy(&out.stdout).to_string()
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
    section("HIDHIDE CLOAKING");

    let cli = match hidhide_cli() {
        Some(p) => { info(&format!("CLI found at: {}", p.display())); p }
        None => {
            warn("HidHide CLI not found — physical controller will NOT be hidden.");
            warn("Re-run after rebooting if HidHide was just installed.");
            sep();
            return;
        }
    };

    info("Scanning for gaming devices...");
    let devices = gaming_device_paths(&cli);
    if devices.is_empty() {
        warn("No gaming devices found. Is the controller connected?");
        sep();
        return;
    }

    for dev in &devices {
        info(&format!("Hiding: {}", dev));
        hidhide_run(&cli, &["--dev-hide", dev]);
    }

    let exe_str = own_exe.to_str().unwrap_or("");
    info(&format!("Whitelisting: {}", exe_str));
    hidhide_run(&cli, &["--app-reg", exe_str]);

    hidhide_run(&cli, &["--cloak-on"]);
    ok("Cloaking active — physical controller hidden from games.");
    sep();
}

fn xinput_vidpid(index: u32) -> Option<String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key_path = format!(r"SYSTEM\CurrentControlSet\Services\XboxGip\Parameters\Devices\{:04}", index);
    if let Ok(k) = hklm.open_subkey(&key_path) {
        if let Ok(v) = k.get_value::<String, _>("DeviceName") {
            return Some(v.to_uppercase());
        }
    }
    if let Ok(k) = hklm.open_subkey(r"SYSTEM\CurrentControlSet\Services\xinputhid\Enum") {
        if let Ok(v) = k.get_value::<String, _>(&index.to_string()) {
            return Some(v.to_uppercase());
        }
    }
    None
}

fn is_vigem_virtual(index: u32) -> bool {
    xinput_vidpid(index)
        .map(|s| s.contains("PID_028E"))
        .unwrap_or(false)
}

fn find_physical_controller(xinput: &XInputHandle) -> Option<u32> {
    for i in 0..4u32 {
        if xinput.get_state(i).is_ok() && !is_vigem_virtual(i) {
            return Some(i);
        }
    }
    (0..4).find(|&i| xinput.get_state(i).is_ok())
}

fn ps_axis_to_i16(v: u8) -> i16 {
    let centered = v as i32 - 0x80;
    let scaled = if centered >= 0 {
        (centered * 32767 / 127).min(32767)
    } else {
        (centered * 32767 / 128).max(-32767)
    };
    scaled as i16
}

fn ps_trigger_to_u8(v: u8) -> u8 { v }

fn ask_controller_type() -> ControllerType {
    section("CONTROLLER TYPE");
    println!("{}│{}  Select your controller type:", DIM, RESET);
    println!("{}│{}", DIM, RESET);
    println!("{}│{}  {}[1]{} Xbox / Xbox-compatible", DIM, RESET, CYAN, RESET);
    println!("{}│{}  {}[2]{} DualSense (PS5)", DIM, RESET, CYAN, RESET);
    println!("{}│{}  {}[3]{} DualShock 4 (PS4)", DIM, RESET, CYAN, RESET);
    println!("{}│{}", DIM, RESET);
    print!("{}│{}  Enter choice (1/2/3): ", DIM, RESET);
    std::io::stdout().flush().unwrap();

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        match input.trim() {
            "1" => { ok("Xbox controller selected."); sep(); return ControllerType::Xbox; }
            "2" => { ok("DualSense (PS5) selected."); sep(); return ControllerType::DualSense; }
            "3" => { ok("DualShock 4 (PS4) selected."); sep(); return ControllerType::DualShock4; }
            _ => {
                print!("{}│{}  Invalid — enter 1, 2 or 3: ", DIM, RESET);
                std::io::stdout().flush().unwrap();
            }
        }
    }
}

fn wait_for_sony_controller(hid: &hidapi::HidApi, pids: &[u16]) -> hidapi::HidDevice {
    let mut elapsed = 0u64;
    let mut hint_shown = false;
    let mut dot_count = 0u32;
    loop {
        let devices: Vec<_> = hid.device_list()
            .filter(|d| d.vendor_id() == SONY_VID && pids.contains(&d.product_id()))
            .collect();

        if let Some(dev_info) = devices.first() {
            let lines_to_clear = if hint_shown { 1 + 3 + dot_count / 60 } else { dot_count / 60 };
            for _ in 0..=lines_to_clear {
                print!("\x1b[1A\x1b[2K");
            }
            std::io::stdout().flush().unwrap();

            match dev_info.open_device(hid) {
                Ok(dev) => {
                    ok(&format!("Controller found (VID={:04X} PID={:04X})", SONY_VID, dev_info.product_id()));
                    return dev;
                }
                Err(e) => {
                    fail(&format!("Found device but could not open: {}", e));
                    warn("Make sure no other app (DS4Windows, Steam) is using the controller.");
                    sep();
                    pause_and_exit(1);
                    unreachable!()
                }
            }
        }

        if elapsed >= 10 && !hint_shown {
            println!();
            warn("Controller not detected after 10 seconds.");
            warn("Make sure the controller is connected via USB or Bluetooth.");
            warn("If Bluetooth: unpair in Windows Settings, then re-pair.");
            hint_shown = true;
        }
        print!(".");
        dot_count += 1;
        if dot_count % 60 == 0 { println!(); }
        std::io::stdout().flush().unwrap();
        thread::sleep(Duration::from_millis(1000));
        elapsed += 1;
    }
}

fn read_ds4(buf: &[u8]) -> PhysicalState {
    let offset = if buf[0] == 0x01 { 1 } else { 3 };
    let lx  =  ps_axis_to_i16(buf[offset]);
    let ly  = -ps_axis_to_i16(buf[offset + 1]);
    let rx  =  ps_axis_to_i16(buf[offset + 2]);
    let ry  = -ps_axis_to_i16(buf[offset + 3]);
    let lt  = ps_trigger_to_u8(buf[offset + 7]);
    let rt  = ps_trigger_to_u8(buf[offset + 8]);

    let face     = buf[offset + 4];
    let shoulder = buf[offset + 5];

    let dpad = face & 0x0F;
    let dpad_up    = matches!(dpad, 0|1|7);
    let dpad_down  = matches!(dpad, 3|4|5);
    let dpad_left  = matches!(dpad, 5|6|7);
    let dpad_right = matches!(dpad, 1|2|3);

    let square   = (face & 0x10) != 0;
    let cross    = (face & 0x20) != 0;
    let circle   = (face & 0x40) != 0;
    let triangle = (face & 0x80) != 0;

    let l1      = (shoulder & 0x01) != 0; // bit0
    let r1      = (shoulder & 0x02) != 0; // bit1
    let share   = (shoulder & 0x10) != 0; // bit4
    let options = (shoulder & 0x20) != 0; // bit5
    let l3      = (shoulder & 0x40) != 0; // bit6
    let r3      = (shoulder & 0x80) != 0; // bit7

    let mut btns: u16 = 0;
    if dpad_up    { btns |= 0x0001; }
    if dpad_down  { btns |= 0x0002; }
    if dpad_left  { btns |= 0x0004; }
    if dpad_right { btns |= 0x0008; }
    if cross      { btns |= 0x1000; } // A
    if circle     { btns |= 0x2000; } // B
    if square     { btns |= 0x4000; } // X
    if triangle   { btns |= 0x8000; } // Y
    if l1         { btns |= 0x0100; } // LB
    if r1         { btns |= 0x0200; } // RB
    if l3         { btns |= 0x0040; } // LS
    if r3         { btns |= 0x0080; } // RS
    if options    { btns |= 0x0010; } // Start
    if share      { btns |= 0x0020; } // Back

    let lt_held = lt > 30;
    PhysicalState { lx, ly, rx, ry, lt, rt, buttons: btns, lt_held }
}

fn read_dualsense(buf: &[u8]) -> PhysicalState {
    let offset = if buf[0] == 0x31 { 3 } else { 1 };
    let lx  =  ps_axis_to_i16(buf[offset]);
    let ly  = -ps_axis_to_i16(buf[offset + 1]);
    let rx  =  ps_axis_to_i16(buf[offset + 2]);
    let ry  = -ps_axis_to_i16(buf[offset + 3]);
    let lt  = ps_trigger_to_u8(buf[offset + 4]);
    let rt  = ps_trigger_to_u8(buf[offset + 5]);

    let btns1 = buf[offset + 7]; // dpad + face
    let btns2 = buf[offset + 8]; // l1/r1/l2/r2/create/options/l3/r3

    let dpad = btns1 & 0x0F;
    let dpad_up    = matches!(dpad, 0|1|7);
    let dpad_down  = matches!(dpad, 3|4|5);
    let dpad_left  = matches!(dpad, 5|6|7);
    let dpad_right = matches!(dpad, 1|2|3);

    let square   = (btns1 & 0x10) != 0;
    let cross    = (btns1 & 0x20) != 0;
    let circle   = (btns1 & 0x40) != 0;
    let triangle = (btns1 & 0x80) != 0;

    let l1      = (btns2 & 0x01) != 0;
    let r1      = (btns2 & 0x02) != 0;
    let create  = (btns2 & 0x10) != 0; // Create → Back
    let options = (btns2 & 0x20) != 0; // Options → Start
    let l3      = (btns2 & 0x40) != 0;
    let r3      = (btns2 & 0x80) != 0;

    let mut btns: u16 = 0;
    if dpad_up    { btns |= 0x0001; }
    if dpad_down  { btns |= 0x0002; }
    if dpad_left  { btns |= 0x0004; }
    if dpad_right { btns |= 0x0008; }
    if cross      { btns |= 0x1000; } // A
    if circle     { btns |= 0x2000; } // B
    if square     { btns |= 0x4000; } // X
    if triangle   { btns |= 0x8000; } // Y
    if l1         { btns |= 0x0100; } // LB
    if r1         { btns |= 0x0200; } // RB
    if l3         { btns |= 0x0040; } // LS
    if r3         { btns |= 0x0080; } // RS
    if options    { btns |= 0x0010; } // Start
    if create     { btns |= 0x0020; } // Back

    let lt_held = lt > 30;
    PhysicalState { lx, ly, rx, ry, lt, rt, buttons: btns, lt_held }
}

fn play_connect() {
    unsafe {
        let alias: Vec<u16> = "DeviceConnect\0".encode_utf16().collect();
        PlaySoundW(alias.as_ptr(), std::ptr::null_mut(), SND_ALIAS | SND_ASYNC | SND_NODEFAULT);
    }
}

fn play_disconnect() {
    unsafe {
        let alias: Vec<u16> = "DeviceDisconnect\0".encode_utf16().collect();
        PlaySoundW(alias.as_ptr(), std::ptr::null_mut(), SND_ALIAS | SND_ASYNC | SND_NODEFAULT);
    }
}

fn main() {
    enable_ansi();
    set_random_window_title();
    banner();

    check_dependencies();

    let controller_type = ask_controller_type();

    let own_exe = std::env::current_exe().expect("Failed to get exe path");
    setup_hidhide(&own_exe);

    let game_running = Command::new("tasklist")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains("fortniteclient"))
        .unwrap_or(false);

    if game_running {
        section("WARNING");
        warn("A game is already running!");
        warn("The virtual controller may get a higher XInput index than");
        warn("the physical one, causing double input.");
        warn("Close the game, restart this tool first, then open the game.");
        sep();
        println!("\nPress Enter to continue anyway, or Ctrl+C to exit...");
        let _ = std::io::stdin().read_line(&mut String::new());
    }

    section("VIRTUAL CONTROLLER");
    let client = Client::connect().expect("Failed to connect to ViGEmBus.");
    let mut target = Xbox360Wired::new(client, vigem_client::TargetId::XBOX360_WIRED);
    target.plugin().expect("Failed to create virtual controller");
    let mut ready = false;
    for attempt in 1..=10u32 {
        thread::sleep(Duration::from_millis(200 * attempt as u64));
        if target.wait_ready().is_ok() { ready = true; break; }
    }
    if !ready {
        fail("Virtual controller failed to initialize after several attempts.");
        warn("Make sure ViGEmBus is properly installed and try running as Administrator.");
        sep();
        pause_and_exit(1);
    }
    ok("Xbox 360 virtual controller is live.");
    sep();

    println!("\n{GREEN}{BOLD}╔{bar}╗{RESET}", bar="═".repeat(W-2), GREEN=GREEN, BOLD=BOLD, RESET=RESET);
    println!("{GREEN}{BOLD}║  {:<width$}║{RESET}", "READY  —  open your game NOW if not yet open", width=W-4, GREEN=GREEN, BOLD=BOLD, RESET=RESET);
    println!("{GREEN}{BOLD}║  {:<width$}║{RESET}", "F5       →  toggle Aim Assist ON / OFF",       width=W-4, GREEN=GREEN, BOLD=BOLD, RESET=RESET);
    println!("{GREEN}{BOLD}║  {:<width$}║{RESET}", "L2       →  jitter while held (even if F5 OFF)", width=W-4, GREEN=GREEN, BOLD=BOLD, RESET=RESET);
    println!("{GREEN}{BOLD}║  {:<width$}║{RESET}", "Ctrl+C   →  exit (BEFORE closing the game)",    width=W-4, GREEN=GREEN, BOLD=BOLD, RESET=RESET);
    println!("{GREEN}{BOLD}╚{bar}╝{RESET}\n", bar="═".repeat(W-2), GREEN=GREEN, BOLD=BOLD, RESET=RESET);

    let mut enabled = false;
    let mut f5_was_down = false;
    let mut tick: u64 = 0;

    match controller_type {
        ControllerType::Xbox => {
            section("CONTROLLER DETECTION");
            let xinput = XInputHandle::load_default().expect("Failed to load XInput.");
            info("Waiting for Xbox controller (USB or Bluetooth)...");

            let phys_idx = {
                let mut elapsed = 0u64;
                let mut hint_shown = false;
                let mut dot_count = 0u32;
                loop {
                    match find_physical_controller(&xinput) {
                        Some(i) => {
                            let lines_to_clear = if hint_shown { 1 + 3 + dot_count / 60 } else { dot_count / 60 };
                            for _ in 0..=lines_to_clear {
                                print!("\x1b[1A\x1b[2K");
                            }
                            std::io::stdout().flush().unwrap();
                            ok(&format!("Controller found at XInput index {}", i));
                            break i;
                        }
                        None => {
                            if elapsed >= 10 && !hint_shown {
                                println!();
                                warn("Controller not detected after 10 seconds.");
                                warn("If using Bluetooth: unpair the controller in Windows Settings,");
                                warn("  then re-pair it. Do NOT just disconnect — full unpair required.");
                                warn("If using USB: unplug and replug the cable.");
                                hint_shown = true;
                            }
                            print!(".");
                            dot_count += 1;
                            if dot_count % 60 == 0 { println!(); }
                            std::io::stdout().flush().unwrap();
                            thread::sleep(Duration::from_millis(1000));
                            elapsed += 1;
                        }
                    }
                }
            };
            sep();

            loop {
                let f5_down = unsafe { GetAsyncKeyState(VK_F5) } as u16 & 0x8000 != 0;
                if f5_down && !f5_was_down {
                    enabled = !enabled;
                    if enabled {
                        play_connect();
                        println!("  {}{}[F5]{}  Aim Assist  ██ ON  ██{}", BOLD, GREEN, RESET, RESET);
                    } else {
                        play_disconnect();
                        println!("  {}{}[F5]{}  Aim Assist  ░░ OFF ░░{}", BOLD, DIM, RESET, RESET);
                    }
                }
                f5_was_down = f5_down;

                let phys = match xinput.get_state(phys_idx) {
                    Ok(s)  => s,
                    Err(_) => { eprint!("  [!] Controller disconnected, retrying..."); thread::sleep(Duration::from_millis(500)); continue; }
                };

                let gp = &phys.raw.Gamepad;
                let lt_held = gp.bLeftTrigger > 30;
                let stick_idle = (gp.sThumbLX as i32).abs() < 8000 && (gp.sThumbLY as i32).abs() < 8000;
                let jitter_active = (enabled || lt_held) && stick_idle;
                let axis_value: i16 = if tick % 2 == 0 { AXIS_MIN } else { AXIS_MAX };

                let gamepad = XGamepad {
                    buttons:       vigem_client::XButtons(gp.wButtons),
                    left_trigger:  gp.bLeftTrigger,
                    right_trigger: gp.bRightTrigger,
                    thumb_lx:      if jitter_active { axis_value } else { gp.sThumbLX },
                    thumb_ly:      gp.sThumbLY,
                    thumb_rx:      gp.sThumbRX,
                    thumb_ry:      gp.sThumbRY,
                };

                if let Err(e) = target.update(&gamepad) {
                    eprintln!("  [!] Virtual controller update error: {:?}", e);
                }

                tick = tick.wrapping_add(1);
                thread::sleep(Duration::from_millis(POLL_MS));
            }
        }

        ControllerType::DualShock4 | ControllerType::DualSense => {
            section("CONTROLLER DETECTION");
            let pids: &[u16] = match controller_type {
                ControllerType::DualSense  => &[DUALSENSE_PID],
                ControllerType::DualShock4 => DS4_PIDS,
                ControllerType::Xbox       => unreachable!(),
            };
            let name = match controller_type {
                ControllerType::DualSense  => "DualSense",
                ControllerType::DualShock4 => "DualShock 4",
                ControllerType::Xbox       => unreachable!(),
            };
            info(&format!("Waiting for {} (USB or Bluetooth)...", name));

            let hid = hidapi::HidApi::new().expect("Failed to init HidApi");
            let dev = wait_for_sony_controller(&hid, pids);
            dev.set_blocking_mode(false).unwrap_or(());
            sep();

            let mut buf = [0u8; 128];
            loop {
                let f5_down = unsafe { GetAsyncKeyState(VK_F5) } as u16 & 0x8000 != 0;
                if f5_down && !f5_was_down {
                    enabled = !enabled;
                    if enabled {
                        play_connect();
                        println!("  {}{}[F5]{}  Aim Assist  ██ ON  ██{}", BOLD, GREEN, RESET, RESET);
                    } else {
                        play_disconnect();
                        println!("  {}{}[F5]{}  Aim Assist  ░░ OFF ░░{}", BOLD, DIM, RESET, RESET);
                    }
                }
                f5_was_down = f5_down;

                let n = match dev.read_timeout(&mut buf, 1) {
                    Ok(0) | Err(_) => continue,
                    Ok(n) => n,
                };

                if n < 10 { continue; }

                let state = match controller_type {
                    ControllerType::DualShock4 => read_ds4(&buf),
                    ControllerType::DualSense  => read_dualsense(&buf),
                    ControllerType::Xbox       => unreachable!(),
                };

                let stick_idle = (state.lx as i32).abs() < 8000 && (state.ly as i32).abs() < 8000;
                let jitter_active = (enabled || state.lt_held) && stick_idle;
                let axis_value: i16 = if tick % 2 == 0 { AXIS_MIN } else { AXIS_MAX };

                let gamepad = XGamepad {
                    buttons:       vigem_client::XButtons(state.buttons),
                    left_trigger:  state.lt,
                    right_trigger: state.rt,
                    thumb_lx:      if jitter_active { axis_value } else { state.lx },
                    thumb_ly:      state.ly,
                    thumb_rx:      state.rx,
                    thumb_ry:      state.ry,
                };

                if let Err(e) = target.update(&gamepad) {
                    eprintln!("  [!] Virtual controller update error: {:?}", e);
                }

                tick = tick.wrapping_add(1);
            }
        }
    }
}
