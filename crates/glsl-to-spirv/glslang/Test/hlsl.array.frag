float4 a[4];

struct {
    float4 m[7];
} s[11];

float4 PixelShaderFunction(int i, float4 input[3]) : COLOR0
{
    float4 b[10];
    return a[1] + a[i] + input[2] + input[i] + b[5] + b[i] + s[i].m[i];
}