# [![Bevy](assets/branding/bevy_logo_light_small.svg)](https://bevyengine.org)

[![Crates.io](https://img.shields.io/crates/v/bevy.svg)](https://crates.io/crates/bevy)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/blob/master/LICENSE)
[![Crates.io](https://img.shields.io/crates/d/bevy.svg)](https://crates.io/crates/bevy)
[![Rust](https://github.com/bevyengine/bevy/workflows/CI/badge.svg)](https://github.com/bevyengine/bevy/actions)
![iOS cron CI](https://github.com/bevyengine/bevy/workflows/iOS%20cron%20CI/badge.svg)

## O que é Bevy?

Bevy é um motor de jogo incrivelmente simples, baseado em dados e construído em Rust. É gratuito e de código aberto para sempre!

## ATENÇÃO

Bevy ainda está nos estágios _muito_ iniciais de desenvolvimento. As APIs podem e irão mudar (agora é a hora de fazer sugestões!). Recursos importantes estão faltando. A documentação é escassa. Por favor, não construa nenhum projeto sério em Bevy a menos que esteja preparado para ser interrompido por mudanças de API constantemente.

## Metas de Design

* **Capaz**: Oferece um conjunto completo de recursos 2D e 3D
* **Simples**: Fácil de aprender para iniciantes, mas infinitamente flexível para usuários avançados
* **Focado em Dados**: Arquitetura orientada a dados usando o paradigma Entity Component System
* **Modular**: Use apenas o que precisar. Substitua o que você não gosta
* **Rápido**: A lógica do aplicativo deve ser executada rapidamente e, quando possível, em paralelo
* **Produtivo**: As alterações devem ser compiladas rapidamente... esperar não é divertido

## Sobre

* **[Recursos](https://bevyengine.org):** Uma rápida visão geral dos recursos de Bevy.
* **[Roteiro](https://github.com/bevyengine/bevy/projects/1):**O plano de desenvolvimento da equipe de Bevy.
* **[Apresentando Bevy](https://bevyengine.org/news/introducing-bevy/)**: Uma postagem de um blog cobrindo alguns dos recursos de Bevy

## Documentos

* **[O Livro De Bevy](https://bevyengine.org/learn/book/introduction):** Documentação oficial de Bevy. O melhor lugar para começar a aprender Bevy.
* **[Documentos da API Rust de Bevy](https://docs.rs/bevy):** Documentos da API Rust de Bevy, que são gerados automaticamente a partir dos comentários de documentos neste repositório.

## Comunidade

Antes de contribuir ou participar de discussões com a comunidade, você deve se familiarizar com nosso **[Código de Conduta](https://github.com/bevyengine/bevy/blob/master/CODE_OF_CONDUCT.md)**

* **[Discord](https://discord.gg/gMUk5Ph):** Servidor oficial de Bevy no discord.
* **[Reddit](https://reddit.com/r/bevy):** Subreddit oficial de Bevy.
* **[Stack Overflow](https://stackoverflow.com/questions/tagged/bevy):** Perguntas marcadas com Bevy no Stack Overflow.
* **[Incrível Bevy](https://github.com/bevyengine/awesome-bevy):** Uma coleção de projetos incríveis da Bevy.

## Começando

Recomendamos verificar [Livro de receitas](https://bevyengine.org/learn/book/introduction) para um tutorial completo.

Siga o [Guia de Configuração](https://bevyengine.org/learn/book/getting-started/setup/) para garantir que seu ambiente de desenvolvimento esteja configurado corretamente.
Uma vez configurado, você pode testar rapidamente os [exemplos](/examples), clonando este repositório e executando o seguinte comando:

```sh
# Executa o exemplo "breakout"
cargo run --example breakout
```

### Compilar Rapidamente

O Bevy pode ser construído perfeitamente usando a configuração padrão no Rust estável. No entanto, para compilações iterativas realmente rápidas, você deve habilitar a configuração de "compilações rápidas" [seguindo as instruções aqui](http://bevyengine.org/learn/book/getting-started/setup/).

## Áreas de Foco

Bevy tem as seguintes [Áreas de Foco](https://github.com/bevyengine/bevy/labels/focus-area). Atualmente, estamos concentrando nossos esforços de desenvolvimento nessas áreas, e elas receberão prioridade durante o tempo dos desenvolvedores Bevy. Se você gostaria de contribuir com Bevy, você é fortemente encorajado a se juntar a estes esforços:

### [Editor-Ready UI](https://github.com/bevyengine/bevy/issues/254)

### [PBR / Clustered Forward Rendering](https://github.com/bevyengine/bevy/issues/179)

### [Scenes](https://github.com/bevyengine/bevy/issues/255)

## Bibliotecas Usadas

Bevy só é possível por causa do trabalho árduo colocado nessas tecnologias fundamentais:

* [wgpu-rs](https://github.com/gfx-rs/wgpu-rs): Biblioteca gráfica moderna/de baixo nível/multiplataforma inspirada em Vulkan.
* [glam-rs](https://github.com/bitshifter/glam-rs): Uma biblioteca matemática 3D simples e rápida para jogos e gráficos
* [winit](https://github.com/rust-windowing/winit): Criação e gerenciamento de janela multiplataforma em Rust
* [spirv-reflect](https://github.com/gwihlidal/spirv-reflect-rs): API de reflexão em Rust para shader byte-code SPIR-V

## [Características do Cargo em Bevy][cargo_features]

Esta [lista][cargo_features] descreve os diferentes recursos de cargo suportados por Bevy. Isso permite que você personalize o conjunto de recursos do Bevy para seu caso de uso.

[cargo_features]: docs/cargo_features.md

## Obrigado e Algumas Alternativas

Além disso, gostaríamos de agradecer aos projetos [Amethyst](https://github.com/amethyst/amethyst), [macroquad](https://github.com/not-fl3/macroquad), [coffee](https://github.com/hecrj/coffee), [ggez](https://github.com/ggez/ggez), [rg3d](https://github.com/mrDIMAS/rg3d), e [Piston](https://github.com/PistonDevelopers/piston) por fornecer exemplos sólidos de desenvolvimento de motor de jogo em Rust. Se você está procurando um motor de jogo Rust, vale a pena considerar todas as suas opções. Cada mecanismo tem objetivos de design diferentes e alguns provavelmente farão mais sentido com você do que outros.
