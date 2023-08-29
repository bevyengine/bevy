#include "obj_loader.h"
#include "mikktspace.h"

#include <stdio.h>
#include <stdint.h>
#include <assert.h>

typedef struct {
    vec3_t position;
    vec3_t normal;
    vec2_t texture_coords;
    vec3_t tangent;
} vertex_t;

typedef struct {
    vertex_t vertices[4096];
    uint32_t vertices_len;
    uint32_t indices[4096];
    uint32_t indices_len;
} model_data_t;

static size_t find_vertex(vertex_t *list, size_t len, vertex_t vertex) {
    for (size_t i = 0; i < len; i++) {
        if (vec3_eq(list[i].position, vertex.position) &&
            vec3_eq(list[i].normal, vertex.normal) &&
            vec2_eq(list[i].texture_coords, vertex.texture_coords)
        ) {
            return i;
        }
    }

    return SIZE_MAX;
}

static void interleave_obj_to_model(mtg_obj_data_t *data, model_data_t *model) {
    printf("\nInterleaving vertices...\n");

    for (size_t i = 0; i < data->faces_len; i++) {
        for (size_t v = 0; v < 3; v++) {
            // Unwrap the actual data in this vertex
            mtg_obj_vertex_t obj_vertex = data->faces[i].vertices[v];
            vec3_t position = data->positions[obj_vertex.position];
            vec3_t normal = data->normals[obj_vertex.normal];
            vec2_t texture_coords = data->texture_coords[obj_vertex.texture_coords];
            vertex_t vertex = {position, normal, texture_coords};

            // Find or insert the vertex
            size_t index = find_vertex(model->vertices, model->vertices_len, vertex);
            if (index == SIZE_MAX) {
                index = model->vertices_len;
                model->vertices[index] = vertex;
                model->vertices_len++;
            }

            // Store the index
            model->indices[(i * 3) + v] = index;
        }
    }

    printf("Unique vertices: %u\n", model->vertices_len);
}

int mtg_get_num_faces(const SMikkTSpaceContext *context) {
    model_data_t *data = context->m_pUserData;
    return data->indices_len / 3;
}

int mtg_get_num_vertices_of_face(const SMikkTSpaceContext *context, const int face) {
    return 3;
}

void mtg_get_position(
    const SMikkTSpaceContext *context,
    float pos_out[],
    const int face,
    const int vert
) {
    model_data_t *data = context->m_pUserData;

    uint32_t index = data->indices[face * 3 + vert];
    vertex_t vertex = data->vertices[index];

    pos_out[0] = vertex.position.x;
    pos_out[1] = vertex.position.y;
    pos_out[2] = vertex.position.z;
}

void mtg_get_normal(
    const SMikkTSpaceContext *context,
    float norm_out[],
    const int face,
    const int vert
) {
    model_data_t *data = context->m_pUserData;

    uint32_t index = data->indices[face * 3 + vert];
    vertex_t vertex = data->vertices[index];

    norm_out[0] = vertex.normal.x;
    norm_out[1] = vertex.normal.y;
    norm_out[2] = vertex.normal.z;
}

void mtg_get_tex_coord(
    const SMikkTSpaceContext *context,
    float texc_out[],
    const int face,
    const int vert
) {
    model_data_t *data = context->m_pUserData;

    uint32_t index = data->indices[face * 3 + vert];
    vertex_t vertex = data->vertices[index];

    texc_out[0] = vertex.texture_coords.x;
    texc_out[1] = vertex.texture_coords.y;
}

void mtg_set_tspace_basic(
    const SMikkTSpaceContext *context,
    const float tangent[],
    const float sign,
    const int face,
    const int vert
) {
    model_data_t *data = context->m_pUserData;

    uint32_t index = data->indices[face * 3 + vert];
    vertex_t *vertex = &data->vertices[index];

    vertex->tangent.x = tangent[0];
    vertex->tangent.y = tangent[1];
    vertex->tangent.z = tangent[2];
}

int main(int argc, char *argv[]) {
    // Sanity check system assumption
    assert(sizeof(vertex_t) == 4 * 11);
    assert(sizeof(size_t) == 8);

    if (argc != 3) {
        printf("Error: Generator must receive source and target file paths\n");
        return EXIT_FAILURE;
    }
    char *source_path = argv[1];
    char *target_path = argv[2];
    printf("Source: %s\n", source_path);
    printf("Target: %s\n", target_path);

    // Open the source data file
    printf("\nLoading source file...\n");
    FILE* file = fopen(source_path, "r");

    mtg_obj_data_t obj_data = {};

    // OBJ files contain data line-by-line
    char buffer[256];
    while (fgets(buffer, sizeof buffer, file) != NULL) {
        mtg_obj_parse_line(buffer, &obj_data);
    }
    fclose(file);

    // Print information about the parsed OBJ
    printf("Vertex Positions: %u\n", obj_data.positions_len);
    printf("Vertex Normals: %u\n", obj_data.normals_len);
    printf("Vertex Texture Coords: %u\n", obj_data.texture_coords_len);
    printf("Faces: %u\n", obj_data.faces_len);

    // Generate interleaved vertices, we need those for modern graphics (and mikktspace)
    model_data_t model_data = {};
    model_data.indices_len = obj_data.faces_len * 3;
    assert(model_data.indices_len <= 4096);
    interleave_obj_to_model(&obj_data, &model_data);

    // Generate tangents
    printf("\nRunning MikkTSpace...\n");
    SMikkTSpaceInterface interface = {
        mtg_get_num_faces,
        mtg_get_num_vertices_of_face,
        mtg_get_position,
        mtg_get_normal,
        mtg_get_tex_coord,
        mtg_set_tspace_basic
    };
    SMikkTSpaceContext context = { &interface, &model_data };
    genTangSpaceDefault(&context);

    // Dump to target file
    printf("\nWriting results to target file...\n");
    FILE *file_out = fopen(target_path, "wb");

    // Vertices
    fwrite(&model_data.vertices_len, 4, 1, file_out);
    fwrite(model_data.vertices, sizeof(vertex_t), model_data.vertices_len, file_out);

    // Indices
    fwrite(&model_data.indices_len, 4, 1, file_out);
    fwrite(model_data.indices, 4, model_data.indices_len, file_out);

    fclose(file_out);

    return EXIT_SUCCESS;
}
