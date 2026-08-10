"""Export an ORCA Zero-Day FBX as one self-contained glTF binary (.glb) for the
`zero_day` Bevy example.

Zero-Day (NVIDIA ORCA) supplies each measure (`MEASURE_ONE`, `MEASURE_SEVEN`, and others)
as an `.fbx` file with a `tex/` folder of `.dds` textures adjacent to it. This script
converts the `.fbx` file that you give to it. The README of the example lists the output
name for each measure.

Bevy cannot load FBX. The FBX importer of Blender also reads the material conventions of
this Octane export incorrectly: it puts the ORM map into `KHR_materials_specular`, changes
the BaseColor opacity into alpha blend, and removes most of the emissive maps. This script
does not use the imported material graph. It builds each material again from the naming
convention that the README of the download specifies:

    <name>_BaseColor.dds  RGB = base color            (sRGB)
    <name>_Specular.dds   R = occlusion, G = roughness, B = metallic (Non-Color, ORM)
    <name>_Normal.dds     DirectX normal map          (Non-Color)
    <name>_Emissive.dds   RGB = emissive color        (sRGB)

The roughness (G) and the metallic (B) channels of the shared `_Specular` image become one
glTF `metallicRoughnessTexture`. The script does not use the occlusion channel (R): it
needs the unreliable glTF-settings node group and has a small effect only. The normal maps
use the DirectX convention (+Y down). The script inverts their green channel. The exported
glTF then uses the OpenGL convention that the specification requires, and the example needs
no correction at run time.

The script deletes the meshes that the FBX marks as hidden. The film does not render them,
but the glTF exporter would export them as visible, solid geometry. See the comment at the
deletion.

The script also prepares the meshes for Bevy Solari, which only traces against a mesh with
POSITION, NORMAL, UV0, and TANGENT. Each mesh gets one UV layer only (an empty layer if it
had none), and the export includes tangents for all of them.

Usage:
    blender --background --python convert.py -- <input.fbx> <output.glb>
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
# Keep the path absolute. Blender reads the textures again at export time, and a relative
# path that was correct at load time fails there, because the export uses a different
# working directory.
texdir = os.path.abspath(os.path.join(os.path.dirname(src), "tex"))

# Start with an empty scene and import the FBX to get the geometry, the material
# assignment, and the names. The script builds the materials again below, and the import is
# necessary only for the relation between the meshes and the materials.
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=src, use_image_search=True)

# The FBX marks many meshes as hidden, and the film does not render them (approximately
# 1,700 in Measure Seven): proxy tubes with placeholder materials that contain the
# flythrough camera for the full animation, ON and OFF variants of the light states, and
# duplicate machinery. Octane and Blender do not render hidden objects, but the glTF
# exporter exports them as usual visible geometry, and Bevy has no per-object visibility to
# keep the difference. In a rasterized or a ray-traced render, they become solid walls
# around the camera and unwanted emissive lights. Solari also uses each BLAS triangle as an
# opaque occluder. The script thus deletes these meshes.
#
# The deletion starts at the leaf objects. A hidden mesh that is the parent of an object
# that stays keeps its node, but loses its geometry, because its children must keep its
# animated transform. The child counts come from one pass over the `parent` links: each
# access to `Object.children` examines the full file again.
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
    """The texture base name for a material, from the ORCA material NAME.

    The README of the download uses this name as the key to `tex/`, and the relation is
    deterministic. Thus use the name only. Do not use the BaseColor image that the FBX
    import of Blender links, because it can refer to a different texture set and remove the
    emissive of the material, which is the only light in the scene. If a name has no texture
    set, this function returns None and the caller puts the material in `skipped`. The
    mismatch is then visible, and no incorrect texture set hides it."""
    nm = mat.name.lower()
    if nm in tex:
        return nm
    # Try again without a final "_suffix" (for example "_c4d").
    stem = nm.rsplit("_", 1)[0]
    return stem if stem in tex else None


def load_image(path, non_color):
    img = bpy.data.images.load(os.path.abspath(path), check_existing=True)
    img.filepath = os.path.abspath(path)  # absolute, for the second read at export time
    img.colorspace_settings.name = "Non-Color" if non_color else "sRGB"
    return img


flipped_normals = set()


def load_normal_image(path):
    """Load a normal map and invert its green channel: DirectX (+Y down) to OpenGL.

    ORCA made these maps for a renderer that uses the DirectX convention, but glTF requires
    the OpenGL convention. The inversion here becomes part of the exported image. The
    alternative is `flip_normal_map_y` in the Bevy example, but that makes an agreement
    between this script and the example that is not visible: nothing in the `.glb` shows
    that the normals use a different convention, and a change to only one of the two lights
    the full scene incorrectly.

    An inversion in the node graph does not survive the export, because glTF encodes a
    direct link from the image to the normal map only. This function changes the pixels and
    packs the result. The exporter then reads the datablock and not the `.dds` file on the
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
        # The BaseColor image also contains an opacity channel, because the decal and
        # label materials are transparent around their glyphs. Do not link its Alpha
        # output: Zero-Day is fully opaque here, and each rebuilt material exports as
        # OPAQUE. Solari gets primary visibility from a deferred G-buffer, and a
        # forward-blended surface never becomes part of that G-buffer. No surface in the
        # scene must be transparent. Transparency would only add cost, and the trace would
        # not show it.

    if "specular" in channels:
        n = nt.nodes.new("ShaderNodeTexImage")
        n.image = load_image(channels["specular"], non_color=True)
        sep = nt.nodes.new("ShaderNodeSeparateColor")
        nt.links.new(n.outputs["Color"], sep.inputs["Color"])
        # ORM: G becomes the roughness, B becomes the metallic. The shared image becomes
        # one glTF map.
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

# Give each mesh one UV layer only. Solari traces a mesh only if its vertex layout is
# POSITION, NORMAL, UV0, and TANGENT. Solari thus does not trace a mesh with no UV layer
# (Zero-Day has machinery with no textures) or with a second UV layer (some FBX meshes have
# a lightmap UV). Such a mesh stops to occlude and stops to give light, and there is no
# error message. A new layer contains zeros only, which is correct: these meshes have no
# textures with UV coordinates.
uv_added = uv_dropped = 0
for mesh in {obj.data for obj in bpy.context.scene.objects if obj.type == "MESH"}:
    if not mesh.uv_layers:
        mesh.uv_layers.new(name="UVMap")
        uv_added += 1
    while len(mesh.uv_layers) > 1:
        mesh.uv_layers.remove(mesh.uv_layers[-1])
        uv_dropped += 1
print("ZERO_DAY_UVS added=%d dropped_extra=%d" % (uv_added, uv_dropped))

# The actions continue after the end of the playback range of the scene. The camera of
# Measure One continues to frame 412, and the camera of Measure Seven to frame 441, but the
# scene ends at frame 250. Thus increase the range to include all of the actions. If you do
# not, the baked animation and the flythrough stop too soon.
max_frame = 1
for obj in bpy.context.scene.objects:
    ad = obj.animation_data
    if ad and ad.action:
        max_frame = max(max_frame, int(round(ad.action.frame_range[1])))
bpy.context.scene.frame_start = 1
bpy.context.scene.frame_end = max_frame
print("ZERO_DAY_FRAME_RANGE 1..%d" % max_frame)

# Export one self-contained .glb. glTF is Y-up. The scene has no real lights, but it
# contains the animated camera of the film (each measure gives it a different name:
# `DynamicCamera2`, `DynamicCamera`, and others) and approximately 550 to 640 animated
# objects. `export_animation_mode="SCENE"` bakes them over the frame range of the scene, and
# all of the objects stay on the shared timeline of the film. An export for each action
# gives each object its own duration, and a loop then plays the short animations too many
# times.
bpy.ops.export_scene.gltf(
    filepath=dst,
    export_format="GLB",
    export_yup=True,
    export_cameras=True,
    export_lights=False,
    export_animations=True,
    export_animation_mode="SCENE",
    export_apply=True,
    # Solari needs a TANGENT attribute on each mesh that it traces. The glTF loader of
    # Bevy runs mikktspace for materials with a normal map only. The tangents here cover
    # the meshes with no textures, and they also keep mikktspace off the load path for the
    # other meshes.
    export_tangents=True,
)
print("ZERO_DAY_EXPORT_DONE", dst)


def normalize_materials(path):
    """Correct the materials of the exported .glb in place. This changes the JSON chunk
    only.

    There are two corrections. Without them, the example must do the same work at run time.

    Alpha mode. The rebuilt materials are already ``OPAQUE``, because their BaseColor alpha
    is not linked. But the small number of materials whose ORCA name has no texture set (the
    ``skipped`` list) keep the graph from the FBX import of Blender, which puts the BaseColor
    opacity into an alpha ``BLEND``. ``BLEND`` is incorrect for a Solari scene: blended
    surfaces render in a forward pass and never become part of the deferred G-buffer that
    gives Solari its primary visibility. The trace does not show them. No surface in Zero-Day
    must be transparent. Thus change ``BLEND`` and ``MASK`` to ``OPAQUE`` and remove the
    ``alphaCutoff``, which then has no function.

    Emissive factor. A material with an ``emissiveTexture`` and a black ``emissiveFactor``
    gives no light, because the factor multiplies the texture. The emissive panels are the
    only lights in Zero-Day. A black factor here does not make one unlit surface. It makes a
    dark corridor. Change these factors to white to let the texture through."""
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
    new_json += b" " * ((4 - len(new_json) % 4) % 4)  # glTF fills the JSON chunk with spaces
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
