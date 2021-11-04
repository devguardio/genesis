use serde::{Deserialize, Serialize};
use std::collections::HashMap;


//--- network

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum WifiMode{
    #[serde(rename="sta")]
    Sta,
    #[serde(rename="ap")]
    Ap,
    #[serde(rename="monitor")]
    Monitor,
}

#[derive(Deserialize, Serialize, Clone,Debug)]
pub enum WifiAuth{
    #[serde(rename="psk2")]
    Psk2,
    #[serde(rename="none")]
    None
}

#[derive(Deserialize, Serialize, Clone,Debug)]
pub struct WifiNetwork {
    pub ssid:   String,
    pub key:    Option<String>,
    pub auth:   Option<WifiAuth>,
}

#[derive(Deserialize, Serialize, Clone,Debug)]
pub struct WifiInterface {
    pub mode:       WifiMode,
    pub ssid:       Option<String>,
    pub key:        Option<String>,
    pub auth:       Option<WifiAuth>,
}


#[derive(Deserialize, Serialize, Clone,Debug)]
pub struct WireguardPeer {
    pub public_key: String,
    pub endpoint:   String,
    pub autoroute:  Option<bool>,
    pub psk:        Option<String>,
    pub keepalive:  Option<u32>,
    pub routes:     Vec<String>,
}

#[derive(Deserialize, Serialize, Clone,Debug)]
pub struct WireguardInterface {
    pub private_key: String,
    pub peers:       Option<Vec<WireguardPeer>>,
}


/// interface is something that has ip traffic.
/// it can be attached to a physical device via device or created without one via class
#[derive(Deserialize, Serialize, Clone,Debug)]
pub struct Interface {
    pub device:     Option<String>,
    pub class:      Option<String>,
    // true, false, "yes", "no", "ipv4", "ipv6",
    pub bridge:     Option<String>,
    pub dhcp:       Option<toml::value::Value>,
    pub ipaddr:     Option<Vec<String>>,
    pub gateway:    Option<String>,
    pub dns:        Option<Vec<String>>,
    #[serde(default)]
    pub sta:        Vec<WifiNetwork>,


    // garbage that is unfortunately in use by customers
    pub proto:      Option<String>,

    pub wireguard:  Option<WireguardInterface>,
    pub wifi:       Option<WifiInterface>,
}


#[derive(Deserialize, Serialize, Clone,Debug)]
pub struct WifiDevice {
    pub channel:    Option<toml::value::Value>,
    pub htmode:     Option<String>,
}

/// a device is a physical thing that can have interfaces
#[derive(Deserialize, Serialize, Clone,Debug)]
pub struct Device {
    pub class:      String,
    pub path:       Option<String>,
    pub wifi:       Option<WifiDevice>,
}

#[derive(Deserialize, Serialize, Clone,Debug)]
pub struct Template {
    pub template:   String,
    pub output:     String,
    pub vars:       toml::value::Value,
}

#[derive(Deserialize, Serialize, Clone,Debug)]
pub struct Genesis {
    #[serde(default)]
    pub force:      String,
    #[serde(default)]
    pub interface:  HashMap<String, Interface>,
    #[serde(default)]
    pub device:     HashMap<String, Device>,
    #[serde(default)]
    pub template:   HashMap<String, Template>,
}
