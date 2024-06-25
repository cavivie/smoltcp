use super::*;

use crate::socket::tcp::Socket;

impl InterfaceInner {
    pub(crate) fn process_tcp<'frame, 'socket, S: AnySocketSet<'socket>>(
        &mut self,
        sockets: &mut S,
        ip_repr: IpRepr,
        ip_payload: &'frame [u8],
    ) -> Option<Packet<'frame>> {
        let (src_addr, dst_addr) = (ip_repr.src_addr(), ip_repr.dst_addr());
        let tcp_packet = check!(TcpPacket::new_checked(ip_payload));
        let tcp_repr = check!(TcpRepr::parse(
            &tcp_packet,
            &src_addr,
            &dst_addr,
            &self.caps.checksum
        ));

        for tcp_socket in sockets
            .items_mut()
            .filter_map(|i| Socket::downcast_mut(&mut i.socket))
        {
            if tcp_socket.accepts(self, &ip_repr, &tcp_repr) {
                return tcp_socket
                    .process(self, &ip_repr, &tcp_repr)
                    .map(|(ip, tcp)| Packet::new(ip, IpPayload::Tcp(tcp)));
            }
        }

        // use crate::socket::tcp::{Socket, SocketBuffer};
        //
        // let mut sockets_to_remove = vec![];
        // for (tcp_socket, tcp_handle) in sockets
        //     .items_mut()
        //     .filter_map(|i| Socket::downcast_mut(&mut i.socket).map(|s| (s, i.meta.handle)))
        // {
        //     if tcp_socket.accepts(self, &ip_repr, &tcp_repr) {
        //         return tcp_socket
        //             .process(self, &ip_repr, &tcp_repr)
        //             .map(|(ip, tcp)| Packet::new(ip, IpPayload::Tcp(tcp)));
        //     }
        //     if tcp_socket.is_closed() {
        //         sockets_to_remove.push(tcp_handle);
        //     }
        // }
        // for socket in sockets_to_remove {
        //     sockets.remove(socket);
        // }
        //
        // let tcp_rx_buffer = SocketBuffer::new(vec![0; 64]);
        // let tcp_tx_buffer = SocketBuffer::new(vec![0; 128]);
        // let mut tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
        // tcp_socket.listen(tcp_repr.dst_port).ok()?;
        // if tcp_socket.accepts(self, &ip_repr, &tcp_repr) {
        //     let tcp_handle = sockets.add(tcp_socket);
        //     let tcp_socket = sockets.get_mut::<Socket>(tcp_handle);
        //     return tcp_socket
        //         .process(self, &ip_repr, &tcp_repr)
        //         .map(|(ip, tcp)| Packet::new(ip, IpPayload::Tcp(tcp)));
        // }

        if tcp_repr.control == TcpControl::Rst
            || ip_repr.dst_addr().is_unspecified()
            || ip_repr.src_addr().is_unspecified()
        {
            // Never reply to a TCP RST packet with another TCP RST packet. We also never want to
            // send a TCP RST packet with unspecified addresses.
            None
        } else {
            // The packet wasn't handled by a socket, send a TCP RST packet.
            let (ip, tcp) = tcp::Socket::rst_reply(&ip_repr, &tcp_repr);
            Some(Packet::new(ip, IpPayload::Tcp(tcp)))
        }
    }
}
