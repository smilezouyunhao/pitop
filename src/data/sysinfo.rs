use std::{process::Command, time::Duration};

use tokio::{sync::watch, task::JoinHandle, time};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemStats {
    pub cpu_usage_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
}

impl SystemStats {
    pub fn memory_usage_percent(&self) -> f64 {
        percent(self.memory_used_bytes, self.memory_total_bytes)
    }

    pub fn disk_usage_percent(&self) -> f64 {
        percent(self.disk_used_bytes, self.disk_total_bytes)
    }
}

pub fn spawn_system_monitor(interval: Duration) -> (watch::Receiver<SystemStats>, JoinHandle<()>) {
    let initial = collect_system_stats();
    let (sender, receiver) = watch::channel(initial);

    let handle = tokio::spawn(async move {
        let mut system = ::sysinfo::System::new();
        let mut disks = ::sysinfo::Disks::new_with_refreshed_list();
        let mut ticker = time::interval(interval);

        loop {
            ticker.tick().await;
            let stats = collect_system_stats_with(&mut system, &mut disks);

            if sender.send(stats).is_err() {
                break;
            }
        }
    });

    (receiver, handle)
}

pub fn collect_system_stats() -> SystemStats {
    let mut system = ::sysinfo::System::new();
    let mut disks = ::sysinfo::Disks::new_with_refreshed_list();
    collect_system_stats_with(&mut system, &mut disks)
}

fn collect_system_stats_with(
    system: &mut ::sysinfo::System,
    disks: &mut ::sysinfo::Disks,
) -> SystemStats {
    system.refresh_cpu_usage();
    system.refresh_memory();
    disks.refresh(true);

    let memory_total_bytes = system.total_memory();
    let memory_used_bytes = memory_used_bytes(system);

    let disk_total_bytes: u64 = disks.list().iter().map(|disk| disk.total_space()).sum();
    let disk_available_bytes: u64 = disks.list().iter().map(|disk| disk.available_space()).sum();
    let disk_used_bytes = disk_total_bytes.saturating_sub(disk_available_bytes);

    SystemStats {
        cpu_usage_percent: system.global_cpu_usage(),
        memory_used_bytes,
        memory_total_bytes,
        disk_used_bytes,
        disk_total_bytes,
    }
}

fn memory_used_bytes(system: &::sysinfo::System) -> u64 {
    let total = system.total_memory();

    #[cfg(target_os = "macos")]
    if let Some(used) = macos_memory_used_bytes(total) {
        return used.min(total);
    }

    let available = system.available_memory();
    if available > 0 {
        memory_used_from_available(total, available)
    } else {
        system.used_memory().min(total)
    }
}

#[cfg(target_os = "macos")]
fn macos_memory_used_bytes(total: u64) -> Option<u64> {
    let output = Command::new("vm_stat").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    parse_macos_vm_stat_used_bytes(&text).map(|used| used.min(total))
}

#[cfg(target_os = "macos")]
fn parse_macos_vm_stat_used_bytes(text: &str) -> Option<u64> {
    let page_size = parse_page_size(text.lines().next()?)?;
    let active = parse_vm_stat_value(text, "Pages active")?;
    let wired = parse_vm_stat_value(text, "Pages wired down")?;

    active.checked_add(wired)?.checked_mul(page_size)
}

#[cfg(target_os = "macos")]
fn parse_page_size(line: &str) -> Option<u64> {
    let marker = "page size of ";
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let end = rest.find(" bytes")?;
    rest[..end].trim().parse().ok()
}

#[cfg(target_os = "macos")]
fn parse_vm_stat_value(text: &str, label: &str) -> Option<u64> {
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with(label))?;
    let (_, value) = line.split_once(':')?;
    value.trim().trim_end_matches('.').parse().ok()
}

fn memory_used_from_available(total: u64, available: u64) -> u64 {
    total.saturating_sub(available)
}

fn percent(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        used as f64 / total as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_zero_percent_when_total_is_zero() {
        let stats = SystemStats {
            memory_used_bytes: 42,
            memory_total_bytes: 0,
            disk_used_bytes: 42,
            disk_total_bytes: 0,
            ..SystemStats::default()
        };

        assert_eq!(stats.memory_usage_percent(), 0.0);
        assert_eq!(stats.disk_usage_percent(), 0.0);
    }

    #[test]
    fn calculates_usage_percentages() {
        let stats = SystemStats {
            memory_used_bytes: 25,
            memory_total_bytes: 100,
            disk_used_bytes: 1,
            disk_total_bytes: 4,
            ..SystemStats::default()
        };

        assert_eq!(stats.memory_usage_percent(), 25.0);
        assert_eq!(stats.disk_usage_percent(), 25.0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parses_macos_vm_stat_used_memory() {
        let vm_stat = r#"Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                                    10933.
Pages active:                                 260105.
Pages inactive:                               257633.
Pages speculative:                              2822.
Pages wired down:                             160745.
Pages occupied by compressor:                 319394.
"#;

        assert_eq!(
            parse_macos_vm_stat_used_bytes(vm_stat),
            Some((260_105 + 160_745) * 16_384)
        );
    }

    #[test]
    fn uses_total_minus_available_for_memory_used() {
        assert_eq!(memory_used_from_available(100, 62), 38);
        assert_eq!(memory_used_from_available(100, 120), 0);
    }

    #[test]
    fn collects_real_system_stats() {
        let stats = collect_system_stats();

        assert!(stats.cpu_usage_percent >= 0.0);
        assert!(stats.memory_total_bytes > 0);
        assert!(stats.memory_used_bytes <= stats.memory_total_bytes);
        assert!(stats.disk_used_bytes <= stats.disk_total_bytes);
    }
}
