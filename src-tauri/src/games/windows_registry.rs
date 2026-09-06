// Le registre de desinstallation : chaque programme installe y depose son `InstallLocation`.
//
// Les deux vues, 32 et 64 bits, sont lues -- Battle.net s'inscrit dans la premiere.

use std::path::PathBuf;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW,
    RegQueryValueExW,
};

const UNINSTALL: [&str; 2] = [
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
];

/// 255 est la longueur maximale d'un nom de cle, plus le zero final.
const MAX_KEY_NAME: usize = 256;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

unsafe fn open(parent: HKEY, sub: &str) -> Option<HKEY> {
    let mut key: HKEY = std::ptr::null_mut();
    let code = unsafe { RegOpenKeyExW(parent, wide(sub).as_ptr(), 0, KEY_READ, &mut key) };
    (code == ERROR_SUCCESS).then_some(key)
}

unsafe fn string_value(key: HKEY, name: &str) -> Option<String> {
    let name = wide(name);
    let mut size: u32 = 0;
    let code = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if code != ERROR_SUCCESS || size == 0 {
        return None;
    }
    let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
    let code = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut u8,
            &mut size,
        )
    };
    if code != ERROR_SUCCESS {
        return None;
    }
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..end]))
}

pub fn install_locations() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in UNINSTALL {
        unsafe {
            let Some(parent) = open(HKEY_LOCAL_MACHINE, root) else { continue };
            let mut index = 0u32;
            loop {
                let mut name = [0u16; MAX_KEY_NAME];
                let mut length = name.len() as u32;
                let code = RegEnumKeyExW(
                    parent,
                    index,
                    name.as_mut_ptr(),
                    &mut length,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if code != ERROR_SUCCESS {
                    break;
                }
                index += 1;

                let name = String::from_utf16_lossy(&name[..length as usize]);
                if let Some(child) = open(parent, &name) {
                    if let Some(location) = string_value(child, "InstallLocation") {
                        let location = location.trim().trim_end_matches('\\');
                        if !location.is_empty() {
                            found.push(PathBuf::from(location));
                        }
                    }
                    RegCloseKey(child);
                }
            }
            RegCloseKey(parent);
        }
    }
    found
}
