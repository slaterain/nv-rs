//! PlayStation pads on Windows, found through Windows' HID: a thread looks through the HID devices
//! present every two seconds, opens each one it has not seen without access to read its vendor
//! and product id, and opens each new Sony pad twice (read, and write) for [`HidPads::add`]. Before
//! that it reads the pad's feature reports ([`sony::feature_reports`]), which turns on a Bluetooth
//! DualSense's full reports and gives its firmware's rumble. A device that is not a pad is not
//! opened again while it stays plugged in.

use super::hid::{HidPads, Opened};
use super::sony::{self, Model};
use std::collections::HashSet;
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

/// Starts the thread that looks for PlayStation pads; the pads it finds join the answer.
pub fn start() -> HidPads {
    let pads = HidPads::new();
    let found = pads.clone();
    let _ = std::thread::Builder::new()
        .name("nv-rs pads".into())
        .spawn(move || {
            let mut not_pads = HashSet::new();
            loop {
                look_for_pads(&found, &mut not_pads);
                std::thread::sleep(LOOK_EVERY);
            }
        });
    pads
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

/// One look through the device list: each Sony pad not yet open is opened and run. `not_pads`
/// holds the devices seen that are not pads, while they stay plugged in.
fn look_for_pads(pads: &HidPads, not_pads: &mut HashSet<String>) {
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
            not_pads.insert(path);
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
        look_for_pads(&pads, &mut not_pads);
        assert!(not_pads.iter().all(|p| paths.contains(p)));
    }
}
