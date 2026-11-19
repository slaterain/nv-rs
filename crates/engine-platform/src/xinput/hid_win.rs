//! PlayStation pads on Windows, found through Windows' HID: a thread looks through the HID devices
//! present every two seconds, opens each one it has not seen without access to read its vendor
//! and product id, and opens each new Sony pad twice (read, and write) for [`HidPads::add`]. Before
//! that it reads the pad's feature reports ([`sony::feature_reports`]), which turns on a Bluetooth
//! DualSense's full reports and gives its firmware's rumble. A device that is not a pad is not
//! opened again while it stays plugged in.
//!
//! The other pads (a joystick's or a gamepad's top usage) that are not Xbox-style pads (those
//! XInput reads: an `IG_` in the device's path, as SDL tells them) are read through the
//! controller database's line for them ([`super::sdl_db`]): opened once, for reading, their
//! inputs counted as SDL's DirectInput code counts them ([`super::dinput_order`]) from Windows'
//! HID parser's description of the pad, each report read through that parser. A pad with no line
//! gets one log line naming its ids and is not read: nothing is guessed. These pads get no rumble.

use super::dinput_order::{self, Element, Kind, Layout};
use super::hid::{HidPads, Opened};
use super::layout::Controls;
use super::sdl_db::{Database, Mapping};
use super::sony::{self, Model};
use std::collections::{HashMap, HashSet};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::time::Duration;

/// `GUID_DEVINTERFACE_HID` {4D1E55B2-F16F-11CF-88CB-001111000030} (hidclass.h).
const HID_INTERFACE: windows::core::GUID =
    windows::core::GUID::from_u128(0x4d1e55b2_f16f_11cf_88cb_001111000030);
/// `FILE_SHARE_READ | FILE_SHARE_WRITE`: other programs (Steam, a pad's own tool) keep theirs.
const SHARE_READ_WRITE: u32 = 0x1 | 0x2;
/// How often the device list is looked through for new pads.
const LOOK_EVERY: Duration = Duration::from_secs(2);

/// Starts the thread that looks for PlayStation pads and the pads `db` has lines for; the pads it
/// finds join the answer.
pub fn start(db: Database) -> HidPads {
    let pads = HidPads::new();
    let found = pads.clone();
    let _ = std::thread::Builder::new()
        .name("nv-rs pads".into())
        .spawn(move || {
            let mut not_pads = HashSet::new();
            loop {
                look_for_pads(&found, &mut not_pads, &db);
                std::thread::sleep(LOOK_EVERY);
            }
        });
    pads
}

/// The top usages DirectInput takes as game controllers: Generic Desktop's joystick, gamepad and
/// multi-axis controller.
fn is_game_controller(usage: (u16, u16)) -> bool {
    matches!(usage, (0x01, 0x04) | (0x01, 0x05) | (0x01, 0x08))
}

/// Whether XInput reads the device at `path` (Windows names an Xbox-style pad's HID device with
/// `IG_`, which SDL looks for too).
fn xinput_reads(path: &str) -> bool {
    path.to_ascii_uppercase().contains("IG_")
}

/// Windows' HID parser's description of a pad, freed when dropped.
struct Preparsed(windows::Win32::Devices::HumanInterfaceDevice::PHIDP_PREPARSED_DATA);

// SAFETY: the parsed description is read only, by one thread at a time (the pad's reader).
unsafe impl Send for Preparsed {}

impl Drop for Preparsed {
    fn drop(&mut self) {
        // SAFETY: the data HidD_GetPreparsedData gave, freed once.
        unsafe {
            let _ = windows::Win32::Devices::HumanInterfaceDevice::HidD_FreePreparsedData(self.0);
        }
    }
}

impl Preparsed {
    fn of(file: &std::fs::File) -> Option<Preparsed> {
        use windows::Win32::Devices::HumanInterfaceDevice::{
            HidD_GetPreparsedData, PHIDP_PREPARSED_DATA,
        };
        use windows::Win32::Foundation::HANDLE;
        let mut data = PHIDP_PREPARSED_DATA::default();
        // SAFETY: an open HID handle; the data is freed by the drop.
        unsafe { HidD_GetPreparsedData(HANDLE(file.as_raw_handle()), &mut data) }
            .as_bool()
            .then_some(Preparsed(data))
    }

    /// The pad's inputs: each usage of its input button and value capabilities.
    fn elements(&self) -> Vec<Element> {
        use windows::Win32::Devices::HumanInterfaceDevice::{
            HidP_GetButtonCaps, HidP_GetCaps, HidP_GetValueCaps, HidP_Input, HIDP_BUTTON_CAPS,
            HIDP_CAPS, HIDP_STATUS_SUCCESS, HIDP_VALUE_CAPS,
        };
        let mut caps = HIDP_CAPS::default();
        // SAFETY: the parsed description.
        if unsafe { HidP_GetCaps(self.0, &mut caps) } != HIDP_STATUS_SUCCESS {
            return Vec::new();
        }
        let mut elements = Vec::new();
        let mut n = caps.NumberInputButtonCaps;
        let mut buttons = vec![HIDP_BUTTON_CAPS::default(); usize::from(n)];
        // SAFETY: a buffer of `n` capabilities.
        if n > 0
            && unsafe { HidP_GetButtonCaps(HidP_Input, buttons.as_mut_ptr(), &mut n, self.0) }
                == HIDP_STATUS_SUCCESS
        {
            for b in &buttons[..usize::from(n)] {
                // SAFETY: the union's variant is the one `IsRange` says.
                let (usages, first) = unsafe {
                    if b.IsRange.as_bool() {
                        let r = b.Anonymous.Range;
                        (r.UsageMin..=r.UsageMax, r.DataIndexMin)
                    } else {
                        let u = b.Anonymous.NotRange;
                        (u.Usage..=u.Usage, u.DataIndex)
                    }
                };
                for (i, usage) in usages.enumerate() {
                    elements.push(Element {
                        page: b.UsagePage,
                        usage,
                        index: first.wrapping_add(i as u16),
                        kind: Kind::Button,
                    });
                }
            }
        }
        let mut n = caps.NumberInputValueCaps;
        let mut values = vec![HIDP_VALUE_CAPS::default(); usize::from(n)];
        // SAFETY: a buffer of `n` capabilities.
        if n > 0
            && unsafe { HidP_GetValueCaps(HidP_Input, values.as_mut_ptr(), &mut n, self.0) }
                == HIDP_STATUS_SUCCESS
        {
            for v in &values[..usize::from(n)] {
                // SAFETY: as above.
                let (usages, first) = unsafe {
                    if v.IsRange.as_bool() {
                        let r = v.Anonymous.Range;
                        (r.UsageMin..=r.UsageMax, r.DataIndexMin)
                    } else {
                        let u = v.Anonymous.NotRange;
                        (u.Usage..=u.Usage, u.DataIndex)
                    }
                };
                let (min, max) = dinput_order::logical_range(v.LogicalMin, v.LogicalMax, v.BitSize);
                for (i, usage) in usages.enumerate() {
                    elements.push(Element {
                        page: v.UsagePage,
                        usage,
                        index: first.wrapping_add(i as u16),
                        kind: Kind::Value {
                            min,
                            max,
                            bits: v.BitSize,
                        },
                    });
                }
            }
        }
        elements
    }
}

/// A pad read through its database line: its inputs as SDL counts them, their last values (a
/// report with another report id leaves an input as it was).
struct Mapped {
    data: Preparsed,
    layout: Layout,
    mapping: Mapping,
    pages: Vec<u16>,
    pressed: HashSet<(u16, u16)>,
    values: HashMap<(u16, u16), u32>,
    usages: Vec<u16>,
}

impl Mapped {
    fn new(file: &std::fs::File, mapping: Mapping) -> Option<Mapped> {
        let data = Preparsed::of(file)?;
        let layout = Layout::of(&data.elements());
        let mut pages: Vec<u16> = layout.buttons.iter().map(|&(page, _)| page).collect();
        pages.sort_unstable();
        pages.dedup();
        Some(Mapped {
            usages: vec![0; layout.buttons.len().max(1)],
            data,
            layout,
            mapping,
            pages,
            pressed: HashSet::new(),
            values: HashMap::new(),
        })
    }

    /// One input report's controls; `None` when it carries none of the pad's inputs.
    fn read(&mut self, report: &[u8]) -> Option<Controls> {
        use windows::Win32::Devices::HumanInterfaceDevice::{
            HidP_GetUsageValue, HidP_GetUsages, HidP_Input, HIDP_STATUS_SUCCESS,
        };
        let mut report = report.to_vec();
        let mut carried = false;
        for &page in &self.pages {
            let mut n = self.usages.len() as u32;
            // SAFETY: a usage buffer of `n`; the parsed description and a whole report.
            let status = unsafe {
                HidP_GetUsages(
                    HidP_Input,
                    page,
                    0,
                    self.usages.as_mut_ptr(),
                    &mut n,
                    self.data.0,
                    &mut report,
                )
            };
            if status == HIDP_STATUS_SUCCESS {
                carried = true;
                self.pressed.retain(|&(p, _)| p != page);
                for &usage in &self.usages[..(n as usize).min(self.usages.len())] {
                    self.pressed.insert((page, usage));
                }
            }
        }
        for v in self.layout.axes.iter().chain(&self.layout.hats) {
            let mut raw = 0u32;
            // SAFETY: as above.
            let status = unsafe {
                HidP_GetUsageValue(
                    HidP_Input,
                    v.page,
                    0,
                    v.usage,
                    &mut raw,
                    self.data.0,
                    &report,
                )
            };
            if status == HIDP_STATUS_SUCCESS {
                carried = true;
                self.values.insert((v.page, v.usage), raw);
            }
        }
        if !carried {
            return None;
        }
        let joystick = self.layout.joystick(
            |page, usage| self.pressed.contains(&(page, usage)),
            |page, usage| self.values.get(&(page, usage)).copied(),
        );
        Some(self.mapping.controls(&joystick))
    }
}

/// Opens and runs the pad at `path` through its line, if it is a game controller that XInput does
/// not read and has one; `false` when it is passed over (to be looked at again only when plugged
/// in again).
fn add_mapped(pads: &HidPads, path: &str, device: &Device, db: &Database) -> bool {
    if !is_game_controller(device.usage) || xinput_reads(path) {
        return false;
    }
    let Some(mapping) = db.mapping(device.vendor, device.product) else {
        eprintln!(
            "nv-rs: game pads: the pad {:04x}:{:04x} is not read: the controller database has no \
             line for it (one can be added in {} in the port's own folder)",
            device.vendor,
            device.product,
            super::sdl_db::FILE
        );
        return false;
    };
    let Ok(reader) = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(SHARE_READ_WRITE)
        .open(path)
    else {
        // Tried again at the next look.
        return true;
    };
    let Some(mut mapped) = Mapped::new(&reader, mapping) else {
        return false;
    };
    let _ = pads.add_mapped(path.to_string(), reader, device.input, move |report| {
        mapped.read(report)
    });
    true
}

/// The paths of the HID device interfaces present (`CM_Get_Device_Interface_ListW`).
fn hid_paths() -> Vec<String> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_Device_Interface_ListW, CM_Get_Device_Interface_List_SizeW,
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CR_SUCCESS,
    };
    // The list can grow between the two calls: a few tries.
    for _ in 0..3 {
        let mut len = 0u32;
        // SAFETY: a valid GUID and out pointer; no device id.
        let r = unsafe {
            CM_Get_Device_Interface_List_SizeW(
                &mut len,
                &HID_INTERFACE,
                windows::core::PCWSTR::null(),
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if r != CR_SUCCESS || len == 0 {
            return Vec::new();
        }
        let mut buffer = vec![0u16; len as usize];
        // SAFETY: the buffer is the length the call takes.
        let r = unsafe {
            CM_Get_Device_Interface_ListW(
                &HID_INTERFACE,
                windows::core::PCWSTR::null(),
                &mut buffer,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if r == CR_SUCCESS {
            return buffer
                .split(|&c| c == 0)
                .filter(|s| !s.is_empty())
                .map(String::from_utf16_lossy)
                .collect();
        }
    }
    Vec::new()
}

/// What a HID device says about itself: its vendor and product id, its top usage (page, usage)
/// and its longest input, output and feature report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Device {
    vendor: u16,
    product: u16,
    usage: (u16, u16),
    input: usize,
    output: usize,
    feature: usize,
}

/// A HID device's [`Device`], `None` when it does not answer.
fn identify(file: &std::fs::File) -> Option<Device> {
    use windows::Win32::Devices::HumanInterfaceDevice::{
        HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetPreparsedData, HidP_GetCaps,
        HIDD_ATTRIBUTES, HIDP_CAPS, HIDP_STATUS_SUCCESS, PHIDP_PREPARSED_DATA,
    };
    use windows::Win32::Foundation::HANDLE;
    let handle = HANDLE(file.as_raw_handle());
    let mut attributes = HIDD_ATTRIBUTES {
        Size: std::mem::size_of::<HIDD_ATTRIBUTES>() as u32,
        ..Default::default()
    };
    // SAFETY: an open HID handle and a sized structure.
    if !unsafe { HidD_GetAttributes(handle, &mut attributes) }.as_bool() {
        return None;
    }
    let mut data = PHIDP_PREPARSED_DATA::default();
    // SAFETY: as above; the data is freed below.
    if !unsafe { HidD_GetPreparsedData(handle, &mut data) }.as_bool() {
        return None;
    }
    let mut caps = HIDP_CAPS::default();
    // SAFETY: the preparsed data just read.
    let status = unsafe { HidP_GetCaps(data, &mut caps) };
    // SAFETY: freed once.
    unsafe {
        let _ = HidD_FreePreparsedData(data);
    }
    (status == HIDP_STATUS_SUCCESS).then_some(Device {
        vendor: attributes.VendorID,
        product: attributes.ProductID,
        usage: (caps.UsagePage, caps.Usage),
        input: usize::from(caps.InputReportByteLength),
        output: usize::from(caps.OutputReportByteLength),
        feature: usize::from(caps.FeatureReportByteLength),
    })
}

/// The device at `path`, opened without access (enough for its ids, allowed for every HID device,
/// a keyboard's and a mouse's too) and closed again.
fn probe(path: &str) -> Option<Device> {
    let file = std::fs::OpenOptions::new()
        .access_mode(0)
        .share_mode(SHARE_READ_WRITE)
        .open(path)
        .ok()?;
    identify(&file)
}

/// One look through the device list: each Sony pad and each pad with a database line not yet
/// open is opened and run. `not_pads` holds the devices seen that are not read, while they stay
/// plugged in.
fn look_for_pads(pads: &HidPads, not_pads: &mut HashSet<String>, db: &Database) {
    pads.forget_gone();
    let paths = hid_paths();
    not_pads.retain(|p| paths.contains(p));
    for path in paths {
        if not_pads.contains(&path) || pads.knows(&path) {
            continue;
        }
        let Some(device) = probe(&path) else {
            continue;
        };
        let pad = sony::found(
            device.vendor,
            device.product,
            device.usage,
            device.input,
            device.output,
        );
        let Some(pad) = pad else {
            if !add_mapped(pads, &path, &device, db) {
                not_pads.insert(path);
            }
            continue;
        };
        let open = || {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(SHARE_READ_WRITE)
                .open(&path)
        };
        let (Ok(reader), Ok(writer)) = (open(), open()) else {
            continue;
        };
        let features = read_features(&reader, pad.model, device.feature);
        let _ = pads.add(Opened {
            path: path.clone(),
            model: pad.model,
            link: pad.link,
            reader,
            input_length: pad.input_length,
            writer,
            output_length: pad.output_length,
            rumble: sony::rumble_from_features(device.product, &features),
        });
    }
}

/// The pad's feature reports ([`sony::feature_reports`]) as read, the ones the pad answered.
fn read_features(file: &std::fs::File, model: Model, feature_length: usize) -> Vec<Vec<u8>> {
    use windows::Win32::Devices::HumanInterfaceDevice::HidD_GetFeature;
    use windows::Win32::Foundation::HANDLE;
    let mut read = Vec::new();
    for &id in sony::feature_reports(model) {
        // Windows takes a buffer of the longest feature report's length, the id in its first byte.
        let mut buffer = vec![0u8; feature_length.max(64)];
        buffer[0] = id;
        // SAFETY: an open HID handle and a buffer of the length passed.
        let ok = unsafe {
            HidD_GetFeature(
                HANDLE(file.as_raw_handle()),
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
            )
        };
        if ok.as_bool() {
            read.push(buffer);
        }
    }
    read
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This machine's HID devices: the list is read, each device opened without access answers
    /// for itself or not at all, and one look for pads runs (a pad plugged in would be opened
    /// and get its light bar set; none is asserted). Nothing is written to a device that is not
    /// a Sony pad.
    #[test]
    fn this_machines_hid_devices_are_looked_through() {
        let paths = hid_paths();
        for path in &paths {
            let _ = probe(path);
        }
        let pads = HidPads::new();
        let mut not_pads = HashSet::new();
        look_for_pads(&pads, &mut not_pads, &Database::windows());
        assert!(not_pads.iter().all(|p| paths.contains(p)));
    }

    #[test]
    fn which_devices_are_game_controllers_and_which_xinputs() {
        assert!(is_game_controller((1, 4)) && is_game_controller((1, 5)));
        assert!(is_game_controller((1, 8)));
        assert!(!is_game_controller((1, 2)) && !is_game_controller((1, 6)));
        assert!(!is_game_controller((0x0c, 1)));
        assert!(xinput_reads(r"\\?\HID#VID_045E&PID_028E&IG_00#3&1"));
        assert!(xinput_reads(r"\\?\hid#vid_045e&pid_02ff&ig_00#a"));
        assert!(!xinput_reads(r"\\?\HID#VID_2DC8&PID_6006#7&2"));
    }

    /// This machine's game controllers that XInput does not read: each one's description from
    /// Windows' HID parser counted in SDL's order (whatever is plugged in; none is asserted).
    #[test]
    fn this_machines_game_controllers_are_described() {
        for path in hid_paths() {
            let Some(device) = probe(&path) else {
                continue;
            };
            if !is_game_controller(device.usage) || xinput_reads(&path) {
                continue;
            }
            let Ok(file) = std::fs::OpenOptions::new()
                .access_mode(0)
                .share_mode(SHARE_READ_WRITE)
                .open(&path)
            else {
                continue;
            };
            if let Some(data) = Preparsed::of(&file) {
                let layout = Layout::of(&data.elements());
                assert!(layout.axes.len() <= 8 && layout.hats.len() <= 4);
            }
        }
    }
}
