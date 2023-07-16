#pragma once

#include <math.h>
#include <stdbool.h>

typedef struct {
    float_t x, y;
} vec2_t;

typedef struct {
    float_t x, y, z;
} vec3_t;

static bool vec2_eq(vec2_t a, vec2_t b) {
    return a.x == b.x && a.y == b.y;
}

static bool vec3_eq(vec3_t a, vec3_t b) {
    return a.x == b.x && a.y == b.y && a.z == b.z;
}
