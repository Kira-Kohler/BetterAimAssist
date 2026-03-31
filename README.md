# BetterAimAssist

<div align="center">

![Platform](https://img.shields.io/badge/platform-Windows-blue?style=for-the-badge&logo=windows)
![Language](https://img.shields.io/badge/built%20with-Rust-orange?style=for-the-badge&logo=rust)
![License](https://img.shields.io/badge/license-MIT-green?style=for-the-badge)

**A virtual Xbox controller proxy that adds rapid axis jitter to boost aim assist in PC games.**

</div>

---

## What is this?

BetterAimAssist sits between your physical controller and the game. It creates a virtual Xbox 360 controller that mirrors all your inputs in real time. When toggled on with **F5**, it rapidly oscillates the left stick X axis between -1 and +1 at ~60 Hz — a technique that exploits aim assist algorithms in games that use magnetism-based targeting.

```
Physical controller  →  BetterAimAssist  →  Virtual controller  →  Game
      (hidden)               (proxy)             (visible)
```

Jitter is automatically disabled when you move the left stick, so it never interferes with movement.

---

## Features

- Supports **Xbox**, **DualSense (PS5)** and **DualShock 4 (PS4)** controllers
- Automatic driver detection — opens download page if something is missing
- Hides the physical controller from games via **HidHide** (no double input)
- Mirrors all buttons, triggers and sticks from physical to virtual in real time
- **F5** toggles jitter on/off at any time, even mid-game
- **L2** activates jitter while held, even if F5 is off
- Jitter pauses automatically while you move the left stick
- Colored terminal UI with status output
- Warns you if the game is already running before the tool

---

## Requirements

| Requirement | Details |
|---|---|
| OS | Windows 10 / 11 x64 |
| Controller | Xbox (wired or Bluetooth), DualSense (PS5), DualShock 4 (PS4) |
| Driver | [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) — no reboot needed |
| Driver | [HidHide](https://github.com/nefarius/HidHide/releases) — reboot required after install |
| Permissions | Run as **Administrator** |

---

## Installation

### Step 1 — Install drivers

BetterAimAssist checks for the required drivers on startup. If one or both are missing, it opens the download page(s) automatically with a 3-second countdown, then exits so you can install them.

- **ViGEmBus** does not require a reboot — run the tool again immediately after installing.
- **HidHide** requires a full reboot before the tool will detect it.

Manual links:
- **ViGEmBus**: https://github.com/nefarius/ViGEmBus/releases
- **HidHide**: https://github.com/nefarius/HidHide/releases

### Step 2 — Download BetterAimAssist

Download the latest `BetterAimAssist.exe` from the **[Releases](../../releases)** page. No installation needed — just run the `.exe`.

### Step 3 — Run as Administrator

Right-click the `.exe` → **Run as administrator**. This is required for HidHide to hide the physical controller.

### Step 4 — Launch order matters

> **Always open BetterAimAssist *before* your game.**

This ensures the virtual controller occupies XInput slot 0 (which the game picks up), while the physical controller falls to slot 1 (used internally by the tool). If you open the game first, the tool will warn you.

---

## Usage

When everything is ready, you'll see:

```
╔══════════════════════════════════════════════════════════╗
║           BETTER AIM-ASSIST by Kira Kohler               ║
╚══════════════════════════════════════════════════════════╝

┌─ CONTROLLER TYPE ─────────────────────────────────────────
│  [1] Xbox / Xbox-compatible
│  [2] DualSense (PS5)
│  [3] DualShock 4 (PS4)
└───────────────────────────────────────────────────────────

┌─ DEPENDENCIES ────────────────────────────────────────────
│  [✓] ViGEmBus driver   — installed
│  [✓] HidHide driver    — installed
└───────────────────────────────────────────────────────────

┌─ HIDHIDE CLOAKING ────────────────────────────────────────
│  [✓] Cloaking active — physical controller hidden from games.
└───────────────────────────────────────────────────────────

┌─ VIRTUAL CONTROLLER ──────────────────────────────────────
│  [✓] Xbox 360 virtual controller is live.
└───────────────────────────────────────────────────────────

╔══════════════════════════════════════════════════════════╗
║  READY  —  open your game NOW if not yet open            ║
║  F5       →  toggle axis jitter ON / OFF                 ║
║  L2       →  jitter while held (even if F5 OFF)          ║
║  Ctrl+C   →  exit (do this BEFORE closing the game)      ║
╚══════════════════════════════════════════════════════════╝
```

| Key | Action |
|---|---|
| `F5` | Toggle left stick X axis jitter **ON / OFF** (persistent) |
| `L2` | Activate jitter **while held**, even if F5 is OFF |
| `Ctrl+C` | Exit cleanly — always do this before closing the game |

---

## Controller support

### Xbox / Xbox-compatible
Full support via XInput. USB and Bluetooth.

### DualSense (PS5)
Full support via direct HID. USB and Bluetooth. Button mapping:

| PS5 | Xbox (virtual) |
|---|---|
| Cross | A |
| Circle | B |
| Square | X |
| Triangle | Y |
| L1 | LB |
| R1 | RB |
| L2 | LT |
| R2 | RT |
| L3 | LS |
| R3 | RS |
| Options | Start |
| Create | Back |
| D-Pad | D-Pad |

### DualShock 4 (PS4)
Same button mapping as DualSense. Both V1 (CUH-ZCT1) and V2 (CUH-ZCT2) are supported.

> **Note:** Close Steam, DS4Windows or any other app that might be using the controller before running BetterAimAssist.

---

## Game compatibility

> BetterAimAssist has been tested on **Fortnite**. It will likely work on other games that have strong magnetism-based aim assist and XInput support on PC — but results are not guaranteed.

### Tested

| Game | Status |
|---|---|
| Fortnite | ✅ Works |

### Probably works (untested)

Games with aggressive magnetism-based aim assist and XInput support on PC:

- Warzone / Modern Warfare
- Apex Legends

---

## Building from source

```bash
# Requires Rust 1.75+ (https://rustup.rs) and Windows
git clone https://github.com/Kira-Kohler/BetterAimAssist
cd BetterAimAssist
cargo build --release
# Output → target/release/BetterAimAssist.exe
```

---

## Troubleshooting

| Problem | Fix |
|---|---|
| **Controller not detected** | Wait 10 seconds — the tool will show a hint automatically |
| **Bluetooth Xbox controller not detected** | Unpair the controller in Windows Settings (Bluetooth → remove device), then re-pair it. Simply disconnecting is not enough |
| **Bluetooth PS4/PS5 controller not detected** | Same — fully unpair and re-pair in Windows Settings |
| **USB controller not detected** | Unplug and replug the cable. Make sure you're using a data cable, not a charge-only cable |
| **PS4/PS5 controller not opening** | Close Steam, DS4Windows or any other app using the controller, then restart the tool |
| **Double input / button registers twice** | Launch BetterAimAssist *before* the game. If the game was already open: close game → close tool → open tool → open game |
| **HidHide CLI not found after install** | Reboot your PC — the HidHide driver requires a full restart to initialize |
| **Virtual controller not detected by the game** | Close both the game and the tool, reopen the tool first, wait for `Virtual controller is live`, then open the game |
| **Virtual controller not ready (WinError)** | The tool retries automatically. If it keeps failing, reinstall ViGEmBus and run as Administrator |
| **Jitter ON but aim assist doesn't feel different** | Make sure aim assist is enabled in-game for controllers |
| **Tool says "access denied"** | Run as Administrator — right-click the exe → Run as administrator |

---

## License

MIT — do whatever you want with it.
