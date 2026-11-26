//! The platform's side of `WSOCK32.dll`, which the game imports by ordinal for one user only:
//! Havok's Visual Debugger server (`bhkWorld::SetVisualDebugger` 00c6a640, `hkVisualDebugger::serve`
//! 00cd0d00 on port 25001, the `hkBsdSocket` unit at 00da2db0..00da3230). The game turns it on
//! only when the `bHavokDebug:HAVOK` setting is set (`[HAVOK] bHavokDebug=1` in `Fallout.ini`, or
//! `SetINISetting` in the console) when a cell's physics world is made or attached.
//!
//! nv-rs offers no network: every call answers as Windows' Winsock does on a PC whose network
//! subsystem is not ready (the documented answers, Microsoft's Winsock reference):
//!
//! - `WSAStartup` (#115) returns `WSASYSNOTREADY` (10091) and leaves the `WSADATA` as it is;
//! - after a failed `WSAStartup` every socket call fails with `WSANOTINITIALISED` (10093) as its
//!   last error: `socket` (#23) and `accept` (#1) answer `INVALID_SOCKET`, `closesocket` (#3),
//!   `setsockopt` (#21), `bind` (#2), `listen` (#13), `gethostname` (#57) and `select` (#18)
//!   answer `SOCKET_ERROR`;
//! - `WSAGetLastError` (#111) answers the thread's last error (it is `GetLastError` on Windows);
//! - `htons` (#9) and `ntohs` (#15) swap the two bytes, as they do without `WSAStartup`.
//!
//! The game's answer to that (checked in the exe): `hkBsdNetworkInit` (00da2ff0) reports only a
//! result of -1, so 10091 goes by; the server's socket (00da2de0) is `INVALID_SOCKET`, so
//! `hkBsdSocket::listen` (00da30d0) answers 1 at once and `hkVisualDebugger::serve` goes on; each
//! frame's `pollForNewClient` (00da3230) answers no client for a socket of -1. The debugger
//! stays made and serves nobody, as on such a PC.

use crate::{Engine, Ptr};

/// `WSASYSNOTREADY`: the network subsystem is not ready for network communication.
pub const WSASYSNOTREADY: u32 = 10091;
/// `WSANOTINITIALISED`: a successful `WSAStartup` call must occur before this function.
pub const WSANOTINITIALISED: u32 = 10093;
/// `INVALID_SOCKET` (`~0`) and `SOCKET_ERROR` (-1).
pub const INVALID_SOCKET: u32 = 0xffff_ffff;
pub const SOCKET_ERROR: u32 = 0xffff_ffff;

// Platform: replaces WSOCK32.dll WSAStartup (import, ordinal 115)
/// `WSAStartup(version, data)`: `WSASYSNOTREADY`; `data` is not written and the last error is
/// left alone (`WSAStartup` returns its error, it does not set one). The log says once what asked
/// for the network, so a report shows what turned the debugger on.
pub fn wsa_startup(_e: &mut Engine, _version: u32, _data: Ptr) -> u32 {
    static SAID: std::sync::Once = std::sync::Once::new();
    SAID.call_once(|| {
        println!(
            "Havok's Visual Debugger is on (bHavokDebug=1 under [HAVOK] in Fallout.ini): nv-rs has no network for it, the game goes on without it"
        )
    });
    WSASYSNOTREADY
}

// Platform: replaces WSOCK32.dll WSAGetLastError (import, ordinal 111)
/// `WSAGetLastError()`: the calling thread's last error.
pub fn wsa_get_last_error(e: &mut Engine) -> u32 {
    e.kernel.last_error
}

/// A socket call made with no successful `WSAStartup`: the last error `WSANOTINITIALISED`, the
/// call's failure answer `fail`.
fn not_initialised(e: &mut Engine, fail: u32) -> u32 {
    e.kernel.last_error = WSANOTINITIALISED;
    fail
}

// Platform: replaces WSOCK32.dll socket (import, ordinal 23)
/// `socket(family, type, protocol)`: `INVALID_SOCKET`, `WSANOTINITIALISED`.
pub fn socket(e: &mut Engine, _family: u32, _kind: u32, _protocol: u32) -> u32 {
    not_initialised(e, INVALID_SOCKET)
}

// Platform: replaces WSOCK32.dll accept (import, ordinal 1)
/// `accept(s, address, length)`: `INVALID_SOCKET`, `WSANOTINITIALISED`; nothing written.
pub fn accept(e: &mut Engine, _s: u32, _address: Ptr, _length: Ptr) -> u32 {
    not_initialised(e, INVALID_SOCKET)
}

// Platform: replaces WSOCK32.dll closesocket (import, ordinal 3)
/// `closesocket(s)`: `SOCKET_ERROR`, `WSANOTINITIALISED`.
pub fn close_socket(e: &mut Engine, _s: u32) -> u32 {
    not_initialised(e, SOCKET_ERROR)
}

// Platform: replaces WSOCK32.dll setsockopt (import, ordinal 21)
/// `setsockopt(s, level, name, value, length)`: `SOCKET_ERROR`, `WSANOTINITIALISED`.
pub fn set_socket_option(
    e: &mut Engine,
    _s: u32,
    _level: u32,
    _name: u32,
    _value: Ptr,
    _length: u32,
) -> u32 {
    not_initialised(e, SOCKET_ERROR)
}

// Platform: replaces WSOCK32.dll bind (import, ordinal 2)
/// `bind(s, address, length)`: `SOCKET_ERROR`, `WSANOTINITIALISED`.
pub fn bind(e: &mut Engine, _s: u32, _address: Ptr, _length: u32) -> u32 {
    not_initialised(e, SOCKET_ERROR)
}

// Platform: replaces WSOCK32.dll listen (import, ordinal 13)
/// `listen(s, backlog)`: `SOCKET_ERROR`, `WSANOTINITIALISED`.
pub fn listen(e: &mut Engine, _s: u32, _backlog: u32) -> u32 {
    not_initialised(e, SOCKET_ERROR)
}

// Platform: replaces WSOCK32.dll gethostname (import, ordinal 57)
/// `gethostname(buffer, length)`: `SOCKET_ERROR`, `WSANOTINITIALISED`; nothing written.
pub fn get_host_name(e: &mut Engine, _buffer: Ptr, _length: u32) -> u32 {
    not_initialised(e, SOCKET_ERROR)
}

// Platform: replaces WSOCK32.dll select (import, ordinal 18)
/// `select(count, read, write, except, timeout)`: `SOCKET_ERROR`, `WSANOTINITIALISED`; the
/// sets are left as they are.
pub fn select(
    e: &mut Engine,
    _count: u32,
    _read: Ptr,
    _write: Ptr,
    _except: Ptr,
    _timeout: Ptr,
) -> u32 {
    not_initialised(e, SOCKET_ERROR)
}

// Platform: replaces WSOCK32.dll htons (import, ordinal 9)
/// `htons(value)`: the low 16 bits with their two bytes swapped (x86 is little-endian).
pub fn host_to_net_short(_e: &mut Engine, value: u32) -> u32 {
    u32::from((value as u16).swap_bytes())
}

// Platform: replaces WSOCK32.dll ntohs (import, ordinal 15)
/// `ntohs(value)`: as [`host_to_net_short`].
pub fn net_to_host_short(_e: &mut Engine, value: u32) -> u32 {
    u32::from((value as u16).swap_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's slots for the imports above (the exe's import table, `WSOCK32.dll` by ordinal).
    const SLOTS: [(u32, &str); 12] = [
        (0x00fd_f364, "#115"),
        (0x00fd_f344, "#111"),
        (0x00fd_f340, "#23"),
        (0x00fd_f384, "#1"),
        (0x00fd_f33c, "#3"),
        (0x00fd_f374, "#21"),
        (0x00fd_f370, "#2"),
        (0x00fd_f36c, "#13"),
        (0x00fd_f368, "#57"),
        (0x00fd_f37c, "#18"),
        (0x00fd_f35c, "#9"),
        (0x00fd_f388, "#15"),
    ];

    fn engine() -> Engine {
        crate::imports();
        let mut e = Engine::new();
        let imports: Vec<_> = SLOTS
            .iter()
            .map(|&(slot, name)| engine_core::exe::Import {
                slot,
                dll: "WSOCK32.dll".into(),
                name: name.into(),
            })
            .collect();
        assert_eq!(e.register_imports(&imports), SLOTS.len());
        e
    }

    #[test]
    fn startup_fails_as_without_a_network_and_sockets_are_not_initialised() {
        let mut e = engine();
        let data = e.mem.alloc(0x190);
        e.mem.set_u32(data, 0x1234_5678);
        e.kernel.last_error = 5;
        assert_eq!(e.call(0x00fd_f364, &[0x202, data]).u32(), WSASYSNOTREADY);
        assert_eq!(e.mem.u32(data), 0x1234_5678, "WSADATA is not written");
        assert_eq!(
            e.call(0x00fd_f344, &[]).u32(),
            5,
            "WSAStartup sets no error"
        );
        assert_eq!(e.call(0x00fd_f340, &[2, 1, 0]).u32(), INVALID_SOCKET);
        assert_eq!(e.call(0x00fd_f344, &[]).u32(), WSANOTINITIALISED);
        e.kernel.last_error = 0;
        assert_eq!(e.call(0x00fd_f384, &[7, 0, 0]).u32(), INVALID_SOCKET);
        assert_eq!(e.kernel.last_error, WSANOTINITIALISED);
        let fails: [(u32, &[u32]); 6] = [
            (0x00fd_f33c, &[7]),
            (0x00fd_f374, &[7, 0xffff, 4, data, 4]),
            (0x00fd_f370, &[7, data, 0x10]),
            (0x00fd_f36c, &[7, 2]),
            (0x00fd_f368, &[data, 0x100]),
            (0x00fd_f37c, &[8, data, 0, data, data]),
        ];
        for (slot, args) in fails {
            e.kernel.last_error = 0;
            assert_eq!(e.call(slot, args).u32(), SOCKET_ERROR, "{slot:08x}");
            assert_eq!(e.kernel.last_error, WSANOTINITIALISED, "{slot:08x}");
        }
        assert_eq!(
            e.mem.u32(data),
            0x1234_5678,
            "no call writes the caller's memory"
        );
    }

    #[test]
    fn byte_order_swaps_the_low_sixteen_bits() {
        let mut e = engine();
        assert_eq!(e.call(0x00fd_f35c, &[0x61a9]).u32(), 0xa961);
        assert_eq!(e.call(0x00fd_f388, &[0xdead_a961]).u32(), 0x61a9);
    }
}
