package main

import (
	"errors"
	"net"
)

type (
	wifiMode int
	wifiAuth int
	ip       net.IPNet

	wifiDevice struct {
		Channel interface{} `toml:"channel"`
		Htmode  string      `toml:"device"`
	}
	wifiNetwork struct {
		Ssid string   `toml:"ssid"`
		Key  string   `toml:"key"`
		Auth wifiAuth `toml:"auth"`
	}

	wifiInterface struct {
		Mode wifiMode `toml:"mode"`
		Ssid string   `toml:"ssid"`
		Key  string   `toml:"key"`
		Auth wifiAuth `toml:"auth"`
	}

	wireguardPeer struct {
		PublicKey string `toml:"public_key"`
		Endpoint  string `toml:"endpoint"`
		Autoroute bool   `toml:"autoroute"`
		Psk       string `toml:"psk"`
		//a keepalive of 0 could actaully be a valid keepalive, so we have to use this.
		Keepalive *int32   `toml:"keepalive"`
		Routes    []string `toml:"routes"`
	}

	wireguardInterface struct {
		PrivateKey string          `toml:"private_key"`
		Peers      []wireguardPeer `toml:"peers"`
	}

	DevInterface struct {
		Device string `toml:"device"`
		Class  string `toml:"class"`
		// true, false, "yes", "no", "ipv4", "ipv6",
		Bridge   string        `toml:"bridge"`
		Dhcp     interface{}   `toml:"dhcp"`
		IPAddres []string      `toml:"ipaddr"`
		Gateway  string        `toml:"gateway"`
		DNS      []string      `toml:"dns"`
		STA      []wifiNetwork `toml:"sta"`
		// garbage that is unfortunately in use by customers
		Proto     string             `toml:"proto"`
		Wireguard wireguardInterface `toml:"wireguard"`
		Wifi      wifiInterface      `toml:"wifi"`
	}

	Device struct {
		Class string      `toml:"class"`
		Path  string      `toml:"path"`
		Wifi  *wifiDevice `toml:"wifi"`
	}

	Template struct {
		Template  string      `toml:"template"`
		Output    string      `toml:"output"`
		Variables interface{} `toml:"vars"`
	}

	Genesis struct {
		Interfaces map[string]DevInterface `toml:"interface"`
		Devices    map[string]Device       `toml:"device"`
		Templates  map[string]Template     `toml:"template"`
	}
)

const (
	sta wifiMode = iota + 1
	ap
	monitor
)
const (
	unset wifiAuth = iota
	none
	psk2
)

func (wa wifiAuth) MarshalTextL() ([]byte, error) {
	switch wa {
	case psk2:
		return []byte("psk2"), nil
	case none:
		return []byte("none"), nil
	}
	return nil, nil
}
func (wa *wifiAuth) UnmarshalText(data []byte) error {
	switch string(data) {
	case "psk2":
		*wa = psk2
		return nil
	case "none":
		*wa = none
		return nil
	}
	return errors.New("Could not decode wifiauth: " + string(data))
}

func (wm wifiMode) MarshalText() ([]byte, error) {
	switch wm {
	case sta:
		return []byte("sta"), nil
	case ap:
		return []byte("ap"), nil
	case monitor:
		return []byte("monitor"), nil
	}
	return nil, nil
}

func (wm *wifiMode) UnmarshalText(data []byte) error {
	switch string(data) {
	case "sta":
		*wm = sta
		return nil
	case "ap":
		*wm = ap
		return nil
	case "monitor":
		*wm = monitor
		return nil
	}
	if len(data) == 0 {
		return errors.New("missing wifi.mode")
	}
	return errors.New("Could not decode wifimode: " + string(data))
}

func (i ip) MarshalText() ([]byte, error) {
	return []byte((*net.IPNet)(&i).String()), nil
}

func (i *ip) UnmarshalText(data []byte) error {
	addr, err := IPFromBytes(data)
	if err != nil {
		return err

	}
	*i = ip(*addr)
	return nil
}

func IPFromBytes(data []byte) (*ip, error) {
	IP, addr, err := net.ParseCIDR(string(data))
	if err != nil {
		return nil, err
	}
	addr.IP = IP
	return (*ip)(addr), nil
}
