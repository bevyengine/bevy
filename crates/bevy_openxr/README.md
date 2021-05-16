

## Testing

### Prequisites

Monado

    git clone https://gitlab.freedesktop.org/monado/monado.git
    git checkout 27f872fdfc95bba6fce4c70e4c33af7e66bd7cd8

    cd monado
    meson --prefix=~/.local -Dinstall-active-runtime=false build
    ninja -C build install

    # TODO: document to the rest, probably need to create /etc entry for runtime

    # Start monado service
    monado-service


### Running

    cargo watch -x "test -- --nocapture"

