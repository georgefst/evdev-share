use clap::Clap;
use evdev_rs::{enums::EventCode, enums::EV_KEY, Device, GrabMode, ReadFlag};
use std::{
    fs::File,
    net::{IpAddr, SocketAddr, UdpSocket},
    str::FromStr,
};

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
    key_wrapped: KeyWrapped,
    #[clap(long = "start-idle")]
    idle: bool,
}

fn main() {
    let args: Args = Args::parse();
    let sock = &UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap();
    let addr = SocketAddr::new(args.ip, args.port);
    let mut dev = Device::new_from_fd(File::open(args.device).unwrap()).unwrap();
    let mut buf = [0; 2];
    let mut active = !args.idle;

    loop {
        match dev.next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING) {
            Ok((_, event)) => match event.event_code {
                EventCode::EV_KEY(EV_KEY::KEY_RIGHTALT) => match event.value {
                    0 => {
                        active = !active;
                        if active {
                            dev.grab(GrabMode::Grab)
                                .unwrap_or_else(|e| println!("Failed to grab device: {}", e));
                            println!("Switched to active");
                        } else {
                            dev.grab(GrabMode::Ungrab)
                                .unwrap_or_else(|e| println!("Failed to ungrab device: {}", e));
                            println!("Switched to idle");
                        }
                    }
                    _ => (),
                },
                EventCode::EV_KEY(k) => {
                    if active {
                        println!("Sending: {:?}, {}", k, event.value);
                        buf[0] = k as u8;
                        buf[1] = event.value as u8;
                        sock.send_to(&mut buf, addr).unwrap_or_else(|e| {
                            println!("Failed to send: {}", e);
                            0
                        });
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

//TODO implement upstream using 'libevdev_event_code_from_name'
#[derive(Debug)]
struct KeyWrapped {
    key: EV_KEY,
}
impl FromStr for KeyWrapped {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let k = match s {
            "A" => Ok(EV_KEY::KEY_A),
            "KEY_RIGHTALT" => Ok(EV_KEY::KEY_RIGHTALT),
            _ => Result::Err(String::from("Unrecognised key: ") + s),
        };
        k.map(|x| KeyWrapped { key: x })
    }
}
