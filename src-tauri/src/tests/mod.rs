#[cfg(test)]
mod hardware_tests {
    use crate::hardware::HardwareDetector;

    #[tokio::test]
    async fn test_detect_hardware() {
        let result = HardwareDetector::new().detect().await;
        assert!(result.is_ok());
        
        let hw = result.unwrap();
        assert!(hw.ram_gb > 0.0);
        assert!(hw.cpu_cores > 0);
    }

    #[tokio::test]
    async fn test_ram_detection() {
        let result = HardwareDetector::new().detect().await;
        assert!(result.is_ok());
        
        let hw = result.unwrap();
        assert!(hw.ram_gb > 0.0);
    }
}

#[cfg(test)]
mod model_tests {
    #[test]
    fn test_model_name_parsing() {
        let name = "llama2:7b";
        assert!(name.contains(":"));
        
        let parts: Vec<&str> = name.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "llama2");
        assert_eq!(parts[1], "7b");
    }
}

#[cfg(test)]
mod download_tests {
    use super::{calculate_progress, format_bytes};

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_calculate_progress() {
        let progress = calculate_progress(50, 100);
        assert_eq!(progress, 50.0);
        
        let progress = calculate_progress(0, 100);
        assert_eq!(progress, 0.0);
        
        let progress = calculate_progress(100, 100);
        assert_eq!(progress, 100.0);
    }
}

#[cfg(test)]
mod optimization_tests {
    use super::{calculate_context_size, calculate_optimal_threads};

    #[test]
    fn test_calculate_optimal_threads() {
        let threads = calculate_optimal_threads(8, 16.0);
        assert!(threads > 0);
        assert!(threads <= 8);
    }

    #[test]
    fn test_calculate_context_size() {
        let context = calculate_context_size(16.0);
        assert!(context >= 2048);
        assert!(context <= 32768);
    }
}

// Helper functions for tests
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn calculate_progress(current: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    (current as f64 / total as f64) * 100.0
}

fn calculate_optimal_threads(cpu_cores: usize, ram_gb: f64) -> usize {
    let max_threads = cpu_cores.min((ram_gb / 2.0) as usize);
    max_threads.max(1).min(cpu_cores)
}

fn calculate_context_size(ram_gb: f64) -> usize {
    if ram_gb >= 32.0 {
        32768
    } else if ram_gb >= 16.0 {
        8192
    } else if ram_gb >= 8.0 {
        4096
    } else {
        2048
    }
}
