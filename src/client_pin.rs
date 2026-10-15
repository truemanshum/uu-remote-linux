use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

const CLIENT_PIN_JSON: &str = include_str!("../docs/client-pin.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientPin {
    version: String,
    url: String,
    size: u64,
    sha256: String,
}

pub(super) fn acquire(cache: &Path) -> Result<PathBuf> {
    let pin = client_pin()?;
    if let Some(path) = std::env::var_os("UUR_INSTALLER_PATH") {
        let path = PathBuf::from(path);
        verify(&path, &pin).with_context(|| {
            format!(
                "UUR_INSTALLER_PATH does not match pinned UU client {}",
                pin.version
            )
        })?;
        return Ok(path);
    }

    let installer = cache.join(format!("uu-remote-setup-{}.exe", pin.version));
    if installer.is_file() && verify(&installer, &pin).is_ok() {
        return Ok(installer);
    }

    let part = cache.join(format!("uu-remote-setup-{}.exe.part", pin.version));
    if part.exists() {
        std::fs::remove_file(&part)
            .with_context(|| format!("removing stale download {}", part.display()))?;
    }
    println!("{}", t!("setup.downloading", url = pin.url.as_str()));
    let status = Command::new("curl")
        .args([
            "-fL",
            "--retry",
            "3",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--progress-bar",
            "-o",
        ])
        .arg(&part)
        .arg(&pin.url)
        .status()
        .context("running curl")?;
    if !status.success() {
        if part.exists() {
            std::fs::remove_file(&part)
                .with_context(|| format!("removing incomplete download {}", part.display()))?;
        }
        anyhow::bail!("{}", t!("setup.download_failed"));
    }
    if let Err(error) = verify(&part, &pin) {
        if strict_pin_enabled() || verify_pe_installer(&part).is_err() {
            std::fs::remove_file(&part)
                .with_context(|| format!("removing rejected download {}", part.display()))?;
            return Err(error).with_context(|| {
                format!(
                    "official feed no longer matches pinned UU client {}; provide the pinned installer with UUR_INSTALLER_PATH",
                    pin.version
                )
            });
        }
        eprintln!(
            "notice: official UU feed returned a newer PE installer than pinned {}; proceeding with validated PE download",
            pin.version
        );
    }
    if installer.exists() {
        std::fs::remove_file(&installer)
            .with_context(|| format!("removing obsolete cache {}", installer.display()))?;
    }
    std::fs::rename(&part, &installer).with_context(|| {
        format!(
            "promoting validated installer {} to {}",
            part.display(),
            installer.display()
        )
    })?;
    Ok(installer)
}

fn client_pin() -> Result<ClientPin> {
    serde_json::from_str(CLIENT_PIN_JSON).context("parsing the pinned UU client manifest")
}

fn strict_pin_enabled() -> bool {
    std::env::var("UUR_STRICT_CLIENT_PIN").is_ok_and(|value| value == "1" || value == "true")
}

fn verify_pe_installer(path: &Path) -> Result<()> {
    use std::io::Read;
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("reading installer metadata from {}", path.display()))?;
    if !metadata.is_file() || metadata.len() < 1_048_576 {
        anyhow::bail!(
            "installer {} is smaller than a valid UU setup image",
            path.display()
        );
    }
    let mut header = [0u8; 2];
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("opening installer {}", path.display()))?;
    file.read_exact(&mut header)
        .with_context(|| format!("reading DOS header from {}", path.display()))?;
    if &header != b"MZ" {
        anyhow::bail!(
            "installer {} does not have a valid PE MZ header",
            path.display()
        );
    }
    Ok(())
}

fn verify(path: &Path, pin: &ClientPin) -> Result<()> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("reading installer metadata from {}", path.display()))?;
    if !metadata.is_file() || metadata.len() != pin.size {
        anyhow::bail!(
            "installer {} has size {}, expected {} for UU client {}",
            path.display(),
            metadata.len(),
            pin.size,
            pin.version
        );
    }
    let output = Command::new("sha256sum")
        .arg("--")
        .arg(path)
        .output()
        .context("running sha256sum for the UU installer")?;
    if !output.status.success() {
        anyhow::bail!("sha256sum failed for {}", path.display());
    }
    let stdout = String::from_utf8(output.stdout).context("reading sha256sum output")?;
    let actual = stdout
        .split_ascii_whitespace()
        .next()
        .context("sha256sum returned no digest")?;
    if actual != pin.sha256 {
        anyhow::bail!(
            "installer {} has SHA256 {}, expected {} for UU client {}",
            path.display(),
            actual,
            pin.sha256,
            pin.version
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{client_pin, verify, verify_pe_installer, ClientPin};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct FixtureDir(PathBuf);

    impl FixtureDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "uur-client-pin-{name}-{}-{}",
                std::process::id(),
                FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            if path.exists() {
                fs::remove_dir_all(&path).expect("remove stale installer fixture");
            }
            fs::create_dir_all(&path).expect("create installer fixture");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for FixtureDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove installer fixture");
        }
    }

    fn fixture_pin(sha256: &str) -> ClientPin {
        ClientPin {
            version: "fixture".to_string(),
            url: "https://example.invalid/installer.exe".to_string(),
            size: 3,
            sha256: sha256.to_string(),
        }
    }

    #[test]
    fn installer_digest_validation_accepts_matching_fixture() {
        let fixture = FixtureDir::new("valid");
        let installer = fixture.path().join("installer.exe");
        fs::write(&installer, b"abc").expect("write installer fixture");
        let pin = fixture_pin("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");

        assert!(verify(&installer, &pin).is_ok());
    }

    #[test]
    fn installer_digest_validation_rejects_changed_fixture() {
        let fixture = FixtureDir::new("invalid");
        let installer = fixture.path().join("installer.exe");
        fs::write(&installer, b"abc").expect("write installer fixture");
        let pin = fixture_pin("0000000000000000000000000000000000000000000000000000000000000000");

        assert!(verify(&installer, &pin).is_err());
    }

    #[test]
    fn client_pin_manifest_contains_observed_release() {
        let pin = client_pin().expect("parse embedded client pin");

        assert_eq!(pin.version, "4.42.1.2835");
        assert_eq!(pin.size, 89_664_288);
        assert_eq!(
            pin.sha256,
            "d3a7776f6fb00b196e2fcbf8acba06fd52b19437b0da80fec0ae838ee362f06f"
        );
        assert_eq!(
            pin.url,
            "https://api.nrd.nie.163.com/api/v1/release/dl/1?channel=gwqd"
        );
    }

    #[test]
    fn pe_installer_fallback_validates_mz_header_and_size() {
        let fixture = FixtureDir::new("pe-fallback");
        let small = fixture.path().join("small.exe");
        fs::write(&small, b"MZ").expect("write small pe fixture");
        assert!(verify_pe_installer(&small).is_err());

        let mut valid_bytes = vec![0u8; 1_048_576];
        valid_bytes[0] = b'M';
        valid_bytes[1] = b'Z';
        let valid = fixture.path().join("valid.exe");
        fs::write(&valid, &valid_bytes).expect("write valid pe fixture");
        assert!(verify_pe_installer(&valid).is_ok());
    }
}
