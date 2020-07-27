package main

import (
	"bytes"
	"errors"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"unicode"

	"github.com/aymerick/raymond"
)

type (
	wifiMode int
	wifiAuth int
	ip       net.IPNet

	wifiDevice struct {
		Channel interface{} `toml:"channel"`
		Htmode  string      `toml:"device"`
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
		Bridge   string      `toml:"bridge"`
		Dhcp     interface{} `toml:"dhcp"`
		IPAddres []string    `toml:"ipaddr"`
		// garbage that is unfortunately in use by customers
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
	netInterface struct {
		ifname   []string
		dhcp     bool
		typ      string
		ipAddres *ip
	}
	emitter struct {
		devices    map[string]struct{}
		interfaces map[string]netInterface
		wireless   *bytes.Buffer
		network    *bytes.Buffer
		tplout     map[string]string
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

func (e *emitter) load(config Genesis) error {
	for name, device := range config.Devices {
		e.loadDevice(name, device)
	}
	for name, devIntf := range config.Interfaces {
		e.loadDeviceInterface(name, devIntf)
	}
	for name, tmpl := range config.Templates {
		e.loadTemplate(name, tmpl)
	}

	_, err := fmt.Fprintf(e.network, `
config interface   'loopback'
	option ifname  'lo'
	option proto   'static'
	option ipaddr  '127.0.0.1'
	option netmask '255.0.0.0'

`)
	if err != nil {
		return err
	}

	for k, v := range e.interfaces {
		_, err = fmt.Fprintf(e.network, "config interface   '%s'\n", k)
		if err != nil {
			return err
		}
		if v.typ != "" {
			_, err = fmt.Fprintf(e.network, "    option type    '%s'\n", v.typ)
			if err != nil {
				return err
			}
		}

		if len(v.ifname) != 0 {
			_, err = fmt.Fprintf(e.network, "    option ifname  '%s'\n", strings.Join(v.ifname, " "))
			if err != nil {
				return err
			}
		}

		if v.dhcp {
			_, err = fmt.Fprintf(e.network, "    option proto   'dhcp'\n")
			if err != nil {
				return err
			}
		} else if v.ipAddres != nil {
			_, err = fmt.Fprintf(e.network, `    option proto   'static'
	option ipaddr  '%s'
	option netmask '%s
`, v.ipAddres.IP, v.ipAddres.Mask)
			if err != nil {
				return err
			}

		}
		_, err = fmt.Fprintln(e.network, "")
		if err != nil {
			return err
		}
	}
	return nil
}

func (e *emitter) loadDeviceInterface(name string, devIntf DevInterface) error {
	for _, r := range name {
		if !unicode.IsLetter(r) && (48 > r || r > 58) {
			return fmt.Errorf("invalid interface name %s", name)
		}
	}
	if name == "" {
		panic(devIntf)
	}
	switch devIntf.Class {
	case "":
		e.createDeviceInterface(name, devIntf)
	case "bridge":
		e.createDeviceInterface(name, devIntf)
		intf := e.interfaces[name]
		intf.typ = "bridge"
		e.interfaces[name] = intf
	case "wifi":
		return e.createWifi(name, devIntf)
	case "wireguard":
		return e.createWireguard(name, devIntf)
	default:
		return fmt.Errorf("invalid class '%s'", devIntf.Class)

	}
	return nil
}

func (e *emitter) createDeviceInterface(name string, devIntf DevInterface) error {
	if devIntf.Bridge != "" && devIntf.Device != "" {
		intf := e.interfaces[devIntf.Bridge]
		intf.ifname = append(intf.ifname, devIntf.Device)
		e.interfaces[devIntf.Bridge] = intf
		return nil
	}
	intf := e.interfaces[name]

	if devIntf.Device != "" {
		intf.ifname = append(intf.ifname, devIntf.Device)
	}

	if len(devIntf.IPAddres) != 0 {
		addr, err := IPFromBytes([]byte(devIntf.IPAddres[0]))
		if err != nil {
			return err
		}
		intf.ipAddres = addr
	}

	//defaults to true. ""
	intf.dhcp = true
	switch dhcp := devIntf.Dhcp.(type) {
	case string:
		switch dhcp {
		case "false", "no":
			intf.dhcp = false
		}
	case bool:
		intf.dhcp = dhcp
	}
	e.interfaces[name] = intf
	return nil
}

func (e *emitter) createWifi(name string, devIntf DevInterface) error {
	if devIntf.Device == "" {
		return fmt.Errorf("wifi interface missing device '%s'", name)
	}
	if _, ok := e.devices[devIntf.Device]; ok {
		return fmt.Errorf("undeclared interface '%s' used in device '%s'", devIntf.Device, name)
	}
	var mode string
	switch devIntf.Wifi.Mode {
	case sta:
		mode = "sta"
	case ap:
		mode = "ap"
	case monitor:
		mode = "monitor"
	}
	_, err := fmt.Fprintf(e.wireless, `
config wifi-iface   '%s'
	option device   '%s'
	option mode     '%s'
	option ifname   '%s'
`, name, devIntf.Device, mode, name)
	if err != nil {
		return err
	}

	if devIntf.Bridge != "" {
		_, err = fmt.Fprintf(e.wireless, "    option network  '%s'\n", devIntf.Bridge)
		if err != nil {
			return err
		}
	}
	if devIntf.Wifi.Ssid != "" {
		_, err = fmt.Fprintf(e.wireless, "    option ssid     '%s'\n", devIntf.Bridge)
		if err != nil {
			return err
		}

		switch devIntf.Wifi.Auth {
		case psk2:
			_, err = fmt.Fprint(e.wireless, "    option encryption 'psk2' \n")
		case none:
			_, err = fmt.Fprint(e.wireless, "    option encryption 'none' \n")
		}
		if err != nil {
			return err
		}

	}
	if devIntf.Wifi.Key != "" {
		_, err = fmt.Fprintf(e.wireless, "    option key       '%s'\n", devIntf.Wifi.Key)
		if err != nil {
			return err
		}

	}
	return nil
}

func (e *emitter) createWireguard(name string, devIntf DevInterface) error {
	if devIntf.Wireguard.PrivateKey == "" {
		return fmt.Errorf("missing [interface.%s.wireguard] section", name)
	}
	_, err := fmt.Fprintf(e.network, `
config interface '%s'
	option proto 'wireguard'
	option private_key '%s'
	option disabled '0'
`, name, devIntf.Wireguard.PrivateKey)
	if err != nil {
		return err
	}

	for _, addr := range devIntf.IPAddres {
		_, err = fmt.Fprintf(e.network, "    list addresses '%v'\n", addr)
		if err != nil {
			return err
		}
	}

	if len(devIntf.Wireguard.Peers) != 1 {
		return fmt.Errorf("need exactly one [interface.%s.wireguard.peers] section", name)
	}
	peer := devIntf.Wireguard.Peers[0]
	autoroute := "0"
	if peer.Autoroute {
		autoroute = "1"
	}

	_, err = fmt.Fprintf(e.network, `
config wireguard_%s
	option public_key '%s'
	option route_allowed_ips '%s'
`, name, peer.PublicKey, autoroute)
	if err != nil {
		return err
	}

	if peer.Psk != "" {
		_, err = fmt.Fprintf(e.network, "    option preshared_key '%v'\n", peer.Psk)
		if err != nil {
			return err
		}
	}

	endpoints := strings.Split(peer.Endpoint, ":")
	if len(endpoints) != 2 {
		return fmt.Errorf("invalid wg endpoint: %s", peer.Endpoint)
	}
	endpointPort, err := strconv.Atoi(endpoints[1])
	if err != nil {
		return fmt.Errorf("invalid wg endpoint port: %s: %s", endpoints[1], err)
	}

	_, err = fmt.Fprintf(e.network, `    option endpoint_port '%d'
	option endpoint_host
'%s'`, endpointPort, endpoints[0])
	if err != nil {
		return err
	}

	if peer.Keepalive != nil {
		_, err = fmt.Fprintf(e.network, "    option persistent_keepalive '%d'\n", *peer.Keepalive)
		if err != nil {
			return err
		}
	}

	for _, route := range peer.Routes {
		_, err = fmt.Fprintf(e.network, "    list allowed_ips '%s'\n", route)
		if err != nil {
			return err
		}
	}

	return nil
}

func (e *emitter) loadDevice(name string, dev Device) error {
	e.devices[name] = struct{}{}
	switch dev.Class {
	case "wifi":
		if dev.Path == "" {
			return fmt.Errorf("wifi device must be matched by path %s", name)
		}
		path := dev.Path
		if !strings.HasPrefix(dev.Path, "/sys/device") {
			path, _ = filepath.EvalSymlinks(filepath.Join(dev.Path, "device"))
			//if the error != nil, path will be empty and thus will fail the next check.
		}
		if !strings.HasPrefix(path, "/sys/devices/") {
			return fmt.Errorf("path %s did not resolve to a sysfs device", path)
		}

		path = strings.TrimPrefix(path, "/sys/devices/")

		_, err := fmt.Fprintf(e.wireless, `
config wifi-device '%s'
	option type     'mac80211'
	option path     '%s'
`, name, path)
		if err != nil {
			return err
		}

		if dev.Wifi == nil {
			return fmt.Errorf("missing [device.%s.wifi] section", name)
		}
		//all we need to do is make sure its one of the suported input types.
		//fmt will format strings and ints for us.
		_, ok1 := dev.Wifi.Channel.(int)
		_, ok2 := dev.Wifi.Channel.(string)
		if ok1 || ok2 {
			_, err = fmt.Fprintf(e.wireless, "    option channel  '%v'\n", dev.Wifi.Channel)
			if err != nil {
				return err
			}
		}

		if dev.Wifi.Htmode != "" {
			_, err = fmt.Fprintf(e.wireless, "    option htmode  '%v'\n", dev.Wifi.Htmode)
			if err != nil {
				return err
			}
		}
	default:
		return fmt.Errorf("invalid class '%s'", dev.Class)

	}
	return nil
}

func (e *emitter) loadTemplate(name string, tmpl Template) error {
	handlebar, err := raymond.ParseFile(filepath.Join("/etc/devguard/genesis/templates/", tmpl.Template))
	if err != nil {
		return err
	}
	res, err := handlebar.Exec(tmpl.Variables)
	if err != nil {
		return err
	}
	e.tplout[tmpl.Output] = res
	return nil
}

func (e *emitter) commit() error {
	err := os.MkdirAll("./etc/config", 0700)
	if err != nil {
		return err
	}
	file, err := os.Create("./etc/config/wireless")
	if err != nil {
		return err
	}
	e.wireless.WriteTo(file)
	file.Close()
	file, err = os.Create("./etc/config/network")
	if err != nil {
		return err
	}
	e.network.WriteTo(file)
	file.Close()
	for path, data := range e.tplout {
		file, err = os.Create(path)
		if err != nil {
			return err
		}
		file.WriteString(data)
	}

	return exec.Command("./etc/devguard/genesis/post").Run()
}

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
