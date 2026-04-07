use serde::Serialize;
use tokio::process::Command;

use crate::error::DokkiError;

#[derive(Debug, Clone, Serialize)]
pub struct Device {
    pub serial: String,
    pub status: String,
    pub model: Option<String>,
}

/// Runs `adb devices -l` and parses the output into a list of devices.
pub async fn list_devices() -> Result<Vec<Device>, DokkiError> {
    let output = Command::new("adb")
        .args(["devices", "-l"])
        .output()
        .await
        .map_err(|_| DokkiError::AdbNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(DokkiError::AdbCommandFailed(stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let devices = parse_adb_output(&stdout);
    Ok(devices)
}

/// Parses the output of `adb devices -l`.
///
/// Example line:
/// `R5CT72XXXXX          device usb:1234 product:xxx model:Galaxy_S21 device:xxx transport_id:1`
fn parse_adb_output(output: &str) -> Vec<Device> {
    output
        .lines()
        .skip(1) // skip "List of devices attached"
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            // Split on whitespace: first token is serial, second is status
            let mut parts = line.split_whitespace();
            let serial = parts.next()?.to_string();
            let status = parts.next()?.to_string();

            // Extract model from remaining key:value pairs
            let model = parts
                .find(|p| p.starts_with("model:"))
                .map(|p| p.trim_start_matches("model:").replace('_', " "));

            Some(Device {
                serial,
                status,
                model,
            })
        })
        .collect()
}

/// List installed packages on a device matching a filter string.
/// Runs `adb -s <serial> shell pm list packages <filter>` and parses results.
pub async fn list_packages(serial: &str, filter: &str) -> Result<Vec<String>, DokkiError> {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "pm", "list", "packages"])
        .output()
        .await
        .map_err(|_| DokkiError::AdbNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(DokkiError::AdbCommandFailed(stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let filter_lower = filter.to_lowercase();

    let packages: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            let pkg = line.trim().strip_prefix("package:")?;
            if pkg.to_lowercase().contains(&filter_lower) {
                Some(pkg.to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_adb_output_with_devices() {
        let output = "\
List of devices attached
R5CT72XXXXX          device usb:1-1 product:beyond1 model:Galaxy_S21 device:beyond1 transport_id:1
192.168.1.100:5555   device product:raven model:Pixel_6_Pro device:raven transport_id:2

";
        let devices = parse_adb_output(output);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].serial, "R5CT72XXXXX");
        assert_eq!(devices[0].status, "device");
        assert_eq!(devices[0].model, Some("Galaxy S21".to_string()));
        assert_eq!(devices[1].serial, "192.168.1.100:5555");
        assert_eq!(devices[1].model, Some("Pixel 6 Pro".to_string()));
    }

    #[test]
    fn test_parse_adb_output_empty() {
        let output = "List of devices attached\n\n";
        let devices = parse_adb_output(output);
        assert_eq!(devices.len(), 0);
    }

    #[test]
    fn test_parse_adb_output_unauthorized() {
        let output = "\
List of devices attached
R5CT72XXXXX          unauthorized usb:1-1 transport_id:1

";
        let devices = parse_adb_output(output);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].status, "unauthorized");
        assert_eq!(devices[0].model, None);
    }
}
