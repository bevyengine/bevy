# Bevy Outline

## High Level

* ECS at its core (but only where needed)
* Simple api backed by flexible systems
    * ex: PBR renderer built on a render graph system 
* Flexbox ui
    * simple, standard, good implementations exist
* 100% rust (except for the few cases where this is impossible)
* Batteries included
    * 2d/3d rendering, ui, physics, networking, etc
* Editor: also a "game"
    * dogfood components
* Fast app compile times (< 5 seconds) 

## Dependencies

* Legion ecs
* wgfx-rs
* nalgebra
* nphysics/ncollide

## Outline

* Core
    * Shared
        * Types
            * enum PropertyValue
                * DATATYPE_WRAPPERS_HERE
                * Analog: godot's Variant
            * struct Property
                * Description: Dynamic data
                    * Ex: exported to editor, uniforms in shaders 
                * Tags: ```HashSet<string>```
            * struct Texture
        * Components
            <!-- Hierarchy -->
            * Parent
                * Children ```Vec<EntityId>```
            <!-- Properties-->
            * Properties
                * ```HashMap<string, Property>```
            <!-- Rendering -->
            * Mesh
            * Armature
            * Material 
        * Systems
            <!-- Rendering -->
            * UpdateArmatureTransforms
            * SyncPropertiesToMaterialUniforms
    * 3d
        * Components
            <!-- Position -->
            * Transform
            * GlobalTransform
            <!-- Physics -->
            * PhysicsBody
            * CollisionShape
            * RigidBody
        * Systems
            <!-- Position -->
            * CalculateGlobalTransform
                * Dep: Child, GlobalTransform, Transform
            <!-- Physics -->
            * UpdateCollisions/NCollide
                * Dep: CollisionShape, PhysicsBody, GlobalTransform
            * UpdateRigidBodies/NCollide
                * Dep: PhysicsBody, RigidBody, GlobalTransform
    * 2d
        * Components
            <!-- Position -->
            * Transform2d
            * GlobalTransform2d
            <!-- UI -->
            * Element
            <!-- Physics -->
            * PhysicsBody2d
            * CollisionShape2d
            * RigidBody2d
        * Systems
            <!-- Position -->
            * CalculateGlobalTransform2d
                * Dep: Child, GlobalTransform2d, Transform2d
            <!-- Physics -->
            * UpdateCollisions2d/NCollide
                * Dep: CollisionShape2d, PhysicsBody2d, GlobalTransform2d
            * UpdateRigidBodies2d/NCollide
                * Dep: PhysicsBody2d, RigidBody2d, GlobalTransform2d

