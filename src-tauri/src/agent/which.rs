//! Finding the vendor CLIs the assistant drives, and starting them.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A vendor CLI and the arguments/environment needed to start it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Launch {
    pub program: PathBuf,
    /// Arguments before the subcommand, including the script for npm installs run through Node.
    pub prefix: Vec<OsString>,
    pub env: Vec<(String, OsString)>,
    pub remove: Vec<&'static str>,
    /// A temporary folder that belongs to this launch, removed with its session.
    pub scratch: Option<PathBuf>,
}

impl Launch {
    pub fn display(&self) -> String {
        match self.prefix.first().filter(|p| !p.to_string_lossy().starts_with('-')) {
            Some(script) => Path::new(script).display().to_string(),
            None => self.program.display().to_string(),
        }
    }

    pub fn command(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.program);
        for key in &self.remove {
            cmd.env_remove(key);
        }
        cmd.args(&self.prefix)
            .env("PATH", search_path())
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }

    pub async fn output(&self, args: &[&str], timeout: Duration) -> Result<(String, String, bool), String> {
        let mut cmd = self.command();
        cmd.args(args);
        let out = tokio::time::timeout(timeout, cmd.output())
            .await
            .map_err(|_| format!("{} {} did not answer within {}s", self.display(), args.join(" "), timeout.as_secs()))?
            .map_err(|e| format!("{}: {e}", self.display()))?;
        Ok((
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.success(),
        ))
    }
}

/// Ends a child's whole process tree when dropped. Agents start helpers of
/// their own (Antigravity's unpacker, npm's node wrapper) that outlive a plain kill.
pub(crate) struct Tree {
    job: windows_sys::Win32::Foundation::HANDLE,
}

// SAFETY: the job handle is owned by this value alone and closed once, in Drop.
unsafe impl Send for Tree {}
unsafe impl Sync for Tree {}

impl Tree {
    pub fn attach(child: &tokio::process::Child) -> Option<Self> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };
        let process = child.raw_handle()?;
        // SAFETY: plain Win32 calls on a job we create here and the live child's
        // handle; `info` outlives the call that reads it.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return None;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let limited = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) != 0;
            if !limited || AssignProcessToJobObject(job, process.cast()) == 0 {
                CloseHandle(job);
                return None;
            }
            Some(Self { job })
        }
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        // SAFETY: closing the job we own kills every process in it.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.job);
        }
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

/// Where installers put these CLIs when the app's own PATH does not say.
fn known_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    if let Some(h) = home() {
        dirs.push(h.join(".local").join("bin"));
        dirs.push(h.join(".bun").join("bin"));
        dirs.push(h.join(".volta").join("bin"));
        dirs.push(h.join(".claude").join("local"));
        dirs.push(h.join(".cargo").join("bin"));
        dirs.push(h.join(".grok").join("bin"));
        dirs.push(h.join(".kimi-code").join("bin"));
        dirs.push(h.join("scoop").join("shims"));
    }
    if let Some(a) = env("APPDATA") {
        dirs.push(a.join("npm"));
    }
    if let Some(l) = env("LOCALAPPDATA") {
        dirs.push(l.join("pnpm"));
        dirs.push(l.join("Volta").join("bin"));
        dirs.push(l.join("Programs").join("nodejs"));
        dirs.push(l.join("Programs").join("OpenAI").join("Codex").join("bin"));
        dirs.push(l.join("cursor-agent"));
    }
    if let Some(p) = env("ProgramFiles") {
        dirs.push(p.join("nodejs"));
    }
    dirs
}

/// The app's own PATH is fixed at launch, so a CLI installed since then is only
/// on the PATH Windows keeps in the registry.
fn registry_path() -> Vec<PathBuf> {
    use std::os::windows::process::CommandExt;
    let expand = |s: &str| {
        let re = regex::Regex::new(r"%([^%]+)%").expect("valid regex");
        re.replace_all(s, |c: &regex::Captures| std::env::var(&c[1]).unwrap_or_else(|_| c[0].to_string())).into_owned()
    };
    let mut out = Vec::new();
    for key in [r"HKCU\Environment", r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment"] {
        let Ok(o) = std::process::Command::new("reg").args(["query", key, "/v", "Path"]).creation_flags(0x0800_0000).output() else {
            continue;
        };
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let line = line.trim_start();
            if line.len() < 4 || !line[..4].eq_ignore_ascii_case("path") {
                continue;
            }
            let value = line[4..].trim_start().split_once(char::is_whitespace).map(|(_, v)| v.trim()).unwrap_or("");
            out.extend(value.split(';').filter(|s| !s.is_empty()).map(|s| PathBuf::from(expand(s))));
        }
    }
    out
}

static PATH: std::sync::RwLock<Option<OsString>> = std::sync::RwLock::new(None);

pub(crate) fn search_path() -> OsString {
    if let Some(p) = PATH.read().unwrap().clone() {
        return p;
    }
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    dirs.extend(registry_path());
    dirs.extend(known_dirs());
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| !d.as_os_str().is_empty() && seen.insert(d.clone()));
    let joined = std::env::join_paths(dirs).unwrap_or_default();
    *PATH.write().unwrap() = Some(joined.clone());
    joined
}

/// Forget the search path so the next lookup sees CLIs installed meanwhile.
pub(crate) fn refresh_search_path() {
    *PATH.write().unwrap() = None;
}

fn node() -> Option<PathBuf> {
    std::env::split_paths(&search_path()).map(|d| d.join("node.exe")).find(|p| p.is_file())
}

/// The npm package's `codex.js` only starts this binary; running it directly saves a node process.
fn native_codex(script: &Path) -> Option<PathBuf> {
    let pkg = script.parent()?.parent()?;
    let (arch, triple) = if cfg!(target_arch = "aarch64") { ("arm64", "aarch64-pc-windows-msvc") } else { ("x64", "x86_64-pc-windows-msvc") };
    let platform = format!("codex-win32-{arch}");
    [pkg.join("node_modules").join("@openai").join(&platform), pkg.parent()?.join(&platform), pkg.to_path_buf()]
        .into_iter()
        .map(|root| root.join("vendor").join(triple).join("bin").join("codex.exe"))
        .find(|p| p.is_file())
}

/// Cursor's installer puts a `.cmd` beside `versions/<date>-<commit>/` holding
/// its own node.exe and index.js; the shim runs the newest through PowerShell.
fn versioned_node(shim: &Path) -> Option<Launch> {
    let dir = shim.parent()?;
    let key = |name: &str| -> Option<(u32, u32, u32)> {
        let mut parts = name.split('-').next()?.split('.').map(|p| p.parse::<u32>().ok());
        Some((parts.next()??, parts.next()??, parts.next()??))
    };
    let newest = std::fs::read_dir(dir.join("versions"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("node.exe").is_file() && p.join("index.js").is_file())
        .max_by_key(|p| p.file_name().and_then(|n| n.to_str()).and_then(key))?;
    Some(Launch {
        program: newest.join("node.exe"),
        prefix: vec![newest.join("index.js").into_os_string()],
        env: vec![("CURSOR_INVOKED_AS".into(), shim.file_name()?.to_os_string())],
        ..Default::default()
    })
}

/// cmd.exe cannot carry these arguments, so follow an npm or pnpm `.cmd` shim to its target.
fn follow_shim(shim: &Path) -> Option<Launch> {
    let text = std::fs::read_to_string(shim).ok()?;
    let dir = shim.parent()?;
    let re = regex::Regex::new(r#"(?i)"%(?:~dp0|dp0%)\\?([^"%]+?\.(?:exe|js|cjs|mjs))""#).ok()?;
    for cap in re.captures_iter(&text) {
        let target: PathBuf = dir.join(&cap[1]).components().fold(PathBuf::new(), |mut p, c| {
            if c == std::path::Component::ParentDir {
                p.pop();
            } else {
                p.push(c);
            }
            p
        });
        if !target.is_file() {
            continue;
        }
        let is_exe = target.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"));
        if is_exe && target.file_stem().is_some_and(|s| s.eq_ignore_ascii_case("node")) {
            continue;
        }
        if is_exe {
            return Some(Launch { program: target, ..Default::default() });
        }
        if let Some(exe) = native_codex(&target) {
            return Some(Launch { program: exe, ..Default::default() });
        }
        return Some(Launch { program: node()?, prefix: vec![target.into_os_string()], ..Default::default() });
    }
    None
}

pub(crate) fn find(name: &str, configured: Option<&str>) -> Option<Launch> {
    if let Some(p) = configured.map(str::trim).filter(|p| !p.is_empty()) {
        let p = PathBuf::from(p);
        if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat")) {
            return follow_shim(&p);
        }
        return p.is_file().then(|| Launch { program: p, ..Default::default() });
    }
    for dir in std::env::split_paths(&search_path()) {
        let exe = dir.join(format!("{name}.exe"));
        if exe.is_file() {
            return Some(Launch { program: exe, ..Default::default() });
        }
        let shim = dir.join(format!("{name}.cmd"));
        if shim.is_file() {
            if let Some(l) = follow_shim(&shim).or_else(|| versioned_node(&shim)) {
                return Some(l);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn npm_shim_resolves_to_its_script() {
        let dir = std::env::temp_dir().join(format!("pobforge-shim-{}", std::process::id()));
        let pkg = dir.join("node_modules").join("@openai").join("codex").join("bin");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(pkg.join("codex.js"), "").unwrap();
        let shim = dir.join("codex.cmd");
        std::fs::write(
            &shim,
            "@ECHO off\r\nIF EXIST \"%dp0%\\node.exe\" (\r\n  SET \"_prog=%dp0%\\node.exe\"\r\n)\r\n\"%_prog%\"  \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*\r\n",
        )
        .unwrap();
        let launch = super::follow_shim(&shim);
        let _ = std::fs::remove_dir_all(&dir);
        if super::node().is_none() {
            return;
        }
        let launch = launch.expect("shim resolves");
        assert!(launch.display().ends_with("codex.js"), "{}", launch.display());
    }

    #[test]
    fn npm_codex_prefers_the_native_binary() {
        let dir = std::env::temp_dir().join(format!("pobforge-codex-{}", std::process::id()));
        let openai = dir.join("node_modules").join("@openai");
        let script = openai.join("codex").join("bin").join("codex.js");
        let triple = if cfg!(target_arch = "aarch64") { ("arm64", "aarch64-pc-windows-msvc") } else { ("x64", "x86_64-pc-windows-msvc") };
        let exe = openai.join(format!("codex-win32-{}", triple.0)).join("vendor").join(triple.1).join("bin").join("codex.exe");
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&script, "").unwrap();
        std::fs::write(&exe, "").unwrap();
        let found = super::native_codex(&script);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.as_deref(), Some(exe.as_path()));
    }
}
