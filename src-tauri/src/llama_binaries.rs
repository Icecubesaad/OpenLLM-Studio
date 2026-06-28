use crate::error::AppError;
use crate::models::GpuBackend;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const MIN_RUNTIME_ARCHIVE_BYTES: u64 = 1_048_576;

#[derive(Debug, Clone, Copy)]
enum ArchiveKind {
    Zip,
    TarGz,
}

impl ArchiveKind {
    fn label(self) -> &'static str {
        match self {
            ArchiveKind::Zip => "zip",
            ArchiveKind::TarGz => "gzip tar",
        }
    }
}

struct DownloadCandidate {
    response: reqwest::Response,
    requested_url: String,
    final_url: String,
    content_type: Option<String>,
    content_length: Option<u64>,
    expected_sha256: Option<String>,
}

/// Binary variant types for different GPU backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryVariant {
    Cpu,
    Cuda12_4,
    Cuda13_1,
    Vulkan,
    Sycl,
    Metal,
}

impl BinaryVariant {
    /// Get the download URL for this binary variant based on OS
    pub fn download_url(&self) -> String {
        self.download_urls()
            .into_iter()
            .next()
            .unwrap_or_else(|| format!("{}/cpu/cpu.zip", "https://pub-db624e2b2bb447f2b29b8cd6b57e4518.r2.dev"))
    }

    pub fn download_urls(&self) -> Vec<String> {
        let base_url = "https://pub-db624e2b2bb447f2b29b8cd6b57e4518.r2.dev";
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        
        match os {
            "windows" => self.windows_url(base_url),
            "macos" => self.macos_url(base_url, arch),
            "linux" => self.linux_url(base_url, arch),
            _ => vec![format!("{}/cpu/cpu.zip", base_url)],
        }
    }

    fn windows_url(&self, base_url: &str) -> Vec<String> {
        match self {
            BinaryVariant::Cpu => vec![format!("{}/windows/cpu/cpu.zip", base_url)],
            BinaryVariant::Cuda12_4 => vec![format!("{}/windows/nvidia/cuda-12.4/cuda-12.4.zip", base_url)],
            BinaryVariant::Cuda13_1 => vec![format!("{}/windows/nvidia/cuda-13.1/cuda-13.1.zip", base_url)],
            BinaryVariant::Vulkan => vec![format!("{}/windows/amd/vulkan/vulkan.zip", base_url)],
            BinaryVariant::Sycl => vec![
                format!("{}/windows/intel/sycl/sycl.zip", base_url),
                format!("{}/windows/intel/sycl.zip", base_url),
                format!("{}/windows/intel/intel.zip", base_url),
            ],
            BinaryVariant::Metal => vec![format!("{}/windows/cpu/cpu.zip", base_url)], // Metal not for Windows
        }
    }

    fn macos_url(&self, base_url: &str, arch: &str) -> Vec<String> {
        let arch_suffix = if arch == "aarch64" { "arm64" } else { "x64" };
        match self {
            BinaryVariant::Cpu => vec![format!("{}/macos/cpu/cpu-{}.zip", base_url, arch_suffix)],
            BinaryVariant::Metal => vec![format!("{}/macos/metal/metal-{}.zip", base_url, arch_suffix)],
            _ => vec![format!("{}/macos/cpu/cpu-{}.zip", base_url, arch_suffix)], // CUDA/Vulkan not for macOS
        }
    }

    fn linux_url(&self, base_url: &str, arch: &str) -> Vec<String> {
        let arch_suffix = if arch == "aarch64" { "arm64" } else { "x64" };
        match self {
            BinaryVariant::Cpu => vec![format!("{}/linux/cpu/cpu-{}.zip", base_url, arch_suffix)],
            BinaryVariant::Cuda12_4 => vec![format!("{}/linux/nvidia/cuda-12.4/cuda-12.4-{}.zip", base_url, arch_suffix)],
            BinaryVariant::Cuda13_1 => vec![format!("{}/linux/nvidia/cuda-13.1/cuda-13.1-{}.zip", base_url, arch_suffix)],
            BinaryVariant::Vulkan => vec![format!("{}/linux/amd/vulkan/vulkan-{}.zip", base_url, arch_suffix)],
            BinaryVariant::Metal => vec![format!("{}/linux/cpu/cpu-{}.zip", base_url, arch_suffix)], // Metal not for Linux
            BinaryVariant::Sycl => vec![format!("{}/linux/cpu/cpu-{}.zip", base_url, arch_suffix)],
        }
    }

    /// Get the expected file size in bytes (for progress calculation)
    pub fn expected_size_bytes(&self) -> u64 {
        match self {
            BinaryVariant::Cpu => 120_000_000,
            BinaryVariant::Cuda12_4 => 200_000_000,
            BinaryVariant::Cuda13_1 => 200_000_000,
            BinaryVariant::Vulkan => 180_000_000,
            BinaryVariant::Sycl => 220_000_000,
            BinaryVariant::Metal => 150_000_000,
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            BinaryVariant::Cpu => "CPU Only",
            BinaryVariant::Cuda12_4 => "NVIDIA GPU (CUDA 12.4)",
            BinaryVariant::Cuda13_1 => "NVIDIA GPU (CUDA 13.1)",
            BinaryVariant::Vulkan => "Vulkan (AMD)",
            BinaryVariant::Sycl => "Intel GPU (SYCL)",
            BinaryVariant::Metal => "Apple Metal",
        }
    }

    /// Check if this variant is available for the current OS
    pub fn is_available_for_os(&self) -> bool {
        let os = std::env::consts::OS;
        match (self, os) {
            (BinaryVariant::Metal, "macos") => true,
            (BinaryVariant::Metal, _) => false,
            (BinaryVariant::Sycl, "windows") => true,
            (BinaryVariant::Sycl, _) => false,
            (BinaryVariant::Cuda12_4 | BinaryVariant::Cuda13_1, "macos") => false,
            (BinaryVariant::Vulkan, "macos") => false,
            _ => true,
        }
    }
}

/// Recommended binary based on detected hardware and OS
pub fn recommend_binary(gpu_backend: &GpuBackend, os: &str) -> BinaryVariant {
    match (gpu_backend, os) {
        // Windows
        (GpuBackend::Nvidia, "windows") => BinaryVariant::Cuda12_4,
        (GpuBackend::Amd, "windows") => BinaryVariant::Vulkan,
        (GpuBackend::Intel, "windows") => BinaryVariant::Sycl,
        (_, "windows") => BinaryVariant::Cpu,
        
        // macOS - use Metal for Apple Silicon, CPU for Intel
        (GpuBackend::AppleMetal, "macos") => BinaryVariant::Metal,
        (_, "macos") => BinaryVariant::Cpu,
        
        // Linux
        (GpuBackend::Nvidia, "linux") => BinaryVariant::Cuda12_4,
        (GpuBackend::Amd, "linux") => BinaryVariant::Vulkan,
        (_, "linux") => BinaryVariant::Cpu,
        
        // Fallback
        _ => BinaryVariant::Cpu,
    }
}

/// Status of binary download/installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStatus {
    pub variant: BinaryVariant,
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
}

/// Download progress event
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub variant: BinaryVariant,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percentage: f32,
}

/// Extract a zip file to the target directory (blocking operation)
fn extract_zip_blocking(zip_path: &Path, target_dir: &Path) -> Result<(), AppError> {
    use std::fs::File;
    use std::io::Read;
    use std::io::Write;
    use zip::ZipArchive;

    println!("[extract_zip] Extracting {} to {}", zip_path.to_string_lossy(), target_dir.to_string_lossy());

    let file = File::open(zip_path)
        .map_err(|e| AppError::DownloadError(format!("Failed to open zip: {}", e)))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| {
            AppError::DownloadError(format!(
                "The downloaded runtime archive is not a valid ZIP file. It may be corrupted or incomplete. Details: {}",
                e
            ))
        })?;

    println!("[extract_zip] Archive contains {} files", archive.len());

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| AppError::DownloadError(format!("Failed to read archive entry: {}", e)))?;
        let outpath = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => continue,
        };

        println!("[extract_zip] Extracting: {} -> {}", file.name(), outpath.to_string_lossy());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(p) = outpath.parent() {
                std::fs::create_dir_all(p).ok();
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| AppError::DownloadError(format!("Failed to create file: {}", e)))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| AppError::DownloadError(format!("Failed to read archive entry: {}", e)))?;
            outfile.write_all(&buffer)
                .map_err(|e| AppError::DownloadError(format!("Failed to write archive entry: {}", e)))?;
        }
    }

    println!("[extract_zip] Extraction complete");
    Ok(())
}

fn extract_tar_gz_blocking(archive_path: &Path, target_dir: &Path) -> Result<(), AppError> {
    use flate2::read::GzDecoder;
    use std::fs::File;

    println!(
        "[extract_tar_gz] Extracting {} to {}",
        archive_path.to_string_lossy(),
        target_dir.to_string_lossy()
    );

    let file = File::open(archive_path)
        .map_err(|e| AppError::DownloadError(format!("Failed to open gzip archive: {}", e)))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut extracted = 0usize;

    for entry in archive
        .entries()
        .map_err(|e| AppError::DownloadError(format!("Failed to read gzip tar archive: {}", e)))?
    {
        let mut entry = entry
            .map_err(|e| AppError::DownloadError(format!("Failed to read archive entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| AppError::DownloadError(format!("Failed to read archive entry path: {}", e)))?
            .into_owned();

        if !is_safe_relative_path(&path) {
            return Err(AppError::DownloadError(format!(
                "Runtime archive contains an unsafe path: {}",
                path.display()
            )));
        }

        let unpacked = entry
            .unpack_in(target_dir)
            .map_err(|e| AppError::DownloadError(format!("Failed to extract archive entry: {}", e)))?;
        if !unpacked {
            return Err(AppError::DownloadError(format!(
                "Runtime archive entry would extract outside the install directory: {}",
                path.display()
            )));
        }
        extracted += 1;
    }

    if extracted == 0 {
        return Err(AppError::DownloadError(
            "The downloaded runtime archive was empty.".to_string(),
        ));
    }

    println!("[extract_tar_gz] Extraction complete ({} entries)", extracted);
    Ok(())
}

fn validate_archive_blocking(archive_path: &Path) -> Result<ArchiveKind, AppError> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use zip::ZipArchive;

    println!("[LlamaBinaryManager] File exists: {}", archive_path.exists());

    let metadata = std::fs::metadata(archive_path).map_err(|e| {
        AppError::DownloadError(format!("Downloaded runtime file is not accessible: {}", e))
    })?;
    println!("[LlamaBinaryManager] Downloaded file size: {}", metadata.len());

    if metadata.len() == 0 {
        return Err(AppError::DownloadError(
            "The downloaded runtime file is empty.".to_string(),
        ));
    }
    if metadata.len() < MIN_RUNTIME_ARCHIVE_BYTES {
        return Err(AppError::DownloadError(format!(
            "The downloaded runtime is unexpectedly small ({} bytes). It may be an error page or incomplete download.",
            metadata.len()
        )));
    }

    let mut file = File::open(archive_path)
        .map_err(|e| AppError::DownloadError(format!("Failed to open downloaded runtime: {}", e)))?;
    let mut header = [0u8; 4];
    let read = file
        .read(&mut header)
        .map_err(|e| AppError::DownloadError(format!("Failed to read archive header: {}", e)))?;

    let kind = if read >= 4
        && header[0] == b'P'
        && header[1] == b'K'
        && matches!(header[2], 3 | 5 | 7)
    {
        ArchiveKind::Zip
    } else if read >= 2 && header[0] == 0x1f && header[1] == 0x8b {
        ArchiveKind::TarGz
    } else if read > 0 && header[0] == b'<' {
        return Err(AppError::DownloadError(
            "The downloaded runtime looks like HTML, not an archive.".to_string(),
        ));
    } else {
        return Err(AppError::DownloadError(
            "The downloaded runtime is not a supported archive format.".to_string(),
        ));
    };

    println!(
        "[LlamaBinaryManager] ZIP header valid: {}",
        matches!(kind, ArchiveKind::Zip)
    );
    println!("[LlamaBinaryManager] Archive format: {}", kind.label());

    file.seek(SeekFrom::Start(0))
        .map_err(|e| AppError::DownloadError(format!("Failed to validate archive: {}", e)))?;

    match kind {
        ArchiveKind::Zip => {
            let mut archive = ZipArchive::new(file).map_err(|e| {
                AppError::DownloadError(format!(
                    "The downloaded runtime archive is not a valid ZIP file. It may be corrupted or incomplete. Details: {}",
                    e
                ))
            })?;
            if archive.len() == 0 {
                return Err(AppError::DownloadError(
                    "The downloaded runtime archive was empty.".to_string(),
                ));
            }
            for i in 0..archive.len() {
                let mut entry = archive.by_index(i).map_err(|e| {
                    AppError::DownloadError(format!("Failed to validate archive entry: {}", e))
                })?;
                if entry.enclosed_name().is_none() {
                    return Err(AppError::DownloadError(format!(
                        "Runtime archive contains an unsafe path: {}",
                        entry.name()
                    )));
                }
                std::io::copy(&mut entry, &mut std::io::sink()).map_err(|e| {
                    AppError::DownloadError(format!("Archive validation failed while reading entry: {}", e))
                })?;
            }
        }
        ArchiveKind::TarGz => validate_tar_gz_blocking(archive_path)?,
    }

    Ok(kind)
}

fn extract_archive_blocking(
    archive_path: &Path,
    target_dir: &Path,
    archive_kind: ArchiveKind,
) -> Result<(), AppError> {
    match archive_kind {
        ArchiveKind::Zip => extract_zip_blocking(archive_path, target_dir),
        ArchiveKind::TarGz => extract_tar_gz_blocking(archive_path, target_dir),
    }
}

fn validate_tar_gz_blocking(archive_path: &Path) -> Result<(), AppError> {
    use flate2::read::GzDecoder;
    use std::fs::File;

    let file = File::open(archive_path)
        .map_err(|e| AppError::DownloadError(format!("Failed to open gzip archive: {}", e)))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut entries = 0usize;

    for entry in archive
        .entries()
        .map_err(|e| AppError::DownloadError(format!("Failed to read gzip tar archive: {}", e)))?
    {
        let mut entry = entry
            .map_err(|e| AppError::DownloadError(format!("Failed to validate archive entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| AppError::DownloadError(format!("Failed to validate archive path: {}", e)))?
            .into_owned();

        if !is_safe_relative_path(&path) {
            return Err(AppError::DownloadError(format!(
                "Runtime archive contains an unsafe path: {}",
                path.display()
            )));
        }

        std::io::copy(&mut entry, &mut std::io::sink()).map_err(|e| {
            AppError::DownloadError(format!("Archive validation failed while reading entry: {}", e))
        })?;
        entries += 1;
    }

    if entries == 0 {
        return Err(AppError::DownloadError(
            "The downloaded runtime archive was empty.".to_string(),
        ));
    }

    Ok(())
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path.components().all(|component| {
            matches!(component, Component::Normal(_) | Component::CurDir)
        })
}

fn is_rejected_content_type(content_type: Option<&str>) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };
    let content_type = content_type.to_ascii_lowercase();
    content_type.starts_with("text/")
        || content_type.contains("html")
        || content_type.contains("json")
        || content_type.contains("xml")
}

fn parse_sha256_from_text(value: &str) -> Option<String> {
    let mut run = String::with_capacity(64);

    for ch in value.chars() {
        if ch.is_ascii_hexdigit() {
            run.push(ch);
            if run.len() == 64 {
                return Some(run.to_ascii_lowercase());
            }
        } else {
            run.clear();
        }
    }

    None
}

async fn find_expected_sha256(
    client: &reqwest::Client,
    requested_url: &str,
    final_url: &str,
    headers: &HeaderMap,
) -> Option<String> {
    for header_name in [
        "x-amz-meta-sha256",
        "x-checksum-sha256",
        "x-content-sha256",
        "x-amz-checksum-sha256",
    ] {
        if let Some(value) = headers.get(header_name).and_then(|value| value.to_str().ok()) {
            if let Some(checksum) = parse_sha256_from_text(value) {
                println!(
                    "[LlamaBinaryManager] Found SHA-256 checksum in {} header",
                    header_name
                );
                return Some(checksum);
            }
        }
    }

    let mut checked = HashSet::new();
    for base_url in [requested_url, final_url] {
        for suffix in [".sha256", ".sha256sum"] {
            let checksum_url = format!("{}{}", base_url, suffix);
            if !checked.insert(checksum_url.clone()) {
                continue;
            }

            println!("[LlamaBinaryManager] Checking checksum URL: {}", checksum_url);
            let response = match client.get(&checksum_url).send().await {
                Ok(response) => response,
                Err(error) => {
                    println!(
                        "[LlamaBinaryManager] Checksum URL unavailable: {} ({})",
                        checksum_url, error
                    );
                    continue;
                }
            };

            if response.status() != reqwest::StatusCode::OK {
                println!(
                    "[LlamaBinaryManager] Checksum URL returned {}: {}",
                    response.status(),
                    checksum_url
                );
                continue;
            }

            match response.text().await {
                Ok(text) => {
                    if let Some(checksum) = parse_sha256_from_text(&text) {
                        println!(
                            "[LlamaBinaryManager] Found SHA-256 checksum from sidecar: {}",
                            checksum_url
                        );
                        return Some(checksum);
                    }
                }
                Err(error) => {
                    println!(
                        "[LlamaBinaryManager] Failed to read checksum sidecar {}: {}",
                        checksum_url, error
                    );
                }
            }
        }
    }

    None
}

fn sha256_file_blocking(path: &Path) -> Result<String, AppError> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)
        .map_err(|e| AppError::DownloadError(format!("Failed to open downloaded runtime: {}", e)))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| AppError::DownloadError(format!("Failed to read downloaded runtime: {}", e)))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn find_file_recursive(root: &Path, file_name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    let target_name = OsStr::new(file_name);

    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name() == Some(target_name) && path.is_file() {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, file_name) {
                return Some(found);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sha256_from_sidecar_text() {
        let checksum = "17a2f0f8dc7b3c96c1fbfed7516159223ea587e32862df93f6692befd28e275d";
        assert_eq!(
            parse_sha256_from_text(&format!("{}  metal-arm64.zip", checksum)),
            Some(checksum.to_string())
        );
    }

    #[test]
    fn rejects_text_content_types() {
        assert!(is_rejected_content_type(Some("text/html; charset=utf-8")));
        assert!(is_rejected_content_type(Some("application/json")));
        assert!(!is_rejected_content_type(Some("application/gzip")));
        assert!(!is_rejected_content_type(Some("application/zip")));
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[tokio::test]
    #[ignore = "downloads and extracts the live Apple Metal runtime archive"]
    async fn live_downloads_apple_metal_runtime() {
        let temp_dir = std::env::temp_dir().join(format!(
            "openllm-studio-metal-runtime-{}",
            uuid::Uuid::new_v4()
        ));
        let manager = LlamaBinaryManager::new(temp_dir.clone());

        let binary_path = manager
            .download_binary(BinaryVariant::Metal, |_| {})
            .await
            .expect("Apple Metal runtime should download and extract");

        assert!(binary_path.exists(), "missing binary at {}", binary_path.display());
        assert_eq!(binary_path.file_name(), Some(OsStr::new("llama-server")));

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}

/// Manager for llama.cpp binaries
pub struct LlamaBinaryManager {
    app_data_dir: PathBuf,
}

impl LlamaBinaryManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }

    /// Get the directory where binaries are stored
    pub fn binaries_dir(&self) -> PathBuf {
        self.app_data_dir.join("llama-binaries")
    }

    fn variant_dir(&self, variant: &BinaryVariant) -> PathBuf {
        let dir_name = match variant {
            BinaryVariant::Cpu => "cpu",
            BinaryVariant::Cuda12_4 => "cuda-12.4",
            BinaryVariant::Cuda13_1 => "cuda-13.1",
            BinaryVariant::Vulkan => "vulkan",
            BinaryVariant::Sycl => "sycl",
            BinaryVariant::Metal => "metal",
        };

        self.binaries_dir().join(dir_name)
    }

    fn executable_name() -> &'static str {
        match std::env::consts::OS {
            "windows" => "llama-server.exe",
            _ => "llama-server",
        }
    }

    pub fn find_installed_binary_path(&self, variant: &BinaryVariant) -> Option<PathBuf> {
        let expected_path = self.get_binary_path(variant);
        if expected_path.exists() {
            return Some(expected_path);
        }

        let flat_path = self.binaries_dir().join(Self::executable_name());
        if flat_path.exists() {
            return Some(flat_path);
        }

        find_file_recursive(&self.variant_dir(variant), Self::executable_name())
    }

    /// Check if a binary variant is already installed
    pub async fn is_installed(&self, variant: &BinaryVariant) -> bool {
        // Check the expected subdirectory path (e.g., llama-binaries/vulkan/llama-server.exe)
        if let Some(binary_path) = self.find_installed_binary_path(variant) {
            println!("[LlamaBinaryManager] {:?} found at: {}", variant, binary_path.to_string_lossy());
            return true;
        }

        println!(
            "[LlamaBinaryManager] {:?} NOT found under {}",
            variant,
            self.variant_dir(variant).to_string_lossy()
        );
        false
    }

    /// Get the path to the main llama-server executable (subdirectory path)
    pub fn get_binary_path(&self, variant: &BinaryVariant) -> PathBuf {
        let os = std::env::consts::OS;
        
        let dir_name = match variant {
            BinaryVariant::Cpu => "cpu",
            BinaryVariant::Cuda12_4 => "cuda-12.4",
            BinaryVariant::Cuda13_1 => "cuda-13.1",
            BinaryVariant::Vulkan => "vulkan",
            BinaryVariant::Sycl => "sycl",
            BinaryVariant::Metal => "metal",
        };
        
        let executable_name = match os {
            "windows" => "llama-server.exe",
            "macos" => "llama-server",
            "linux" => "llama-server",
            _ => "llama-server",
        };
        
        self.binaries_dir().join(dir_name).join(executable_name)
    }

    /// Download and extract a binary variant
    pub async fn download_binary(
        &self,
        variant: BinaryVariant,
        progress_callback: impl Fn(DownloadProgress) + Send + 'static,
    ) -> Result<PathBuf, AppError> {
        let urls = variant.download_urls();

        // Extract into variant-specific subdirectory (e.g., llama-binaries/vulkan/)
        let variant_dir = self.variant_dir(&variant);

        println!("[LlamaBinaryManager] Downloading {:?} using candidate URLs: {:?}", variant, urls);
        println!("[LlamaBinaryManager] Target directory: {}", variant_dir.to_string_lossy());

        // Create target directory
        fs::create_dir_all(&variant_dir).await
            .map_err(|e| AppError::DownloadError(format!("Failed to create directory: {}", e)))?;

        // Download the zip file
        let mut last_error = None;
        let mut selected = None;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| AppError::DownloadError(format!("Failed to create download client: {}", e)))?;

        for url in &urls {
            println!("[LlamaBinaryManager] Download URL: {}", url);

            match client.get(url).send().await {
                Ok(candidate_response) => {
                    let status = candidate_response.status();
                    let final_url = candidate_response.url().to_string();
                    let content_type = candidate_response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .map(ToOwned::to_owned);
                    let content_length = candidate_response.content_length();

                    println!("[LlamaBinaryManager] HTTP status: {}", status);
                    println!("[LlamaBinaryManager] Final redirected URL: {}", final_url);
                    println!(
                        "[LlamaBinaryManager] Content-Type: {}",
                        content_type.as_deref().unwrap_or("unknown")
                    );
                    println!(
                        "[LlamaBinaryManager] Content-Length: {}",
                        content_length
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    );

                    if status != reqwest::StatusCode::OK {
                        let error = format!("{} returned HTTP {}", final_url, status);
                        println!("[LlamaBinaryManager] {}", error);
                        last_error = Some(error);
                        continue;
                    }

                    if is_rejected_content_type(content_type.as_deref()) {
                        let error = format!(
                            "{} returned {} instead of a runtime archive",
                            final_url,
                            content_type.as_deref().unwrap_or("unknown content")
                        );
                        println!("[LlamaBinaryManager] {}", error);
                        last_error = Some(error);
                        continue;
                    }

                    if let Some(length) = content_length {
                        if length < MIN_RUNTIME_ARCHIVE_BYTES {
                            let error = format!(
                                "{} returned only {} bytes, which is too small for a runtime archive",
                                final_url, length
                            );
                            println!("[LlamaBinaryManager] {}", error);
                            last_error = Some(error);
                            continue;
                        }
                    }

                    let headers = candidate_response.headers().clone();
                    let expected_sha256 =
                        find_expected_sha256(&client, url, &final_url, &headers).await;

                    println!("[LlamaBinaryManager] Selected download URL: {}", url);
                    selected = Some(DownloadCandidate {
                        response: candidate_response,
                        requested_url: url.clone(),
                        final_url,
                        content_type,
                        content_length,
                        expected_sha256,
                    });
                    break;
                }
                Err(e) => {
                    let error = format!("{} failed: {}", url, e);
                    println!("[LlamaBinaryManager] {}", error);
                    last_error = Some(error);
                }
            }
        }
        let selected = selected.ok_or_else(|| {
            AppError::DownloadError(format!(
                "Failed to start download from all candidate URLs: {}",
                last_error.unwrap_or_else(|| "unknown error".to_string())
            ))
        })?;
        let DownloadCandidate {
            response,
            requested_url,
            final_url,
            content_type,
            content_length,
            expected_sha256,
        } = selected;

        let total_size = content_length.unwrap_or(variant.expected_size_bytes());
        let mut downloaded: u64 = 0;

        // Stream the response and write to file
        let archive_path = variant_dir.join("download.archive");
        let mut file = fs::File::create(&archive_path).await
            .map_err(|e| AppError::DownloadError(format!("Failed to create file: {}", e)))?;

        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AppError::DownloadError(format!("Download error: {}", e)))?;
            file.write_all(&chunk).await
                .map_err(|e| AppError::DownloadError(format!("Write error: {}", e)))?;
            downloaded += chunk.len() as u64;

            progress_callback(DownloadProgress {
                variant: variant.clone(),
                bytes_downloaded: downloaded,
                total_bytes: total_size,
                percentage: (downloaded as f32 / total_size as f32) * 100.0,
            });
        }
        file.flush().await
            .map_err(|e| AppError::DownloadError(format!("Failed to flush downloaded runtime: {}", e)))?;
        drop(file);

        println!(
            "[LlamaBinaryManager] Downloaded file path: {}",
            archive_path.to_string_lossy()
        );
        println!("[LlamaBinaryManager] Downloaded file size: {}", downloaded);
        println!(
            "[LlamaBinaryManager] Downloaded from {} (final URL: {}, content type: {})",
            requested_url,
            final_url,
            content_type.as_deref().unwrap_or("unknown")
        );

        if let Some(expected_length) = content_length {
            if downloaded != expected_length {
                return Err(AppError::DownloadError(format!(
                    "The downloaded runtime is incomplete. Expected {} bytes but received {} bytes.",
                    expected_length, downloaded
                )));
            }
        }

        if let Some(expected_sha256) = expected_sha256 {
            let archive_path_for_hash = archive_path.clone();
            let actual_sha256 = tokio::task::spawn_blocking(move || {
                sha256_file_blocking(&archive_path_for_hash)
            })
            .await
            .map_err(|e| AppError::DownloadError(format!("Checksum task failed: {}", e)))??;

            if actual_sha256 != expected_sha256 {
                return Err(AppError::DownloadError(format!(
                    "The downloaded runtime checksum did not match. Expected SHA-256 {}, got {}.",
                    expected_sha256, actual_sha256
                )));
            }

            println!("[LlamaBinaryManager] SHA-256 checksum verified: {}", actual_sha256);
        } else {
            println!("[LlamaBinaryManager] SHA-256 checksum not available for this runtime archive");
        }

        // Validate and extract the archive into the variant subdirectory (blocking operation)
        let archive_path_for_validation = archive_path.clone();
        let archive_kind = tokio::task::spawn_blocking(move || {
            validate_archive_blocking(&archive_path_for_validation)
        }).await.map_err(|e| AppError::DownloadError(format!("Archive validation task failed: {}", e)))??;

        let archive_path_for_extract = archive_path.clone();
        let variant_dir_clone = variant_dir.clone();
        tokio::task::spawn_blocking(move || {
            extract_archive_blocking(&archive_path_for_extract, &variant_dir_clone, archive_kind)
        }).await.map_err(|e| AppError::DownloadError(format!("Archive extraction task failed: {}", e)))??;

        // Clean up archive file
        fs::remove_file(&archive_path).await.ok();

        let binary_path = self.find_installed_binary_path(&variant).ok_or_else(|| {
            AppError::DownloadError(format!(
                "The runtime archive extracted successfully, but {} was not found under {}.",
                Self::executable_name(),
                variant_dir.to_string_lossy()
            ))
        })?;
        println!("[LlamaBinaryManager] Download complete. Binary should be at: {}", binary_path.to_string_lossy());
        println!("[LlamaBinaryManager] Binary exists: {}", binary_path.exists());

        Ok(binary_path)
    }

    /// Get status of all binary variants available for current OS
    pub async fn get_all_statuses(&self) -> Vec<BinaryStatus> {
        let all_variants = [
            BinaryVariant::Cpu,
            BinaryVariant::Cuda12_4,
            BinaryVariant::Cuda13_1,
            BinaryVariant::Vulkan,
            BinaryVariant::Sycl,
            BinaryVariant::Metal,
        ];

        // Filter to only show variants available for this OS
        let available_variants: Vec<_> = all_variants
            .into_iter()
            .filter(|v| v.is_available_for_os())
            .collect();

        let mut statuses = Vec::new();
        for variant in available_variants {
            let installed = self.is_installed(&variant).await;
            let path = if installed {
                self.find_installed_binary_path(&variant)
            } else {
                None
            };

            statuses.push(BinaryStatus {
                variant,
                installed,
                version: None,
                path,
            });
        }

        statuses
    }
}
