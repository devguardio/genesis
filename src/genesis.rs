use serde::{Deserialize};
use std::collections::HashMap;


//--- network

#[derive(Deserialize, Debug)]
pub enum WifiMode{
    #[serde(rename="sta")]
    Sta,
    #[serde(rename="ap")]
    Ap,
    #[serde(rename="monitor")]
    Monitor,
}

#[derive(Deserialize, Debug)]
pub enum WifiAuth{
    #[serde(rename="psk2")]
    Psk2,
    #[serde(rename="none")]
    None
}

#[derive(Deserialize, Debug)]
pub struct WifiNetwork {
    pub ssid:   String,
    pub key:    Option<String>,
    pub auth:   Option<WifiAuth>,
}

#[derive(Deserialize, Debug)]
pub struct WifiInterface {
    pub mode:       WifiMode,
    pub ssid:       Option<String>,
    pub key:        Option<String>,
    pub auth:       Option<WifiAuth>,
}


#[derive(Deserialize, Debug)]
pub struct WireguardPeer {
    pub public_key: String,
    pub endpoint:   String,
    pub autoroute:  Option<bool>,
    pub psk:        Option<String>,
    pub keepalive:  Option<u32>,
    pub routes:     Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct WireguardInterface {
    pub private_key: String,
    pub peers:       Option<Vec<WireguardPeer>>,
}


/// interface is something that has ip traffic.
/// it can be attached to a physical device via device or created without one via class
#[derive(Deserialize, Debug)]
pub struct Interface {
    pub device:     Option<String>,
    pub class:      Option<String>,
    // true, false, "yes", "no", "ipv4", "ipv6",
    pub dhcp:       Option<toml::value::Value>,
    pub ipaddr:     Option<Vec<String>>,
    pub bridge:     Option<String>,

    pub wireguard:  Option<WireguardInterface>,
    pub wifi:       Option<WifiInterface>,
    #[serde(default)]
    pub sta: Vec<WifiNetwork>,
}


#[derive(Deserialize, Debug)]
pub struct WifiDevice {
    pub channel:    Option<toml::value::Value>,
    pub htmode:     Option<String>,
}

/// a device is a physical thing that can have interfaces
#[derive(Deserialize, Debug)]
pub struct Device {
    pub class:      String,
    pub path:       Option<String>,
    pub wifi:       Option<WifiDevice>,
}

#[derive(Deserialize, Debug)]
pub struct Template {
    pub template:   String,
    pub output:     String,
    pub vars:       toml::value::Value,
}

#[derive(Deserialize, Debug)]
pub struct Genesis {
    #[serde(default)]
    pub interface:  HashMap<String, Interface>,
    #[serde(default)]
    pub device:     HashMap<String, Device>,
    #[serde(default)]
    pub template:   HashMap<String, Template>,
}
