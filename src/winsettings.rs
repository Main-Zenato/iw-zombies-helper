//! Windows graphics settings specific to the game (Settings > System > Display > Graphics).
//!
//! Two of them matter for Infinite Warfare: forcing the dedicated graphics card, and turning off the
//! "optimizations for windowed games", a known cause of the game crashing on Windows 11.
//! Windows keeps them in the user's registry, one value per executable.

use std::io;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ};

const KEY: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
const GPU_PREFERENCE: &str = "GpuPreference";
const HIGH_PERFORMANCE: &str = "2";
const WINDOWED_OPTIMIZATIONS: &str = "SwapEffectUpgradeEnable";
const DISABLED: &str = "0";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphicsSettings {
    /// The game is pinned to the "high performance" graphics card.
    pub dedicated_gpu: bool,
    /// Optimizations for windowed games are off for this game.
    pub windowed_optimizations_off: bool,
}

impl GraphicsSettings {
    pub fn is_recommended(&self) -> bool {
        self.dedicated_gpu && self.windowed_optimizations_off
    }
}

/// Usual location of the game under Steam, if it is there.
pub fn default_game_exe() -> Option<PathBuf> {
    let path = PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Call of Duty - Infinite Warfare\iw7_ship.exe");
    path.is_file().then_some(path)
}

pub fn read(exe: &str) -> GraphicsSettings {
    parse(&read_raw(exe).unwrap_or_default())
}

/// Applies the two recommended settings while keeping the other options already present.
pub fn apply_recommended(exe: &str) -> io::Result<()> {
    let merged = merge_recommended(&read_raw(exe).unwrap_or_default());
    let (key, name, data) = (wide(KEY), wide(exe), wide(&merged));
    let size = (data.len() * 2) as u32;
    let status = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), REG_SZ, data.as_ptr().cast(), size) };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

fn read_raw(exe: &str) -> Option<String> {
    let (key, name) = (wide(KEY), wide(exe));
    let mut buffer = [0u16; 512];
    let mut size = (buffer.len() * 2) as u32;
    let status = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buffer.as_mut_ptr().cast(), &mut size)
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]))
}

/// Shades of the accent color chosen in Windows, from lightest to darkest (the fourth is the color
/// itself). `None` if they cannot be read.
pub fn accent_shades() -> Option<[[u8; 3]; 7]> {
    let (key, name) = (wide(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent"), wide("AccentPalette"));
    // Eight four-byte colors (red, green, blue, reserved), from lightest to darkest.
    let mut palette = [0u8; 32];
    let mut size = palette.len() as u32;
    let status = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_BINARY, std::ptr::null_mut(), palette.as_mut_ptr().cast(), &mut size)
    };
    (status == ERROR_SUCCESS && size as usize == palette.len()).then(|| shades(&palette))
}

/// The palette's seven shades; the eighth color is a complementary color, unused here.
fn shades(palette: &[u8; 32]) -> [[u8; 3]; 7] {
    std::array::from_fn(|shade| [palette[shade * 4], palette[shade * 4 + 1], palette[shade * 4 + 2]])
}

/// The value is a sequence of `Name=value;`.
fn pairs(raw: &str) -> Vec<(&str, &str)> {
    raw.split(';').filter_map(|pair| pair.split_once('=')).map(|(k, v)| (k.trim(), v.trim())).collect()
}

fn parse(raw: &str) -> GraphicsSettings {
    let has = |name: &str, value: &str| pairs(raw).iter().any(|&(k, v)| k == name && v == value);
    GraphicsSettings {
        dedicated_gpu: has(GPU_PREFERENCE, HIGH_PERFORMANCE),
        windowed_optimizations_off: has(WINDOWED_OPTIMIZATIONS, DISABLED),
    }
}

fn merge_recommended(raw: &str) -> String {
    let mut out: Vec<(&str, &str)> =
        pairs(raw).into_iter().filter(|(k, _)| *k != GPU_PREFERENCE && *k != WINDOWED_OPTIMIZATIONS).collect();
    out.push((GPU_PREFERENCE, HIGH_PERFORMANCE));
    out.push((WINDOWED_OPTIMIZATIONS, DISABLED));
    out.iter().map(|(k, v)| format!("{k}={v};")).collect()
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_two_settings_from_a_registry_value() {
        assert_eq!(parse(""), GraphicsSettings::default());
        let only_windowed = parse("SwapEffectUpgradeEnable=0;");
        assert!(only_windowed.windowed_optimizations_off && !only_windowed.dedicated_gpu);
        assert!(!only_windowed.is_recommended());
        assert!(parse("GpuPreference=2;SwapEffectUpgradeEnable=0;").is_recommended());
        // 1 = power saving (integrated card), which is not the wanted setting.
        assert!(!parse("GpuPreference=1;").dedicated_gpu);
    }

    #[test]
    fn merging_keeps_unrelated_options_and_replaces_ours() {
        assert_eq!(merge_recommended(""), "GpuPreference=2;SwapEffectUpgradeEnable=0;");
        assert_eq!(
            merge_recommended("AppStatus=0;GpuPreference=1;SwapEffectUpgradeEnable=1;"),
            "AppStatus=0;GpuPreference=2;SwapEffectUpgradeEnable=0;"
        );
        assert!(parse(&merge_recommended("SwapEffectUpgradeEnable=0;")).is_recommended());
    }

    #[test]
    fn accent_shades_are_read_in_palette_order() {
        let mut palette = [0u8; 32];
        palette[4..8].copy_from_slice(&[0x60, 0xCD, 0xFF, 0x00]);
        palette[28..32].copy_from_slice(&[0x88, 0x17, 0x98, 0x00]);
        let shades = shades(&palette);
        assert_eq!(shades[1], [0x60, 0xCD, 0xFF]);
        assert_eq!(shades[6], [0, 0, 0]);
    }

    #[test]
    fn unknown_executable_reads_as_not_configured() {
        assert_eq!(read(r"C:\dossier\inexistant\jeu-qui-n-existe-pas.exe"), GraphicsSettings::default());
    }
}
