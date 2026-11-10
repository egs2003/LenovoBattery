# Workspace Guide

Lenovo Legion battery charge mode switcher (Normal / Storage = Conservation, max. 80%), in two forms:
- Rust GUI app (egui/eframe): [src/main.rs](src/main.rs) (GUI), [src/energy.rs](src/energy.rs) (driver access via `DeviceIoControl`).
- Original PowerShell script: [Set-LegionBatteryMode.ps1](Set-LegionBatteryMode.ps1) (reference implementation).

## Conventions

- Windows only. Rust edition 2021 with MSVC toolchain; build with `cargo build --release`. `app.manifest` (via [build.rs](build.rs)) requests Administrator, so run the exe or `cargo run` from an elevated shell.
- Do not run the app or script to "test" it: it changes real hardware state.
- User-facing text and comments are German, written without umlauts (`ae/oe/ue`, e.g. "ueber", "oeffnen"). Keep that style.
- PowerShell output via `Write-Host` with `-ForegroundColor` (Red = error, Green = success, Yellow = action, Cyan = status).

## Pitfalls

- Hardware is controlled via `DeviceIoControl` on `\\.\EnergyDrv`, not via the registry. The registry write only keeps Lenovo Vantage in sync.
- IOCTL code `0x831020F8` and the command byte sequences are taken from LenovoLegionToolkit: Conservation = `0x08`, `0x03`; Normal = `0x05`, `0x08`; status query = `0xFF` (bit `0x20` = Conservation, `0x04` = RapidCharge). Keep Rust and PowerShell in sync and do not change these values without a verified source.
- The Rust code declares the kernel32 functions by hand (no `windows` crate); the driver handle is closed in `Drop`.
