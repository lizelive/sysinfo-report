use ::serde::{Deserialize, Serialize};
use local_ip_address::list_afinet_netifas;

pub const KIBIBYTE: u64 = 1024;
use std::{
    collections::HashMap,
    net::{IpAddr, ToSocketAddrs},
};

//pub type Bytes = u64;

use bytes::Bytes;
use sysinfo::{CpuExt, DiskExt, System, SystemExt, UserExt};

#[derive(Serialize, Deserialize, Debug)]
pub struct Disk {
    pub kind: DiskKind,
    pub name: String,
    pub file_system: Bytes,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
}

pub type DnsResult = Result<Vec<IpAddr>, String>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Report {
    pub host_name: Option<String>,
    pub disks: Vec<Disk>,
    pub memory: MemoryReport,
    pub processor: Processor,
    pub processors: Vec<Processor>,
    pub uptime: u64,
    pub users: Vec<User>,
    pub networks: Vec<String>,
    pub os: OperatingSystem,
    pub kernel: Kernel,
    pub dns_test: HashMap<String, DnsResult>,
    pub network_interfaces: Result<Vec<NetworkInterface>, String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: IpAddr,
}

impl Default for Report {
    fn default() -> Self {
        get_report()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum DiskKind {
    /// HDD type.
    HardDiskDrive,
    /// SSD type.
    SolidStateDrive,
    /// Unknown type.
    Unknown(isize),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MemoryReport {
    pub memory: Memory,
    pub swap: Memory,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Memory {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

#[cfg(test)]
mod tests {
    use crate::get_report;

    #[test]
    fn get_local_report() {
        let report = get_report();
        println!("{:#?}", report);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Processor {
    pub name: String,
    pub vendor_id: String,
    pub brand: String,
}

pub type Uid = u32;
pub type Gid = u32;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub uid: Uid,
    pub gid: Gid,
    pub name: String,
    pub groups: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug)]

pub struct OperatingSystem {
    pub name: Option<String>,
    pub version: Option<String>,
    pub long_version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Kernel {
    pub version: Option<String>,
}

pub const TEST_DNS_FOR: &[&str] = &["localhost", "ghcr.io", "docker.io", "mcr.microsoft.com"];

fn convert_user(#[allow(unused_variables)] user: &sysinfo::User) -> User {
    #[cfg(unix)]
    return User {
        groups: user.groups().into(),
        name: user.name().into(),
        gid: (*user.group_id()),
        uid: (**user.id()),
    };

    #[cfg(windows)]
    return User {
        groups: user.groups().into(),
        name: user.name().into(),
        gid: 1000,
        uid: 1000,
    };

    #[allow(unreachable_code)]
    {
        unimplemented!("cannot convert user on this platform");
    }
}

pub fn get_report() -> Report {
    let sys = System::new_all();

    let os = OperatingSystem {
        name: sys.name(),
        version: sys.os_version(),
        long_version: sys.long_os_version(),
    };

    let kernel = Kernel {
        version: sys.kernel_version(),
    };

    let host_name = sys.host_name();

    let memory = MemoryReport {
        memory: Memory {
            total: sys.total_memory() * KIBIBYTE,
            used: sys.used_memory() * KIBIBYTE,
            free: sys.free_memory() * KIBIBYTE,
        },
        swap: Memory {
            total: sys.total_swap() * KIBIBYTE,
            used: sys.used_swap() * KIBIBYTE,
            free: sys.free_swap() * KIBIBYTE,
        },
    };

    let processor = sys.global_cpu_info();

    let processor = Processor {
        brand: processor.brand().into(),
        name: processor.name().into(),
        vendor_id: processor.vendor_id().into(),
    };

    let processors: Vec<_> = sys
        .cpus()
        .iter()
        .map(|processor| Processor {
            brand: processor.brand().into(),
            name: processor.name().into(),
            vendor_id: processor.vendor_id().into(),
        })
        .collect();

    let uptime = sys.uptime();

    let users = sys.users();

    let users: Vec<_> = users.iter().map(convert_user).collect();

    let networks: Vec<_> = sys
        .networks()
        .into_iter()
        .map(|(interface_name, _)| interface_name.to_owned())
        .collect();

    let dns_test = TEST_DNS_FOR
        .iter()
        .map(|host_name| {
            (
                host_name.to_string(),
                (*host_name, 443)
                    .to_socket_addrs()
                    .map(|x| x.map(|socket_addr| socket_addr.ip()).collect())
                    .map_err(|err| err.to_string()),
            )
        })
        .collect();

    // println!("{:#?}", dns_test);

    let disks = sys
        .disks()
        .iter()
        .map(|disk| Disk {
            available_space: disk.available_space(),
            kind: match disk.kind() {
                ::sysinfo::DiskKind::HDD => DiskKind::HardDiskDrive,
                ::sysinfo::DiskKind::SSD => DiskKind::SolidStateDrive,
                ::sysinfo::DiskKind::Unknown(kind) => DiskKind::Unknown(kind),
            },
            name: disk.name().to_string_lossy().into(),
            file_system: Bytes::copy_from_slice(disk.file_system()),
            mount_point: disk.mount_point().to_string_lossy().into(),
            total_space: disk.total_space(),
            is_removable: disk.is_removable(),
        })
        .collect();

    let network_interfaces = list_afinet_netifas()
        .map_err(|err| err.to_string())
        .map(|ok| {
            ok.into_iter()
                .map(|(name, ip)| NetworkInterface { ip, name })
                .collect()
        });

    Report {
        disks,
        host_name,
        memory,
        processor,
        processors,
        uptime,
        users,
        networks,
        os,
        kernel,
        dns_test,
        network_interfaces,
    }
}
