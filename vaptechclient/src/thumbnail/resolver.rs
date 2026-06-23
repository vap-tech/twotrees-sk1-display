//! Source resolver для thumbnails.
//!
//! Encoder работает только с локальным G-code. Этот слой отвечает за то, чтобы
//! remote Moonraker path стал локальным cache-файлом, после чего используется
//! обычный `GcodeFile` pipeline.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone)]
pub struct MoonrakerFileSource {
    pub path: String,
    pub modified: Option<i64>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ThumbnailResolverConfig {
    pub moonraker_http_url: String,
    pub cache_dir: PathBuf,
}

impl ThumbnailResolverConfig {
    pub fn new(moonraker_http_url: impl Into<String>, cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            moonraker_http_url: moonraker_http_url.into(),
            cache_dir: cache_dir.into(),
        }
    }
}

pub fn resolve_moonraker_file(
    config: &ThumbnailResolverConfig,
    source: &MoonrakerFileSource,
) -> Result<PathBuf> {
    let cache_path = cache_path_for_source(&config.cache_dir, source);

    if cache_path.exists() {
        return Ok(cache_path);
    }

    let parent = cache_path
        .parent()
        .context("thumbnail cache path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create thumbnail cache dir {}", parent.display()))?;

    let normalized_path = normalize_gcode_path(&source.path);
    let request_path = format!(
        "/server/files/gcodes/{}",
        percent_encode_path(&normalized_path)
    );
    let bytes = http_get_bytes(&config.moonraker_http_url, &request_path)?;

    let tmp_path = cache_path.with_extension("tmp");
    {
        let mut file = fs::File::create(&tmp_path).with_context(|| {
            format!(
                "failed to create temp thumbnail source {}",
                tmp_path.display()
            )
        })?;
        file.write_all(&bytes).with_context(|| {
            format!(
                "failed to write temp thumbnail source {}",
                tmp_path.display()
            )
        })?;
    }
    fs::rename(&tmp_path, &cache_path).with_context(|| {
        format!(
            "failed to move thumbnail source {} to {}",
            tmp_path.display(),
            cache_path.display()
        )
    })?;

    Ok(cache_path)
}

fn cache_path_for_source(cache_dir: &Path, source: &MoonrakerFileSource) -> PathBuf {
    let mut file_name = sanitize_cache_component(&source.path);

    if let Some(modified) = source.modified {
        file_name.push_str(&format!(".m{modified}"));
    }

    if let Some(size) = source.size {
        file_name.push_str(&format!(".s{size}"));
    }

    file_name.push_str(".gcode");

    cache_dir.join("gcode").join(file_name)
}

pub fn normalize_gcode_path(path: &str) -> String {
    let path = path.trim();

    if let Some((_, relative)) = path.split_once("/printer_data/gcodes/") {
        return relative.trim_start_matches('/').to_string();
    }

    path.strip_prefix("gcodes/")
        .unwrap_or(path)
        .trim_start_matches('/')
        .to_string()
}

pub fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());

    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }

    encoded
}

fn sanitize_cache_component(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                sanitized.push(byte as char)
            }
            _ => sanitized.push_str(&format!("_{byte:02X}")),
        }
    }

    sanitized
}

fn http_get_bytes(base_url: &str, path: &str) -> Result<Vec<u8>> {
    let (host, port) = parse_http_base_url(base_url)?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .with_context(|| format!("failed to connect Moonraker HTTP {host}:{port}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .context("failed to set Moonraker HTTP read timeout")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .context("failed to set Moonraker HTTP write timeout")?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\nAccept: application/octet-stream\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .context("failed to write Moonraker HTTP request")?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .context("failed to read Moonraker HTTP response")?;

    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .context("Moonraker HTTP response has no header terminator")?;
    let header = String::from_utf8_lossy(&response[..header_end]);
    let status_line = header.lines().next().unwrap_or_default();

    if !status_line.contains(" 2") {
        bail!("Moonraker HTTP GET failed: {status_line}");
    }

    Ok(response[header_end + 4..].to_vec())
}

fn parse_http_base_url(base_url: &str) -> Result<(String, u16)> {
    let raw = base_url
        .strip_prefix("http://")
        .context("thumbnail Moonraker resolver supports only http:// URLs")?;
    let authority = raw.split('/').next().unwrap_or(raw);
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse::<u16>()
                .with_context(|| format!("invalid Moonraker HTTP port: {port}"))?,
        ),
        None => (authority.to_string(), 80),
    };

    if host.is_empty() {
        bail!("Moonraker HTTP host is empty");
    }

    Ok((host, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::TcpListener;
    use std::thread;

    const GCODE_FIXTURE: &str = include_str!("../../fixtures/thumbnails/orca_thumbnail.gcode");

    #[test]
    fn normalizes_absolute_moonraker_gcode_path() {
        assert_eq!(
            normalize_gcode_path("/home/mks/printer_data/gcodes/folder/cube.gcode"),
            "folder/cube.gcode"
        );
    }

    #[test]
    fn percent_encoding_keeps_path_separators() {
        assert_eq!(
            percent_encode_path("folder/Single Drawer_PLA.gcode"),
            "folder/Single%20Drawer_PLA.gcode"
        );
    }

    #[test]
    fn cache_path_includes_file_fingerprint() {
        let source = MoonrakerFileSource {
            path: "folder/cube.gcode".to_string(),
            modified: Some(123),
            size: Some(456),
        };

        let path = cache_path_for_source(Path::new("/tmp/thumbs"), &source);

        assert_eq!(
            path,
            PathBuf::from("/tmp/thumbs/gcode/folder_2Fcube.gcode.m123.s456.gcode")
        );
    }

    #[test]
    fn resolver_downloads_moonraker_file_to_cache() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let read_len = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read_len]);

            assert!(request.starts_with("GET /server/files/gcodes/folder/Single%20Drawer.gcode "));

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                GCODE_FIXTURE.len(),
                GCODE_FIXTURE
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let cache_dir = std::env::temp_dir().join(format!(
            "vaptechclient-thumbnail-resolver-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&cache_dir);
        let config = ThumbnailResolverConfig::new(format!("http://127.0.0.1:{port}"), &cache_dir);
        let source = MoonrakerFileSource {
            path: "folder/Single Drawer.gcode".to_string(),
            modified: Some(123),
            size: Some(456),
        };

        let path = resolve_moonraker_file(&config, &source).unwrap();
        let cached = fs::read_to_string(&path).unwrap();

        assert_eq!(cached, GCODE_FIXTURE);

        server.join().unwrap();
        let _ = fs::remove_dir_all(&cache_dir);
    }
}
