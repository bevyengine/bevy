"""Export an ORCA Zero-Day FBX as one self-contained glTF binary (.glb) for the
`zero_day` Bevy example.

Usage:
    blender --background --python convert.py -- <input.fbx> <output.glb>

The example's README.md lists the command line and the output filename for each measure.
The comments at each step below explain what the conversion does and why.
"""

import glob
import json
import os
import struct
import sys

import bpy
import numpy as np

argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
if len(argv) != 2:
    raise SystemExit(
        "usage: blender --background --python convert.py -- <input.fbx> <output.glb>"
    )
src, dst = argv
# Keep the path absolute, because Blender reads the textures again at export time from a
# different working directory.
texdir = os.path.abspath(os.path.join(os.path.dirname(src), "tex"))

# Import the FBX for the geometry, the object/material relations, and the names only.
# Blender's FBX importer misreads the material conventions of this Octane export, so the
# script ignores the imported material graphs and rebuilds each material below.
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=src, use_image_search=True)

# The FBX marks many meshes as hidden, mostly proxy shells and light-state variants.
# Measure Seven has approximately 1,700 of them. The glTF exporter would export them as
# solid, visible geometry that encloses the camera, so delete them.
#
# Deletion starts at the leaves. A hidden mesh with a surviving child keeps its node but
# loses its geometry, because the child needs its animated transform. Child counts come
# from one pass over the `parent` links, because each `Object.children` access scans the
# full file.
hidden = [
    obj
    for obj in bpy.context.scene.objects
    if obj.type == "MESH"
    and (obj.hide_render or obj.hide_viewport or not obj.visible_get())
]
child_count = {obj: 0 for obj in hidden}
for obj in bpy.data.objects:
    if obj.parent in child_count:
        child_count[obj.parent] += 1
deletable = [obj for obj in hidden if child_count[obj] == 0]
doomed = set()
while deletable:
    obj = deletable.pop()
    doomed.add(obj)
    parent = obj.parent
    if parent in child_count:
        child_count[parent] -= 1
        if child_count[parent] == 0:
            deletable.append(parent)
remaining = [obj for obj in hidden if obj not in doomed]
for obj in remaining:
    obj.data = bpy.data.meshes.new(obj.name + "_hidden")
bpy.data.batch_remove(doomed)
print(
    "ZERO_DAY_HIDDEN removed=%d kept_as_empty_nodes=%d" % (len(doomed), len(remaining))
)

# Index the texture set by base name: {base_lower: {channel_lower: path}}.
tex = {}
for path in glob.glob(os.path.join(texdir, "*.dds")):
    stem = os.path.splitext(os.path.basename(path))[0]
    if "_" not in stem:
        continue
    base, chan = stem.rsplit("_", 1)
    tex.setdefault(base.lower(), {})[chan.lower()] = path


def base_for_material(mat):
    """The texture base name for a material, from the material name.

    The material name is the key to `tex/` per the download's README. Don't use the
    BaseColor image that the FBX import links, because it can refer to a different
    texture set. A name with no texture set returns None, and the caller records it in
    `skipped`."""
    nm = mat.name.lower()
    if nm in tex:
        return nm
    # Try again without a final underscore suffix, for example "_c4d".
    stem = nm.rsplit("_", 1)[0]
    return stem if stem in tex else None


def load_image(path, non_color):
    img = bpy.data.images.load(os.path.abspath(path), check_existing=True)
    img.filepath = os.path.abspath(path)  # absolute, for the second read at export time
    img.colorspace_settings.name = "Non-Color" if non_color else "sRGB"
    return img


flipped_normals = set()


def load_normal_image(path):
    """Load a normal map and invert its green channel from the DirectX +Y-down
    convention to the OpenGL convention that glTF requires.

    Baking the flip into the image keeps the `.glb` self-contained; the alternative,
    `flip_normal_map_y` in the example, is an invisible agreement between the script and
    the example. A node-graph inversion doesn't survive the export, so change the pixels
    and pack them, and the exporter then reads the datablock instead of the `.dds` on
    disk."""
    img = load_image(path, non_color=True)
    if img.name in flipped_normals:
        return img
    flipped_normals.add(img.name)
    pixels = np.empty(len(img.pixels), dtype=np.float32)
    img.pixels.foreach_get(pixels)
    pixels[1::4] = 1.0 - pixels[1::4]  # G
    img.pixels.foreach_set(pixels)
    img.pack()
    return img


def rebuild(mat, base):
    channels = tex[base]
    mat.use_nodes = True
    nt = mat.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
    nt.links.new(bsdf.outputs["BSDF"], out.inputs["Surface"])

    if "basecolor" in channels:
        n = nt.nodes.new("ShaderNodeTexImage")
        n.image = load_image(channels["basecolor"], non_color=False)
        nt.links.new(n.outputs["Color"], bsdf.inputs["Base Color"])
        # Don't link the Alpha output. Zero-Day is fully opaque, and blended surfaces
        # never reach the deferred G-buffer that gives Solari its primary visibility.

    if "specular" in channels:
        n = nt.nodes.new("ShaderNodeTexImage")
        n.image = load_image(channels["specular"], non_color=True)
        sep = nt.nodes.new("ShaderNodeSeparateColor")
        nt.links.new(n.outputs["Color"], sep.inputs["Color"])
        # The specular texture is packed as ORM, with roughness in green and metallic
        # in blue. The occlusion channel in red stays unused, because it needs the
        # unreliable glTF-settings node group and has only a small effect.
        nt.links.new(sep.outputs["Green"], bsdf.inputs["Roughness"])
        nt.links.new(sep.outputs["Blue"], bsdf.inputs["Metallic"])

    if "normal" in channels:
        n = nt.nodes.new("ShaderNodeTexImage")
        n.image = load_normal_image(channels["normal"])
        nmap = nt.nodes.new("ShaderNodeNormalMap")
        nt.links.new(n.outputs["Color"], nmap.inputs["Color"])
        nt.links.new(nmap.outputs["Normal"], bsdf.inputs["Normal"])

    if "emissive" in channels:
        n = nt.nodes.new("ShaderNodeTexImage")
        n.image = load_image(channels["emissive"], non_color=False)
        nt.links.new(n.outputs["Color"], bsdf.inputs["Emission Color"])
        bsdf.inputs["Emission Strength"].default_value = 1.0


rebuilt = 0
skipped = []
for mat in bpy.data.materials:
    base = base_for_material(mat)
    if base is None:
        skipped.append(mat.name)
        continue
    rebuild(mat, base)
    rebuilt += 1
print("ZERO_DAY_MATERIALS rebuilt=%d skipped=%d" % (rebuilt, len(skipped)))
if skipped:
    print("ZERO_DAY_SKIPPED", skipped[:20])

# Solari only traces meshes whose vertex layout is exactly POSITION, NORMAL, UV0, and
# TANGENT, so give each mesh exactly one UV layer, an empty one where a mesh has none.
# A mesh with zero or two UV layers stops occluding and giving light, with no error
# message.
uv_added = uv_dropped = 0
for mesh in {obj.data for obj in bpy.context.scene.objects if obj.type == "MESH"}:
    if not mesh.uv_layers:
        mesh.uv_layers.new(name="UVMap")
        uv_added += 1
    while len(mesh.uv_layers) > 1:
        mesh.uv_layers.remove(mesh.uv_layers[-1])
        uv_dropped += 1
print("ZERO_DAY_UVS added=%d dropped_extra=%d" % (uv_added, uv_dropped))

# Some actions continue past the scene's playback range. The scene ends at frame 250,
# but the cameras run to frame 412 or 441. Extend the range so the bake includes all of
# them.
max_frame = 1
for obj in bpy.context.scene.objects:
    ad = obj.animation_data
    if ad and ad.action:
        max_frame = max(max_frame, int(round(ad.action.frame_range[1])))
bpy.context.scene.frame_start = 1
bpy.context.scene.frame_end = max_frame
print("ZERO_DAY_FRAME_RANGE 1..%d" % max_frame)

# `export_animation_mode="SCENE"` bakes all objects over the scene frame range, on the
# film's shared timeline. A per-action export gives each object its own duration, and a
# loop then plays the short animations too many times.
bpy.ops.export_scene.gltf(
    filepath=dst,
    export_format="GLB",
    export_yup=True,
    export_cameras=True,
    export_lights=False,
    export_animations=True,
    export_animation_mode="SCENE",
    export_apply=True,
    # Solari needs a TANGENT attribute on every mesh, and Bevy only generates tangents
    # for materials with a normal map, so bake them for all meshes here.
    export_tangents=True,
)
print("ZERO_DAY_EXPORT_DONE", dst)


def normalize_materials(path):
    """Correct the materials of the exported .glb in place, in the JSON chunk only.

    The ``skipped`` materials keep Blender's imported graph, which exports as alpha
    ``BLEND``. Blended surfaces never reach the deferred G-buffer that gives Solari its
    primary visibility, and no surface in Zero-Day needs transparency, so force
    ``OPAQUE``.

    A black ``emissiveFactor`` multiplies the ``emissiveTexture`` to zero, and the
    emissive panels are the only lights in the scene, so set those factors to white."""
    with open(path, "rb") as f:
        data = f.read()
    magic, version, _ = struct.unpack_from("<III", data, 0)
    json_len, json_type = struct.unpack_from("<II", data, 12)
    gltf = json.loads(data[20 : 20 + json_len])
    bin_chunk = memoryview(data)[20 + json_len :]  # BIN chunk header and payload, not changed

    opaqued = emissive_promoted = 0
    for material in gltf.get("materials", []):
        if material.get("alphaMode", "OPAQUE") != "OPAQUE":
            material["alphaMode"] = "OPAQUE"
            material.pop("alphaCutoff", None)
            opaqued += 1
        if "emissiveTexture" in material and not any(
            material.get("emissiveFactor", [0, 0, 0])
        ):
            material["emissiveFactor"] = [1.0, 1.0, 1.0]
            emissive_promoted += 1

    new_json = json.dumps(gltf, separators=(",", ":")).encode("utf-8")
    new_json += b" " * ((4 - len(new_json) % 4) % 4)  # glTF pads the JSON chunk with spaces
    total = 12 + 8 + len(new_json) + len(bin_chunk)
    with open(path, "wb") as f:
        f.write(struct.pack("<III", magic, version, total))
        f.write(struct.pack("<II", len(new_json), json_type))
        f.write(new_json)
        f.write(bin_chunk)
    print(
        "ZERO_DAY_MATERIALS_PATCHED opaqued=%d emissive_promoted=%d"
        % (opaqued, emissive_promoted)
    )


normalize_materials(dst)
