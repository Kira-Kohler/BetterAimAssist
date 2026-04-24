use std::io::{stdout, Write};
use std::sync::atomic::Ordering;

use crossterm::{
    cursor,
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};

use crate::{STATUS_ROW, TRIGGER_MODE_ROW};

pub const W: u16 = 66;

const C_BORDER: Color = Color::Cyan;
const C_TITLE:  Color = Color::White;
const C_DIM:    Color = Color::DarkGrey;
const C_OK:     Color = Color::Green;
const C_WARN:   Color = Color::Yellow;
const C_ERR:    Color = Color::Red;
const C_ACCENT: Color = Color::Cyan;

fn vlen(s: &str) -> usize {
    s.chars().count()
}

fn pad(s: &str, width: usize) -> String {
    let len = vlen(s);
    if len >= width { s.to_string() } else { format!("{}{}", s, " ".repeat(width - len)) }
}

fn gap_spaces(used: usize) -> String {
    let inner = W as usize - 4;
    " ".repeat(inner.saturating_sub(used))
}

pub fn enable_ansi() {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
    };
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0u32;
        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

pub fn clear_screen() {
    execute!(stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0)).ok();
}

fn w() -> usize { W as usize }
fn inner() -> usize { W as usize - 4 }

pub fn box_top(color: Color) {
    execute!(stdout(),
        SetForegroundColor(color), SetAttribute(Attribute::Bold),
        Print(format!("╔{}╗\r\n", "═".repeat(w() - 2))),
        ResetColor,
    ).ok();
}

pub fn box_mid(color: Color) {
    execute!(stdout(),
        SetForegroundColor(color), SetAttribute(Attribute::Bold),
        Print(format!("╠{}╣\r\n", "═".repeat(w() - 2))),
        ResetColor,
    ).ok();
}

pub fn box_bot(color: Color) {
    execute!(stdout(),
        SetForegroundColor(color), SetAttribute(Attribute::Bold),
        Print(format!("╚{}╝\r\n", "═".repeat(w() - 2))),
        ResetColor,
    ).ok();
}

pub fn box_empty(color: Color) {
    execute!(stdout(),
        SetForegroundColor(color), SetAttribute(Attribute::Bold),
        Print(format!("║{}║\r\n", " ".repeat(w() - 2))),
        ResetColor,
    ).ok();
}

#[allow(dead_code)]
pub fn box_line(border: Color, text_color: Color, dim: bool, text: &str) {
    let content = pad(text, inner());
    let mut out = stdout();
    queue!(out,
        SetForegroundColor(border), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
        SetForegroundColor(text_color),
    ).ok();
    if dim { queue!(out, SetAttribute(Attribute::Dim)).ok(); }
    queue!(out,
        Print(&content), ResetColor,
        SetForegroundColor(border), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
    ).ok();
    out.flush().ok();
}

pub fn section_header(label: &str) {
    let used = 4 + vlen(label);
    let bar  = "─".repeat(w().saturating_sub(used));
    let mut out = stdout();
    queue!(out, Print("\r\n"),
        SetForegroundColor(C_DIM), Print("  ┌╴"), ResetColor,
        SetAttribute(Attribute::Bold), SetForegroundColor(C_ACCENT), Print(label), ResetColor,
        SetForegroundColor(C_DIM), Print(format!(" {}\r\n", bar)), ResetColor,
    ).ok();
    out.flush().ok();
}

pub fn section_end() {
    execute!(stdout(),
        SetForegroundColor(C_DIM),
        Print(format!("  └{}\r\n", "─".repeat(w() - 3))),
        ResetColor,
    ).ok();
}

pub fn log_ok(msg: &str) {
    execute!(stdout(),
        SetForegroundColor(C_DIM), Print("  │  "),
        SetForegroundColor(C_OK), SetAttribute(Attribute::Bold), Print("✓ "),
        ResetColor, Print(msg), Print("\r\n"),
    ).ok();
}

pub fn log_info(msg: &str) {
    execute!(stdout(),
        SetForegroundColor(C_DIM), Print("  │  "),
        SetForegroundColor(Color::Blue), Print("· "),
        ResetColor, SetAttribute(Attribute::Dim), Print(msg), ResetColor, Print("\r\n"),
    ).ok();
}

pub fn log_warn(msg: &str) {
    execute!(stdout(),
        SetForegroundColor(C_DIM), Print("  │  "),
        SetForegroundColor(C_WARN), SetAttribute(Attribute::Bold), Print("! "),
        ResetColor, SetForegroundColor(C_WARN), Print(msg), ResetColor, Print("\r\n"),
    ).ok();
}

pub fn log_fail(msg: &str) {
    execute!(stdout(),
        SetForegroundColor(C_DIM), Print("  │  "),
        SetForegroundColor(C_ERR), SetAttribute(Attribute::Bold), Print("✗ "),
        ResetColor, SetForegroundColor(C_ERR), Print(msg), ResetColor, Print("\r\n"),
    ).ok();
}

const AIM_ART: &[&str] = &[
    r"  ▄▀█ █ █▀▄▀█   ▄▀█ █▀ █▀ █ █▀ ▀█▀",
    r"  █▀█ █ █ ▀ █   █▀█ ▄█ ▄█ █ ▄█  █ ",
];

pub fn banner() {
    clear_screen();

    let mut out = stdout();
    queue!(out, Print("\r\n")).ok();

    box_top(C_BORDER);
    box_empty(C_BORDER);

    for line in AIM_ART {
        let content = pad(line, inner());
        queue!(out,
            SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
            SetForegroundColor(C_ACCENT), SetAttribute(Attribute::Bold),
            Print(&content), ResetColor,
            SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
        ).ok();
    }

    box_empty(C_BORDER);
    box_mid(C_BORDER);

    let author = "  by Kira Kohler";
    let repo   = "github.com/Kira-Kohler/BetterAimAssist  ";
    let g      = gap_spaces(vlen(author) + vlen(repo));
    queue!(out,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
        SetForegroundColor(C_DIM), SetAttribute(Attribute::Dim), Print(author), ResetColor,
        Print(&g),
        SetForegroundColor(C_DIM), SetAttribute(Attribute::Dim), Print(repo), ResetColor,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
    ).ok();

    box_empty(C_BORDER);
    box_bot(C_BORDER);

    queue!(out, Print("\r\n")).ok();
    out.flush().ok();
}

pub fn draw_dashboard(controller_name: &str, enabled: bool) {
    clear_screen();

    let mut out = stdout();
    queue!(out, cursor::Hide, Print("\r\n")).ok();

    box_top(C_BORDER);

    let title = "  BETTER AIM-ASSIST";
    let tag   = "* ACTIVE  ";
    let g     = gap_spaces(vlen(title) + vlen(tag));
    queue!(out,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
        SetForegroundColor(C_TITLE), SetAttribute(Attribute::Bold), Print(title), ResetColor,
        Print(&g),
        SetForegroundColor(C_OK), SetAttribute(Attribute::Bold), Print(tag), ResetColor,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
    ).ok();

    box_mid(C_BORDER);
    box_empty(C_BORDER);

    {
        let (badge, badge_color) = if enabled {
            ("[ ENABLED ]", C_OK)
        } else {
            ("[ STANDBY ]", C_DIM)
        };
        let label = "  Aim Assist";
        let g     = gap_spaces(vlen(label) + vlen(badge));

        let row = cursor::position().map(|p| p.1).unwrap_or(0);
        STATUS_ROW.store(row, Ordering::Relaxed);

        queue!(out,
            SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
            SetAttribute(Attribute::Bold), SetForegroundColor(C_TITLE), Print(label), ResetColor,
            Print(&g),
            SetForegroundColor(badge_color), SetAttribute(Attribute::Bold), Print(badge), ResetColor,
            SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
        ).ok();
    }

    {
        let (tmbadge, tmbadge_color) = ("[ ON ]", C_OK);
        let label = "  Trigger Mode";
        let g     = gap_spaces(vlen(label) + vlen(tmbadge));

        let row = cursor::position().map(|p| p.1).unwrap_or(0);
        TRIGGER_MODE_ROW.store(row, Ordering::Relaxed);

        queue!(out,
            SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
            SetAttribute(Attribute::Bold), SetForegroundColor(C_TITLE), Print(label), ResetColor,
            Print(&g),
            SetForegroundColor(tmbadge_color), SetAttribute(Attribute::Bold), Print(tmbadge), ResetColor,
            SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
        ).ok();
    }

    {
        let label = "  Controller";
        let g     = gap_spaces(vlen(label) + vlen(controller_name));
        queue!(out,
            SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
            SetForegroundColor(C_DIM), Print(label), ResetColor,
            Print(&g),
            SetForegroundColor(C_ACCENT), SetAttribute(Attribute::Dim), Print(controller_name), ResetColor,
            SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
        ).ok();
    }

    box_empty(C_BORDER);
    box_mid(C_BORDER);
    box_empty(C_BORDER);

    keybind(&mut out, "F5",     "Toggle aim assist on / off");
    keybind(&mut out, "F6",     "Toggle trigger (LT/L2) activation");
    keybind(&mut out, "Ctrl+C","Exit - restores controller automatically");

    box_empty(C_BORDER);
    box_mid(C_BORDER);
    box_empty(C_BORDER);

    let warn_line = "  !  Do NOT close via Task Manager or End Task";
    queue!(out,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
        SetForegroundColor(C_WARN), SetAttribute(Attribute::Bold),
        Print(&pad(warn_line, inner())), ResetColor,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
    ).ok();

    let sub_line = "     Physical controller stays hidden until reboot if forced";
    queue!(out,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
        SetForegroundColor(C_DIM), SetAttribute(Attribute::Dim),
        Print(&pad(sub_line, inner())), ResetColor,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
    ).ok();

    box_empty(C_BORDER);
    box_bot(C_BORDER);

    out.flush().ok();
}

fn keybind(out: &mut impl Write, key: &str, desc: &str) {
    let key_col = format!("  {:>7}", key);
    let sep     = "  |  ";
    let used    = vlen(&key_col) + vlen(sep) + vlen(desc);
    let g       = gap_spaces(used);
    let _ = queue!(out,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
        SetForegroundColor(C_ACCENT), SetAttribute(Attribute::Bold), Print(&key_col), ResetColor,
        SetForegroundColor(C_DIM), Print(sep), ResetColor,
        SetForegroundColor(C_TITLE), Print(desc), ResetColor,
        Print(&g),
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║\r\n"), ResetColor,
    );
}

pub fn update_aim_status(enabled: bool) {
    let (badge, color) = if enabled {
        ("[ ENABLED ]", C_OK)
    } else {
        ("[ STANDBY ]", C_DIM)
    };
    let label = "  Aim Assist";
    let g     = gap_spaces(vlen(label) + vlen(badge));
    let row   = STATUS_ROW.load(Ordering::Relaxed);

    execute!(stdout(),
        cursor::SavePosition,
        cursor::MoveTo(0, row),
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
        SetAttribute(Attribute::Bold), SetForegroundColor(C_TITLE), Print(label), ResetColor,
        Print(&g),
        SetForegroundColor(color), SetAttribute(Attribute::Bold), Print(badge), ResetColor,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║"), ResetColor,
        cursor::RestorePosition,
    ).ok();
}

pub fn update_trigger_mode_status(enabled: bool) {
    let (badge, color) = if enabled {
        ("[ ON ]  ", C_OK)
    } else {
        ("[ OFF ] ", C_WARN)
    };
    let label = "  Trigger Mode";
    let g     = gap_spaces(vlen(label) + vlen(badge));
    let row   = TRIGGER_MODE_ROW.load(Ordering::Relaxed);

    execute!(stdout(),
        cursor::SavePosition,
        cursor::MoveTo(0, row),
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print("║ "), ResetColor,
        SetAttribute(Attribute::Bold), SetForegroundColor(C_TITLE), Print(label), ResetColor,
        Print(&g),
        SetForegroundColor(color), SetAttribute(Attribute::Bold), Print(badge), ResetColor,
        SetForegroundColor(C_BORDER), SetAttribute(Attribute::Bold), Print(" ║"), ResetColor,
        cursor::RestorePosition,
    ).ok();
}

pub fn pause_and_exit(code: i32) {
    execute!(stdout(), cursor::Show, Print("\r\n"),
        SetForegroundColor(C_DIM), Print("  Press Enter to exit..."), ResetColor, Print("\r\n"),
    ).ok();
    let _ = std::io::stdin().read_line(&mut String::new());
    std::process::exit(code);
}
