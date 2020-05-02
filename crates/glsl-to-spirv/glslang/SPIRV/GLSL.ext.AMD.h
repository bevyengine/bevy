/*
** Copyright (c) 2014-2016 The Khronos Group Inc.
**
** Permission is hereby granted, free of charge, to any person obtaining a copy
** of this software and/or associated documentation files (the "Materials"),
** to deal in the Materials without restriction, including without limitation
** the rights to use, copy, modify, merge, publish, distribute, sublicense,
** and/or sell copies of the Materials, and to permit persons to whom the
** Materials are furnished to do so, subject to the following conditions:
**
** The above copyright notice and this permission notice shall be included in
** all copies or substantial portions of the Materials.
**
** MODIFICATIONS TO THIS FILE MAY MEAN IT NO LONGER ACCURATELY REFLECTS KHRONOS
** STANDARDS. THE UNMODIFIED, NORMATIVE VERSIONS OF KHRONOS SPECIFICATIONS AND
** HEADER INFORMATION ARE LOCATED AT https://www.khronos.org/registry/
**
** THE MATERIALS ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
** OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
** FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
** THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
** LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
** FROM,OUT OF OR IN CONNECTION WITH THE MATERIALS OR THE USE OR OTHER DEALINGS
** IN THE MATERIALS.
*/

#ifndef GLSLextAMD_H
#define GLSLextAMD_H

enum BuiltIn;
enum Capability;
enum Decoration;
enum Op;

static const int GLSLextAMDVersion = 100;
static const int GLSLextAMDRevision = 4;

// SPV_AMD_shader_ballot
static const char* const E_SPV_AMD_shader_ballot = "SPV_AMD_shader_ballot";

static const Op OpGroupIAddNonUniformAMD = static_cast<Op>(5000);
static const Op OpGroupFAddNonUniformAMD = static_cast<Op>(5001);
static const Op OpGroupFMinNonUniformAMD = static_cast<Op>(5002);
static const Op OpGroupUMinNonUniformAMD = static_cast<Op>(5003);
static const Op OpGroupSMinNonUniformAMD = static_cast<Op>(5004);
static const Op OpGroupFMaxNonUniformAMD = static_cast<Op>(5005);
static const Op OpGroupUMaxNonUniformAMD = static_cast<Op>(5006);
static const Op OpGroupSMaxNonUniformAMD = static_cast<Op>(5007);

enum ShaderBallotAMD {
    ShaderBallotBadAMD = 0, // Don't use

    SwizzleInvocationsAMD = 1,
    SwizzleInvocationsMaskedAMD = 2,
    WriteInvocationAMD = 3,
    MbcntAMD = 4,

    ShaderBallotCountAMD
};

// SPV_AMD_shader_trinary_minmax
static const char* const E_SPV_AMD_shader_trinary_minmax = "SPV_AMD_shader_trinary_minmax";

enum ShaderTrinaryMinMaxAMD {
    ShaderTrinaryMinMaxBadAMD = 0, // Don't use

    FMin3AMD = 1,
    UMin3AMD = 2,
    SMin3AMD = 3,
    FMax3AMD = 4,
    UMax3AMD = 5,
    SMax3AMD = 6,
    FMid3AMD = 7,
    UMid3AMD = 8,
    SMid3AMD = 9,

    ShaderTrinaryMinMaxCountAMD
};

// SPV_AMD_shader_explicit_vertex_parameter
static const char* const E_SPV_AMD_shader_explicit_vertex_parameter = "SPV_AMD_shader_explicit_vertex_parameter";

static const BuiltIn BuiltInBaryCoordNoPerspAMD           = static_cast<BuiltIn>(4992);
static const BuiltIn BuiltInBaryCoordNoPerspCentroidAMD   = static_cast<BuiltIn>(4993);
static const BuiltIn BuiltInBaryCoordNoPerspSampleAMD     = static_cast<BuiltIn>(4994);
static const BuiltIn BuiltInBaryCoordSmoothAMD            = static_cast<BuiltIn>(4995);
static const BuiltIn BuiltInBaryCoordSmoothCentroidAMD    = static_cast<BuiltIn>(4996);
static const BuiltIn BuiltInBaryCoordSmoothSampleAMD      = static_cast<BuiltIn>(4997);
static const BuiltIn BuiltInBaryCoordPullModelAMD         = static_cast<BuiltIn>(4998);

static const Decoration DecorationExplicitInterpAMD       = static_cast<Decoration>(4999);

enum ShaderExplicitVertexParameterAMD {
    ShaderExplicitVertexParameterBadAMD = 0, // Don't use

    InterpolateAtVertexAMD = 1,

    ShaderExplicitVertexParameterCountAMD
};

// SPV_AMD_gcn_shader
static const char* const E_SPV_AMD_gcn_shader = "SPV_AMD_gcn_shader";

enum GcnShaderAMD {
    GcnShaderBadAMD = 0, // Don't use

    CubeFaceIndexAMD = 1,
    CubeFaceCoordAMD = 2,
    TimeAMD = 3,

    GcnShaderCountAMD
};

// SPV_AMD_gpu_shader_half_float
static const char* const E_SPV_AMD_gpu_shader_half_float = "SPV_AMD_gpu_shader_half_float";

// SPV_AMD_texture_gather_bias_lod
static const char* const E_SPV_AMD_texture_gather_bias_lod = "SPV_AMD_texture_gather_bias_lod";

static const Capability CapabilityImageGatherBiasLodAMD = static_cast<Capability>(5009);

// SPV_AMD_gpu_shader_int16
static const char* const E_SPV_AMD_gpu_shader_int16 = "SPV_AMD_gpu_shader_int16";

#endif  // #ifndef GLSLextAMD_H
