use clap::Clap;
use evdev_rs::{enums::EventCode, enums::EV_KEY, Device, GrabMode, ReadFlag};
use std::{
    fs::File,
    net::{IpAddr, SocketAddr, UdpSocket},
    str::FromStr,
    sync::atomic::AtomicBool,
    sync::atomic::Ordering,
    thread,
    time::Duration,
};

/*TODO
grab at start if not idle, ungrab on kill
if device disconnects (e.g. bluetooth keyboard, inotifywait for it to come back)
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
    let activ = AtomicBool::new(!args.idle);
    let active = &activ;

    for dev_path in args.devices.into_iter() {
        // thread::spawn(|| {
        thread::spawn(move || {
            let sock = &UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap();
            let mut dev = Device::new_from_fd(File::open(dev_path).unwrap()).unwrap();
            let mut buf = [0; 2];

            loop {
                match dev.next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING) {
                    Ok((_, event)) => match event.event_code {
                        EventCode::EV_KEY(EV_KEY::KEY_RIGHTALT) => match event.value {
                            0 => {
                                let a1 = &active.load(Ordering::Relaxed);
                                let a1 = true;
                                let a = !a1;
                                // active.store(a, Ordering::Relaxed);
                                if a {
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
                            _ => (),
                        },
                        EventCode::EV_KEY(k) => {
                            // if active.load(std::sync::atomic::Ordering::Relaxed) {
                            if true {
                                println!("Sending: {:?}, {}", k, event.value);
                                buf[0] = k as u8;
                                buf[1] = event.value as u8;
                                sock.send_to(&mut buf, &addr).unwrap_or_else(|e| {
                                    println!("Failed to send: {}", e);
                                    0
                                });
                            }
                        }
                        e => {
                            // if active.load(Ordering::Relaxed) {
                            if true {
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
