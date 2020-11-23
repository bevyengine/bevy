# Exemplos

Esses exemplos demonstram os principais recursos de Bevy e como usá-los.
Para executar um exemplo, use o comando `cargo run --example <Example>`, e adicione a opção `--features x11` ou `--features wayland` para forçar o exemplo a ser executado em um compositor de janela específico, e.g.

```sh
cargo run --features wayland --example hello_world
```

### ⚠️ Nota: para usuários da versão de crates.io,

Devido às alterações e adições às APIs, geralmente há diferenças entre os exemplos de desenvolvimento e as versões lançadas do Bevy em crates.io.
Se você estiver usando uma versão de lançamento de [crates.io](https://crates.io/crates/bevy), veja os exemplos verificando a tag git apropriada, e.g., usuários de `0.3` devem usar os exemplos de [https://github.com/bevyengine/bevy/tree/v0.3.0/examples](https://github.com/bevyengine/bevy/tree/v0.3.0/examples)

Se você clonou o repositório do Bevy localmente, use `git checkout` com a tag da versão apropriada.
```
git checkout v0.3.0
```

---

### Tabela de Conteúdos

- [O Básico](#the-bare-minimum)
  - [Hello, World!](#hello-world)
- [Cross-Platform Examples](#cross-platform-examples)
  - [2D Rendering](#2d-rendering)
  - [3D Rendering](#3d-rendering)
  - [Application](#application)
  - [Assets](#assets)
  - [Audio](#audio)
  - [Diagnostics](#diagnostics)
  - [ECS (Entity Component System)](#ecs-entity-component-system)
  - [Games](#games)
  - [Input](#input)
  - [Scene](#scene)
  - [Shaders](#shaders)
  - [Tools](#tools)
  - [UI (User Interface)](#ui-user-interface)
  - [Window](#window)
- [Platform-Specific Examples](#platform-specific-examples)
  - [Android](#android)
  - [iOS](#ios)
  - [WASM](#wasm)

# O Básico

## Olá, Mundo!

Exemplo | Arquivo | Descrição
--- | --- | ---
`hello_world` | [`hello_world.rs`](./hello_world.rs) | Executa um exemplo mínimo que produz "hello world".

# Exemplos Multiplataforma

## Renderização 2D

Exemplo | Arquivo | Descrição
--- | --- | ---
`contributors` | [`2d/contributors.rs`](./2d/contributors.rs) | Exibe cada contribuidor como uma bola saltitante!
`sprite_sheet` | [`2d/sprite_sheet.rs`](./2d/sprite_sheet.rs) | Renderiza um sprite animado.
`sprite` | [`2d/sprite.rs`](./2d/sprite.rs) | Renderiza um sprite.
`texture_atlas` | [`2d/texture_atlas.rs`](./2d/texture_atlas.rs) | Gera um atlas de textura (folha de sprite) com sprites individuais.

## Renderização 3D

Exemplo | Arquivo | Descrição
--- | --- | ---
`3d_scene` | [`3d/3d_scene.rs`](./3d/3d_scene.rs) | Cena 3D simples com formas e iluminação básicas.
`load_gltf` | [`3d/load_gltf.rs`](./3d/load_gltf.rs) | Carrega e renderiza um arquivo gltf como uma cena.
`msaa` | [`3d/msaa.rs`](./3d/msaa.rs) | Configura MSAA (Multi-Sample Anti-Aliasing) para bordas mais suaves.
`parenting` | [`3d/parenting.rs`](./3d/parenting.rs) | Demonstra relacionamentos pai-> filho e transformações relativas.
`spawner` | [`3d/spawner.rs`](./3d/spawner.rs) | Processa um grande número de cubos com mudança de posição e material.
`texture` | [`3d/texture.rs`](./3d/texture.rs) | Mostra a configuração dos materiais de textura.
`z_sort_debug` | [`3d/z_sort_debug.rs`](./3d/z_sort_debug.rs) | Visualiza a ordem Z da câmera.

## Aplicação

Exemplo | Arquivo | Descrição
--- | --- | ---
`custom_loop` | [`app/custom_loop.rs`](./app/custom_loop.rs) | Demonstra como criar um runner personalizado (para atualizar um aplicativo manualmente).
`empty_defaults` | [`app/empty_defaults.rs`](./app/empty_defaults.rs) | Um aplicativo vazio com plugins padrão.
`empty` | [`app/empty.rs`](./app/empty.rs) | Um aplicativo vazio (não faz nada).
`headless` | [`app/headless.rs`](./app/headless.rs) | Um aplicativo que é executado sem plugins padrão.
`logs` | [`app/logs.rs`](./app/logs.rs) | Ilustra como usar a geração de saída de log.
`plugin_group` | [`app/plugin_group.rs`](./app/plugin_group.rs) | Demonstra a criação e registro de um grupo de plugins personalizados.
`plugin` | [`app/plugin.rs`](./app/plugin.rs) | Demonstra a criação e registro de um plugin personalizado.
`return_after_run` | [`app/return_after_run.rs`](./app/return_after_run.rs) | Mostra como voltar ao Main depois que o aplicativo for encerrado.
`thread_pool_resources` | [`app/thread_pool_resources.rs`](./app/thread_pool_resources.rs) | Cria e personaliza o pool interno de threads.

## Ativos

Exemplo | Arquivo | Descrição
--- | --- | ---
`asset_loading` | [`asset/asset_loading.rs`](./asset/asset_loading.rs) | Demonstra vários métodos para carregar ativos (assets).
`custom_asset` | [`asset/custom_asset.rs`](./asset/custom_asset.rs) | Implementa um carregador de ativos (assets) personalizado
`hot_asset_reloading` | [`asset/hot_asset_reloading.rs`](./asset/hot_asset_reloading.rs) | Demonstra o recarregamento automático de ativos(assets) quando modificados no disco.

## Audio

Exemplo | Arquivo | Descrição
--- | --- | ---
`audio` | [`audio/audio.rs`](./audio/audio.rs) | Mostra como carregar e reproduzir um arquivo de áudio.

## Diagnóstico

Exemplo | Arquivo | Descrição
--- | --- | ---
`custom_diagnostic` | [`diagnostics/custom_diagnostic.rs`](./diagnostics/custom_diagnostic.rs) | Mostra como criar um diagnóstico personalizado.
`print_diagnostics` | [`diagnostics/print_diagnostics.rs`](./diagnostics/print_diagnostics.rs) | Adicione um plugin que imprime diagnósticos para o console.

## ECS (Entity Component System)

Exemplo | Arquivo | Descrição
--- | --- | ---
`ecs_guide` | [`ecs/ecs_guide.rs`](./ecs/ecs_guide.rs) | Guia completo para ECS de Bevy.
`event` | [`ecs/event.rs`](./ecs/event.rs) | Ilustra a criação, ativação e recepção de eventos.
`hierarchy` | [`ecs/hierarchy.rs`](./ecs/hierarchy.rs) | Cria uma hierarquia de entidades pais e filhos.
`parallel_query` | [`ecs/parallel_query.rs`](./ecs/parallel_query.rs) | Ilustra consultas paralelas com `ParallelIterator`.
`startup_system` | [`ecs/startup_system.rs`](./ecs/startup_system.rs) | Demonstra um sistema de inicialização (que é executado uma vez quando o aplicativo é inicializado).

## Jogos

Exemplo | Arquivo | Descrição
--- | --- | ---
`breakout` | [`game/breakout.rs`](./game/breakout.rs) | Uma implementação do clássico jogo "Breakout".

## Entrada

Exemplo | Arquivo | Descrição
--- | --- | ---
`char_input_events` | [`input/char_input_events.rs`](./input/char_input_events.rs) | Imprime todos os caracteres à medida que são inseridos.
`gamepad_input_events` | [`input/gamepad_input_events.rs`](./input/gamepad_input_events.rs) | Itera e imprime os eventos de entrada e conexão do gamepad.
`gamepad_input` | [`input/gamepad_input.rs`](./input/gamepad_input.rs) | Mostra o manuseio da entrada, conexões e desconexões do gamepad.
`keyboard_input_events` | [`input/keyboard_input_events.rs`](./input/keyboard_input_events.rs) | Imprime todos os eventos de teclado.
`keyboard_input` | [`input/keyboard_input.rs`](./input/keyboard_input.rs) | Demonstra como lidar com um pressionamento/liberação de tecla.
`mouse_input_events` | [`input/mouse_input_events.rs`](./input/mouse_input_events.rs) | Imprime todos os eventos do mouse (botões, movimento, etc.).
`mouse_input` | [`input/mouse_input.rs`](./input/mouse_input.rs) | Demonstra como lidar com o pressionamento/liberação do botão do mouse.
`touch_input_events` | [`input/touch_input_events.rs`](./input/touch_input_input_events.rs) | Imprime todas as entradas dos toques.
`touch_input` | [`input/touch_input.rs`](./input/touch_input.rs) | Exibe pressionamentos, liberações e cancelamentos dos toques.

## Cena

Exemplo | Arquivo | Descrição
--- | --- | ---
`properties` | [`scene/properties.rs`](./scene/properties.rs) | Demonstra propriedades (semelhantes a reflexos em outros idiomas) em Bevy.
`scene` | [`scene/scene.rs`](./scene/scene.rs) | Demonstra o carregamento e o salvamento de cenas em arquivos.

## Shaders

Exemplo | Arquivo | Descrição
--- | --- | ---
`mesh_custom_attribute` | [`shader/mesh_custom_attribute.rs`](./shader/mesh_custom_attribute.rs) | Ilustra como adicionar um atributo personalizado a uma malha e usá-lo em um sombreador personalizado.
`shader_custom_material` | [`shader/shader_custom_material.rs`](./shader/shader_custom_material.rs) | Ilustra a criação de um material personalizado e um sombreador que o usa.
`shader_defs` | [`shader/shader_defs.rs`](./shader/shader_defs.rs) | Demonstra a criação de um material personalizado que usa "shaders defs" (uma ferramenta para alternar seletivamente as partes de um shader).

## Ferramentas

Exemplo | Arquivo | Descrição
--- | --- | ---
`bevymark` | [`tools/bevymark.rs`](./tools/bevymark.rs) | Uma carga de trabalho pesada para ver até onde Bevy pode levar seu sistema.

## UI (User Interface)

Exemplo | Arquivo | Descrição
--- | --- | ---
`button` | [`ui/button.rs`](./ui/button.rs) | Ilustra a criação e atualização de um botão.
`font_atlas_debug` | [`ui/font_atlas_debug.rs`](./ui/font_atlas_debug.rs) | Ilustra como FontAtlases são preenchidos (usados para otimizar a renderização de texto internamente).
`text_debug` | [`ui/text_debug.rs`](./ui/text_debug.rs) | Um exemplo para depuração de layout de texto.
`text` | [`ui/text.rs`](./ui/text.rs) | Ilustra a criação e atualização de texto.
`ui` | [`ui/ui.rs`](./ui/ui.rs) | Ilustra vários recursos da IU do Bevy.

## Janela

Exemplo | Arquivo | Descrição
--- | --- | ---
`clear_color` | [`window/clear_color.rs`](./window/clear_color.rs) | Cria uma janela de cor sólida.
`multiple_windows` | [`window/multiple_windows.rs`](./window/multiple_windows.rs) | Cria duas janelas e câmeras visualizando a mesma malha.
`window_settings` | [`window/window_settings.rs`](./window/window_settings.rs) | Demonstra como personalizar configurações de janela padrão.

# Exemplos Específicos de Plataforma

## Android

#### Configuração

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-apk
```

O Android SDK deve ser instalado e a variável de ambiente `ANDROID_SDK_ROOT` definida para a pasta root Android `sdk`.

Ao usar `NDK (Side by side)`, a variável de ambiente `ANDROID_NDK_ROOT` também deve ser definido como um dos NDKs em `sdk\ndk\[NDK number]`.

#### Construir e Executar

Para executar em uma configuração de dispositivo para desenvolvimento Android, execute:

```sh
cargo apk run --example android
```

:warning: No momento, o Bevy não funciona no Emulador Android.

Ao usar Bevy como uma biblioteca, os seguintes campos devem ser adicionados a `Cargo.toml`:

```toml
[package.metadata.android]
build_targets = ["aarch64-linux-android", "armv7-linux-androideabi"]
target_sdk_version = 29
min_sdk_version = 16
```

Por favor referencie `cargo-apk` [LEIA-ME](https://crates.io/crates/cargo-apk) para outros campos de manifesto do Android.

#### Telefones Antigos

Por padrão, o Bevy tem como alvo a API Android nível 29 em seus exemplos, que é o [API mínima da Play Store para fazer upload ou atualizar aplicativos](https://developer.android.com/distribute/best-practices/develop/target-sdk). Usuários de telefones mais antigos podem querer usar uma API mais antiga durante o teste.

Para usar uma API diferente, os seguintes campos devem ser atualizados em Cargo.toml:

```toml
[package.metadata.android]
target_sdk_version = >>API<<
min_sdk_version = >>API or less<<
```

## iOS

#### Configuração

```sh
rustup target add aarch64-apple-ios x86_64-apple-ios
cargo install cargo-lipo
```

#### Construir e Executar

Usando o bash:

```sh
cd examples/ios
make run
```

Em um mundo ideal, isso irá inicializar, instalar e executar o aplicativo para o primeiro simulador iOS em seu `xcrun simctl devices list`. Se isso falhar, você pode especificar o UUID do simulador de dispositivo via:

```sh
DEVICE_ID=${YOUR_DEVICE_ID} make run
```

Se você gostaria de ver o xcode fazer coisas, você pode executar

```sh
open bevy_ios_example.xcodeproj/
```

que irá abrir o xcode. Em seguida, você deve apertar o botão zoom zoom play e aguardar a mágica.

A GUI de construção do Xcode irá, por padrão, construir a biblioteca de Rust para ambos `x86_64-apple-ios`, e `aarch64-apple-ios` o que pode demorar um pouco. Se quiser acelerar isso, atualize a variável de ambiente `IOS_TARGETS` definida pelo usuário no "destino `cargo_ios`" para ser `x86_64-apple-ios` ou `aarch64-applo-ios`, depende do seu objetivo.

Nota: se você atualizar esta variável no Xcode, também mudará o padrão usado para o `Makefile`.

## WASM

#### Configuração

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

#### Construir e Executar

A seguir está um exemplo para `headless_wasm`. Para outros exemplos no wasm/diretório altere o `headless_wasm` nos seguintes comandos **e altere** `examples/wasm/index.html` apontar para o arquivo correto `.js`.

```sh
cargo build --example headless_wasm --target wasm32-unknown-unknown --no-default-features
wasm-bindgen --out-dir examples/wasm/target --target web target/wasm32-unknown-unknown/debug/examples/headless_wasm.wasm
```

Em seguida, envie o diretório `examples/wasm` para o navegador. ou seja

```sh
basic-http-server examples/wasm
```

Exemplo | Arquivo | Descrição
--- | --- | ---
`hello_wasm` | [`wasm/hello_wasm.rs`](./wasm/hello_wasm.rs) | Executa um exemplo mínimo que registra "hello world" no console do navegador
`headless_wasm` | [`wasm/headless_wasm.rs`](./wasm/headless_wasm.rs) | Configura um executor de programação e registra continuamente um contador para o console do navegador
`assets_wasm` | [`wasm/assets_wasm.rs`](./wasm/assets_wasm.rs) | Demonstra como carregar ativos do wasm
`winit_wasm` | [`wasm/winit_wasm.rs`](./wasm/winit_wasm.rs) | Registra a entrada do usuário no console do navegador. Requer os recursos `bevy_winit`
