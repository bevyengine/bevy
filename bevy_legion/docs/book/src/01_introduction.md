# Introduction

Welcome to the Legion book! This book is intended to be a summary overview of legion, including: 
- An overview of how to use it
- Some examples of different use case scenarios
- how it is different than other Entity-Component-Systems in the rust ecosystem
- Overviews of some pertinent internals

This book assumes a general understanding of the concepts of the Entity-Component-System design and data composition as a design pattern. If you need a summary of what an ECS is, please see the [Wikipedia article on ECS].

## Design

Legions internal architecture is heavily inspired by the new Unity ECS architecture [^1], while the publicly facing API is strongly built upon specs [^2], while expanding on it and learning from many of the faults found in that API.

#### Quick Version
The core concept of Legion design is based around the concept of `Entities`, `Archetypes` and `Chunks`. These three core concepts are the building blocks of legion, and its entity component system.

##### Entities
Entities are strictly ID's, allocated within a given `Universe` of legion, which allow for uniquely referencing component instances. ID's may be reused generationally, but legion guarantees that they are unique in any given universe; this is accomplished by providing each `World` in a `Universe` its own Entity Allocator, which will be unique in that universe. 

##### Archetypes
An Archetype is considered a "Grouping of Components and Tags". Entities may have varying numbers and types of components; any combination of these tags and components is considered an `Archetype`. In legion, entity storage and parallelization of system execution are all centered on this concept of Archetypes, or like-entities. 
 


## Other resources



[^1]: https://docs.unity3d.com/Packages/com.unity.entities@0.1/manual/ecs_core.html
[^2]: https://github.com/amethyst/specs

[Wikipedia article on ECS]: https://en.wikipedia.org/wiki/Entity_component_system
