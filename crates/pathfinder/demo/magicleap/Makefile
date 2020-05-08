.PHONY: debug release install

debug:
	cargo build --target aarch64-linux-android
	$(MAGICLEAP_SDK)/mabu -t debug_device PathfinderDemo.package -s $(MLCERT)

release:
	cargo build --release --target aarch64-linux-android
	$(MAGICLEAP_SDK)/mabu -t release_device PathfinderDemo.package -s $(MLCERT)

install: .out/PathfinderDemo/PathfinderDemo.mpk
	$(MAGICLEAP_SDK)/tools/mldb/mldb install -u .out/PathfinderDemo/PathfinderDemo.mpk

run:
	${MAGICLEAP_SDK}/tools/mldb/mldb launch -w com.mozilla.pathfinder.demo
