#![allow(unsafe_code)]

use std::io;
use std::mem::{size_of, size_of_val};
use std::os::unix::io::{AsRawFd, RawFd};

use crate::{
    phy::{sys::*, Medium},
    wire::EthernetFrame,
};

#[derive(Debug)]
pub struct TunTapInterfaceDesc {
    lower: libc::c_int,
    medium: Medium,
}

impl AsRawFd for TunTapInterfaceDesc {
    fn as_raw_fd(&self) -> RawFd {
        self.lower
    }
}

impl TunTapInterfaceDesc {
    pub fn new(name: &str, medium: Medium) -> io::Result<TunTapInterfaceDesc> {
        match medium {
            #[cfg(feature = "medium-ip")]
            Medium::Ip => {}
            #[cfg(feature = "medium-ethernet")]
            Medium::Ethernet => todo!(),
            #[cfg(feature = "medium-ieee802154")]
            Medium::Ieee802154 => todo!(),
        }

        let utun_id = Self::parse_utun_name(name)?;

        let lower = match unsafe {
            libc::socket(libc::PF_SYSTEM, libc::SOCK_DGRAM, libc::SYSPROTO_CONTROL)
        } {
            -1 => return Err(io::Error::last_os_error()),
            lower => lower,
        };

        Self::set_non_blocking(lower)?;
        Self::attach_interface(lower, utun_id)?;

        Self::start(&Self::interface_name(lower)?);

        Ok(TunTapInterfaceDesc { lower, medium })
    }

    pub fn from_fd(fd: RawFd) -> io::Result<TunTapInterfaceDesc> {
        Ok(TunTapInterfaceDesc {
            lower: fd,
            medium: Medium::Ip,
        })
    }

    // On Darwin tunnel can only be named utun[0-9]+.
    fn parse_utun_name(name: &str) -> io::Result<u32> {
        if !name.starts_with("utun") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "tun name must be named utun[0-9]+",
            ));
        }
        match name.get(4..) {
            // The name is simply "utun"
            None | Some("") => Ok(0),
            // Everything past utun should represent an integer index
            Some(idx) => idx
                .parse::<u32>()
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "tun name must be named utun[0-9]+",
                    )
                })
                .map(|x| x + 1),
        }
    }

    /// Starts the tunnel
    fn start(name: &str) {
        use std::net::IpAddr;
        use std::process::Command;

        // Assign the ipv4 address to the interface
        Command::new("ifconfig")
            .args(&[name, "192.168.69.100", "192.168.69.100", "alias"])
            .status()
            .expect("failed to assign ip to tunnel");

        // Assign the ipv6 address to the interface
        // Command::new("ifconfig")
        //     .args(&[name, "inet6", "addr_v6", "prefixlen", "128", "alias"])
        //     .status()
        //     .expect("failed to assign ipv6 to tunnel");

        // Start the tunnel
        Command::new("ifconfig")
            .args(&[name, "up"])
            .status()
            .expect("failed to start the tunnel");

        // Add each peer to the routing table
        Command::new("route")
            .args(&[
                "-q",
                "-n",
                "add",
                "-inet",
                "192.168.69.100/24",
                "-interface",
                name,
            ])
            .status()
            .expect("failed to add route");
    }

    fn set_non_blocking(lower: libc::c_int) -> io::Result<()> {
        match unsafe { libc::fcntl(lower, libc::F_GETFL) } {
            -1 => Err(io::Error::last_os_error()),
            flags => match unsafe { libc::fcntl(lower, libc::F_SETFL, flags | libc::O_NONBLOCK) } {
                -1 => Err(io::Error::last_os_error()),
                _ => Ok(()),
            },
        }
    }

    fn attach_interface(lower: libc::c_int, utun_id: u32) -> io::Result<()> {
        let mut info = ctl_info {
            ctl_id: 0,
            ctl_name: [0u8; 96],
        };
        info.ctl_name[..UTUN_CONTROL_NAME.len()].copy_from_slice(UTUN_CONTROL_NAME);

        if unsafe { libc::ioctl(lower, CTLIOCGINFO, &mut info as *mut ctl_info) } < 0 {
            unsafe { libc::close(lower) };
            return Err(io::Error::last_os_error());
        }

        let addr = libc::sockaddr_ctl {
            sc_len: size_of::<libc::sockaddr_ctl>() as u8,
            sc_family: libc::AF_SYSTEM as _,
            ss_sysaddr: libc::AF_SYS_CONTROL as _,
            sc_id: info.ctl_id,
            sc_unit: utun_id,
            sc_reserved: Default::default(),
        };
        if unsafe {
            libc::connect(
                lower,
                &addr as *const libc::sockaddr_ctl as _,
                size_of_val(&addr) as _,
            )
        } < 0
        {
            unsafe { libc::close(lower) };
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn interface_name(fd: libc::c_int) -> io::Result<String> {
        let mut tunnel_name = [0u8; 256];
        let mut tunnel_name_len: libc::socklen_t = tunnel_name.len() as u32;
        if unsafe {
            libc::getsockopt(
                fd,
                libc::SYSPROTO_CONTROL,
                libc::UTUN_OPT_IFNAME,
                tunnel_name.as_mut_ptr() as _,
                &mut tunnel_name_len,
            )
        } < 0
            || tunnel_name_len == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(String::from_utf8_lossy(&tunnel_name[..(tunnel_name_len - 1) as usize]).to_string())
    }

    pub fn interface_mtu(&self) -> io::Result<usize> {
        let mut ifreq = ifreq_for(&Self::interface_name(self.lower)?);

        let fd = match unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, libc::IPPROTO_IP) } {
            -1 => return Err(io::Error::last_os_error()),
            fd => fd,
        };

        let res = ifreq_ioctl(fd, &mut ifreq, SIOCGIFMTU);
        unsafe { libc::close(fd) };
        // Propagate error after close, to ensure we always close.
        res?;

        let ip_mtu = unsafe { ifreq.ifr_ifru.ifru_mtu as usize };

        // SIOCGIFMTU returns the IP MTU (typically 1500 bytes.)
        // smoltcp counts the entire Ethernet packet in the MTU, so add the Ethernet header size to it.
        let mtu = match self.medium {
            #[cfg(feature = "medium-ip")]
            Medium::Ip => ip_mtu,
            #[cfg(feature = "medium-ethernet")]
            Medium::Ethernet => ip_mtu + EthernetFrame::<&[u8]>::header_len(),
            #[cfg(feature = "medium-ieee802154")]
            Medium::Ieee802154 => todo!(),
        };

        Ok(mtu)
    }

    pub fn recv(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        // [IFF_NO_PI] skip packet infromation
        let mut hdr = [0u8; 4];

        let mut iov = [
            libc::iovec {
                iov_base: hdr.as_mut_ptr() as _,
                iov_len: hdr.len(),
            },
            libc::iovec {
                iov_base: buffer.as_mut_ptr() as _,
                iov_len: buffer.len(),
            },
        ];

        let mut msg_hdr = libc::msghdr {
            msg_name: std::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: &mut iov[0],
            msg_iovlen: iov.len() as _,
            msg_control: std::ptr::null_mut(),
            msg_controllen: 0,
            msg_flags: 0,
        };

        match unsafe { libc::recvmsg(self.lower, &mut msg_hdr, 0) } {
            -1 => Err(io::Error::last_os_error()),
            0..=4 => Ok(0),
            n => Ok((n - 4) as usize),
        }
    }

    pub fn send(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let mut hdr = [0u8, 0u8, 0u8, libc::AF_INET as u8];
        let mut iov = [
            libc::iovec {
                iov_base: hdr.as_mut_ptr() as _,
                iov_len: hdr.len(),
            },
            libc::iovec {
                iov_base: buffer.as_ptr() as _,
                iov_len: buffer.len(),
            },
        ];

        let msg_hdr = libc::msghdr {
            msg_name: std::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: &mut iov[0],
            msg_iovlen: iov.len() as _,
            msg_control: std::ptr::null_mut(),
            msg_controllen: 0,
            msg_flags: 0,
        };

        match unsafe { libc::sendmsg(self.lower, &msg_hdr, 0) } {
            -1 => Err(io::Error::last_os_error()),
            n => Ok(n as usize),
        }
    }
}

impl Drop for TunTapInterfaceDesc {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.lower);
        }
    }
}
