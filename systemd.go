// +build systemd, !openwrt

package main

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
)

type emitter struct {
	prefix string
}

func newEmitter() emitter {
	return emitter{
		prefix: "/",
	}
}

func (se emitter) load(config Genesis) error {
	for name, devIntf := range config.Interfaces {
		se.loadDeviceInterface(name, devIntf)
	}
	return exec.Command("systemctl", "restart", "systemd-networkd").Run()
}

func (se emitter) commit() error {
	return nil
}

func (se emitter) loadDeviceInterface(name string, devIntf DevInterface) error {
	path := filepath.Join(se.prefix, "etc/systemd/network/")
	err := os.MkdirAll(path, 0700)
	if err != nil {
		return err
	}
	path = filepath.Join(path, fmt.Sprintf("20-genesis-%s.network", name))
	file, err := os.Create(path)
	if err != nil {
		return err
	}
	if devIntf.Device == "" {
		return errors.New("interface.device is required")
	}
	_, err = fmt.Fprintf(file, "[Match]\nName=%s\n\n", devIntf.Device)
	if err != nil {
		return err
	}

	var dhcpSetting string
	switch dhcp := devIntf.Dhcp.(type) {
	case string:
		switch dhcp {
		case "yes", "true":
			dhcpSetting = "true"
		case "no", "false":
			break
		case "ipv4":
			dhcpSetting = "ipv4"
		case "ipv6":
			dhcpSetting = "ipv6"
		}
	case bool:
		if dhcp {
			dhcpSetting = "true"
		}
	}
	if devIntf.Proto == "dhcp" {
		dhcpSetting = "ipv4"
	}
	if dhcpSetting != "" {
		_, err = fmt.Fprintf(file, `[Network]
DHCP=%s
[DHCP]
RouteMetric=10
`, dhcpSetting)
		if err != nil {
			return err
		}

	} else {
		_, err = fmt.Fprintf(file, "[Network]\n", devIntf.Device)
		if err != nil {
			return err
		}
		if len(devIntf.IPAddres) == 0 {
			return errors.New("either interface.dhcp or ipaddr required")
		}

		for _, ip := range devIntf.IPAddres {
			_, err = fmt.Fprintf(file, "Address=%s\n", ip)
			if err != nil {
				return err
			}
		}
		if devIntf.Gateway != "" {
			_, err = fmt.Fprintf(file, "Gateway=%s\n", devIntf.Gateway)
			if err != nil {
				return err
			}
		}
		for _, dns := range devIntf.DNS {
			_, err = fmt.Fprintf(file, "DNS=%s\n", dns)
			if err != nil {
				return err
			}
		}
	}
	file.Close()
	switch devIntf.Wifi.Mode {
	case sta:
		path := filepath.Join(se.prefix, "etc/systemd/network/")
		os.RemoveAll(path)
		os.MkdirAll(path, 0700)
		path = filepath.Join(path, fmt.Sprintf("wpa_supplicant-%s.conf", devIntf.Device))
		file, err = os.Create(path)
		if err != nil {
			return err
		}
		_, err = fmt.Fprintf(file, `
ctrl_interface=/var/run/wpa_supplicant
ap_scan=1
`)
		if err != nil {
			return err
		}
		for _, sta := range devIntf.STA {
			if sta.Auth == psk2 && sta.Key != "" {
				_, err = fmt.Fprintf(file, `
network={{
	key_mgmt=WPA-PSK
	ssid="%s"
	psk="%s"
}}
`, sta.Ssid, sta.Key)
				if err != nil {
					return err
				}
			} else {
				_, err = fmt.Fprintf(file, `
network={{
	ssid="%s"
	key_mgmt=NONE
}}
`, sta.Ssid)
				if err != nil {
					return err
				}
			}
		}
		file.Close()
		err = exec.Command("systemctl", "start", fmt.Sprintf("wpa_supplicant@%s.service", devIntf.Device)).Run()
		if err != nil {
			return err
		}
	case ap, monitor:
		return errors.New("wifi mode not supported")
	}
	return nil
}
