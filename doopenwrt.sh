set -ex
zz build --release


rm -rf ~/proj/captif/openwrt/package/devguard/genesis/src/
mkdir -p ~/proj/captif/openwrt/package/devguard/genesis/src/

cp -a target/gen target/c/ target/make/ target/release ~/proj/captif/openwrt/package/devguard/genesis/src/


cd ~/proj/captif/openwrt/
make V=s CONFIG_DEBUG=y -j20 package/devguard/genesis/{clean,compile}
scp ~/proj/captif/openwrt/build_dir/target-mips_24kc_musl/genesis-0.13/genesis root@192.168.0.187:/tmp/genesis

