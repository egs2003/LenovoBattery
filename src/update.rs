//! Selbst-Update ueber die GitHub-Releases (https://github.com/egs2003/LenovoBattery/releases).
//!
//! Ablauf:
//!   1. Beim Start wird im Hintergrund und ohne Nachfrage geprueft, ob es eine neuere Version gibt.
//!   2. Gibt es eine, fragt die App nach. Erst nach Bestaetigung wird heruntergeladen.
//!   3. Die laufende exe wird in `<name>.exe.old` umbenannt (das geht unter Windows auch im
//!      Betrieb), die neue Datei nimmt ihren Platz ein, die App startet neu. Die `.old`-Datei
//!      wird beim naechsten Start geloescht.
//!
//! Der Workflow `.github/workflows/build.yml` veroeffentlicht bei jedem Push auf `main` ein
//! Release `v<Version aus Cargo.toml>` mit der Datei `legion-battery.exe`. Eine neue Version
//! erkennt die App deshalb nur, wenn die Versionsnummer in `Cargo.toml` erhoeht wurde.

use anyhow::{anyhow, bail, Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const API: &str = "https://api.github.com/repos/egs2003/LenovoBattery/releases/latest";
const DOWNLOAD_PREFIX: &str = "https://github.com/egs2003/LenovoBattery/releases/download/";
const ASSET: &str = "legion-battery.exe";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub size: u64,
    pub notes: String,
}

#[derive(Clone, Debug)]
pub enum UpState {
    /// nichts zu melden
    Idle,
    /// manuelle Pruefung laeuft
    Checking,
    /// manuelle Pruefung: schon die neueste Version
    UpToDate,
    /// neuere Version gefunden, Rueckfrage an den Benutzer steht aus
    Available(Release),
    Downloading { done: u64, total: u64 },
    Installing,
    /// neue exe liegt bereit, die App muss neu starten
    ReadyToRestart,
    /// die neue Version wurde gestartet, diese Instanz schliesst sich. Verhindert, dass der
    /// Neustart in jedem weiteren Frame noch einmal ausgeloest wird (zweites Fenster).
    Restarting,
    Failed(String),
}

impl UpState {
    /// Download/Installation laeuft oder ist abgeschlossen: kein weiteres Update starten.
    pub fn busy(&self) -> bool {
        matches!(
            self,
            UpState::Downloading { .. } | UpState::Installing | UpState::ReadyToRestart | UpState::Restarting
        )
    }
}

// ---------------------------------------------------------------- Versionen

/// "v1.2.3", "1.2" oder "1.2.3-beta" -> [1, 2, 3]
pub fn parse_version(s: &str) -> Option<[u64; 3]> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut out = [0u64; 3];
    let mut n = 0;
    for part in core.split('.') {
        if n == 3 {
            return None;
        }
        out[n] = part.trim().parse().ok()?;
        n += 1;
    }
    Some(out)
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

// ---------------------------------------------------------------- Pfade

/// Pfad der exe, so wie sie beim Start hiess. Nach dem Umbenennen im laufenden Betrieb waere
/// `current_exe()` nicht mehr verlaesslich.
static EXE: OnceLock<PathBuf> = OnceLock::new();

fn exe() -> PathBuf {
    EXE.get_or_init(|| std::env::current_exe().unwrap_or_else(|_| PathBuf::from("legion-battery.exe")))
        .clone()
}

fn side_file(suffix: &str) -> PathBuf {
    let mut s = exe().into_os_string();
    s.push(".");
    s.push(suffix);
    PathBuf::from(s)
}

/// Beim Start aufrufen: merkt sich den exe-Pfad und raeumt Reste eines frueheren Updates auf.
pub fn cleanup() {
    let _ = exe();
    let _ = fs::remove_file(side_file("old")); // schlaegt fehl, falls der alte Prozess noch laeuft
    let _ = fs::remove_file(side_file("new"));
}

// ---------------------------------------------------------------- GitHub

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .user_agent("legion-battery-updater")
        .build()
}

fn fetch_latest() -> Result<Release> {
    let resp = agent()
        .get(API)
        .set("Accept", "application/vnd.github+json")
        .call()
        .context("GitHub ist nicht erreichbar")?;
    let v: serde_json::Value = serde_json::from_str(&resp.into_string()?).context("Antwort von GitHub unlesbar")?;
    let tag = v["tag_name"].as_str().ok_or_else(|| anyhow!("Release ohne Versionsnummer"))?;
    let asset = v["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"].as_str() == Some(ASSET)))
        .ok_or_else(|| anyhow!("Das Release {tag} enthaelt keine {ASSET}"))?;
    let url = asset["browser_download_url"]
        .as_str()
        .ok_or_else(|| anyhow!("Release ohne Download-Link"))?
        .to_string();
    if !url.starts_with(DOWNLOAD_PREFIX) {
        bail!("Unerwarteter Download-Link: {url}");
    }
    Ok(Release {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        url,
        size: asset["size"].as_u64().unwrap_or(0),
        notes: v["body"].as_str().unwrap_or("").trim().to_string(),
    })
}

fn set(state: &Mutex<UpState>, ctx: &eframe::egui::Context, s: UpState) {
    *state.lock().unwrap() = s;
    ctx.request_repaint();
}

/// Sucht nach einer neueren Version (im Hintergrund). `manual`: Der Benutzer hat die Pruefung
/// selbst ausgeloest und sieht auch "aktuell" und Fehler. Der Start-Check bleibt dagegen still.
pub fn check(state: Arc<Mutex<UpState>>, ctx: eframe::egui::Context, manual: bool) {
    {
        let mut s = state.lock().unwrap();
        if matches!(*s, UpState::Checking) || s.busy() {
            return;
        }
        if manual {
            *s = UpState::Checking;
        }
    }
    ctx.request_repaint();
    thread::spawn(move || {
        let next = match fetch_latest() {
            Ok(r) if is_newer(&r.version, CURRENT) => UpState::Available(r),
            Ok(_) if manual => UpState::UpToDate,
            Err(e) if manual => UpState::Failed(format!("{e:#}")),
            _ => UpState::Idle,
        };
        let mut s = state.lock().unwrap();
        if !s.busy() {
            *s = next;
        }
        drop(s);
        ctx.request_repaint();
    });
}

// ---------------------------------------------------------------- Installation

fn download(rel: &Release, dest: &PathBuf, state: &Mutex<UpState>, ctx: &eframe::egui::Context) -> Result<()> {
    let resp = agent().get(&rel.url).call().with_context(|| format!("Download von {}", rel.url))?;
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|v| v.parse().ok())
        .filter(|&t| t > 0)
        .unwrap_or(rel.size);
    let mut reader = resp.into_reader();
    let mut out = File::create(dest).context("Datei neben der Programmdatei nicht anlegbar (Schreibrechte?)")?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut done: u64 = 0;
    let mut last = Instant::now();
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        done += n as u64;
        if last.elapsed() > Duration::from_millis(100) {
            set(state, ctx, UpState::Downloading { done, total });
            last = Instant::now();
        }
    }
    out.flush()?;
    if total > 0 && done < total {
        bail!("Download unvollstaendig");
    }
    Ok(())
}

/// Grober Plausibilitaetstest: Groesse wie angekuendigt und Windows-Programmdatei ("MZ").
fn verify(path: &PathBuf, expected: u64) -> Result<()> {
    let len = fs::metadata(path)?.len();
    if len < 100_000 || (expected > 0 && len != expected) {
        bail!("Die heruntergeladene Datei ist unvollstaendig oder fehlerhaft");
    }
    let mut head = [0u8; 2];
    File::open(path)?.read_exact(&mut head)?;
    if &head != b"MZ" {
        bail!("Die heruntergeladene Datei ist keine Programmdatei");
    }
    Ok(())
}

/// Laedt die Version herunter und ersetzt die laufende exe. Danach steht der Status auf
/// `ReadyToRestart`. Wurde nichts ersetzt, bleibt die alte exe unveraendert.
pub fn install(state: Arc<Mutex<UpState>>, ctx: eframe::egui::Context, rel: Release) {
    thread::spawn(move || {
        let (exe, new, old) = (exe(), side_file("new"), side_file("old"));
        let res = (|| -> Result<()> {
            set(&state, &ctx, UpState::Downloading { done: 0, total: rel.size });
            download(&rel, &new, &state, &ctx)?;
            verify(&new, rel.size)?;
            set(&state, &ctx, UpState::Installing);
            let _ = fs::remove_file(&old);
            fs::rename(&exe, &old).context("Die Programmdatei kann nicht ersetzt werden (Schreibrechte?)")?;
            if let Err(e) = fs::rename(&new, &exe) {
                let _ = fs::rename(&old, &exe); // zurueckrollen
                return Err(anyhow!(e).context("Die neue Version konnte nicht eingesetzt werden"));
            }
            Ok(())
        })();
        match res {
            Ok(()) => set(&state, &ctx, UpState::ReadyToRestart),
            Err(e) => {
                let _ = fs::remove_file(&new);
                set(&state, &ctx, UpState::Failed(format!("{e:#}")));
            }
        }
    });
}

/// Startet die (neue) exe. Der Aufrufer beendet danach die App. Die App laeuft mit
/// Administratorrechten, der Kindprozess erbt sie, es gibt also keine neue UAC-Abfrage.
pub fn restart() -> Result<()> {
    let exe = exe();
    let mut cmd = Command::new(&exe);
    if let Some(dir) = exe.parent() {
        cmd.current_dir(dir);
    }
    cmd.spawn().context("Neustart fehlgeschlagen, bitte die App von Hand starten")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version("0.3.0"), Some([0, 3, 0]));
        assert_eq!(parse_version("v1.2"), Some([1, 2, 0]));
        assert_eq!(parse_version("V10.20.30-beta.1+x"), Some([10, 20, 30]));
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("abc"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn compares_numerically() {
        assert!(is_newer("0.4.0", "0.3.0"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.4.0", "0.4.0"));
        assert!(!is_newer("0.3.9", "0.4.0"));
        assert!(!is_newer("unsinn", "0.4.0"));
    }
}
