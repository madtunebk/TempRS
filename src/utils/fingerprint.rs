use base64::Engine;
use sha2::{Digest, Sha256};

#[cfg(target_os = "linux")]
fn get_machine_key() -> Option<String> {
    let machine_id = std::fs::read_to_string("/etc/machine-id")
        .ok()
        .map(|s| s.trim().to_string())?;

    let cpu = cpu_signature();
    Some(format!("{}{}", machine_id, cpu))
}

#[cfg(target_os = "linux")]
fn cpu_signature() -> String {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

    let mut vendor = String::new();
    let mut model = String::new();

    for line in cpuinfo.lines() {
        if vendor.is_empty() && line.starts_with("vendor_id") {
            vendor = line.to_string();
        }
        if model.is_empty() && line.starts_with("model name") {
            model = line.to_string();
        }
        if !vendor.is_empty() && !model.is_empty() {
            break;
        }
    }

    format!("{}{}", vendor, model)
}

#[cfg(target_os = "windows")]
fn get_machine_key() -> Option<String> {
    use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

    // Main machine GUID
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SOFTWARE\\Microsoft\\Cryptography")
        .ok()?;

    let guid: String = key.get_value("MachineGuid").ok()?;

    let cpu = cpu_signature();

    Some(format!("{}{}", guid, cpu))
}

#[cfg(target_os = "windows")]
fn cpu_signature() -> String {
    use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    if let Ok(cpu0) = hklm.open_subkey("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0") {
        let mut sig = String::new();

        let vendor: Result<String, _> = cpu0.get_value("VendorIdentifier");
        if let Ok(v) = vendor {
            sig.push_str(&v);
        }

        let name: Result<String, _> = cpu0.get_value("ProcessorNameString");
        if let Ok(n) = name {
            sig.push_str(&n);
        }

        return sig;
    }
    "".into()
}

#[cfg(target_os = "android")]
fn get_machine_key() -> Option<String> {
    // Injected automatically by cargo-apk
    let pkg = std::env::var("CARGO_APK_PACKAGE_NAME").unwrap_or_default();

    let cpu = cpu_signature();
    Some(format!("{}{}", pkg, cpu))
}

#[cfg(target_os = "android")]
fn cpu_signature() -> String {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

    let mut hardware = String::new();
    let mut processor = String::new();
    let mut arch = String::new();

    for line in cpuinfo.lines() {
        if hardware.is_empty() && line.starts_with("Hardware") {
            hardware = line.to_string();
        }
        if processor.is_empty() && line.starts_with("Processor") {
            processor = line.to_string();
        }
        if processor.is_empty() && line.starts_with("model name") {
            processor = line.to_string();
        }
        if arch.is_empty() && line.starts_with("CPU architecture") {
            arch = line.to_string();
        }
        if !hardware.is_empty() && !processor.is_empty() && !arch.is_empty() {
            break;
        }
    }
    format!("{}{}{}", hardware, processor, arch)
}

pub fn fingerprint() -> String {
    let key = get_machine_key().unwrap_or_else(|| "fallback".into());
    let hash = Sha256::digest(key.as_bytes());
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(hash)
}
