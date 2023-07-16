#include "obj_loader.h"

#include <string.h>

static float_t parse_float() {
    char *token = strtok(NULL, " ");
    return strtof(token, NULL);
}

static vec3_t parse_vec3() {
    float_t x = parse_float();
    float_t y = parse_float();
    float_t z = parse_float();
    vec3_t value = {x, y, z};
    return value;
}

static vec2_t parse_vec2() {
    float_t x = parse_float();
    float_t y = parse_float();
    vec2_t value = {x, y};
    return value;
}

static mtg_obj_vertex_t parse_vertex() {
    char *token = strtok(NULL, " ");
    size_t position = strtoul(token, &token, 10);
    size_t normal = strtoul(token + 1, &token, 10);
    size_t texture_coords = strtoul(token + 1, &token, 10);
    mtg_obj_vertex_t value = {position, normal, texture_coords};
    return value;
}

void mtg_obj_parse_line(char *line, mtg_obj_data_t *data) {
    // Lines are space-separated
    char *token = strtok(line, " ");

    // First token is the command
    if (strcmp(token, "v") == 0) {
        // Vertex position
        vec3_t position = parse_vec3();
        data->positions[data->positions_len] = position;
        data->positions_len += 1;
    }
    else if (strcmp(token, "vn") == 0) {
        // Vertex normal
        vec3_t normal = parse_vec3();
        data->normals[data->normals_len] = normal;
        data->normals_len += 1;
    }
    else if (strcmp(token, "vt") == 0) {
        // Vertex texture coordinate
        vec2_t texture_coords = parse_vec2();
        data->texture_coords[data->texture_coords_len] = texture_coords;
        data->texture_coords_len += 1;
    }
    else if (strcmp(token, "f") == 0) {
        // Face, currently always assuming tris
        mtg_obj_vertex_t v0 = parse_vertex();
        mtg_obj_vertex_t v1 = parse_vertex();
        mtg_obj_vertex_t v2 = parse_vertex();

        mtg_obj_face_t face = { { v0, v1, v2} };
        data->faces[data->faces_len] = face;
        data->faces_len += 1;
    }
}
