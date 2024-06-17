#![allow(unused)]
#![allow(non_camel_case_types)]

pub const SIOCGIFMTU: libc::c_ulong = 0x8921;
pub const SIOCGIFINDEX: libc::c_ulong = 0x8933;
pub const ETH_P_ALL: libc::c_short = 0x0003;
pub const ETH_P_IEEE802154: libc::c_short = 0x00F6;

// Constant definition as per
// https://github.com/golang/sys/blob/master/unix/zerrors_linux_<arch>.go
pub const TUNSETIFF: libc::c_ulong = if cfg!(any(
    target_arch = "mips",
    target_arch = "mips64",
    target_arch = "mips64el",
    target_arch = "mipsel",
    target_arch = "powerpc",
    target_arch = "powerpc64",
    target_arch = "powerpc64le",
    target_arch = "sparc64"
)) {
    0x800454CA
} else {
    0x400454CA
};
pub const IFF_TUN: libc::c_int = 0x0001;
pub const IFF_TAP: libc::c_int = 0x0002;
pub const IFF_NO_PI: libc::c_int = 0x1000;

#[cfg(any(feature = "phy-tuntap_interface", feature = "phy-raw_socket"))]
#[repr(C)]
#[derive(Debug)]
pub(crate) struct ifreq {
    ifr_name: [libc::c_uchar; libc::IF_NAMESIZE], /* ifr_ifname, e.g.: eth0 */
    ifr_data: libc::c_int,                        /* ifr_ifindex or ifr_mtu */
}

#[cfg(any(feature = "phy-tuntap_interface", feature = "phy-raw_socket"))]
pub(crate) fn ifreq_for(name: &str) -> ifreq {
    let mut ifreq = ifreq {
        ifr_name: [0; libc::IF_NAMESIZE],
        ifr_data: 0,
    };
    ifreq.ifr_name[..name.len()].copy_from_slice(name.as_bytes());
    ifreq
}

#[cfg(any(feature = "phy-tuntap_interface", feature = "phy-raw_socket"))]
pub(crate) fn ifreq_ioctl(
    lower: libc::c_int,
    ifreq: &mut ifreq,
    cmd: libc::c_ulong,
) -> std::io::Result<libc::c_int> {
    unsafe {
        let res = libc::ioctl(lower, cmd as _, ifreq as *mut ifreq);
        if res == -1 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(ifreq.ifr_data)
}
