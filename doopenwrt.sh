set -ex
zz build

cd ~/proj/captif/openwrt/
make V=s -j20 package/devguard/genesis/{clean,compile}
scp ~/proj/captif/openwrt/build_dir/target-mips_24kc_musl/genesis-0.13/genesis  root@192.168.0.83:/tmp/genesis

