use clap::Clap;
use evdev_rs::{enums::EventCode, enums::EV_KEY, Device, GrabMode, ReadFlag};
use std::{
    fs::File,
    net::{IpAddr, SocketAddr, UdpSocket},
    str::FromStr,
    sync::Arc,
    sync::Mutex,
    thread,
    time::Duration,
};

/*TODO
grabbing
    grab at start if not idle, ungrab on kill
    grab all devices (particularly seeing as we want to use this with a keyboard that has 3 physical devices)
        this is difficult, because 'grab' is tied to the thread, and rust's async is confusing...
if device disconnects (e.g. bluetooth keyboard, inotifywait for it to come back)
remove 'unwrap's
    important
        'active' mutex
*/

//TODO add help info
#[derive(Clap, Debug)]
struct Args {
    #[clap(short = 'p')]
    port: u16,
    #[clap(short = 'a')]
    ip: IpAddr,
    #[clap(short = 'd')]
    devices: Vec<String>,
    #[clap(short = 'k')]
    key_wrapped: KeyWrapped,
    #[clap(long = "start-idle")]
    idle: bool,
}

fn main() {
    let args: Args = Args::parse();
    let addr = SocketAddr::new(args.ip, args.port);
    let active = Arc::new(Mutex::new(!args.idle));
    let switch_key = args.key_wrapped.key;

    for dev_path in args.devices.into_iter() {
        let active = Arc::clone(&active);
        let switch_key = switch_key.clone();
        thread::spawn(move || {
            let sock = &UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap();
            let mut dev = Device::new_from_fd(File::open(dev_path).unwrap()).unwrap();
            let mut buf = [0; 2];
            let mut interrupted = false; // have there been any events from other keys since switch was last pressed?

            loop {
                match dev.next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING) {
                    Ok((_, event)) => match event.event_code {
                        EventCode::EV_KEY(key) => {
                            let mut active = active.lock().unwrap();
                            if *active {
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
                                            *active = !*active;
                                            if *active {
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
                            if *active.lock().unwrap() {
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
        });
    }
    loop {
        //TODO use a proper async/threading library instead - rayon?
        thread::sleep(Duration::from_secs(u64::MAX));
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
