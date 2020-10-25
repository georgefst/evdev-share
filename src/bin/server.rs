use clap::Clap;
use evdev_rs::{
    enums::{int_to_ev_key, EventCode, EventType, EV_KEY, EV_SYN},
    Device, InputEvent, TimeVal, UInputDevice,
};
use std::net::{SocketAddr, UdpSocket};

#[derive(Clap, Debug)]
struct Args {
    #[clap(short = 'p')]
    port: u16,
    #[clap(short = 'n')]
    name: String,
}

fn main() {
    let args = Args::parse();
    let sock = &UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], args.port))).unwrap();
    let mut buf = [0; 2];

    let fake_dev = Device::new().unwrap();
    fake_dev.set_name(&args.name);
    fake_dev.enable(&EventType::EV_KEY).unwrap();
    // KEY_RESERVED is the "minimum" key event, and the key events are all consecutive
    for code in EventCode::EV_KEY(EV_KEY::KEY_RESERVED).iter() {
        if let EventCode::EV_KEY(_) = code {
            fake_dev
                .enable(&code)
                .unwrap_or_else(|e| println!("Failed to enable code ({}): {}", e, code));
        } else {
            break;
        }
    }
    let dev = UInputDevice::create_from_device(&fake_dev).unwrap();

    loop {
        match sock.recv_from(&mut buf) {
            Ok((n_bytes, addr)) => {
                if n_bytes != 2 {
                    panic!("Received {} bytes (expected 2) from {}: {:?}", n_bytes, addr, buf);
                }
                let key_code = buf[0] as u32;
                if let Some(k) = int_to_ev_key(key_code) {
                    /*TODO in the C API, we would set this to null to use the current time -
                    does 'evdev-rs' have an equivalent? */
                    let t = TimeVal::new(0, 0);
                    let c = EventCode::EV_KEY(k);
                    let v = buf[1] as i32;
                    let ev = InputEvent::new(&t, &c, v);
                    println!("From {}: {:?}", addr, ev);
                    dev.write_event(&ev)
                        .unwrap_or_else(|e| println!("Failed to write event: {}", e));
                    dev.write_event(&InputEvent::new(
                        &t,
                        &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
                        0,
                    ))
                    .unwrap_or_else(|e| println!("Failed to write sync event: {}", e));
                } else {
                    println!(
                        "Int received over network is not a valid key code: {:?}",
                        key_code
                    )
                }
            }
            Err(e) => println!("Received invalid network message: {:?} ({:?})", buf, e),
        }
    }
}
