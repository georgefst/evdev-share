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
    switch_key: KeyWrapped,
    #[clap(long = "start-idle")]
    idle: bool,
}

fn main() {
    let args: Args = Args::parse();
    let addr = SocketAddr::new(args.ip, args.port);
    let switch_key = args.switch_key.key;
    let sock = &UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap();
    let mut dev = Device::new_from_fd(File::open(args.device).unwrap()).unwrap();
    let mut buf = [0; 2];
    let mut active = !args.idle; // currently grabbed and sending events
    let mut interrupted = false; // have there been any events from other keys since switch was last pressed?

    loop {
        match dev.next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING) {
            Ok((_, event)) => match event.event_code {
                EventCode::EV_KEY(key) => {
                    if active {
                        println!("Sending: {:?}, {}", key, event.value);
                        buf[0] = key.clone() as u8;
                        buf[1] = event.value as u8;
                        sock.send_to(&mut buf, addr).unwrap_or_else(|e| {
                            println!("Failed to send: {}", e);
                            0
                        });
                    }
                    if key == switch_key {
                        match event.value {
                            1 => interrupted = false,
                            0 => {
                                if !interrupted {
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
                                    }
                                }
                            }
                            _ => (),
                        }
                    } else {
                        interrupted = true;
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
