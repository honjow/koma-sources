//! A local civil clock for sources whose update-day listing uses device time.
//! This does not emulate account state or grant any private-session capability.
use serde_json::{json, Value};

pub(crate) fn response(request: &Value) -> Option<Value> {
    let object = request.as_object()?;
    if object.len() != 3
        || request["type"] != "private_session"
        || request["version"] != 1
        || request["op"] != "clock"
    {
        return None;
    }
    // The development host runs on macOS/Linux. Use the OS civil clock rather
    // than UTC so midnight and daylight-saving boundaries match the app route.
    // No shell is involved and no source-supplied input reaches the command.
    let output = std::process::Command::new("/bin/date")
        .arg("+%Y-%m-%d+%H:%M:%S")
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let gts = std::str::from_utf8(&output.stdout).ok()?.trim();
    if gts.len() != 19
        || !gts.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            10 => b == b'+',
            13 | 16 => b == b':',
            _ => b.is_ascii_digit(),
        })
    {
        return None;
    }
    Some(json!({"ok":true,"gts":gts,"networkPerformed":false}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_clock_is_read_only_and_does_not_accept_session_operations() {
        let clock = response(&json!({"type":"private_session","version":1,"op":"clock"}))
            .expect("OS local clock");
        assert_eq!(clock["networkPerformed"], false);
        assert_eq!(clock["gts"].as_str().unwrap().len(), 19);
        for request in [
            json!({"type":"private_session","version":1,"op":"login"}),
            json!({"type":"private_session","version":2,"op":"clock"}),
            json!({"type":"private_session","version":1,"op":"clock","token":"unused"}),
        ] {
            assert!(response(&request).is_none());
        }
    }
}
