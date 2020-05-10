TARGET_DIR?=../resources/shaders

EMPTY=

SHADERS=\
	blit.fs.glsl \
	blit.vs.glsl \
	clear.fs.glsl \
	clear.vs.glsl \
	debug_solid.fs.glsl \
	debug_solid.vs.glsl \
	debug_texture.fs.glsl \
	debug_texture.vs.glsl \
	demo_ground.fs.glsl \
	demo_ground.vs.glsl \
	fill.fs.glsl \
	fill.vs.glsl \
	reproject.fs.glsl \
	reproject.vs.glsl \
	stencil.fs.glsl \
	stencil.vs.glsl \
	tile.fs.glsl \
	tile.vs.glsl \
	tile_clip.fs.glsl \
	tile_clip.vs.glsl \
	tile_copy.fs.glsl \
	tile_copy.vs.glsl \
	$(EMPTY)

COMPUTE_SHADERS=\
	fill.cs.glsl \
	$(EMPTY)

INCLUDES=\
	fill.inc.glsl \
	$(EMPTY)

OUT=\
	$(SHADERS:%=$(TARGET_DIR)/gl3/%) \
	$(SHADERS:%=$(TARGET_DIR)/gl4/%) \
	$(SHADERS:%.glsl=$(TARGET_DIR)/metal/%.metal) \
	$(SHADERS:%.glsl=build/metal/%.spv) \
	$(SHADERS:%.glsl=$(TARGET_DIR)/vulkan/%.spv) \
	$(COMPUTE_SHADERS:%.glsl=$(TARGET_DIR)/metal/%.metal) \
	$(COMPUTE_SHADERS:%.glsl=build/metal/%.spv) \
	$(COMPUTE_SHADERS:%.glsl=$(TARGET_DIR)/gl4/%.glsl) \
	$(COMPUTE_SHADERS:%.glsl=$(TARGET_DIR)/vulkan/%.spv) \
	$(EMPTY)

GLSL_3_VERSION=330
GLSL_4_VERSION=430
GLSLANGFLAGS=--auto-map-locations -I.
GLSLANGFLAGS_METAL=$(GLSLANGFLAGS) -DPF_ORIGIN_UPPER_LEFT=1
GLSLANGFLAGS_VULKAN=$(GLSLANGFLAGS)

SPIRVCROSS?=spirv-cross
SPIRVCROSSFLAGS_METAL=--msl --msl-version 020100
SPIRVCROSSFLAGS_GL3=--version $(GLSL_3_VERSION) --no-420pack-extension --glsl-emit-ubo-as-plain-uniforms --flatten-ubo --combined-samplers-inherit-bindings
SPIRVCROSSFLAGS_GL4=--version $(GLSL_4_VERSION) --glsl-emit-ubo-as-plain-uniforms --flatten-ubo --combined-samplers-inherit-bindings

GLSL_VERSION_HEADER="\#version {{version}}"
HEADER="// Automatically generated from files in pathfinder/shaders/. Do not edit!"

GLSL_SED_ARGS=-e "s/\#version .*//" -e "s/\#line.*$$//"

all:	$(OUT)

.PHONY: clean

clean:
	rm -f $(OUT)

build/metal/%.fs.spv:	%.fs.glsl $(INCLUDES)
	mkdir -p build/metal && glslangValidator $(GLSLANGFLAGS_METAL) -V -S frag -o $@ $<

build/metal/%.vs.spv:	%.vs.glsl $(INCLUDES)
	mkdir -p build/metal && glslangValidator $(GLSLANGFLAGS_METAL) -V -S vert -o $@ $<

build/metal/%.cs.spv:	%.cs.glsl $(INCLUDES)
	mkdir -p build/metal && glslangValidator $(GLSLANGFLAGS_METAL) -V -S comp -o $@ $<

$(TARGET_DIR)/metal/%.metal:	build/metal/%.spv
	mkdir -p $(TARGET_DIR)/metal && echo $(HEADER) > $@ && ( $(SPIRVCROSS) $(SPIRVCROSSFLAGS_METAL) $< >> $@ ) || ( rm $@ && exit 1 )

$(TARGET_DIR)/vulkan/%.vs.spv:	%.vs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR)/vulkan && glslangValidator $(GLSLANGFLAGS_VULKAN) -V -S vert -o $@ $<

$(TARGET_DIR)/vulkan/%.fs.spv:	%.fs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR)/vulkan && glslangValidator $(GLSLANGFLAGS_VULKAN) -V -S frag -o $@ $<

$(TARGET_DIR)/vulkan/%.cs.spv:	%.cs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR)/vulkan && glslangValidator $(GLSLANGFLAGS_VULKAN) -V -S comp -o $@ $<

$(TARGET_DIR)/gl3/%.glsl:	$(TARGET_DIR)/vulkan/%.spv
	mkdir -p $(TARGET_DIR)/gl3 && echo $(GLSL_VERSION_HEADER) > $@ && echo $(HEADER) >> $@ && ( $(SPIRVCROSS) $(SPIRVCROSSFLAGS_GL3) $< | sed $(GLSL_SED_ARGS) >> $@ ) || ( rm $@ && exit 1 )

$(TARGET_DIR)/gl4/%.glsl:	$(TARGET_DIR)/vulkan/%.spv
	mkdir -p $(TARGET_DIR)/gl4 && echo $(GLSL_VERSION_HEADER) > $@ && echo $(HEADER) >> $@ && ( $(SPIRVCROSS) $(SPIRVCROSSFLAGS_GL4) $< | sed $(GLSL_SED_ARGS) >> $@ ) || ( rm $@ && exit 1 )