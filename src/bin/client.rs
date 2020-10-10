use clap::Clap;
use evdev_rs::{enums::EventCode, Device, ReadFlag};
use std::{
    fs::File,
    net::{IpAddr, SocketAddr, UdpSocket},
};

#[derive(Clap, Debug)]
struct Args {
    #[clap(short = 'p')]
    port: u16,
    #[clap(short = 'a')]
    ip: IpAddr,
    #[clap(short = 'd')]
    device: String,
}

fn main() {
    let args: Args = Args::parse();
    let sock = &UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).unwrap();
    let addr = SocketAddr::new(args.ip, args.port);
    let dev = Device::new_from_fd(File::open(args.device).unwrap()).unwrap();
    let mut buf = [0; 2];

    loop {
        match dev.next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING) {
            Ok((_, event)) => match event.event_code {
                EventCode::EV_KEY(k) => {
                    println!("Sending: {:?}, {}", k, event.value);
                    buf[0] = k as u8;
                    buf[1] = event.value as u8;
                    sock.send_to(&mut buf, addr).unwrap_or_else(|e| {
                        println!("Failed to send: {}", e);
                        0
                    });
                }
                e => println!("Non-key event, ignoring: {}", e),
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
