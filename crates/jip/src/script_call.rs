//! JIP's `CallFunction` (`internal/jip_core.h` (JIP LN NVSE)): a script function called through
//! xNVSE's script interface (`NVSEScriptInterface::CallFunction`, an `InternalFunctionCaller`
//! with the arguments as raw words), on a reference or none. The main-loop callbacks and the LN
//! events call their scripts through this one place; the crate's tests record the calls here
//! (xNVSE's call itself is `nvse::functions`' and tested there).

use engine_core::prelude::*;

/// Calls `script` on `this_obj` (0 for none) with `args` (an integer, a form's address, ...).
#[cfg(not(test))]
pub fn call(e: &mut Engine, script: u32, this_obj: u32, args: Vec<u32>) {
    nvse::functions::call(
        e,
        nvse::functions::Caller::internal(script, this_obj, 0, args),
    );
}

#[cfg(test)]
thread_local! {
    static CALLS: std::cell::RefCell<Vec<(u32, u32, Vec<u32>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// The tests' stand-in: the call recorded.
#[cfg(test)]
pub fn call(_: &mut Engine, script: u32, this_obj: u32, args: Vec<u32>) {
    CALLS.with(|c| c.borrow_mut().push((script, this_obj, args)));
}

/// The calls recorded since the last look (script, reference, arguments).
#[cfg(test)]
pub fn calls() -> Vec<(u32, u32, Vec<u32>)> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}
