use std::path::PathBuf;
use super::genesis::*;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;
use super::genesis;
use std::io::{Error,ErrorKind};

pub struct Emitter{
    prefix: PathBuf,
}

impl Emitter {
    pub fn new(prefix: PathBuf) -> Self {
        Self {
            prefix
        }
    }


    pub fn load(&mut self, config: &genesis::Genesis) -> Result<(), Error> {
        for (name, i) in &config.interface {
            println!("{:#?}", i);
            self.interface(name, i)?;
        }
        Command::new("systemctl")
            .args(&["restart", "systemd-networkd"])
            .status()?;
        Ok(())
    }
    pub fn commit(&self) -> Result<(), Error>  {
        Ok(())
    }

    fn interface(&self, name: &str, config: &Interface) -> Result<(), Error> {
        let p = self.prefix.join("etc/systemd/network/");
        std::fs::create_dir_all(&p)?;
        let p = p.join(format!("20-genesis-{}.network", name));
        let mut f = File::create(&p)?;

        let device = match &config.device {
            Some(v) => v,
            None => {
                return Err(Error::new(ErrorKind::Other, format!("interface.device is required")));
            }
        };

        write!(f, "[Match]\nName={}\n\n", device).unwrap();

        let mut dhcp = match &config.dhcp {
            Some(toml::value::Value::String(s)) =>  {
                match s.as_str() {
                    "yes" | "true" => {
                        Some("true".to_string())
                    },
                    "ipv6" => {
                        Some("ipv6".to_string())
                    },
                    "ipv4" => {
                        Some("ipv4".to_string())
                    },
                    "false" | "no" => {
                        None
                    },
                    _ => None,
                }
            },
            Some(toml::value::Value::Boolean(v)) => {
                if *v {
                    Some("true".to_string())
                } else {
                    None
                }
            },
            _ => None
        };

        if let Some(s) = &config.proto {
            if s == "dhcp" {
                dhcp = Some("ipv4".to_string())
            }
        }

        if let Some(dhcp) = dhcp {
            write!(f, "[Network]\nDHCP={}\n", dhcp)?;
            write!(f, "[DHCP]\nRouteMetric=10\n")?;
        } else {
            write!(f, "[Network]\n")?;
            if let Some(ipaddr) = &config.ipaddr {
                for ipaddr in ipaddr {
                    write!(f, "Address={}\n", ipaddr)?;
                }
            } else {
                return Err(Error::new(ErrorKind::Other, format!("either interface.dhcp or ipaddr required")));
            }
            if let Some(gw) = &config.gateway {
                write!(f, "Gateway={}\n", gw)?;
            }
            if let Some(dns) = &config.dns {
                for dns in dns {
                    write!(f, "DNS={}\n", dns)?;
                }
            }
        }


        if let Some(wifi) = &config.wifi {
            match wifi.mode{
                WifiMode::Ap | WifiMode::Monitor => {
                    return Err(Error::new(ErrorKind::Other, format!("wifi mode not supported")));
                }
                WifiMode::Sta=> {
                    let p = self.prefix.join("etc/wpa_supplicant/");
                    std::fs::remove_dir_all(&p).ok();
                    std::fs::create_dir_all(&p)?;
                    let p = p.join(format!("wpa_supplicant-{}.conf", device ));
                    let mut f = File::create(&p)?;
                    write!(f, "
                        ctrl_interface=/var/run/wpa_supplicant
                        ap_scan=1
                    ")?;
                    for sta in &config.sta {
                        match (&sta.auth, &sta.key) {
                            (Some(WifiAuth::None), _) | (None, _) | (_, None) => {
                                write!(f, "
                                network={{
                                    ssid=\"{}\"
                                    key_mgmt=NONE
                                }}
                                ", sta.ssid)?;
                            }
                            (Some(WifiAuth::Psk2), Some(key)) => {
                                write!(f, "
                                network={{
                                    key_mgmt=WPA-PSK
                                    ssid=\"{}\"
                                    psk=\"{}\"
                                }}
                                ", sta.ssid, key)?;
                            }
                        }
                    }
                    drop(f);
                    Command::new("systemctl")
                        .args(&["start", &format!("wpa_supplicant@{}.service", device)])
                        .status()?;
                }
            }
        }

        Ok(())
    }
}


