#![feature(generators, generator_trait)]

extern crate serde;
extern crate toml;


use std::fs::File;
use std::io::{Read};
use carrier_rs::osaka::{self, osaka};

pub mod genesis;

#[cfg(feature = "systemd")]
pub mod systemd;
#[cfg(feature = "openwrt")]
pub mod openwrt;


fn map_err<E: std::error::Error> (e: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e))
}

pub fn genesis_stream(
    poll: osaka::Poll,
    headers: carrier_rs::headers::Headers,
    _: &carrier_rs::identity::Identity,
    mut stream: carrier_rs::endpoint::Stream,
) -> Option<osaka::Task<()>> {

    let gdir = carrier_rs::config::persistence_dir().join("genesis");
    match std::fs::create_dir_all(&gdir) {
        Ok(_) => (),
        Err(e) => {
            log::warn!("cannot create {:?}: {}", gdir, e);
        }
    };
    let mut p = gdir.join("current.toml");
    if !p.exists() {
        p = gdir.join("stable.toml");
    }
    let mut f = match File::open(&p) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cannot open {:?}: {}", p, e);
            stream.send(carrier_rs::headers::Headers::with_error(503, "misconfigured").encode());
            return None;
        }
    };

    let sha256 = match carrier_rs::util::sha256file(&p) {
        Err(e) => {
            log::warn!("sha err {}", e);
            stream.send(carrier_rs::headers::Headers::with_error(503, "misconfigured").encode());
            return None;
        },
        Ok(v) => v,
    };

    match headers.get(b":method") {
        None | Some(b"GET") => {
            stream.send(carrier_rs::headers::Headers::ok().encode());
            let mut s = Vec::new();
            f.read_to_end(&mut s).unwrap();
            stream.message(carrier_rs::proto::GenesisCurrent{
                sha256,
                commit: String::new(),
                data:   s,
                stable: !gdir.join("current.toml").exists(),
            });
        },
        Some(b"HEAD") => {
            stream.send(carrier_rs::headers::Headers::ok().encode());
            stream.message(carrier_rs::proto::GenesisCurrent{
                sha256,
                commit: String::new(),
                data:   Vec::new(),
                stable: !gdir.join("current.toml").exists(),
            });
        },
        Some(b"POST") => {
            stream.send(carrier_rs::headers::Headers::with_error(100, "go ahead").encode());
            return Some(genesis_post_handler(poll, stream, gdir.clone()));
        }
        _ => {
            stream.send(carrier_rs::headers::Headers::with_error(404, "no such method").encode());
            return None;
        }
    }

    None
}


#[osaka]
fn genesis_post_handler(_poll: osaka::Poll, mut stream: carrier_rs::endpoint::Stream, gdir: std::path::PathBuf) {
    use carrier_rs::prost::Message;
    use std::io::Write;

    let mut p = gdir.join("current.toml");
    if !p.exists() {
        p = gdir.join("stable.toml");
    }
    let sha256 = match carrier_rs::util::sha256file(&p) {
        Err(e) => {
            log::warn!("sha err {}", e);
            stream.send(carrier_rs::headers::Headers::with_error(503, format!("{:?}", e)).encode());
            return;
        },
        Ok(v) => v,
    };

    let m = osaka::sync!(stream);
    let m = match carrier_rs::proto::GenesisUpdate::decode(&m) {
        Err(e) => {
            stream.send(carrier_rs::headers::Headers::with_error(400, format!("{:?}", e)).encode());
            log::warn!("proto: {:?}", e);
            return;
        },
        Ok(v) => v,
    };

    if m.previous_sha256 != sha256 {
        stream.send(carrier_rs::headers::Headers::with_error(408, "outdated handle").encode());
        return;
    }

    let g : genesis::Genesis = match toml::de::from_slice(&m.data) {
        Err(e) => {
            stream.send(carrier_rs::headers::Headers::with_error(400, format!("{:?}", e)).encode());
            log::warn!("toml: {:?}", e);
            return;
        }
        Ok(v) => v,
    };


    #[cfg(feature = "openwrt")]
    let mut em = openwrt::Emitter::new();
    #[cfg(feature = "systemd")]
    let mut em = systemd::Emitter::new("/".into());
    #[cfg(not(any(feature = "systemd", feature = "openwrt")))]
    compile_error!("missing feature config: systemd or openwrt");


    match em.load(&g) {
        Err(e) => {
            stream.send(carrier_rs::headers::Headers::with_error(400, format!("{:?}", e)).encode());
            log::warn!("{:?}", e);
            return;
        }
        Ok(v) => v,
    }
    let unixtime = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::default()).as_secs();

    let mut p = gdir.join("current.toml");
    if !g.force.is_empty()  {
        std::fs::remove_file(p);
        p = gdir.join("stable.toml");
    }

    let r = std::fs::File::create(&p)
        .and_then(|mut f|{
            write!(f, "#commit {} {}\n", unixtime, m.commit.replace("\n"," \\ "))?;
            f.write_all(&m.data)
        });
    if let Err(e) = r {
        stream.send(carrier_rs::headers::Headers::with_error(500, format!("{:?}", e)).encode());
        log::warn!("{:?}", e);
        return;
    }

    let r = std::fs::File::create(&gdir.join("changing")).and_then(|mut f|f.write_all(b"0"));
    if let Err(e) = r {
        stream.send(carrier_rs::headers::Headers::with_error(500, format!("{:?}", e)).encode());
        log::warn!("{:?}", e);
        return;
    }

    if let Err(e) = em.commit() {
        stream.send(carrier_rs::headers::Headers::with_error(500, format!("{:?}", e)).encode());
        log::warn!("{:?}", e);
        return;
    }

    stream.send(carrier_rs::headers::Headers::ok().encode());

    std::thread::spawn(||{
        std::thread::sleep(std::time::Duration::from_secs(10));
        std::process::exit(1);
    });
}


pub fn genesis() -> Result<(), std::io::Error> {
    let gdir = carrier_rs::config::persistence_dir().join("genesis");

    let mut f = File::open(gdir.join("current.toml")).or_else(|_|File::open(gdir.join("stable.toml")))?;
    let mut s = Vec::new();
    f.read_to_end(&mut s)?;

    let g : genesis::Genesis = toml::de::from_slice(&s).map_err(map_err)?;

    #[cfg(feature = "openwrt")]
    let mut em = openwrt::Emitter::new();
    #[cfg(feature = "systemd")]
    let mut em = systemd::Emitter::new("/".into());

    #[cfg(not(any(feature = "systemd", feature = "openwrt")))]
    compile_error!("missing feature config: systemd or openwrt");

    em.load(&g)?;
    em.commit()?;

    Ok(())
}

pub fn stabilize(stable: bool) {
    use std::io::Write;

    let gdir = carrier_rs::config::persistence_dir().join("genesis");
    match std::fs::create_dir_all(&gdir) {
        Ok(_) => (),
        Err(e) => {
            log::warn!("cannot create {:?}: {}", gdir, e);
        }
    };

    if stable {
        log::info!("genesis stabilized");
        std::fs::remove_file(gdir.join("changing")).ok();

        let filep1 = gdir.join("current.toml");
        let filep2 = gdir.join("stable.toml");
        if let Err(e) = std::fs::rename(&filep1, &filep2) {
            log::warn!("{:?}", e);
            return;
        }
        return;
    }

    let mut ci = std::fs::read_to_string(&gdir.join("changing")).and_then(|f|f.parse::<u32>().map_err(map_err)).unwrap_or(0);
    ci += 1;
    std::fs::File::create(&gdir.join("changing")).and_then(|mut f|f.write_all(format!("{}", ci).as_bytes())).ok();

    log::info!("genesis stabililization attempt {}", ci);

    if ci > 30 && gdir.join("current.toml").exists() {
        log::info!("genesis reverting");
        std::fs::remove_file(gdir.join("current.toml")).ok();
        std::fs::remove_file(gdir.join("changing")).ok();

        if let Err(e) = genesis() {
            log::error!("genesis: {:?}", e);
        }
    }

}
