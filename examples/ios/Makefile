.PHONY: xcodebuild run install boot-sim generate clean

DEVICE = ${DEVICE_ID}
ifndef DEVICE_ID
	DEVICE=$(shell xcrun simctl list devices 'iOS' | grep -v 'unavailable' | grep -v '^--' | grep -v '==' | head -n 1 | cut -d ' ' -f 7 | sed 's/[()]//g')
endif

run: install
	xcrun simctl launch --console $(DEVICE) com.rust.bevy-ios-example

boot-sim:
	xcrun simctl boot $(DEVICE) || true

install: xcodebuild-simulator boot-sim
	 xcrun simctl install $(DEVICE) build/Build/Products/Debug-iphonesimulator/bevy_ios_example.app

clean:
	rm -r build
	cargo clean
