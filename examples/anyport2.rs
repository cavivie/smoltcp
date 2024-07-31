mod utils;

use log::debug;
use std::fmt::Write;
use std::os::unix::io::AsRawFd;

use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{wait as phy_wait, Device, Medium};
use smoltcp::socket::{tcp, udp};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address};

fn main() {
    utils::setup_logging("");

    let (mut opts, mut free) = utils::create_options();
    utils::add_tuntap_options(&mut opts, &mut free);
    utils::add_middleware_options(&mut opts, &mut free);

    let mut matches = utils::parse_options(&opts, free);
    let device = utils::parse_tuntap_options(&mut matches);
    let fd = device.as_raw_fd();
    let mut device =
        utils::parse_middleware_options(&mut matches, device, /*loopback=*/ false);

    // Create interface
    let mut config = match device.capabilities().medium {
        Medium::Ethernet => {
            Config::new(EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]).into())
        }
        Medium::Ip => Config::new(smoltcp::wire::HardwareAddress::Ip),
        Medium::Ieee802154 => todo!(),
    };

    config.random_seed = rand::random();

    let mut iface = Interface::new(config, &mut device, Instant::now());
    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(IpCidr::new(IpAddress::v4(0, 0, 0, 1), 0))
            .expect("iface IPv4");
        ip_addrs
            .push(IpCidr::new(IpAddress::v6(0, 0, 0, 0, 0, 0, 0, 1), 0))
            .expect("iface IPv6");
    });
    iface
        .routes_mut()
        .add_default_ipv4_route(Ipv4Address::new(0, 0, 0, 1))
        .expect("IPv4 default route");
    iface
        .routes_mut()
        .add_default_ipv6_route(Ipv6Address::new(0, 0, 0, 0, 0, 0, 0, 1))
        .expect("IPv6 default route");
    iface.set_any_ip(true);
    // iface.update_ip_addrs(|ip_addrs| {
    //     ip_addrs
    //         .push(IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24))
    //         .unwrap();
    //     ip_addrs
    //         .push(IpCidr::new(IpAddress::v6(0xfdaa, 0, 0, 0, 0, 0, 0, 1), 64))
    //         .unwrap();
    //     ip_addrs
    //         .push(IpCidr::new(IpAddress::v6(0xfe80, 0, 0, 0, 0, 0, 0, 1), 64))
    //         .unwrap();
    // });
    // iface
    //     .routes_mut()
    //     .add_default_ipv4_route(Ipv4Address::new(192, 168, 69, 100))
    //     .unwrap();
    // iface
    //     .routes_mut()
    //     .add_default_ipv6_route(Ipv6Address::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x100))
    //     .unwrap();

    // Create sockets
    let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 64]);
    let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 128]);
    let mut tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
    tcp_socket.set_any_port(true);

    // prioritize matching port 6969, any port should be added last
    let mut sockets = SocketSet::new(vec![]);
    let mut tcp_handle = sockets.add(tcp_socket);

    let mut socks_map = std::collections::HashMap::new();
    let mut tcp_any_port_active = false;
    loop {
        let timestamp = Instant::now();
        iface.poll(timestamp, &mut device, &sockets);

        let cur_tcp_handle = tcp_handle;

        // tcp:*: echo with reverse
        {
            let mut socket = sockets.get_mut::<tcp::Socket>(cur_tcp_handle);
            if !socket.is_open() {
                // set any port can recv any tcp connection
                socket.listen(0).unwrap();
            }
    
            if socket.is_active() && !tcp_any_port_active {
                debug!("tcp:* connected");
                match (
                    socket.state(),
                    socket.local_endpoint(),
                    socket.remote_endpoint(),
                ) {
                    (tcp::State::SynReceived, Some(local), Some(remote))
                        if !socks_map.contains_key(&(local, remote)) =>
                    {
                        debug!("tcp recv 4-tuple {:?}", (local, remote));
                        // Create sockets
                        let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 64]);
                        let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 128]);
                        let mut tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
                        tcp_socket.set_any_port(true);
                        drop(socket);
                        tcp_handle = sockets.add(tcp_socket);
                        socks_map.insert((local, remote), tcp_handle);
                    }
                    _ => unreachable!(),
                }
            } else if !socket.is_active() && tcp_any_port_active {
                debug!("tcp:* disconnected");
            }
            let mut socket = sockets.get_mut::<tcp::Socket>(cur_tcp_handle);
            tcp_any_port_active = socket.is_active();
    
            let local = socket.local_endpoint();
            if socket.may_recv() {
                let data = socket
                    .recv(|buffer| {
                        let recvd_len = buffer.len();
                        let mut data = buffer.to_owned();
                        if !data.is_empty() {
                            debug!("tcp:*{:?} recv data: {:?}", local, data);
                            data = data.split(|&b| b == b'\n').collect::<Vec<_>>().concat();
                            data.reverse();
                            data.extend(b"\n");
                        }
                        (recvd_len, data)
                    })
                    .unwrap();
                if socket.can_send() && !data.is_empty() {
                    debug!("tcp:* send data: {:?}", data);
                    socket.send_slice(&data[..]).unwrap();
                }
            } else if socket.may_send() {
                debug!("tcp:* close");
                socket.close();
            }
            // drop(socket);
        }

        phy_wait(fd, iface.poll_delay(timestamp, &sockets)).expect("wait error");
    }
}
