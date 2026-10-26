//! How long the network takes to answer: an echo to an address, and a DNS lookup, timed. Pings go
//! out through an unprivileged ICMP socket, which Linux allows to the groups in
//! `net.ipv4.ping_group_range`.

use std::{
    io, mem,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const ECHO_REQUEST: u8 = 8;
const ECHO_REPLY: u8 = 0;
const PAYLOAD: &[u8] = b"island network";
const DNS_PORT: u16 = 53;

fn checksum(bytes: &[u8]) -> u16 {
    let mut sum: u32 = bytes
        .chunks(2)
        .map(|pair| u32::from(u16::from_be_bytes([pair[0], *pair.get(1).unwrap_or(&0)])))
        .sum();
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

/// An echo request; the kernel sets its identifier for a ping socket.
pub fn echo(sequence: u16) -> Vec<u8> {
    let mut packet = vec![ECHO_REQUEST, 0, 0, 0, 0, 0];
    packet.extend_from_slice(&sequence.to_be_bytes());
    packet.extend_from_slice(PAYLOAD);
    let sum = checksum(&packet);
    packet[2..4].copy_from_slice(&sum.to_be_bytes());
    packet
}

/// Whether `packet` answers the echo numbered `sequence`.
pub fn is_reply(packet: &[u8], sequence: u16) -> bool {
    packet.len() >= 8 && packet[0] == ECHO_REPLY && packet[6..8] == sequence.to_be_bytes()
}

struct Fd(libc::c_int);

impl Drop for Fd {
    fn drop(&mut self) {
        // SAFETY: the descriptor is owned here and closed once.
        unsafe {
            libc::close(self.0);
        }
    }
}

fn socket(timeout: Duration) -> io::Result<Fd> {
    // SAFETY: plain socket calls on a descriptor this function owns.
    unsafe {
        let fd = libc::socket(
            libc::AF_INET,
            libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
            libc::IPPROTO_ICMP,
        );
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let fd = Fd(fd);
        let wait = libc::timeval {
            tv_sec: timeout.as_secs() as libc::time_t,
            tv_usec: timeout.subsec_micros() as libc::suseconds_t,
        };
        if libc::setsockopt(
            fd.0,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            (&raw const wait).cast(),
            mem::size_of::<libc::timeval>() as libc::socklen_t,
        ) < 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(fd)
    }
}

/// Milliseconds until `address` answered, or `None` when it did not within `timeout`.
pub fn ping(address: Ipv4Addr, sequence: u16, timeout: Duration) -> Option<f32> {
    let fd = socket(timeout).ok()?;
    let packet = echo(sequence);
    // SAFETY: the address and buffers live for the length of each call.
    unsafe {
        let mut to: libc::sockaddr_in = mem::zeroed();
        to.sin_family = libc::AF_INET as libc::sa_family_t;
        to.sin_addr.s_addr = u32::from_ne_bytes(address.octets());
        let started = Instant::now();
        if libc::sendto(
            fd.0,
            packet.as_ptr().cast(),
            packet.len(),
            0,
            (&raw const to).cast(),
            mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        ) < 0
        {
            return None;
        }
        let mut buffer = [0u8; 256];
        while started.elapsed() < timeout {
            let read = libc::recv(fd.0, buffer.as_mut_ptr().cast(), buffer.len(), 0);
            if read < 0 {
                return None;
            }
            if is_reply(&buffer[..read as usize], sequence) {
                return Some(started.elapsed().as_secs_f32() * 1000.0);
            }
        }
    }
    None
}

/// A query for the A record of `name`.
pub fn query(id: u16, name: &str) -> Vec<u8> {
    let mut packet = Vec::with_capacity(32);
    packet.extend_from_slice(&id.to_be_bytes());
    // Recursion desired, one question.
    packet.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
    for label in name.split('.').filter(|label| !label.is_empty()) {
        packet.push(label.len().min(63) as u8);
        packet.extend_from_slice(&label.as_bytes()[..label.len().min(63)]);
    }
    packet.extend_from_slice(&[0, 0, 1, 0, 1]);
    packet
}

/// Milliseconds until `server` answered a lookup of `name`.
pub fn lookup(server: Ipv4Addr, name: &str, timeout: Duration) -> Option<f32> {
    let socket = UdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))).ok()?;
    socket.set_read_timeout(Some(timeout)).ok()?;
    socket.connect(SocketAddr::from((server, DNS_PORT))).ok()?;
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos() as u16);
    let started = Instant::now();
    socket.send(&query(id, name)).ok()?;
    let mut buffer = [0u8; 512];
    while started.elapsed() < timeout {
        let read = socket.recv(&mut buffer).ok()?;
        if read >= 2 && buffer[..2] == id.to_be_bytes() {
            return Some(started.elapsed().as_secs_f32() * 1000.0);
        }
    }
    None
}

#[cfg(test)]
#[path = "probe_test.rs"]
mod tests;
