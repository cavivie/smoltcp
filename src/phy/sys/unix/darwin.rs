#![allow(unused)]
#![allow(non_camel_case_types)]

pub(crate) const CTLIOCGINFO: u64 = 0x0000_0000_c064_4e03;
pub(crate) const SIOCGIFMTU: u64 = 0x0000_0000_c020_6933;
pub(crate) const UTUN_CONTROL_NAME: &[u8] = b"com.apple.net.utun_control";

#[cfg(any(feature = "phy-tuntap_interface", feature = "phy-raw_socket"))]
#[repr(C)]
pub union ifr_ifru {
    // pub ifru_addr: libc::sockaddr,
    // pub ifru_addr_v4: libc::sockaddr_in,
    // pub ifru_addr_v6: libc::sockaddr_in,
    // pub ifru_dstaddr: libc::sockaddr,
    // pub ifru_broadaddr: libc::sockaddr,
    // pub ifru_flags: libc::c_short,
    // pub ifru_metric: libc::c_int,
    pub ifru_mtu: libc::c_int,
    // pub ifru_phys: libc::c_int,
    // pub ifru_media: libc::c_int,
    // pub ifru_intval: libc::c_int,
    // pub ifru_data: caddr_t,
    // pub ifru_devmtu: ifdevmtu,
    // pub ifru_kpi: ifkpi,
    // pub ifru_wake_flags: u32,
    // pub ifru_route_refcnt: u32,
    // pub ifru_cap: [libc::c_int; 2],
    // pub ifru_functional_type: u32,
}

impl std::fmt::Debug for ifr_ifru {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[cfg(any(feature = "phy-tuntap_interface", feature = "phy-raw_socket"))]
#[repr(C)]
#[derive(Debug)]
pub(crate) struct ifreq {
    pub ifr_name: [libc::c_uchar; libc::IF_NAMESIZE],
    pub ifr_ifru: ifr_ifru,
}

#[cfg(any(feature = "phy-tuntap_interface", feature = "phy-raw_socket"))]
#[repr(C)]
#[derive(Debug)]
pub struct ctl_info {
    pub ctl_id: u32,
    pub ctl_name: [libc::c_uchar; 96],
}

#[cfg(any(feature = "phy-tuntap_interface", feature = "phy-raw_socket"))]
pub(crate) fn ifreq_for(name: &str) -> ifreq {
    let mut ifreq = ifreq {
        ifr_name: [0; libc::IF_NAMESIZE],
        ifr_ifru: unsafe { std::mem::zeroed() },
    };
    ifreq.ifr_name[..name.len()].copy_from_slice(name.as_bytes());
    ifreq
}

#[cfg(any(feature = "phy-tuntap_interface", feature = "phy-raw_socket"))]
pub(crate) fn ifreq_ioctl(
    lower: libc::c_int,
    ifreq: &mut ifreq,
    cmd: libc::c_ulong,
) -> std::io::Result<()> {
    unsafe {
        let res = libc::ioctl(lower, cmd as _, ifreq as *mut ifreq);
        if res == -1 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}
