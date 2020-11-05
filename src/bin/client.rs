use clap::Clap;
use evdev_rs::{enums::EventCode, enums::EV_KEY, Device, GrabMode, ReadFlag};
use std::{
    fs::File,
    net::{IpAddr, SocketAddr, UdpSocket},
    str::FromStr,
};

/*TODO
grab at start if not idle, ungrab on kill
*/

//TODO add help info
#[derive(Clap, Debug)]
struct Args {
    #[clap(short = 'p')]
    port: u16,
    #[clap(short = 'a')]
    ip: IpAddr,
    #[clap(short = 'd')]
    device: String,
    #[clap(short = 'k')]
    switch_key: EV_KEY,
    #[clap(long = "start-idle")]
    idle: bool,
}

fn main() {
    let args: Args = Args::parse();
    let addr = SocketAddr::new(args.ip, args.port);
    let switch_key = args.switch_key;
    let sock = &UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap();
    let mut dev = Device::new_from_fd(File::open(args.device).unwrap()).unwrap();
    let mut buf = [0; 2];
    let mut active = !args.idle; // currently grabbed and sending events
    let mut interrupted = false; // have there been any events from other keys since switch was last pressed?
    let mut hanging_switch = false; // we don't necessarily want to send a switch down event, since we might actually
                                    // be switching mode - so we carry over to the next round

    let mut send_key = |key: EV_KEY, event_value: i32| {
        println!("Sending: {:?}, {}", key, event_value);
        buf[0] = key as u8;
        buf[1] = event_value as u8;
        sock.send_to(&mut buf, addr).unwrap_or_else(|e| {
            println!("Failed to send: {}", e);
            0
        });
    };

    loop {
        match dev.next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING) {
            Ok((_, event)) => match event.event_code {
                EventCode::EV_KEY(key) => {
                    if key == switch_key {
                        match event.value {
                            1 => {
                                if active {
                                    interrupted = false;
                                    hanging_switch = true;
                                }
                            }
                            0 => {
                                if interrupted && active {
                                    send_key(key, event.value);
                                } else {
                                    active = !active;
                                    if active {
                                        dev.grab(GrabMode::Grab).unwrap_or_else(|e| {
                                            println!("Failed to grab device: {}", e)
                                        });
                                        println!("Switched to active");
                                    } else {
                                        dev.grab(GrabMode::Ungrab).unwrap_or_else(|e| {
                                            println!("Failed to ungrab device: {}", e)
                                        });
                                        println!("Switched to idle");
                                        hanging_switch = false;
                                    }
                                }
                            }
                            _ => (),
                        }
                    } else {
                        if active {
                            if hanging_switch {
                                send_key(switch_key.clone(), 1);
                            }
                            send_key(key, event.value);
                            interrupted = true;
                            hanging_switch = false;
                        }
                    }
                }
                e => {
                    if active {
                        println!("Non-key event, ignoring: {}", e)
                    }
                }
            },
            Err(e) => {
                println!(
                    "Failed to get next event, abandoning device: {} ({:?})",
                    dev.phys().unwrap_or("NO_PHYS"),
                    e
                );
                break;
            }
        }
    }
}

//TODO implement upstream using 'libevdev_event_code_from_name' (along with Copy)
#[derive(Debug)]
struct KeyWrapped {
    key: EV_KEY,
}
impl FromStr for KeyWrapped {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let k = match s {
            "A" => Ok(EV_KEY::KEY_A),
            "KEY_COMPOSE" => Ok(EV_KEY::KEY_COMPOSE),
            "KEY_RIGHTALT" => Ok(EV_KEY::KEY_RIGHTALT),
            _ => Result::Err(String::from("Unrecognised key: ") + s),
        };
        k.map(|x| KeyWrapped { key: x })
    }
}
