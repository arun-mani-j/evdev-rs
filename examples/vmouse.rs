
use std::fs::File;

use structopt::StructOpt;

use evdev_rs::{Device, UInputDevice, UninitDevice, InputEvent, DeviceWrapper, ReadFlag};
use evdev_rs::enums::{BusType, EventType, EventCode, EV_KEY, EV_REL, EV_SYN};

/// Virtual mouse example.
/// 
/// Maps a keyboard input event file to a virtual mouse using UInput.
/// See `ls -al /dev/input/by-id` to list available input event files
#[derive(Clone, PartialEq, Debug, StructOpt)]
pub struct Options {
    /// Input device from which to capture events
    #[structopt()]
    pub device: String,

    /// Mouse scroll X step value
    #[structopt(long, default_value = "20")]
    pub mouse_step_x: i32,

    /// Mouse scroll Y step value
    #[structopt(long, default_value = "20")]
    pub mouse_step_y: i32,
}

fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let opts = Options::from_args();

    // Connect to real keyboard
    let f = File::open(opts.device)?;
    let d = Device::new_from_file(f)?;

    if let Some(n) = d.name() {
        println!("Connected to device: '{}' ({:04x}:{:04x})", 
            n, d.vendor_id(), d.product_id());
    }

    // Create virtual device
    let u = UninitDevice::new().unwrap();

    // Setup device
    // per: https://01.org/linuxgraphics/gfx-docs/drm/input/uinput.html#mouse-movements

    u.set_name("Virtual Mouse");
    u.set_bustype(BusType::BUS_USB as u16);
    u.set_vendor_id(0xabcd);
    u.set_product_id(0xefef);

    // Note mouse keys have to be enabled for this to be detected 
    // as a usable device, see: https://stackoverflow.com/a/64559658/6074942
    u.enable_event_type(&EventType::EV_KEY)?;
    u.enable_event_code(&EventCode::EV_KEY(EV_KEY::BTN_LEFT), None)?;
    u.enable_event_code(&EventCode::EV_KEY(EV_KEY::BTN_RIGHT), None)?;

    u.enable_event_type(&EventType::EV_REL)?;
    u.enable_event_code(&EventCode::EV_REL(EV_REL::REL_X), None)?;
    u.enable_event_code(&EventCode::EV_REL(EV_REL::REL_Y), None)?;

    u.enable_event_code(&EventCode::EV_SYN(EV_SYN::SYN_REPORT), None)?;

    // Attempt to create UInputDevice from UninitDevice
    let v = UInputDevice::create_from_device(&u)?;

    loop {
        // Fetch keyboard events
        let (_status, event) = d.next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING)?;

        // Map these to mouse events
        println!("Event: {:?}", event);

        // Map direction keys to mouse events
        let e = match event.event_code {
            EventCode::EV_KEY(EV_KEY::KEY_RIGHT) => Some((EV_REL::REL_X, opts.mouse_step_x)),
            EventCode::EV_KEY(EV_KEY::KEY_LEFT) =>  Some((EV_REL::REL_X, -opts.mouse_step_x)),
            EventCode::EV_KEY(EV_KEY::KEY_UP) =>    Some((EV_REL::REL_Y, -opts.mouse_step_y)),
            EventCode::EV_KEY(EV_KEY::KEY_DOWN) =>  Some((EV_REL::REL_Y, opts.mouse_step_y)),
            _ => None,
        };

        // Write mapped event
        if let Some((e, n)) = e {
            v.write_event(&InputEvent{
                time: event.time,
                event_code: EventCode::EV_REL(e),
                value: n,
            })?;

            v.write_event(&InputEvent{
                time: event.time,
                event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
                value: 0,
            })?;
        }
    }

    Ok(())
}