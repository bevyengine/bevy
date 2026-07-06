---
title: "Expose system accesses and filters in BRP `schedule.graph`"
pull_requests: [24743]
---

`bevy_dev_tools::SystemData` added fields `combined_access` and `filtered_accesses`

For example,

```rs
pub fn prepare_atmosphere_probe_components(
    probes: Query<(Entity, &AtmosphereEnvironmentMapLight), (Without<AtmosphereEnvironmentMap>,)>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
)
```

Generates the below from `schedule.graph` in BRP.

Note the values in `read_and_writes`, etc., are indexes into `components` array.

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "schedule_data": {
      "name": "Update",
      "systems": [
        {
          "name": "bevy_pbr::atmosphere::environment::prepare_atmosphere_probe_components",
          "apply_deferred": false,
          "deferred": true,
          "exclusive": false,
          "combined_access": {
            "archetypal": [],
            "read_and_writes": [
              2, // Assets<Image>
              3 // AtmosphereEnvironmentMapLight
            ],
            "read_and_writes_inverted": false,
            "writes": [
              2 // Assets<Image>
            ],
            "writes_inverted": false
          },
          "filtered_accesses": [
            {
              "access": {
                "archetypal": [],
                "read_and_writes": [
                  3 // AtmosphereEnvironmentMapLight
                ],
                "read_and_writes_inverted": false,
                "writes": [],
                "writes_inverted": false
              },
              "filter_sets": [
                {
                  "with": [
                    3 // AtmosphereEnvironmentMapLight
                  ],
                  "without": [
                    4, // Disabled
                    6 // AtmosphereEnvironmentMap
                  ]
                }
              ],
              "required": [
                3 // AtmosphereEnvironmentMapLight
              ]
            },
            {
              "access": {
                "archetypal": [],
                "read_and_writes": [
                  2 // Assets<Image>
                ],
                "read_and_writes_inverted": false,
                "writes": [
                  2 // Assets<Image>
                ],
                "writes_inverted": false
              },
              "filter_sets": [
                {
                  "with": [
                    0, // IsResource
                    2 // Assets<Image>
                  ],
                  "without": []
                }
              ],
              "required": [
                2 // Assets<Image>
              ]
            }
          ]
        },
        ...
      ],
      "components": [
        {
          "name": "bevy_ecs::resource::IsResource",
          "required": []
        },
        {
          "name": "bevy_ui_widgets::dialog::DialogStack",
          "required": [
            0
          ]
        },
        {
          "name": "bevy_asset::assets::Assets<bevy_image::image::Image>",
          "required": [
            0
          ]
        },
        {
          "name": "bevy_light::probe::AtmosphereEnvironmentMapLight",
          "required": []
        },
        {
          "name": "bevy_ecs::entity_disabling::Disabled",
          "required": []
        },
        {
          "name": "bevy_render::sync_world::SyncToRenderWorld",
          "required": []
        },
        {
          "name": "bevy_pbr::atmosphere::environment::AtmosphereEnvironmentMap",
          "required": [
            5
          ]
        },
        ...
      ],
      ...
    }
  }
}
```
