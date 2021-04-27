# Bevy ECS

[![Crates.io](https://img.shields.io/crates/v/bevy_ecs.svg)](https://crates.io/crates/bevy_ecs)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/gMUk5Ph)

## What is Bevy ECS?

This crate contains the Entity Component System (ECS) implementation used in and developed for the game engine [Bevy][bevy]. Even though it was created as part of the game engine, Bevy ECS can be used standalone and in combination with other projects or game engines.

## About

Entity component system is an architectural pattern using composition to provide greater flexibility.

### Main concepts

* Entities are identifiers
* Components are data structures that can be attached to entities
* Systems encode a certain behaviour of sets of entities selected based on their components

Entities and components are kept in a `World`. Constructing a `Schedule` with systems allows you to simulate a tick of the world, which results in systems manipulating entities and their components.

## Features

Bevy ECS uses Rust's type safety to represent systems as "normal" functions and components as structs. In most cases this does not require any additional effort by the user.

### Component storage

A unique feature of Bevy ECS is the support for multiple component storage types.

* Tables: fast and cache friendly iteration, slower adding and removing of components
* Sparse Sets: fast add/remove, slower iteration

The used storage type can be configured per component and defaults to table storage.

### Resources

A common pattern when working with ECS is the creation of global singleton components. Bevy ECS makes this pattern a first class citizen. Resources are a special kind of component that do not belong to any entity.

[bevy]: https://bevyengine.org/
