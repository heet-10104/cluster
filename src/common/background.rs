use sysinfo::{CpuRefreshKind, Disks, Networks, RefreshKind, System};

#[derive(Debug)]
pub struct Metrics {
    cpu: Vec<f32>,
    memory: f32,
    swap: f32,
    disk: u64,
    network: [u64; 2],
}

impl Metrics {
    pub fn new(sys: &mut System) -> Metrics {
        let mut cpu_usage: Vec<f32> = Vec::new();
        let mut s = System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
        );
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        s.refresh_cpu_all();
        for cpu in sys.cpus() {
            cpu_usage.push(cpu.cpu_usage());
        }

        let memory_usage = sys.used_memory() as f32 / sys.total_memory() as f32;
        let swap_usage = sys.used_swap() as f32 / sys.total_swap() as f32;

        let mut disk_usage = 0;
        let disks = Disks::new_with_refreshed_list();
        for disk in &disks {
            disk_usage += disk.total_space() - disk.available_space();
        }

        Metrics {
            cpu: cpu_usage,
            memory: memory_usage,
            swap: swap_usage,
            disk: disk_usage,
            network: [0, 0],
        }
    }

    fn update(&mut self, sys: &mut System, system_info: &mut SystemInfo) -> &mut Self {
        let mut cpu_usage: Vec<f32> = Vec::new();
        sys.refresh_cpu_all();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_all();
        for cpu in sys.cpus() {
            cpu_usage.push(cpu.cpu_usage());
        }

        let memory_usage = sys.used_memory() as f32 / sys.total_memory() as f32;
        let swap_usage = sys.used_swap() as f32 / sys.total_swap() as f32;

        let mut disk_usage = 0;
        let disks = Disks::new_with_refreshed_list();
        for disk in &disks {
            disk_usage += disk.total_space() - disk.available_space();
        }

        let (down, up) = net_speed(system_info);

        self.network = [down / 2, up / 2];
        self.cpu = cpu_usage;
        self.memory = memory_usage;
        self.swap = swap_usage;
        self.disk = disk_usage;

        return self;
    }
}

#[derive(Debug)]
pub struct SystemInfo {
    name: String,
    kernel: String,
    os: String,
    hostname: String,
    num_cpus: u32,
    memory: u64,
    disk: u64,
    network: Networks,
}

impl SystemInfo {
    pub fn new(sys: &mut System) -> SystemInfo {
        let mut disk_total = 0;
        let disks = Disks::new_with_refreshed_list();
        for disk in &disks {
            disk_total += disk.total_space();
        }

        SystemInfo {
            name: System::name().expect("Failed to get system name"),
            kernel: System::kernel_version().expect("Failed to get kernel version"),
            os: System::os_version().expect("Failed to get os version"),
            hostname: System::host_name().expect("Failed to get hostname"),
            num_cpus: sys.cpus().len() as u32,
            memory: sys.total_memory(),
            disk: disk_total,
            network: Networks::new_with_refreshed_list(),
        }
    }
}

pub fn dynamic_metrics(sys: &mut System, metrics: &mut Metrics, system_info: &mut SystemInfo) {
    let metrics = metrics.update(sys, system_info);
    println!("Metrics: {:#?}", metrics);
}

fn net_speed(system_info: &mut SystemInfo) -> (u64, u64) {
    let networks = &mut system_info.network;
    let mut rx = 0;
    let mut tx = 0;
    for (_, data) in networks.iter() {
        rx += data.received();
        tx += data.transmitted();
    }
    networks.refresh(true);

    return (rx, tx);
}
