//! The counted-pointer holder the maps keep their values in (`NiPointer<T>`, Xbox PDB).

use super::field_ptr;
use super::layout::{Holder, RefObject};
use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// `NiRefObject::DecRefCount` (Xbox PDB), folded for every counted class: `thiscall` on the object.
const RELEASE: u32 = 0x0044_dd60;
/// The interlocked increment (`cdecl`, the word's address; answers the new count).
const INTERLOCKED_INCREMENT: u32 = 0x0040_b460;

// Typed from 0092c820
/// `NiPointer<T>::operator=` from another holder (Xbox PDB name of the template; one body for
/// every `T`): when the two hold different objects, releases the held one (`0044dd60`), takes the
/// other's and counts it, as `0092c870` does, with an interlocked increment of its count at +8.
/// The other holder is read again after the release, which may free what it points into. Answers
/// `this`.
pub fn assign(e: &mut Engine, this: Ptr<Holder>, other: Ptr<Holder>) -> Ptr<Holder> {
    // Either holder may be a node's (the family's own, moved in stage 2) or a caller's.
    let s = super::store::store(e);
    if s.get(e, this, Holder::object) != s.get(e, other, Holder::object) {
        let held = s.get(e, this, Holder::object);
        if !held.is_null() {
            e.call(RELEASE, &args![held]);
        }
        let taken = s.get(e, other, Holder::object);
        s.set(e, this, Holder::object, taken);
        let held = s.get(e, this, Holder::object);
        if !held.is_null() {
            e.call(
                INTERLOCKED_INCREMENT,
                &args![field_ptr(held, RefObject::m_uiRefCount)],
            );
        }
    }
    this
}
