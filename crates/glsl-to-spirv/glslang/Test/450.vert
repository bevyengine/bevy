#version 450 core

out gl_PerVertex {
    float gl_CullDistance[3];
};

void main()
{
    gl_CullDistance[2] = 4.5;
}

out bool outb;         // ERROR
out sampler2D outo;    // ERROR
out float outa[4];
out float outaa[4][2];
struct S { float f; };
out S outs;
out S[4] outasa;
out S outsa[4];
struct SA { float f[4]; };
out SA outSA;
struct SS { float f; S s; };
out SS outSS;

void foo()
{
    SS::f;
}
