# Legion Battery Mode

Umschalter für den Akku-Lademodus von Lenovo-Legion-Notebooks (Windows, Rust, egui).

- Modi: **Normal** (lädt bis 100 %) und **Storage** (Conservation, Ladung auf ca. 80 % begrenzt). RapidCharge wird angezeigt, kann aber nicht gesetzt werden.
- Steuerung über den Lenovo-Energy-Treiber (`\\.\EnergyDrv`). Der Treiber muss installiert sein (z. B. mit Lenovo Vantage), die App braucht Administratorrechte und fragt beim Start danach.
- Design: dunkel oder hell, automatisch passend zur Windows-Einstellung.
- Icon: Akku mit 80-%-Marke (`assets/icon.ico` in der exe, `assets/icon.rgba` fuer das Fenster).
- Updates: Beim Start prüft die App im Hintergrund, ob es auf GitHub (Releases) eine neuere Version gibt. Gefragt wird erst, wenn es eine gibt; nach Bestätigung lädt die App `legion-battery.exe` herunter, ersetzt sich selbst und startet neu. Unter "Programm" gibt es zusätzlich "Nach Updates suchen".
  Jeder Push auf `main` veröffentlicht ein Release `v<Version aus Cargo.toml>`. Für ein Update muss die Version in `Cargo.toml` erhöht werden.

Zusätzlich liegt das ursprüngliche PowerShell-Skript `Set-LegionBatteryMode.ps1` als Referenz bei.

Bauen: `cargo build --release` (Windows, MSVC-Toolchain).
