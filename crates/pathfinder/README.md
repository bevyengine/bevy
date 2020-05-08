# Pathfinder Fork

Why fork pathfinder? WebGPU does not support some of the datatypes pathfinder uses in its shaders (ex: Short1, Char1). This means both the base shaders and the pathfinder_renderer code needed to change to accommodate that. Ideally these changes can either be merged directly into pathfinder, or be hidden behind a feature flag.


Forked from commit 84bf4341c253ae36756d27671a17cb44d59cd250