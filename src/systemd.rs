use std::path::PathBuf;
use super::genesis::*;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

pub struct Emitter{
    prefix: PathBuf,
}

impl Emitter {
    pub fn new(prefix: PathBuf) -> Self {
        Self {
            prefix
        }
    }


    pub fn run(&self, config: &Genesis) {
        for (name, i) in &config.interface {
            println!("{:#?}", i);
            self.interface(name, i);
        }
        Command::new("systemctl")
            .args(&["restart", "systemd-networkd"])
            .status()
            .expect("failed to execute process");
    }

    fn interface(&self, name: &str, config: &Interface) {
        let p = self.prefix.join("etc/systemd/network/");
        std::fs::create_dir_all(&p).expect(&format!("cannot create {:?}", p));
        let p = p.join(format!("20-genesis-{}.network", name));
        let mut f = File::create(&p).expect(&format!("cannot open {:?}", p));

        write!(f, "[Match]\nName={}\n\n", config.device).unwrap();

        match config.proto {
            Proto::Dhcp => {
                write!(f, "[Network]\nDHCP=ipv4\n").unwrap();
                write!(f, "[DHCP]\nRouteMetric=10\n").unwrap();
            }
        }
        if let Some(wifi) = &config.wifi {
            match wifi.mode{
                WifiMode::Sta=> {
                    let p = self.prefix.join("etc/wpa_supplicant/");
                    std::fs::remove_dir_all(&p).ok();
                    std::fs::create_dir_all(&p).expect(&format!("cannot create {:?}", p));
                    let p = p.join(format!("wpa_supplicant-{}.conf", config.device ));
                    let mut f = File::create(&p).expect(&format!("cannot open {:?}", p));
                    write!(f, "
                        ctrl_interface=/var/run/wpa_supplicant
                        ap_scan=1
                    ").unwrap();
                    for sta in &config.sta {
                        match (&sta.auth, &sta.key) {
                            (Some(WifiAuth::None), _) | (None, _) | (_, None) => {
                                write!(f, "
                                network={{
                                    ssid=\"{}\"
                                    key_mgmt=NONE
                                }}
                                ", sta.ssid).unwrap();
                            }
                            (Some(WifiAuth::Psk2), Some(key)) => {
                                write!(f, "
                                network={{
                                    key_mgmt=WPA-PSK
                                    ssid=\"{}\"
                                    psk=\"{}\"
                                }}
                                ", sta.ssid, key).unwrap();
                            }
                        }
                    }
                    drop(f);
                    Command::new("systemctl")
                        .args(&["start", &format!("wpa_supplicant@{}.service", config.device)])
                        .status()
                        .expect("failed to execute process");
                }
            }

        }
    }
}


