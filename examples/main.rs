use std::env;
use carrier::osaka::{self, osaka};
use devguard_genesis as  genesis;

include!(concat!(env!("OUT_DIR"), "/build_id.rs"));

pub fn main() -> Result<(), carrier::Error> {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "info");
    }
    tinylogger::init().ok();


    let mut args = std::env::args();
    args.next();
    match args.next().as_ref().map(|v|v.as_str()) {
        Some("publish") => {
            genesis::stabilize(false);
            let poll            = osaka::Poll::new();
            let config          = carrier::config::load()?;
            let mut publisher   = carrier::publisher::new(config)
                .route("/v0/shell",         None, carrier::publisher::shell::main)
                .route("/v0/sft",           None, carrier::publisher::sft::main)
                .route("/v2/carrier.sysinfo.v1/sysinfo",    None, carrier::publisher::sysinfo::sysinfo)
                .route("/v2/genesis.v1",                    Some(4048), genesis::genesis_stream)
                .with_disco("captif".to_string(), BUILD_ID.to_string())
                .on_pub(||genesis::stabilize(true))
                .publish(poll);
            publisher.run()?;
        }
        Some("genesis") => {
            genesis::genesis().unwrap();
        }
        Some("identity") => {
            let config = carrier::config::load()?;
            println!("{}", config.secret.identity());
        }
        Some("lolcast") => {
            let config = carrier::config::load()?;
            let msg = format!("CR1:BTN:{}", config.secret.identity()).as_bytes().to_vec();
            let socket = std::net::UdpSocket::bind("224.0.0.251:0")?;
            socket.set_broadcast(true).expect("set_broadcast call failed");
            socket.send_to(&msg, "224.0.0.251:8444").expect("couldn't send message");
            socket.send_to(&msg, "0.0.0.0:8444").expect("couldn't send message");
        }
        _ => {
            eprintln!("cmds: publish, identity, genesis, lolcast");
        }
    }

    Ok(())
}


