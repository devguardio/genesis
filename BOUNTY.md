this is the readme for a contract post.


genesis is a configuration generator for IoT devices.
it supports populating /etc/ from a single toml file.


the contract job is to rwwrite it it in golang.

- read the toml
- generate openwrt or systemd /etc/ configuration files relative to PWD



see this input file:
https://github.com/devguardio/genesis/blob/master/tests/a.toml

on openwrt, generate a new file etc/config/network
see https://openwrt.org/docs/guide-user/base-system/basic-networking

as well as  etc/config/wireless
see https://openwrt.org/docs/guide-user/network/wifi/basic

for systemd see
https://www.freedesktop.org/software/systemd/man/systemd.network.html


for example for this input:

```
[interface.public]
class   = 'bridge'
ipaddr  = ['192.168.44.1/24']



```
generate an entry in /etc/config/network like
```
config interface 'public'
  option type 'bridge'
  option ifname 'wan'
  option proto 'static'
  option ipaddr '192.168.44.1'
  option netmask '255.255.255.0'
```

or on systemd generate a new file:

/etc/systemd/network/public.netdev
```
[NetDev]
Name=bridge
Kind=bridge

```

and /etc/systemd/network/public.network

```
[Match]
Name=public

[Network]
Address=192.168.44.1/24
```


for this input:

```
[interface.wan]
class   = 'bridge'
dhcp    = true

[interface.eth0]
device  = 'eth0'
bridge  = 'wan'

[interface.eth1]
device  = 'eth1'
bridge  = 'wan'
```

accumulate the devices into 

```
config interface 'wan'
  option type 'bridge'
  option ifname 'eth0 eth1'
```

on systemd they are not accumulated.
```
/etc/systemd/network/eth0.network
[Match]
Name=eth0*

[Network]
Bridge=wan
```


see the manual of systemd and openwrt for more options
