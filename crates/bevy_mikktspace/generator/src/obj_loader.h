#pragma once

#include "vmath.h"

#include <stdlib.h>
#include <stddef.h>

typedef struct {
    size_t position;
    size_t normal;
    size_t texture_coords;
} mtg_obj_vertex_t;

typedef struct {
    mtg_obj_vertex_t vertices[3];
} mtg_obj_face_t;

typedef struct {
    vec3_t positions[1024];
    size_t positions_len;
    
    vec3_t normals[1024];
    size_t normals_len;

    vec2_t texture_coords[1024];
    size_t texture_coords_len;

    mtg_obj_face_t faces[1024];
    size_t faces_len;
} mtg_obj_data_t;

void mtg_obj_parse_line(char *line, mtg_obj_data_t *data);
