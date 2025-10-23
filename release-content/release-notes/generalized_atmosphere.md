---
title: "Generalized Atmospheric Scattering Media"
authors: ["@ecoskey"]
pull_requests: [20838]
---

Until now, Bevy's atmospheric scattering system has been fast and beautiful, but
not very customizable. There's only a limited number of ways to customize the
existing parameters, which constrain the system to mostly earth-like scenes.

Bevy 0.18 introduces a new `ScatteringMedium` asset for designing atmospheric
scattering media of all kinds: clear desert skies, foggy coastlines, and
even atmospheres of other planets! We've used Bevy's asset system to the
fullest--alongside some custom optimizations--to make sure rendering stays
fast even for complicated scattering media.

```rust
fn setup_camera(
    mut commands: Commands,
    mut media: ResMut<Assets<ScatteringMedium>>,
) {
    // Also feel free to use `ScatteringMedium::earthlike()`!
    let medium = media.add(ScatteringMedium::new(
        256,
        256,
        [
            ScatteringTerm {
                absorption: Vec3::ZERO,
                scattering: Vec3::new(5.802e-6, 13.558e-6, 33.100e-6),
                falloff: Falloff::Exponential { strength: 12.5 },
                phase: PhaseFunction::Rayleigh,
            },
            ScatteringTerm {
                absorption: Vec3::splat(3.996e-6),
                scattering: Vec3::splat(0.444e-6),
                falloff: Falloff::Exponential { strength: 83.5 },
                phase: PhaseFunction::Mie { asymmetry: 0.8 },
            },
            ScatteringTerm {
                absorption: Vec3::new(0.650e-6, 1.881e-6, 0.085e-6),
                scattering: Vec3::ZERO,
                falloff: Falloff::Tent {
                    center: 0.75,
                    width: 0.3,
                },
                phase: PhaseFunction::Isotropic,
            },
        ],
    ));

    commands.spawn((
        Camera3d,
        Atmosphere::earthlike(medium)
    ));
}

// We've provided a nice `EarthlikeAtmosphere` resource
// for the most common case :)
fn setup_camera_simple(
    mut commands: Commands,
    earthlike_atmosphere: Res<EarthlikeAtmosphere>
) {
    commands.spawn((
        Camera3d,
        earthlike_atmosphere.get(),
    ));
}
```

(TODO: engine example of martian/extraterrestrial sunrise)

Alongside this change we've also added a bunch of documentation, and links to
learn more about the technical terms used. It's definitely a complex feature
under the hood, so we're hoping to make the learning curve a little less steep :)
