use std::time::Duration;

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

    let memory_used_bytes = system.used_memory();
    let memory_total_bytes = system.total_memory();

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

    #[test]
    fn collects_real_system_stats() {
        let stats = collect_system_stats();

        assert!(stats.cpu_usage_percent >= 0.0);
        assert!(stats.memory_total_bytes > 0);
        assert!(stats.memory_used_bytes <= stats.memory_total_bytes);
        assert!(stats.disk_used_bytes <= stats.disk_total_bytes);
    }
}
