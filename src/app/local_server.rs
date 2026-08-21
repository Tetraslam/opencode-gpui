use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    time::Duration,
};

pub(super) const DEFAULT_URL: &str = "http://127.0.0.1:4096";

pub(super) struct ManagedServer {
    child: Option<Child>,
}

impl Drop for ManagedServer {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let _ = child.kill();
        let _ = std::thread::Builder::new()
            .name("opencode-server-shutdown".into())
            .spawn(move || {
                let _ = child.wait();
            });
    }
}

impl ManagedServer {
    pub(super) fn exit_status(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        self.child
            .as_mut()
            .expect("managed server owns a child")
            .try_wait()
            .map_err(|error| format!("could not inspect local opencode server: {error}"))
    }
}

pub(super) fn autostart_enabled(explicit_url: bool, setting: Option<&str>) -> bool {
    let disabled = setting.is_some_and(|setting| {
        matches!(
            setting.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "no" | "off"
        )
    });
    !explicit_url && !disabled
}

pub(super) fn ensure_running() -> Result<Option<ManagedServer>, String> {
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4096);
    if TcpStream::connect_timeout(&address, Duration::from_millis(150)).is_ok() {
        return Ok(None);
    }
    let binary = std::env::var_os("OPENCODE_BIN").unwrap_or_else(|| "opencode".into());
    let child = Command::new(&binary)
        .args([
            "serve",
            "--hostname",
            "127.0.0.1",
            "--port",
            "4096",
            "--log-level",
            "WARN",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| spawn_error(&binary.to_string_lossy(), &error))?;
    Ok(Some(ManagedServer { child: Some(child) }))
}

fn spawn_error(binary: &str, error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::NotFound {
        format!("opencode binary not found ({binary}); install it or set OPENCODE_BIN")
    } else {
        format!("could not start local opencode server with {binary}: {error}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autostart_only_applies_to_the_implicit_default() {
        assert!(autostart_enabled(false, None));
        assert!(autostart_enabled(false, Some("true")));
        assert!(!autostart_enabled(true, None));
        assert!(!autostart_enabled(false, Some("0")));
        assert!(!autostart_enabled(false, Some("false")));
        assert!(!autostart_enabled(false, Some(" OFF ")));
    }
}
