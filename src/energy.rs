use std::ffi::c_void;
use std::io::Error;
use std::iter::once;
use std::thread::sleep;
use std::time::Duration;

use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::RegKey;

type Handle = *mut c_void;

const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
const IOCTL_ENERGY_BATTERY_CHARGE_MODE: u32 = 0x831020F8;

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *mut c_void,
        disposition: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    fn DeviceIoControl(
        device: Handle,
        code: u32,
        in_buf: *mut c_void,
        in_size: u32,
        out_buf: *mut c_void,
        out_size: u32,
        returned: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Storage,
    RapidCharge,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Normal => "Normal (kein Limit)",
            Mode::Storage => "Storage (Conservation - max. 80%)",
            Mode::RapidCharge => "RapidCharge (Schnellladen)",
        }
    }
}

pub struct Driver(Handle);

impl Driver {
    pub fn open() -> Result<Self, String> {
        let path: Vec<u16> = r"\\.\EnergyDrv".encode_utf16().chain(once(0)).collect();
        // Zugriffsmaske 0x03, Share 0x03, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL wie im PowerShell-Original
        let handle = unsafe {
            CreateFileW(path.as_ptr(), 0x03, 0x03, std::ptr::null_mut(), 3, 0x80, std::ptr::null_mut())
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(format!(
                "Kann EnergyDrv nicht oeffnen ({}). Ist der Lenovo Energy Driver installiert?",
                Error::last_os_error()
            ));
        }
        Ok(Driver(handle))
    }

    fn send(&self, mut input: u32) -> Result<u32, String> {
        let mut output: u32 = 0;
        let mut returned: u32 = 0;
        let ok = unsafe {
            DeviceIoControl(
                self.0,
                IOCTL_ENERGY_BATTERY_CHARGE_MODE,
                &mut input as *mut u32 as *mut c_void,
                4,
                &mut output as *mut u32 as *mut c_void,
                4,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(format!("DeviceIoControl fehlgeschlagen ({}).", Error::last_os_error()));
        }
        Ok(output)
    }

    pub fn mode(&self) -> Result<Mode, String> {
        // 0xFF = Statusabfrage; Bit 0x20 = Conservation, Bit 0x04 = RapidCharge
        let raw = self.send(0xFF)?;
        Ok(if raw & 0x20 != 0 {
            Mode::Storage
        } else if raw & 0x04 != 0 {
            Mode::RapidCharge
        } else {
            Mode::Normal
        })
    }

    pub fn set_mode(&self, target: Mode) -> Result<Mode, String> {
        let (name, sequence): (&str, [u32; 2]) = match target {
            Mode::Storage => ("Storage", [0x08, 0x03]),
            Mode::Normal => ("Normal", [0x05, 0x08]),
            Mode::RapidCharge => return Err("RapidCharge kann nicht gesetzt werden.".into()),
        };
        for cmd in sequence {
            self.send(cmd)?;
        }

        // Registry nur fuer Lenovo Vantage; Fehler (z.B. fehlender Key) sind egal
        let _ = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(
                r"Software\Lenovo\VantageService\AddinData\IdeaNotebookAddin",
                KEY_SET_VALUE,
            )
            .and_then(|key| key.set_value("BatteryChargeMode", &name));

        sleep(Duration::from_millis(200));
        self.mode()
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
