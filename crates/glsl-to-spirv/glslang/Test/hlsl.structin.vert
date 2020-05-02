struct VI {
    float4 m[2];
    uint2 coord;
    linear float4 b;
};

VI main(float4 d, VI vi, float4 e) : SV_POSITION
{
    VI local;

    local.b = vi.m[1] + vi.m[0] + float4(vi.coord.x) + d + e;

    return local;
}
