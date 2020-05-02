#version 440 core

layout(std140, row_major) uniform nameless {
    vec3 anonMember1;
    mat3x2 m23;
    int scalarAfterm23;
    vec4 anonDeadMember2;
    vec4 anonMember3;
    int scalarBeforeArray;
    float floatArray[5];
    int scalarAfterArray;
    mat2x2 m22[9];
};

layout(std140, column_major) uniform c_nameless {
    vec3 c_anonMember1;
    mat3x2 c_m23;
    int c_scalarAfterm23;
    vec4 c_anonDeadMember2;
    vec4 c_anonMember3;
};

layout(std140) uniform named {
    vec3 deadMember1;
    int scalar;
    vec4 member2;
    vec4 member3;
    vec2 memvec2;
    float memf1;
    bool  memf2;
    int   memf3;
    vec2 memvec2a;
    mat2x2 m22[7];
} ablock;

layout(std140) uniform namelessdead {
    int a;
};

layout(std140) uniform namedDead {
    int b;
} bblock;

struct N1 {
    float a;
};

struct N2 {
    float b;
    float c;
    float d;
};

struct N3 {
    N1 n1;
    N2 n2;
};

layout(std140) uniform nested {
    N3 foo;
} nest;

struct TS {
    int a;
    int dead;
};

uniform TS s;

uniform float uf1;
uniform float uf2;
uniform float ufDead3;
uniform float ufDead4;

uniform writeonly uimage2D image_ui2D;
uniform sampler2D sampler_2D;
uniform sampler2DMSArray sampler_2DMSArray;

uniform mat2 dm22[10];

struct deep1 {
    vec2 va[3];
    bool b;
};

struct deep2 {
    int i;
    deep1 d1[4];
};

struct deep3 {
    vec4 iv4;
    deep2 d2;
    ivec3 v3;
};

in float attributeFloat;
layout(location = 2) in vec2 attributeFloat2;
in vec3 attributeFloat3;
in vec4 attributeFloat4;
in mat4 attributeMat4;

uniform deep3 deepA[2], deepB[2], deepC[3], deepD[2];

const bool control = true;

void deadFunction()
{
    vec3 v3 = ablock.deadMember1;
    vec4 v = anonDeadMember2;
    float f = ufDead4;
}

void liveFunction2()
{
    vec3 v = anonMember1;
    float f = uf1;
}

void liveFunction1(writeonly uimage2D p_ui2D, sampler2D p_2D, sampler2DMSArray p_2DMSArray)

{
    liveFunction2();
    float f = uf2;
    vec4 v = ablock.member3;
}

uniform abl {
    float foo;
} arrBl[4];

uniform abl2 {
    float foo;
} arrBl2[4];

void main()
{
    liveFunction1(image_ui2D, sampler_2D, sampler_2DMSArray);
    liveFunction2();

    if (! control)
        deadFunction();

    float f;
    int i;
    if (control) {
        liveFunction2();
        f = anonMember3.z;
        f = s.a;
        f = ablock.scalar;
        f = m23[1].y + scalarAfterm23;
        f = c_m23[1].y + c_scalarAfterm23;
        f += scalarBeforeArray;
        f += floatArray[2];
        f += floatArray[4];
        f += scalarAfterArray;
        f += ablock.memvec2.x;
        f += ablock.memf1;
        f += float(ablock.memf2);
        f += ablock.memf3;
        f += ablock.memvec2a.y;
        f += ablock.m22[i][1][0];
        f += dm22[3][0][1];
        f += m22[2][1].y;
        f += nest.foo.n1.a + nest.foo.n2.b + nest.foo.n2.c + nest.foo.n2.d;
        f += deepA[i].d2.d1[2].va[1].x;
        f += deepB[1].d2.d1[i].va[1].x;
        f += deepB[i].d2.d1[i].va[1].x;
        deep3 d = deepC[1];
        deep3 da[2] = deepD;
    } else
        f = ufDead3;

    f += arrBl[2].foo + arrBl[0].foo;
    f += arrBl2[i].foo;

    f += attributeFloat;
    f += attributeFloat2.x;
    f += attributeFloat3.x;
    f += attributeFloat4.x;
    f += attributeMat4[0][1];
}
