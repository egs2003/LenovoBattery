<#
.SYNOPSIS
    Steuert den Lenovo Battery Charge Mode ueber den EnergyDrv Treiber.

.DESCRIPTION
    Sendet IOCTL-Befehle direkt an \\.\EnergyDrv (genau wie LenovoLegionToolkit).
    Die Registry allein reicht NICHT – die Hardware wird ueber DeviceIoControl gesteuert.

    MUSS als Administrator ausgefuehrt werden!

.PARAMETER Mode
    Normal    – normales Laden (kein Limit)
    Storage   – Conservation Mode (max. 80%)

.PARAMETER Status
    Zeigt den aktuellen Modus an.

.EXAMPLE
    .\Set-LegionBatteryMode.ps1 -Mode Storage   # Conservation ein
    .\Set-LegionBatteryMode.ps1 -Mode Normal    # Conservation aus
    .\Set-LegionBatteryMode.ps1 -Status          # Status anzeigen
#>

param(
    [ValidateSet('Normal','Storage')]
    [string]$Mode,
    [switch]$Status
)

# --- Admin-Check ---
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "FEHLER: Dieses Script muss als Administrator ausgefuehrt werden!" -ForegroundColor Red
    exit 1
}

# --- P/Invoke Definitionen ---
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

public static class EnergyDriver
{
    // IOCTL code for battery charge mode
    public const uint IOCTL_ENERGY_BATTERY_CHARGE_MODE = 0x831020F8;

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Auto)]
    public static extern SafeFileHandle CreateFile(
        string lpFileName,
        uint dwDesiredAccess,
        uint dwShareMode,
        IntPtr lpSecurityAttributes,
        uint dwCreationDisposition,
        uint dwFlagsAndAttributes,
        IntPtr hTemplateFile);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool DeviceIoControl(
        SafeFileHandle hDevice,
        uint dwIoControlCode,
        ref uint lpInBuffer,
        uint nInBufferSize,
        ref uint lpOutBuffer,
        uint nOutBufferSize,
        ref uint lpBytesReturned,
        IntPtr lpOverlapped);

    public static SafeFileHandle Open()
    {
        // FILE_READ_DATA | FILE_WRITE_DATA = 0x01 | 0x02
        // FILE_SHARE_READ | FILE_SHARE_WRITE = 0x01 | 0x02
        // OPEN_EXISTING = 3
        // FILE_ATTRIBUTE_NORMAL = 0x80
        var handle = CreateFile(
            @"\\.\EnergyDrv",
            0x03,       // GENERIC_READ | GENERIC_WRITE
            0x03,       // FILE_SHARE_READ | FILE_SHARE_WRITE
            IntPtr.Zero,
            3,          // OPEN_EXISTING
            0x80,       // FILE_ATTRIBUTE_NORMAL
            IntPtr.Zero);

        if (handle.IsInvalid)
        {
            int err = Marshal.GetLastWin32Error();
            throw new InvalidOperationException(
                "Kann EnergyDrv nicht oeffnen (Fehler " + err + "). " +
                "Ist der Lenovo Energy Driver installiert?");
        }
        return handle;
    }

    public static uint SendCommand(SafeFileHandle handle, uint inBuffer)
    {
        uint outBuffer = 0;
        uint bytesReturned = 0;

        bool ok = DeviceIoControl(
            handle,
            IOCTL_ENERGY_BATTERY_CHARGE_MODE,
            ref inBuffer,
            (uint)sizeof(uint),
            ref outBuffer,
            (uint)sizeof(uint),
            ref bytesReturned,
            IntPtr.Zero);

        if (!ok)
        {
            int err = Marshal.GetLastWin32Error();
            throw new InvalidOperationException(
                "DeviceIoControl fehlgeschlagen (Fehler " + err + ").");
        }
        return outBuffer;
    }
}
"@ -ReferencedAssemblies @() -ErrorAction Stop

# --- Treiber oeffnen ---
try {
    $handle = [EnergyDriver]::Open()
    Write-Host "EnergyDrv Treiber geoeffnet." -ForegroundColor Green
} catch {
    Write-Host "FEHLER: $_" -ForegroundColor Red
    exit 1
}

# --- Hilfsfunktionen ---
function Get-BatteryMode {
    # GetInBufferValue = 0xFF
    $raw = [EnergyDriver]::SendCommand($handle, 0xFF)

    # Bit 0x20 = Conservation, Bit 0x04 = RapidCharge
    if (($raw -band 0x20) -ne 0) { return 'Storage (Conservation - max. 80%)' }
    if (($raw -band 0x04) -ne 0) { return 'RapidCharge (Schnellladen)' }
    return 'Normal (kein Limit)'
}

function Set-BatteryMode {
    param([string]$target)

    switch ($target) {
        'Storage' {
            # Conservation: sende 0x08, dann 0x03
            [EnergyDriver]::SendCommand($handle, 0x08) | Out-Null
            [EnergyDriver]::SendCommand($handle, 0x03) | Out-Null
        }
        'Normal' {
            # Normal: sende 0x05, dann 0x08
            [EnergyDriver]::SendCommand($handle, 0x05) | Out-Null
            [EnergyDriver]::SendCommand($handle, 0x08) | Out-Null
        }
    }

    # Registry-Wert ebenfalls setzen (damit Vantage synchron bleibt)
    $regPath = 'HKCU:\Software\Lenovo\VantageService\AddinData\IdeaNotebookAddin'
    $regKey  = 'BatteryChargeMode'
    Set-ItemProperty -Path $regPath -Name $regKey -Value $target -ErrorAction SilentlyContinue
}

# --- Hauptlogik ---
if ($Status -or (-not $Mode)) {
    $current = Get-BatteryMode
    Write-Host "Aktueller Modus: $current" -ForegroundColor Cyan
    exit 0
}

Write-Host "Setze Modus auf: $Mode ..." -ForegroundColor Yellow
Set-BatteryMode -target $Mode

# Verifizieren
Start-Sleep -Milliseconds 200
$verify = Get-BatteryMode
Write-Host "Aktueller Modus: $verify" -ForegroundColor Cyan

$handle.Close()
Write-Host "Fertig." -ForegroundColor Green
