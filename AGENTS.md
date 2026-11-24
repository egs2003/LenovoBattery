# Legion Battery Mode: Hinweise für Agenten

Umschalter für den Lademodus von Lenovo-Legion-Notebooks (Normal / Storage = Conservation, max. 80 %), in zwei Formen:
Rust-GUI (egui/eframe 0.29) und das ursprüngliche PowerShell-Skript als Referenz. Funktionen: siehe `README.md`.

## Workflow

- Änderungen gehen **direkt auf `main`**, ohne Branch und Pull Request.
- **Bei jeder Änderung die Version in `Cargo.toml` erhöhen** (kleine Korrekturen: Patch, neue Funktionen: Minor).
  Der Build veröffentlicht pro Push ein Release `v<Version>` mit `legion-battery.exe`, und die Update-Funktion der App
  vergleicht dagegen. Ohne höhere Version erkennt kein Benutzer ein Update.
- Neue Funktionen oder geändertes Verhalten kurz in `README.md` nachtragen.
- Commit-Nachrichten auf Englisch, Oberflächentexte und Kommentare zur Bedienung auf Deutsch.

## Bauen und prüfen

- Das Programm ist **nur für Windows** (`DeviceIoControl` auf `\\.\EnergyDrv`, `winreg`). Auf Linux lässt es sich nicht
  kompilieren. Geprüft wird über GitHub Actions (`.github/workflows/build.yml`: `cargo test`, `cargo build --release`).
  Fehlermeldungen kommen vom Benutzer aus dem Build-Log.
- Ohne Compiler: Signaturen der APIs im Quelltext der Bibliothek nachsehen (egui 0.29), nicht aus dem Gedächtnis schreiben.
  Reine Logik ohne Windows-Bezug (Versionsvergleich in `update.rs`) lässt sich getrennt mit `rustc --test` ausprobieren.
- **Das Programm oder das Skript nie zum "Testen" ausführen**: Es ändert echten Hardwarezustand.
- Eine Oberfläche kann hier nicht angesehen werden. Optische Änderungen deshalb klein halten und das im Ergebnis sagen.
- `app.manifest` (über [build.rs](build.rs)) verlangt Administratorrechte. `cargo run` also aus einer erhöhten Shell starten.

## Aufbau

| Datei | Inhalt |
|---|---|
| `src/main.rs` | Oberfläche (egui), Farbpalette, Theme, Zustand der App |
| `src/energy.rs` | Treiberzugriff per `DeviceIoControl`, Modus lesen/setzen |
| `src/update.rs` | Selbst-Update über GitHub-Releases, mit Tests |
| `Set-LegionBatteryMode.ps1` | Originalskript (Referenzimplementierung) |
| `app.manifest`, `app.rc`, `build.rs` | Administrator-Manifest in die exe einbetten |

## Regeln und Fallen

**Treiber** (`energy.rs`, PowerShell-Skript)
- Die Hardware wird über `DeviceIoControl` auf `\\.\EnergyDrv` gesteuert, nicht über die Registry. Der Registry-Eintrag hält
  nur Lenovo Vantage synchron.
- IOCTL-Code `0x831020F8` und die Befehlsbytes stammen aus LenovoLegionToolkit: Conservation = `0x08`, `0x03`;
  Statusabfrage = `0xFF` (Bit `0x20` = Conservation, Bit `0x04` = RapidCharge). Werte nie ohne geprüfte Quelle ändern.
- **Offen:** Für Normal sendet `energy.rs` aktuell `0x05`, `0x00`, das PowerShell-Skript `0x05`, `0x08`. Der letzte
  Commit hat die Rust-Folge bewusst geändert. Vor einer Angleichung beider Seiten die Quelle prüfen.
- Die kernel32-Funktionen sind von Hand deklariert (kein `windows`-Crate); das Treiber-Handle wird in `Drop` geschlossen.

**Oberfläche** (`main.rs`)
- Farben nur über `pal()`, **keine festen Farbkonstanten** im Zeichencode. Es gibt ein dunkles und ein helles Design,
  das der Systemeinstellung folgt (`DARK`, `LIGHT`).
- `RichText::strong()` nimmt seine Farbe von `widgets.active.fg_stroke`. Diese Schriftfarbe nicht überschreiben.
- `selection.stroke` ist die **Schriftfarbe** ausgewählter Einträge, `selection.bg_fill` deren Hintergrund. Beide dürfen
  nie dieselbe Farbe haben.
- Zeilen mit mehreren Widgets nicht in `right_to_left` mit abgeschätzter Breite bauen. Wird die Zeile breiter als die
  Karte, wächst das Fenster. Lieber auf mehrere Zeilen verteilen (`horizontal_wrapped`).
- Float-Literale in `Stroke::new` als `1.0_f32` schreiben, sonst gibt es Compiler-Warnungen.
- Texte der Oberfläche und Kommentare **ohne Umlaute** (`ae/oe/ue`, z. B. "ueber", "oeffnen"). Diesen Stil beibehalten.

**Update** (`update.rs`)
- Die Prüfung beim Start ist still, gefragt wird erst vor dem Herunterladen.
- Der Pfad der exe wird beim Start gemerkt (`exe()`), weil `current_exe()` nach dem Umbenennen nicht verlässlich ist.
- Das Release-Asset muss `legion-battery.exe` heißen (`ASSET` in `update.rs` und `files` in `build.yml` gemeinsam ändern).
- Der Neustart erbt die Administratorrechte der laufenden App, eine neue UAC-Abfrage gibt es nicht.

**PowerShell** (`Set-LegionBatteryMode.ps1`)
- Ausgabe über `Write-Host` mit `-ForegroundColor` (Rot = Fehler, Grün = Erfolg, Gelb = Aktion, Cyan = Status).
