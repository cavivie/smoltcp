use crate::time::Duration;
use std::os::unix::io::RawFd;
use std::{io, mem, ptr};

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "linux.rs"]
mod imp;

#[cfg(any(target_os = "macos", target_os = "ios"))]
#[path = "darwin.rs"]
mod imp;

pub(crate) use self::imp::*;

/// Wait until given file descriptor becomes readable, but no longer than given timeout.
#[cfg(any(feature = "phy-tuntap_interface", feature = "phy-raw_socket"))]
pub fn wait(fd: RawFd, duration: Option<Duration>) -> io::Result<()> {
    unsafe {
        let mut readfds = {
            let mut readfds = mem::MaybeUninit::<libc::fd_set>::uninit();
            libc::FD_ZERO(readfds.as_mut_ptr());
            libc::FD_SET(fd, readfds.as_mut_ptr());
            readfds.assume_init()
        };

        let mut writefds = {
            let mut writefds = mem::MaybeUninit::<libc::fd_set>::uninit();
            libc::FD_ZERO(writefds.as_mut_ptr());
            writefds.assume_init()
        };

        let mut exceptfds = {
            let mut exceptfds = mem::MaybeUninit::<libc::fd_set>::uninit();
            libc::FD_ZERO(exceptfds.as_mut_ptr());
            exceptfds.assume_init()
        };

        let mut timeout = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let timeout_ptr = if let Some(duration) = duration {
            timeout.tv_sec = duration.secs() as libc::time_t;
            timeout.tv_usec = (duration.millis() * 1_000) as libc::suseconds_t;
            &mut timeout as *mut _
        } else {
            ptr::null_mut()
        };

        let res = libc::select(
            fd + 1,
            &mut readfds,
            &mut writefds,
            &mut exceptfds,
            timeout_ptr,
        );
        if res == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}
