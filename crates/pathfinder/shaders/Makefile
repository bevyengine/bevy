TARGET_DIR?=../resources/shaders

EMPTY=

SHADERS=\
	blit.fs.glsl \
	blit.vs.glsl \
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

INCLUDES=\
	$(EMPTY)

OUT=\
	$(SHADERS:%.glsl=$(TARGET_DIR)/spirv-vulkan/%.spv) \
	$(EMPTY)

GLSL_VERSION=330
GLSLANGFLAGS=--auto-map-locations -I.
GLSLANGFLAGS_METAL=$(GLSLANGFLAGS) -DPF_ORIGIN_UPPER_LEFT=1
GLSLANGFLAGS_VULKAN=$(GLSLANGFLAGS) -V

SPIRVCROSS?=spirv-cross
SPIRVCROSSFLAGS_METAL=--msl --msl-version 020100 --msl-argument-buffers
SPIRVCROSSFLAGS_VULKAN=

GLSL_VERSION_HEADER="\#version {{version}}"
HEADER="// Automatically generated from files in pathfinder/shaders/. Do not edit!"

GLSL_SED_ARGS=-e "s/\#version 330//" -e "s/\#line.*$$//"

all:	$(OUT)

.PHONY: clean

clean:
	rm -f $(OUT)

build/metal/%.fs.spv:	%.fs.glsl $(INCLUDES)
	mkdir -p build/metal && glslangValidator $(GLSLANGFLAGS_METAL) -G$(GLSL_VERSION) -S frag -o $@ $<

$(TARGET_DIR)/gl3/%.fs.glsl:	%.fs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR)/gl3 && echo $(GLSL_VERSION_HEADER) > $@ && echo $(HEADER) >> $@ && ( glslangValidator $(GLSLANGFLAGS) -S frag -E $< | sed $(GLSL_SED_ARGS) >> $@ ) || ( rm $@ && exit 1 )

$(TARGET_DIR)/spirv/%.fs.spv:	%.fs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR)/spirv && glslangValidator $(GLSLANGFLAGS) -G$(GLSL_VERSION) -S frag -o $@ $<
	
$(TARGET_DIR)/spirv-vulkan/%.fs.spv:	%.fs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR)/spirv-vulkan && glslangValidator $(GLSLANGFLAGS_VULKAN) -S frag -o $@ $<

build/metal/%.vs.spv:	%.vs.glsl $(INCLUDES)
	mkdir -p build/metal && glslangValidator $(GLSLANGFLAGS_METAL) -G$(GLSL_VERSION) -S vert -o $@ $<

$(TARGET_DIR)/gl3/%.vs.glsl:	%.vs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR)/gl3 && echo $(GLSL_VERSION_HEADER) > $@ && echo $(HEADER) >> $@ && ( glslangValidator $(GLSLANGFLAGS) -S vert -E $< | sed $(GLSL_SED_ARGS) >> $@ ) || ( rm $@ && exit 1 )

$(TARGET_DIR)/spirv/%.vs.spv:	%.vs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR)/spirv && glslangValidator $(GLSLANGFLAGS) -G$(GLSL_VERSION) -S vert -o $@ $<

$(TARGET_DIR)/spirv-vulkan/%.vs.spv:	%.vs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR)/spirv-vulkan && glslangValidator $(GLSLANGFLAGS_VULKAN) -S vert -o $@ $<

$(TARGET_DIR)/metal/%.metal:	build/metal/%.spv
	mkdir -p $(TARGET_DIR)/metal && echo $(HEADER) > $@ && ( $(SPIRVCROSS) $(SPIRVCROSSFLAGS_METAL) $< >> $@ ) || ( rm $@ && exit 1 )