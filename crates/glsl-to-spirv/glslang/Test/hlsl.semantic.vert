struct S {
    float clip  : SV_ClipDistance;
    float clip0 : SV_ClipDistance0;
    float clip7 : SV_ClipDistance7;
    float cull  : SV_CullDistance;
    float cull2 : SV_CullDistance2;
    float cull5 : SV_CullDistance5;
    int ii      : SV_InstanceID;
};

S main(S ins)
{
    S s;
    return s;
}
