use crate::abi::{AbiFn, Arg, Ret, RetVal};
use crate::mem::Mem;
use crate::ptr::{Field, Layout, Ptr, Scalar};
use std::collections::HashMap;
use std::path::Path;

/// The game's memory plus every translated function, by exe address.
pub struct Engine {
    pub mem: Mem,
    funcs: HashMap<u32, AbiFn>,
    /// When `Some`, every call made through [`Engine::call`] is recorded
    /// as (address, argument words). Tests use it to check what a
    /// translation called.
    pub call_log: Option<Vec<(u32, Vec<u32>)>>,
    tls: u32,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Empty memory, every translated unit registered.
    pub fn new() -> Self {
        let mut list = Vec::new();
        crate::units::funcs(&mut list);
        let mut funcs = HashMap::with_capacity(list.len());
        for (addr, f) in list {
            if funcs.insert(addr, f).is_some() {
                panic!("{addr:08x} is registered twice");
            }
        }
        Engine {
            mem: Mem::new(),
            funcs,
            call_log: None,
            tls: 0,
        }
    }

    /// [`Engine::new`] with the data sections of the player's FalloutNV.exe
    /// mapped ([`crate::exe::map_data_sections`]).
    pub fn with_exe(exe: &Path) -> Result<Self, String> {
        let mut e = Self::new();
        crate::exe::map_data_sections(&mut e.mem, exe)?;
        Ok(e)
    }

    /// Address of the main thread's TLS block: what the game reaches as
    /// `*(*(FS:[0x2c]) + _tls_index * 4)`, so a field the game reads at
    /// `+0x2b4` of it is at `e.tls() + 0x2b4`. One thread: the game's
    /// threads are `platform`. Allocated (0x1000 bytes, zeroed, then the
    /// exe's `.tls` template when mapped) on first use.
    pub fn tls(&mut self) -> u32 {
        if self.tls == 0 {
            self.tls = self.mem.alloc(0x1000);
            if self.mem.is_mapped(0x0127_2000) {
                let template = self.mem.bytes(0x0127_2000, 0x2c5);
                self.mem.write(self.tls, &template);
            }
        }
        self.tls
    }

    /// Number of registered functions.
    pub fn translated_count(&self) -> usize {
        self.funcs.len()
    }

    pub fn is_translated(&self, addr: u32) -> bool {
        self.funcs.contains_key(&addr)
    }

    /// Registers (or replaces) the function at `addr`. Tests use this for
    /// doubles of callees that are not under test.
    pub fn register(&mut self, addr: u32, f: AbiFn) {
        self.funcs.insert(addr, f);
    }

    /// Calls the function at exe address `addr` with argument words in
    /// the uniform form ([`crate::abi`]). Panics if nobody has translated
    /// it yet: the translation that called it can only be checked once
    /// that callee exists (or a test supplies a double).
    pub fn call(&mut self, addr: u32, args: &[u32]) -> Ret {
        if let Some(log) = &mut self.call_log {
            log.push((addr, args.to_vec()));
        }
        let f = *self.funcs.get(&addr).unwrap_or_else(|| {
            panic!("open function {addr:08x}: not translated yet (FalloutNV.exe 1.4.0.525, docs/LEDGER.md)")
        });
        f(self, args)
    }

    /// [`Engine::call`] with the result converted.
    pub fn call_as<R: RetVal>(&mut self, addr: u32, args: &[u32]) -> R {
        R::from_ret(self.call(addr, args))
    }

    /// Virtual call: reads the vtable pointer at `this + 0` and the
    /// function at byte offset `slot` in that table (as the game's
    /// `call dword ptr [eax + slot]`), then calls it with `this` first.
    pub fn vcall(&mut self, this: u32, slot: u32, args: &[u32]) -> Ret {
        let vtable = self.mem.u32(this);
        let target = self.mem.u32(vtable.wrapping_add(slot));
        let mut words = Vec::with_capacity(args.len() + 1);
        words.push(this);
        words.extend_from_slice(args);
        self.call(target, &words)
    }

    /// Field read through a typed pointer.
    pub fn get<S, T: Scalar>(&self, p: Ptr<S>, f: Field<S, T>) -> T {
        T::load(&self.mem, p.0.wrapping_add(f.off))
    }

    /// Field write through a typed pointer.
    pub fn set<S, T: Scalar>(&mut self, p: Ptr<S>, f: Field<S, T>, v: T) {
        v.store(&mut self.mem, p.0.wrapping_add(f.off))
    }

    /// A global variable at its exe address (in `.data`/`.rdata`).
    pub fn global<T: Scalar>(&self, addr: u32) -> T {
        T::load(&self.mem, addr)
    }

    pub fn set_global<T: Scalar>(&mut self, addr: u32, v: T) {
        v.store(&mut self.mem, addr)
    }

    /// A zeroed heap block the size of `T` (tests and allocators).
    pub fn new_object<T: Layout>(&mut self) -> Ptr<T> {
        Ptr::new(self.mem.alloc(T::SIZE))
    }

    /// Maps zeroed memory at a fixed address (a global or a vtable in a
    /// test that runs without the executable).
    pub fn map(&mut self, addr: u32, len: u32) {
        self.mem.map(addr, len)
    }

    /// Writes a vtable (slot targets) at `addr`; for tests without the
    /// executable. With it mapped, the game's own vtables are in memory.
    pub fn put_vtable(&mut self, addr: u32, slots: &[u32]) {
        self.mem.map(addr, 4 * slots.len() as u32);
        for (i, s) in slots.iter().enumerate() {
            self.mem.set_u32(addr + 4 * i as u32, *s);
        }
    }

    /// Typed argument helper for [`Engine::call`]: `e.call(a, &words![..])`
    /// is usually written with the `args!` macro; this is the one-argument
    /// form.
    pub fn call1<A: Arg, R: RetVal>(&mut self, addr: u32, a: A) -> R {
        let mut w = Vec::with_capacity(A::WORDS);
        a.put(&mut w);
        self.call_as(addr, &w)
    }
}
