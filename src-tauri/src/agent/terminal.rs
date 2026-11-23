//! Opening a visible terminal that runs a vendor's own install or sign-in
//! command, so the user sees it and can answer whatever it asks.

use std::ffi::OsString;

/// Quote for PowerShell: single quotes, with embedded ones doubled.
pub(crate) fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

pub(crate) fn open(title: &str, powershell: &str, path: &OsString) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // `start` gives the command its own console; the inner command keeps only single quotes so cmd leaves it alone.
    let line = format!(
        "/c start \"{}\" powershell.exe -NoExit -NoProfile -ExecutionPolicy Bypass -Command \"{}\"",
        title.replace('"', ""),
        powershell.replace('"', "'")
    );
    std::process::Command::new("cmd.exe")
        .raw_arg(line)
        .env("PATH", path)
        .creation_flags(0x0800_0000)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open a terminal: {e}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn quoting_survives_embedded_quotes() {
        assert_eq!(super::ps_quote("C:\\it's\\claude.exe"), "'C:\\it''s\\claude.exe'");
    }
}
