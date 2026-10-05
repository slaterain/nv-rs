//! Calling a function on the real CPU and recovering from faults.
//!
//! `nv_call_raw` is a small assembly routine. Given a [`CallFrame`] it sets
//! the FPU control word and MXCSR, pushes the stack arguments, loads the
//! registers, calls the target, and copies back EAX, EDX, ST0 (when
//! asked), XMM0, the FPU status word and the stack pointer. The stack
//! pointer is always restored from a saved copy rather than computed, so it
//! does not matter whether the callee or the caller removes the arguments.
//!
//! A vectored exception handler turns CPU faults inside the call into a
//! recorded [`Fault`] and resumes at `nv_call_recover`, which resets the FPU
//! and returns to the Rust caller as if the call had ended. Calls to
//! unresolved imports go through a trap that does the same.

use nv_oracle_core::vectors::Fault;
use nv_win as win;
use std::arch::global_asm;
use std::mem::offset_of;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

/// Everything the assembly routine reads and writes. Field offsets are
/// passed to the assembly as constants, so the order here is free to change.
#[repr(C)]
pub struct CallFrame {
    // Inputs.
    pub target: u32,
    pub nstack: u32,
    pub stack: *const u32,
    /// EAX, ECX, EDX, EBX, ESI, EDI before the call.
    pub regs_in: [u32; 6],
    pub fpcw: u32,
    pub mxcsr: u32,
    pub xmm_in: [[u8; 16]; 4],
    pub want_st0: u32,
    // Outputs.
    pub eax: u32,
    pub edx: u32,
    pub fsw: u32,
    pub fcw: u32,
    pub mxcsr_out: u32,
    pub esp_before: u32,
    pub esp_after: u32,
    pub st0: [u8; 16],
    pub xmm0: [u8; 16],
}

impl CallFrame {
    pub fn new() -> CallFrame {
        CallFrame {
            target: 0,
            nstack: 0,
            stack: std::ptr::null(),
            regs_in: [0; 6],
            fpcw: 0x27f,
            mxcsr: 0x1f80,
            xmm_in: [[0; 16]; 4],
            want_st0: 0,
            eax: 0,
            edx: 0,
            fsw: 0,
            fcw: 0,
            mxcsr_out: 0,
            esp_before: 0,
            esp_after: 0,
            st0: [0; 16],
            xmm0: [0; 16],
        }
    }
}

// State shared between the assembly, the exception handler and Rust. The
// harness runs one call at a time on one thread, so plain atomics suffice.
static SAVED_ESP: AtomicU32 = AtomicU32::new(0);
static CALL_TARGET: AtomicU32 = AtomicU32::new(0);
static IN_CALL: AtomicBool = AtomicBool::new(false);
static TRAP_HIT: AtomicU32 = AtomicU32::new(0);
static TRAP_INDEX: AtomicU32 = AtomicU32::new(0);
static TRAP_CALLER: AtomicU32 = AtomicU32::new(0);
static FAULT_HIT: AtomicU32 = AtomicU32::new(0);
static FAULT_CODE: AtomicU32 = AtomicU32::new(0);
static FAULT_EIP: AtomicU32 = AtomicU32::new(0);
static FAULT_ADDR: AtomicU32 = AtomicU32::new(0);
/// 0 none, 1 read, 2 write, 3 execute.
static FAULT_ACCESS: AtomicU32 = AtomicU32::new(0);

global_asm!(
    ".text",
    ".globl _nv_call_raw",
    "_nv_call_raw:",
    "    push ebp",
    "    mov ebp, esp",
    "    push ebx",
    "    push esi",
    "    push edi",
    "    sub esp, 16",
    "    mov ebx, [ebp + 8]",
    "    mov [esp + 8], ebx",
    "    fnstcw word ptr [esp]",
    "    stmxcsr dword ptr [esp + 4]",
    "    mov dword ptr [{saved_esp}], esp",
    "    mov eax, [ebx + {off_target}]",
    "    mov dword ptr [{call_target}], eax",
    "    fninit",
    "    fldcw word ptr [ebx + {off_fpcw}]",
    "    ldmxcsr dword ptr [ebx + {off_mxcsr}]",
    "    movups xmm0, xmmword ptr [ebx + {off_xmm}]",
    "    movups xmm1, xmmword ptr [ebx + {off_xmm} + 16]",
    "    movups xmm2, xmmword ptr [ebx + {off_xmm} + 32]",
    "    movups xmm3, xmmword ptr [ebx + {off_xmm} + 48]",
    "    mov ecx, [ebx + {off_nstack}]",
    "    mov esi, [ebx + {off_stack}]",
    "2:",
    "    test ecx, ecx",
    "    jz 3f",
    "    push dword ptr [esi + ecx*4 - 4]",
    "    dec ecx",
    "    jmp 2b",
    "3:",
    "    mov [ebx + {off_esp_before}], esp",
    "    mov eax, [ebx + {off_regs}]",
    "    mov ecx, [ebx + {off_regs} + 4]",
    "    mov edx, [ebx + {off_regs} + 8]",
    "    mov esi, [ebx + {off_regs} + 16]",
    "    mov edi, [ebx + {off_regs} + 20]",
    "    mov ebx, [ebx + {off_regs} + 12]",
    "    call dword ptr [{call_target}]",
    // The callee returned. Find the frame again through the saved stack
    // pointer, because the callee may have used every register.
    "    mov ecx, dword ptr [{saved_esp}]",
    "    mov ecx, [ecx + 8]",
    "    mov [ecx + {off_eax}], eax",
    "    mov [ecx + {off_edx}], edx",
    "    mov [ecx + {off_esp_after}], esp",
    "    fnstsw ax",
    "    movzx eax, ax",
    "    mov [ecx + {off_fsw}], eax",
    "    fnstcw word ptr [ecx + {off_fcw}]",
    "    stmxcsr dword ptr [ecx + {off_mxcsr_out}]",
    "    movups xmmword ptr [ecx + {off_xmm0}], xmm0",
    "    cmp dword ptr [ecx + {off_want_st0}], 0",
    "    je 4f",
    // Only read ST0 if the stack is not empty (TOP is not zero).
    "    test eax, 0x3800",
    "    jz 4f",
    "    fstp tbyte ptr [ecx + {off_st0}]",
    "4:",
    ".globl _nv_call_recover",
    "_nv_call_recover:",
    "    cld",
    "    mov esp, dword ptr [{saved_esp}]",
    "    fninit",
    "    fldcw word ptr [esp]",
    "    ldmxcsr dword ptr [esp + 4]",
    "    add esp, 16",
    "    pop edi",
    "    pop esi",
    "    pop ebx",
    "    pop ebp",
    "    ret",
    // Every IAT slot points at a small stub that loads its import index into
    // EAX and jumps here.
    ".globl _nv_call_trap",
    "_nv_call_trap:",
    "    mov dword ptr [{trap_index}], eax",
    "    mov eax, [esp]",
    "    mov dword ptr [{trap_caller}], eax",
    "    mov dword ptr [{trap_hit}], 1",
    "    jmp _nv_call_recover",
    saved_esp = sym SAVED_ESP,
    call_target = sym CALL_TARGET,
    trap_index = sym TRAP_INDEX,
    trap_caller = sym TRAP_CALLER,
    trap_hit = sym TRAP_HIT,
    off_target = const offset_of!(CallFrame, target),
    off_nstack = const offset_of!(CallFrame, nstack),
    off_stack = const offset_of!(CallFrame, stack),
    off_regs = const offset_of!(CallFrame, regs_in),
    off_fpcw = const offset_of!(CallFrame, fpcw),
    off_mxcsr = const offset_of!(CallFrame, mxcsr),
    off_xmm = const offset_of!(CallFrame, xmm_in),
    off_want_st0 = const offset_of!(CallFrame, want_st0),
    off_eax = const offset_of!(CallFrame, eax),
    off_edx = const offset_of!(CallFrame, edx),
    off_fsw = const offset_of!(CallFrame, fsw),
    off_fcw = const offset_of!(CallFrame, fcw),
    off_mxcsr_out = const offset_of!(CallFrame, mxcsr_out),
    off_esp_before = const offset_of!(CallFrame, esp_before),
    off_esp_after = const offset_of!(CallFrame, esp_after),
    off_st0 = const offset_of!(CallFrame, st0),
    off_xmm0 = const offset_of!(CallFrame, xmm0),
);

extern "C" {
    fn nv_call_raw(frame: *mut CallFrame);
    fn nv_call_recover();
    fn nv_call_trap();
}

/// Address of the trap routine, for building import stubs.
pub fn trap_address() -> u32 {
    nv_call_trap as *const () as usize as u32
}

fn is_fault_code(code: u32) -> bool {
    matches!(
        code,
        0xC000_0005 // access violation
            | 0xC000_0006 // in-page error
            | 0xC000_001D // illegal instruction
            | 0xC000_0094 // integer divide by zero
            | 0xC000_0095 // integer overflow
            | 0xC000_0096 // privileged instruction
            | 0xC000_008C
            ..=0xC000_0093 // array bounds and FPU exceptions
            | 0xC000_00FD // stack overflow
            | 0x8000_0003 // breakpoint
    )
}

unsafe extern "system" fn fault_handler(ep: *mut win::ExceptionPointers) -> i32 {
    if !IN_CALL.load(Relaxed) {
        return 0; // EXCEPTION_CONTINUE_SEARCH
    }
    let rec = &*(*ep).exception_record;
    let ctx = &mut *(*ep).context_record;
    if !is_fault_code(rec.exception_code) {
        return 0;
    }
    FAULT_CODE.store(rec.exception_code, Relaxed);
    FAULT_EIP.store(ctx.eip, Relaxed);
    if rec.exception_code == 0xC000_0005 && rec.number_parameters >= 2 {
        FAULT_ACCESS.store(
            match rec.exception_information[0] {
                0 => 1,
                1 => 2,
                _ => 3,
            },
            Relaxed,
        );
        FAULT_ADDR.store(rec.exception_information[1] as u32, Relaxed);
    } else {
        FAULT_ACCESS.store(0, Relaxed);
        FAULT_ADDR.store(0, Relaxed);
    }
    FAULT_HIT.store(1, Relaxed);
    IN_CALL.store(false, Relaxed);
    ctx.eip = nv_call_recover as *const () as usize as u32;
    -1 // EXCEPTION_CONTINUE_EXECUTION
}

/// Install the fault handler. Call once before [`run`].
pub fn install_fault_handler() -> Result<(), String> {
    // SAFETY: registering a handler with a valid function pointer.
    let h = unsafe { win::AddVectoredExceptionHandler(1, Some(fault_handler)) };
    if h.is_null() {
        Err("AddVectoredExceptionHandler failed".into())
    } else {
        Ok(())
    }
}

/// What stopped a call early.
pub enum Stopped {
    Exception(Fault),
    /// Index of the import that was called.
    Import {
        index: usize,
        caller: u32,
    },
}

/// Run one call. Returns `None` if the callee returned normally, in which
/// case the output fields of `frame` are valid.
///
/// # Safety
/// `frame.target` must be an address in this process, and `frame.stack`
/// must point at `frame.nstack` readable dwords. The called code can do
/// anything the original program could; faults are caught, other damage is
/// not.
pub unsafe fn run(frame: &mut CallFrame) -> Option<Stopped> {
    FAULT_HIT.store(0, Relaxed);
    TRAP_HIT.store(0, Relaxed);
    IN_CALL.store(true, Relaxed);
    nv_call_raw(frame);
    IN_CALL.store(false, Relaxed);
    if TRAP_HIT.load(Relaxed) != 0 {
        return Some(Stopped::Import {
            index: TRAP_INDEX.load(Relaxed) as usize,
            caller: TRAP_CALLER.load(Relaxed),
        });
    }
    if FAULT_HIT.load(Relaxed) != 0 {
        let code = FAULT_CODE.load(Relaxed);
        let access = match FAULT_ACCESS.load(Relaxed) {
            1 => Some("read"),
            2 => Some("write"),
            3 => Some("execute"),
            _ => None,
        };
        return Some(Stopped::Exception(Fault::Exception {
            code,
            eip: FAULT_EIP.load(Relaxed),
            address: access.map(|_| FAULT_ADDR.load(Relaxed)),
            access,
        }));
    }
    None
}
