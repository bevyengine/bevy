TARGET_DIR?=spirv

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
	$(SHADERS:%.glsl=$(TARGET_DIR)/%.spv) \
	$(EMPTY)

GLSLANGFLAGS=--auto-map-locations -I.
GLSLANGFLAGS_VULKAN=$(GLSLANGFLAGS) -V

all:	$(OUT)

.PHONY: clean

clean:
	rm -f $(OUT)

$(TARGET_DIR)/%.fs.spv:	glsl/%.fs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR) && glslangValidator $(GLSLANGFLAGS_VULKAN) -S frag -o $@ $<

$(TARGET_DIR)/%.vs.spv:	glsl/%.vs.glsl $(INCLUDES)
	mkdir -p $(TARGET_DIR) && glslangValidator $(GLSLANGFLAGS_VULKAN) -S vert -o $@ $<