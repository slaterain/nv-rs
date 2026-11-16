//! Dialogue: JIP LN NVSE's "Store parent TESTopic for every TESTopicInfo" (step 3.9a).

use super::super::Entry;
use super::JIP;

/// The patch's seams, by address.
pub(in super::super) const SITES: &[Entry] = &[
    // TESTopicInfo's constructor 0061ccd0, the conditions' construction: JIP's InitTopicInfoHook
    // clears the parent topic word it adds (here: the port-side record of the info is dropped).
    Entry {
        addr: 0x0061_cd16,
        owners: JIP,
        step: "3.9a",
    },
    // TESTopicInfo's destructor body 0061ceb0, after ClearData: the port-side record of the info
    // dropped (JIP's word goes with the object; the port keeps the object's size).
    Entry {
        addr: 0x0061_ceee,
        owners: JIP,
        step: "3.9a",
    },
    // TESTopicInfo::ClearData 0061cff0, its call of GetOwnerTopic: JIP reads the parent topic
    // word instead of calling it.
    Entry {
        addr: 0x0061_d041,
        owners: JIP,
        step: "3.9a",
    },
    // TESTopicInfo::Load 0061dbd0, its call of TESTopic::InsertInfo: JIP's LoadTopicInfoHook
    // stores the topic in the info's word, then calls InsertInfo.
    Entry {
        addr: 0x0061_e086,
        owners: JIP,
        step: "3.9a",
    },
];

/// The patch's wraps, by address.
pub(in super::super) const WRAPS: &[Entry] = &[
    // TESTopic::GetOwnerTopic: rewritten by JIP to answer the info's parent topic word. Wrapped
    // after the topic index row's registration (NV_TOPIC_INDEX), which it never calls.
    Entry {
        addr: 0x0061_a310,
        owners: JIP,
        step: "3.9a",
    },
];
