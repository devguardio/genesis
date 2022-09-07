use super::genesis;
use std::io::{Write, Read};
use std::io::{Error,ErrorKind};
use std::collections::HashMap;
use ipnet::IpNet;
use std::process::Command;
use handlebars::Handlebars;

#[derive(Default)]
struct Interface {
    ifname:     Vec<String>,
    dhcp:       bool,
    typ:        Option<String>,
    ipaddr:     Option<(String, String)>,
    gateway:    Option<String>,
    dns:        Option<Vec<String>>,
}

pub struct Device {
}

pub struct Emitter{
    devices:        HashMap<String, Device>,
    interfaces:     HashMap<String, Interface>,
    out_wireless:   Vec<u8>,
    out_network:    Vec<u8>,
    tplout:         HashMap<String, String>,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            devices:        HashMap::new(),
            interfaces:     Default::default(),
            out_wireless:   Vec::new(),
            out_network:    Vec::new(),
            tplout:         HashMap::new(),
        }
    }


    pub fn load(&mut self, config: &genesis::Genesis) -> Result<(), Error> {
        for (name, device ) in &config.device {
            self.device(name, &device)?;
        }

        for (name, interface) in &config.interface {
            self.interface(name, &interface)?;
        }


        for (name, template) in &config.template {
            self.template(name, &template)?;
        }

        write!(&mut self.out_network, "
config interface   'loopback'
    option ifname  'lo'
    option proto   'static'
    option ipaddr  '127.0.0.1'
    option netmask '255.0.0.0'
\n")?;

        //emit
        for (k,v) in &self.interfaces {
            write!(&mut self.out_network, "config interface   '{}'\n", k)?;
            if let Some(typ) = &v.typ{
                write!(&mut self.out_network,"    option type    '{}'\n", typ)?;
            }
            if v.ifname.len() > 0 {
                write!(&mut self.out_network,"    option ifname  '{}'\n", v.ifname.join(" "))?;
            }

            if v.dhcp {
                write!(&mut self.out_network,"    option proto   'dhcp'\n")?;
            }
            if let Some((ipaddr, mask)) = &v.ipaddr {
                write!(&mut self.out_network,"    option proto   'static'\n")?;
                write!(&mut self.out_network,"    option ipaddr  '{}'\n",  ipaddr)?;
                write!(&mut self.out_network,"    option netmask '{}'\n", mask)?;
            }

            if let Some(gw) = &v.gateway{
                write!(&mut self.out_network,"    option gateway '{}'\n", gw)?;
            }
            if let Some(dns) = &v.dns{
                for dns in dns {
                    write!(&mut self.out_network,"    list dns '{}'\n", dns)?;
                }
            }

            write!(&mut self.out_network, "\n")?;
        }


        Ok(())
    }

    fn device(&mut self, name: &str, device: &genesis::Device) -> Result<(),Error> {
        self.devices.insert(name.to_string(), Device{
        });
        match device.class.as_str() {
            "wifi" => {

                let mut path = match &device.path {
                    Some(v) => v.clone(),
                    None => return Err(Error::new(ErrorKind::Other, format!("wifi device must be matched by path '{}'", name))),
                };

                if !path.starts_with("/sys/device") {
                    let path2 = std::path::Path::new(&path);
                    if let Ok(v) = std::fs::read_link(&path2.join("device")).and_then(|v|std::path::Path::new(&path).join(v).canonicalize()) {
                        path = v.clone().to_string_lossy().to_string();
                    }
                }

                if !path.starts_with("/sys/devices/") {
                    return Err(Error::new(ErrorKind::Other, format!("path {} did not resolve to a sysfs device", path)));
                }
                path = path[13..].to_string();


                write!(&mut self.out_wireless , "
config wifi-device '{}'
    option type     'mac80211'
    option path     '{}'
", name, &path)?;

                let w = match device.wifi.as_ref() {
                    Some(v) => v,
                    None => {
                        return Err(Error::new(ErrorKind::Other, format!("missing [device.{}.wifi] section", name)));
                    }
                };

                match &w.channel {
                    Some(toml::value::Value::String(v)) =>  {
                        write!(&mut self.out_wireless , "    option channel  '{}'\n", v)?;
                    }
                    Some(toml::value::Value::Integer(v)) =>  {
                        write!(&mut self.out_wireless , "    option channel  '{}'\n", v)?;
                    },
                    _ => ()
                };

                if let Some(v) = &w.htmode {
                    write!(&mut self.out_wireless , "    option htmode  '{}'\n", v)?;
                }

            },
            a => {
                return Err(Error::new(ErrorKind::Other, format!("invalid class '{}'", a)));
            }
        }
        Ok(())
    }

    fn interface(&mut self, name: &str, interface: &genesis::Interface) -> Result<(),Error> {
        if name.chars().any(|x| !x.is_alphanumeric()) {
            return Err(Error::new(ErrorKind::Other, format!("invalid interface name {}", name)));
        }

        match interface.class.as_ref().map(|x|x.as_ref()) {
            None => {
                return self.create_interface(name, interface);
            },
            Some("bridge") => {
                self.create_interface(name.clone(),interface)?;
                self.interfaces.get_mut(name).unwrap().typ = Some("bridge".to_string());
            },
            Some("wifi") => {
                if interface.bridge.is_none() {
                    let mut interface = interface.clone();
                    interface.device = None;
                    self.create_interface(name.clone(), &interface)?;
                }
                return self.wifi(name, interface);
            },
            Some("wireguard") => {
                self.wireguard(name, interface)?;
            },
            Some(a) => {
                return Err(Error::new(ErrorKind::Other, format!("invalid class '{}'", a)));
            }
        }
        Ok(())
    }

    fn wifi(&mut self, name: &str, interface: &genesis::Interface) -> Result<(),Error> {

        let device_name = match &interface.device {
            None => return Err(Error::new(ErrorKind::Other, format!("wifi interface missing device {}'", name))),
            Some(v) => match self.devices.get(v) {
                None => return Err(Error::new(ErrorKind::Other, format!("undeclared interface '{}' used in device {}'", v, name))),
                Some(_) => v.to_string(),
            }
        };

        let w = match interface.wifi.as_ref() {
            Some(v) => v,
            None => {
                return Err(Error::new(ErrorKind::Other, format!("missing [interface.{}.wifi] section", name)));
            }
        };

        let mode = match w.mode {
            genesis::WifiMode::Sta      => "sta",
            genesis::WifiMode::Ap       => "ap",
            genesis::WifiMode::Monitor  => "monitor",
        };

        write!(&mut self.out_wireless , "
config wifi-iface   '{}'
    option device   '{}'
    option mode     '{}'
    option ifname   '{}'
", name, device_name, mode, name)?;

        if let Some(br) = &interface.bridge {
            write!(&mut self.out_wireless , "    option network  '{}'\n", br)?;
        } else {
            write!(&mut self.out_wireless , "    option network  '{}'\n", name)?;
        }

        if let Some(v) = &w.ssid {
            write!(&mut self.out_wireless , "    option ssid     '{}'\n", v)?;

            match &w.auth {
                Some(genesis::WifiAuth::Psk2) => {
                    write!(&mut self.out_wireless , "    option encryption 'psk2' \n")?;
                },
                Some(genesis::WifiAuth::None) | None  => {
                    write!(&mut self.out_wireless , "    option encryption 'none' \n")?;
                }
            };
        };

        if let Some(v) = &w.key {
            write!(&mut self.out_wireless , "    option key       '{}'\n", v)?;
        };

        Ok(())
    }

    fn create_interface(&mut self, name: &str, interface: &genesis::Interface) -> Result<(),Error> {

        if let Some(br) = &interface.bridge {
            let i = self.interfaces.entry(br.to_string()).or_default();
            if let Some(ifname) = &interface.device {
                i.ifname.push(ifname.to_string());
            }
            return Ok(());
        }

        let i = self.interfaces.entry(name.to_string()).or_default();
        if let Some(ifname) = &interface.device {
            i.ifname.push(ifname.to_string());
        }

        if let Some(ipaddr) = &interface.ipaddr {
            if let Some(addr) = ipaddr.first() {
                let ip : IpNet = addr.parse().map_err(super::map_err)?;
                i.ipaddr = Some((format!("{}", ip.addr()), format!("{}", ip.netmask())));
            }
        }

        i.gateway   = interface.gateway.clone();
        i.dns       = interface.dns.clone();

        match &interface.dhcp {
            Some(toml::value::Value::String(s)) =>  {
                match s.as_str() {
                    "yes" | "true" | "ipv4" => {
                        i.dhcp = true;
                    },
                    "false" | "no" => {
                        i.dhcp = false;
                    },
                    _ => (),
                }
            },
            Some(toml::value::Value::Boolean(v)) => {
                if *v {
                    i.dhcp = true;
                } else {
                    i.dhcp = true;
                }
            },
            _ => (),
        }
        Ok(())
    }


    fn wireguard(&mut self, name: &str, interface: &genesis::Interface) -> Result<(),Error> {
        let wg = match interface.wireguard.as_ref() {
            Some(v) => v,
            None => {
                return Err(Error::new(ErrorKind::Other, format!("missing [interface.{}.wireguard] section", name)));
            }
        };
        write!(&mut self.out_network, "
config interface '{}'
    option proto 'wireguard'
    option private_key '{}'
    option disabled '0'
", name, wg.private_key)?;

        if let Some(ipaddr) = &interface.ipaddr {
            for addr in ipaddr {
                write!(&mut self.out_network, "    list addresses '{}'\n", addr)?;
            }
        }

        let peer = match &wg.peers {
            Some(v) if v.len() == 1 => v.first().unwrap(),
            _ => {
                return Err(Error::new(ErrorKind::Other, format!("need exactly one [interface.{}.wireguard.peers] section", name)));
            }
        };


        write!(&mut self.out_network, "
config wireguard_{}
    option public_key '{}'
    option route_allowed_ips '{}'
", name, peer.public_key, if peer.autoroute.clone().unwrap_or_default() {"1"} else {"0"})?;

        if let Some(psk) = &peer.psk {
            write!(&mut self.out_network, "    option preshared_key '{}'\n", psk)?;
        }

        let endpoint : Vec<&str> = peer.endpoint.split(":").collect();
        if endpoint.len() != 2 {
            return Err(Error::new(ErrorKind::Other, format!("invalid wg endpoint: {}", peer.endpoint)));
        }
        match endpoint[1].parse::<u16>() {
            Err(e) => {
                return Err(Error::new(ErrorKind::Other, format!("invalid wg endpoint port: {}: {}", endpoint[1], e)));
            }
            Ok(v) => {
                write!(&mut self.out_network, "    option endpoint_port '{}'\n", v)?;
            },
        };
        write!(&mut self.out_network, "    option endpoint_host '{}'\n", endpoint[0])?;


        if let Some(v) = &peer.keepalive {
            write!(&mut self.out_network, "    option persistent_keepalive '{}'\n", v)?;
        }

        for route in &peer.routes {
            write!(&mut self.out_network, "    list allowed_ips '{}'\n", route)?;
        }

        Ok(())
    }

    fn template(&mut self, _name: &str, template: &genesis::Template) -> Result<(),Error> {
        let p = std::path::Path::new("/etc/devguard/genesis/templates/").join(&template.template);
        let mut f = std::fs::File::open(p)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;

        let t = Handlebars::new().render_template(&s, &template.vars).map_err(super::map_err)?;

        self.tplout.insert(template.output.to_string(), t);

        Ok(())
    }

    pub fn commit(&self) ->  Result<(), Error> {

        let mut f = std::fs::File::create("/etc/config/wireless")?;
        f.write_all(&self.out_wireless)?;

        let mut f = std::fs::File::create("/etc/config/network")?;
        f.write_all(&self.out_network)?;


        for (path, s) in &self.tplout {
            if let Err(err) = std::fs::File::create(path).and_then(|mut f|f.write_all(s.as_bytes())) {
                log::warn!("{}: {}", path, err);
            }
        }

        Command::new("/etc/devguard/genesis/post")
            .spawn().ok();

        Ok(())
    }



}


