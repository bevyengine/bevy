.PHONY: run
run: export BEVY_REFLECT_AUTO_REGISTER_STATIC = $(shell cargo metadata --no-deps --format-version 1 | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
run: 
	cargo run --features bevy/reflect_auto_register_static
