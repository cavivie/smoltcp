mod utils;

use log::debug;
use std::fmt::Write;
use std::os::unix::io::AsRawFd;

use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::phy::{wait as phy_wait, Device, Medium};
use smoltcp::socket::{tcp, udp, Socket};
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
            .push(IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24))
            .unwrap();
        ip_addrs
            .push(IpCidr::new(IpAddress::v6(0xfdaa, 0, 0, 0, 0, 0, 0, 1), 64))
            .unwrap();
        ip_addrs
            .push(IpCidr::new(IpAddress::v6(0xfe80, 0, 0, 0, 0, 0, 0, 1), 64))
            .unwrap();
    });
    iface
        .routes_mut()
        .add_default_ipv4_route(Ipv4Address::new(192, 168, 69, 100))
        .unwrap();
    iface
        .routes_mut()
        .add_default_ipv6_route(Ipv6Address::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x100))
        .unwrap();

    // // Create sockets
    // let udp_rx_buffer = udp::PacketBuffer::new(
    //     vec![udp::PacketMetadata::EMPTY, udp::PacketMetadata::EMPTY],
    //     vec![0; 65535],
    // );
    // let udp_tx_buffer = udp::PacketBuffer::new(
    //     vec![udp::PacketMetadata::EMPTY, udp::PacketMetadata::EMPTY],
    //     vec![0; 65535],
    // );
    // let udp_socket = udp::Socket::new(udp_rx_buffer, udp_tx_buffer);

    // let tcp1_rx_buffer = tcp::SocketBuffer::new(vec![0; 64]);
    // let tcp1_tx_buffer = tcp::SocketBuffer::new(vec![0; 128]);
    // let tcp1_socket = tcp::Socket::new(tcp1_rx_buffer, tcp1_tx_buffer);

    // let tcp_backlog = 5;
    // let mut tcp2_sockets = vec![];
    // for _ in 0..tcp_backlog {
    //     let tcp2_rx_buffer = tcp::SocketBuffer::new(vec![0; 64]);
    //     let tcp2_tx_buffer = tcp::SocketBuffer::new(vec![0; 128]);
    //     let tcp2_socket = tcp::Socket::new(tcp2_rx_buffer, tcp2_tx_buffer);
    //     tcp2_sockets.push(tcp2_socket);
    // }

    // let mut sockets = SocketSet::new(vec![]);
    // let udp_handle = sockets.add(udp_socket);
    // let tcp1_handle = sockets.add(tcp1_socket);
    // struct TcpHandle {
    //     handle: SocketHandle,
    //     active: bool,
    // }
    // let mut tcp2_handles = vec![];
    // for tcp2_socket in tcp2_sockets {
    //     let tcp2_handle = sockets.add(tcp2_socket);
    //     tcp2_handles.push(TcpHandle {
    //         handle: tcp2_handle,
    //         active: false,
    //     });
    // }

    let mut sockets = SocketSet::new(vec![]);

    loop {
        let timestamp = Instant::now();
        iface.poll(timestamp, &mut device, &mut sockets);

        // // udp:6969: respond "hello"
        // let socket = sockets.get_mut::<udp::Socket>(udp_handle);
        // if !socket.is_open() {
        //     socket.bind(6969).unwrap()
        // }

        // let client = match socket.recv() {
        //     Ok((data, endpoint)) => {
        //         debug!("udp:6969 recv data: {:?} from {}", data, endpoint);
        //         let mut data = data.to_vec();
        //         data.reverse();
        //         Some((endpoint, data))
        //     }
        //     Err(_) => None,
        // };
        // if let Some((endpoint, data)) = client {
        //     debug!("udp:6969 send data: {:?} to {}", data, endpoint,);
        //     socket.send_slice(&data, endpoint).unwrap();
        // }

        println!("{:?}", sockets.iter().count());
        for (socket_handle, socket) in sockets.iter_mut() {
            match socket {
                Socket::Udp(socket) => todo!(),
                Socket::Tcp(socket) => {
                    // tcp:6969: respond "hello"
                    if socket.can_send() {
                        debug!("tcp:6969 send greeting");
                        writeln!(socket, "hello").unwrap();
                        debug!("tcp:6969 close");
                        socket.close();
                    }
                }
                _ => todo!(),
            }
        }

        phy_wait(fd, iface.poll_delay(timestamp, &sockets)).expect("wait error");
    }
}
