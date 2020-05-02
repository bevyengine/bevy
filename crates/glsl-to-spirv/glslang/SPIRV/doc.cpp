//
// Copyright (C) 2014-2015 LunarG, Inc.
//
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
//    Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//
//    Redistributions in binary form must reproduce the above
//    copyright notice, this list of conditions and the following
//    disclaimer in the documentation and/or other materials provided
//    with the distribution.
//
//    Neither the name of 3Dlabs Inc. Ltd. nor the names of its
//    contributors may be used to endorse or promote products derived
//    from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
// FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
// COPYRIGHT HOLDERS OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
// INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
// LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
// ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

//
// 1) Programmatically fill in instruction/operand information.
//    This can be used for disassembly, printing documentation, etc.
//
// 2) Print documentation from this parameterization.
//

#include "doc.h"

#include <cstdio>
#include <cstring>
#include <algorithm>

namespace spv {
    extern "C" {
        // Include C-based headers that don't have a namespace
        #include "GLSL.ext.KHR.h"
#ifdef AMD_EXTENSIONS
        #include "GLSL.ext.AMD.h"
#endif
#ifdef NV_EXTENSIONS
        #include "GLSL.ext.NV.h"
#endif
    }
}

namespace spv {

//
// Whole set of functions that translate enumerants to their text strings for
// the specification (or their sanitized versions for auto-generating the
// spirv headers.
//
// Also, the ceilings are declared next to these, to help keep them in sync.
// Ceilings should be
//  - one more than the maximum value an enumerant takes on, for non-mask enumerants
//    (for non-sparse enums, this is the number of enumerants)
//  - the number of bits consumed by the set of masks
//    (for non-sparse mask enums, this is the number of enumerants)
//

const int SourceLanguageCeiling = 6; // HLSL todo: need official enumerant

const char* SourceString(int source)
{
    switch (source) {
    case 0:  return "Unknown";
    case 1:  return "ESSL";
    case 2:  return "GLSL";
    case 3:  return "OpenCL_C";
    case 4:  return "OpenCL_CPP";
    case 5:  return "HLSL";

    case SourceLanguageCeiling:
    default: return "Bad";
    }
}

const int ExecutionModelCeiling = 7;

const char* ExecutionModelString(int model)
{
    switch (model) {
    case 0:  return "Vertex";
    case 1:  return "TessellationControl";
    case 2:  return "TessellationEvaluation";
    case 3:  return "Geometry";
    case 4:  return "Fragment";
    case 5:  return "GLCompute";
    case 6:  return "Kernel";

    case ExecutionModelCeiling:
    default: return "Bad";
    }
}

const int AddressingModelCeiling = 3;

const char* AddressingString(int addr)
{
    switch (addr) {
    case 0:  return "Logical";
    case 1:  return "Physical32";
    case 2:  return "Physical64";

    case AddressingModelCeiling:
    default: return "Bad";
    }
}

const int MemoryModelCeiling = 3;

const char* MemoryString(int mem)
{
    switch (mem) {
    case 0:  return "Simple";
    case 1:  return "GLSL450";
    case 2:  return "OpenCL";

    case MemoryModelCeiling:
    default: return "Bad";
    }
}

const int ExecutionModeCeiling = 33;

const char* ExecutionModeString(int mode)
{
    switch (mode) {
    case 0:  return "Invocations";
    case 1:  return "SpacingEqual";
    case 2:  return "SpacingFractionalEven";
    case 3:  return "SpacingFractionalOdd";
    case 4:  return "VertexOrderCw";
    case 5:  return "VertexOrderCcw";
    case 6:  return "PixelCenterInteger";
    case 7:  return "OriginUpperLeft";
    case 8:  return "OriginLowerLeft";
    case 9:  return "EarlyFragmentTests";
    case 10: return "PointMode";
    case 11: return "Xfb";
    case 12: return "DepthReplacing";
    case 13: return "Bad";
    case 14: return "DepthGreater";
    case 15: return "DepthLess";
    case 16: return "DepthUnchanged";
    case 17: return "LocalSize";
    case 18: return "LocalSizeHint";
    case 19: return "InputPoints";
    case 20: return "InputLines";
    case 21: return "InputLinesAdjacency";
    case 22: return "Triangles";
    case 23: return "InputTrianglesAdjacency";
    case 24: return "Quads";
    case 25: return "Isolines";
    case 26: return "OutputVertices";
    case 27: return "OutputPoints";
    case 28: return "OutputLineStrip";
    case 29: return "OutputTriangleStrip";
    case 30: return "VecTypeHint";
    case 31: return "ContractionOff";
    case 32: return "Bad";

    case ExecutionModeCeiling:
    default: return "Bad";
    }
}

const int StorageClassCeiling = 13;

const char* StorageClassString(int StorageClass)
{
    switch (StorageClass) {
    case 0:  return "UniformConstant";
    case 1:  return "Input";
    case 2:  return "Uniform";
    case 3:  return "Output";
    case 4:  return "Workgroup";
    case 5:  return "CrossWorkgroup";
    case 6:  return "Private";
    case 7:  return "Function";
    case 8:  return "Generic";
    case 9:  return "PushConstant";
    case 10: return "AtomicCounter";
    case 11: return "Image";
    case 12: return "StorageBuffer";

    case StorageClassCeiling:
    default: return "Bad";
    }
}

const int DecorationCeiling = 45;

const char* DecorationString(int decoration)
{
    switch (decoration) {
    case 0:  return "RelaxedPrecision";
    case 1:  return "SpecId";
    case 2:  return "Block";
    case 3:  return "BufferBlock";
    case 4:  return "RowMajor";
    case 5:  return "ColMajor";
    case 6:  return "ArrayStride";
    case 7:  return "MatrixStride";
    case 8:  return "GLSLShared";
    case 9:  return "GLSLPacked";
    case 10: return "CPacked";
    case 11: return "BuiltIn";
    case 12: return "Bad";
    case 13: return "NoPerspective";
    case 14: return "Flat";
    case 15: return "Patch";
    case 16: return "Centroid";
    case 17: return "Sample";
    case 18: return "Invariant";
    case 19: return "Restrict";
    case 20: return "Aliased";
    case 21: return "Volatile";
    case 22: return "Constant";
    case 23: return "Coherent";
    case 24: return "NonWritable";
    case 25: return "NonReadable";
    case 26: return "Uniform";
    case 27: return "Bad";
    case 28: return "SaturatedConversion";
    case 29: return "Stream";
    case 30: return "Location";
    case 31: return "Component";
    case 32: return "Index";
    case 33: return "Binding";
    case 34: return "DescriptorSet";
    case 35: return "Offset";
    case 36: return "XfbBuffer";
    case 37: return "XfbStride";
    case 38: return "FuncParamAttr";
    case 39: return "FP Rounding Mode";
    case 40: return "FP Fast Math Mode";
    case 41: return "Linkage Attributes";
    case 42: return "NoContraction";
    case 43: return "InputAttachmentIndex";
    case 44: return "Alignment";

    case DecorationCeiling:
    default:  return "Bad";

#ifdef AMD_EXTENSIONS
    case 4999: return "ExplicitInterpAMD";
#endif
#ifdef NV_EXTENSIONS
    case 5248: return "OverrideCoverageNV";
    case 5250: return "PassthroughNV";
    case 5252: return "ViewportRelativeNV";
    case 5256: return "SecondaryViewportRelativeNV";
#endif
    }
}

const int BuiltInCeiling = 44;

const char* BuiltInString(int builtIn)
{
    switch (builtIn) {
    case 0:  return "Position";
    case 1:  return "PointSize";
    case 2:  return "Bad";
    case 3:  return "ClipDistance";
    case 4:  return "CullDistance";
    case 5:  return "VertexId";
    case 6:  return "InstanceId";
    case 7:  return "PrimitiveId";
    case 8:  return "InvocationId";
    case 9:  return "Layer";
    case 10: return "ViewportIndex";
    case 11: return "TessLevelOuter";
    case 12: return "TessLevelInner";
    case 13: return "TessCoord";
    case 14: return "PatchVertices";
    case 15: return "FragCoord";
    case 16: return "PointCoord";
    case 17: return "FrontFacing";
    case 18: return "SampleId";
    case 19: return "SamplePosition";
    case 20: return "SampleMask";
    case 21: return "Bad";
    case 22: return "FragDepth";
    case 23: return "HelperInvocation";
    case 24: return "NumWorkgroups";
    case 25: return "WorkgroupSize";
    case 26: return "WorkgroupId";
    case 27: return "LocalInvocationId";
    case 28: return "GlobalInvocationId";
    case 29: return "LocalInvocationIndex";
    case 30: return "WorkDim";
    case 31: return "GlobalSize";
    case 32: return "EnqueuedWorkgroupSize";
    case 33: return "GlobalOffset";
    case 34: return "GlobalLinearId";
    case 35: return "Bad";
    case 36: return "SubgroupSize";
    case 37: return "SubgroupMaxSize";
    case 38: return "NumSubgroups";
    case 39: return "NumEnqueuedSubgroups";
    case 40: return "SubgroupId";
    case 41: return "SubgroupLocalInvocationId";
    case 42: return "VertexIndex";                 // TBD: put next to VertexId?
    case 43: return "InstanceIndex";               // TBD: put next to InstanceId?

    case 4416: return "SubgroupEqMaskKHR";
    case 4417: return "SubgroupGeMaskKHR";
    case 4418: return "SubgroupGtMaskKHR";
    case 4419: return "SubgroupLeMaskKHR";
    case 4420: return "SubgroupLtMaskKHR";
    case 4438: return "DeviceIndex";
    case 4440: return "ViewIndex";
    case 4424: return "BaseVertex";
    case 4425: return "BaseInstance";
    case 4426: return "DrawIndex";

#ifdef AMD_EXTENSIONS
    case 4992: return "BaryCoordNoPerspAMD";
    case 4993: return "BaryCoordNoPerspCentroidAMD";
    case 4994: return "BaryCoordNoPerspSampleAMD";
    case 4995: return "BaryCoordSmoothAMD";
    case 4996: return "BaryCoordSmoothCentroidAMD";
    case 4997: return "BaryCoordSmoothSampleAMD";
    case 4998: return "BaryCoordPullModelAMD";
#endif

#ifdef NV_EXTENSIONS
    case 5253: return "ViewportMaskNV";
    case 5257: return "SecondaryPositionNV";
    case 5258: return "SecondaryViewportMaskNV";
    case 5261: return "PositionPerViewNV";
    case 5262: return "ViewportMaskPerViewNV";
#endif

    case BuiltInCeiling:
    default: return "Bad";
    }
}

const int DimensionCeiling = 7;

const char* DimensionString(int dim)
{
    switch (dim) {
    case 0:  return "1D";
    case 1:  return "2D";
    case 2:  return "3D";
    case 3:  return "Cube";
    case 4:  return "Rect";
    case 5:  return "Buffer";
    case 6:  return "SubpassData";

    case DimensionCeiling:
    default: return "Bad";
    }
}

const int SamplerAddressingModeCeiling = 5;

const char* SamplerAddressingModeString(int mode)
{
    switch (mode) {
    case 0:  return "None";
    case 1:  return "ClampToEdge";
    case 2:  return "Clamp";
    case 3:  return "Repeat";
    case 4:  return "RepeatMirrored";

    case SamplerAddressingModeCeiling:
    default: return "Bad";
    }
}

const int SamplerFilterModeCeiling = 2;

const char* SamplerFilterModeString(int mode)
{
    switch (mode) {
    case 0: return "Nearest";
    case 1: return "Linear";

    case SamplerFilterModeCeiling:
    default: return "Bad";
    }
}

const int ImageFormatCeiling = 40;

const char* ImageFormatString(int format)
{
    switch (format) {
    case  0: return "Unknown";

    // ES/Desktop float
    case  1: return "Rgba32f";
    case  2: return "Rgba16f";
    case  3: return "R32f";
    case  4: return "Rgba8";
    case  5: return "Rgba8Snorm";

    // Desktop float
    case  6: return "Rg32f";
    case  7: return "Rg16f";
    case  8: return "R11fG11fB10f";
    case  9: return "R16f";
    case 10: return "Rgba16";
    case 11: return "Rgb10A2";
    case 12: return "Rg16";
    case 13: return "Rg8";
    case 14: return "R16";
    case 15: return "R8";
    case 16: return "Rgba16Snorm";
    case 17: return "Rg16Snorm";
    case 18: return "Rg8Snorm";
    case 19: return "R16Snorm";
    case 20: return "R8Snorm";

    // ES/Desktop int
    case 21: return "Rgba32i";
    case 22: return "Rgba16i";
    case 23: return "Rgba8i";
    case 24: return "R32i";

    // Desktop int
    case 25: return "Rg32i";
    case 26: return "Rg16i";
    case 27: return "Rg8i";
    case 28: return "R16i";
    case 29: return "R8i";

    // ES/Desktop uint
    case 30: return "Rgba32ui";
    case 31: return "Rgba16ui";
    case 32: return "Rgba8ui";
    case 33: return "R32ui";

    // Desktop uint
    case 34: return "Rgb10a2ui";
    case 35: return "Rg32ui";
    case 36: return "Rg16ui";
    case 37: return "Rg8ui";
    case 38: return "R16ui";
    case 39: return "R8ui";

    case ImageFormatCeiling:
    default:
        return "Bad";
    }
}

const int ImageChannelOrderCeiling = 19;

const char* ImageChannelOrderString(int format)
{
    switch (format) {
    case 0:  return "R";
    case 1:  return "A";
    case 2:  return "RG";
    case 3:  return "RA";
    case 4:  return "RGB";
    case 5:  return "RGBA";
    case 6:  return "BGRA";
    case 7:  return "ARGB";
    case 8:  return "Intensity";
    case 9:  return "Luminance";
    case 10: return "Rx";
    case 11: return "RGx";
    case 12: return "RGBx";
    case 13: return "Depth";
    case 14: return "DepthStencil";
    case 15: return "sRGB";
    case 16: return "sRGBx";
    case 17: return "sRGBA";
    case 18: return "sBGRA";

    case ImageChannelOrderCeiling:
    default: 
        return "Bad";
    }
}

const int ImageChannelDataTypeCeiling = 17;

const char* ImageChannelDataTypeString(int type)
{
    switch (type)
    {
    case 0: return "SnormInt8";
    case 1: return "SnormInt16";
    case 2: return "UnormInt8";
    case 3: return "UnormInt16";
    case 4: return "UnormShort565";
    case 5: return "UnormShort555";
    case 6: return "UnormInt101010";
    case 7: return "SignedInt8";
    case 8: return "SignedInt16";
    case 9: return "SignedInt32";
    case 10: return "UnsignedInt8";
    case 11: return "UnsignedInt16";
    case 12: return "UnsignedInt32";
    case 13: return "HalfFloat";
    case 14: return "Float";
    case 15: return "UnormInt24";
    case 16: return "UnormInt101010_2";

    case ImageChannelDataTypeCeiling:
    default:
        return "Bad";
    }
}

const int ImageOperandsCeiling = 8;

const char* ImageOperandsString(int format)
{
    switch (format) {
    case 0: return "Bias";
    case 1: return "Lod";
    case 2: return "Grad";
    case 3: return "ConstOffset";
    case 4: return "Offset";
    case 5: return "ConstOffsets";
    case 6: return "Sample";
    case 7: return "MinLod";

    case ImageOperandsCeiling:
    default:
        return "Bad";
    }
}

const int FPFastMathCeiling = 5;

const char* FPFastMathString(int mode)
{
    switch (mode) {
    case 0: return "NotNaN";
    case 1: return "NotInf";
    case 2: return "NSZ";
    case 3: return "AllowRecip";
    case 4: return "Fast";

    case FPFastMathCeiling:
    default:     return "Bad";
    }
}

const int FPRoundingModeCeiling = 4;

const char* FPRoundingModeString(int mode)
{
    switch (mode) {
    case 0:  return "RTE";
    case 1:  return "RTZ";
    case 2:  return "RTP";
    case 3:  return "RTN";

    case FPRoundingModeCeiling:
    default: return "Bad";
    }
}

const int LinkageTypeCeiling = 2;

const char* LinkageTypeString(int type)
{
    switch (type) {
    case 0:  return "Export";
    case 1:  return "Import";

    case LinkageTypeCeiling:
    default: return "Bad";
    }
}

const int FuncParamAttrCeiling = 8;

const char* FuncParamAttrString(int attr)
{
    switch (attr) {
    case 0:  return "Zext";
    case 1:  return "Sext";
    case 2:  return "ByVal";
    case 3:  return "Sret";
    case 4:  return "NoAlias";
    case 5:  return "NoCapture";
    case 6:  return "NoWrite";
    case 7:  return "NoReadWrite";

    case FuncParamAttrCeiling:
    default: return "Bad";
    }
}

const int AccessQualifierCeiling = 3;

const char* AccessQualifierString(int attr)
{
    switch (attr) {
    case 0:  return "ReadOnly";
    case 1:  return "WriteOnly";
    case 2:  return "ReadWrite";

    case AccessQualifierCeiling:
    default: return "Bad";
    }
}

const int SelectControlCeiling = 2;

const char* SelectControlString(int cont)
{
    switch (cont) {
    case 0:  return "Flatten";
    case 1:  return "DontFlatten";

    case SelectControlCeiling:
    default: return "Bad";
    }
}

const int LoopControlCeiling = 2;

const char* LoopControlString(int cont)
{
    switch (cont) {
    case 0:  return "Unroll";
    case 1:  return "DontUnroll";

    case LoopControlCeiling:
    default: return "Bad";
    }
}

const int FunctionControlCeiling = 4;

const char* FunctionControlString(int cont)
{
    switch (cont) {
    case 0:  return "Inline";
    case 1:  return "DontInline";
    case 2:  return "Pure";
    case 3:  return "Const";

    case FunctionControlCeiling:
    default: return "Bad";
    }
}

const int MemorySemanticsCeiling = 12;

const char* MemorySemanticsString(int mem)
{
    // Note: No bits set (None) means "Relaxed"
    switch (mem) {
    case 0: return "Bad"; // Note: this is a placeholder for 'Consume'
    case 1: return "Acquire";
    case 2: return "Release";
    case 3: return "AcquireRelease";
    case 4: return "SequentiallyConsistent";
    case 5: return "Bad"; // Note: reserved for future expansion
    case 6: return "UniformMemory";
    case 7: return "SubgroupMemory";
    case 8: return "WorkgroupMemory";
    case 9: return "CrossWorkgroupMemory";
    case 10: return "AtomicCounterMemory";
    case 11: return "ImageMemory";

    case MemorySemanticsCeiling:
    default:     return "Bad";
    }
}

const int MemoryAccessCeiling = 3;

const char* MemoryAccessString(int mem)
{
    switch (mem) {
    case 0:  return "Volatile";
    case 1:  return "Aligned";
    case 2:  return "Nontemporal";

    case MemoryAccessCeiling:
    default: return "Bad";
    }
}

const int ScopeCeiling = 5;

const char* ScopeString(int mem)
{
    switch (mem) {
    case 0:  return "CrossDevice";
    case 1:  return "Device";
    case 2:  return "Workgroup";
    case 3:  return "Subgroup";
    case 4:  return "Invocation";

    case ScopeCeiling:
    default: return "Bad";
    }
}

const int GroupOperationCeiling = 3;

const char* GroupOperationString(int gop)
{

    switch (gop)
    {
    case 0:  return "Reduce";
    case 1:  return "InclusiveScan";
    case 2:  return "ExclusiveScan";

    case GroupOperationCeiling:
    default: return "Bad";
    }
}

const int KernelEnqueueFlagsCeiling = 3;

const char* KernelEnqueueFlagsString(int flag)
{
    switch (flag)
    {
    case 0:  return "NoWait";
    case 1:  return "WaitKernel";
    case 2:  return "WaitWorkGroup";

    case KernelEnqueueFlagsCeiling:
    default: return "Bad";
    }
}

const int KernelProfilingInfoCeiling = 1;

const char* KernelProfilingInfoString(int info)
{
    switch (info)
    {
    case 0:  return "CmdExecTime";

    case KernelProfilingInfoCeiling:
    default: return "Bad";
    }
}

const int CapabilityCeiling = 58;

const char* CapabilityString(int info)
{
    switch (info)
    {
    case 0:  return "Matrix";
    case 1:  return "Shader";
    case 2:  return "Geometry";
    case 3:  return "Tessellation";
    case 4:  return "Addresses";
    case 5:  return "Linkage";
    case 6:  return "Kernel";
    case 7:  return "Vector16";
    case 8:  return "Float16Buffer";
    case 9:  return "Float16";
    case 10: return "Float64";
    case 11: return "Int64";
    case 12: return "Int64Atomics";
    case 13: return "ImageBasic";
    case 14: return "ImageReadWrite";
    case 15: return "ImageMipmap";
    case 16: return "Bad";
    case 17: return "Pipes";
    case 18: return "Groups";
    case 19: return "DeviceEnqueue";
    case 20: return "LiteralSampler";
    case 21: return "AtomicStorage";
    case 22: return "Int16";
    case 23: return "TessellationPointSize";
    case 24: return "GeometryPointSize";
    case 25: return "ImageGatherExtended"; 
    case 26: return "Bad";
    case 27: return "StorageImageMultisample";
    case 28: return "UniformBufferArrayDynamicIndexing";
    case 29: return "SampledImageArrayDynamicIndexing";
    case 30: return "StorageBufferArrayDynamicIndexing";
    case 31: return "StorageImageArrayDynamicIndexing";
    case 32: return "ClipDistance";
    case 33: return "CullDistance";
    case 34: return "ImageCubeArray";
    case 35: return "SampleRateShading";
    case 36: return "ImageRect";
    case 37: return "SampledRect";
    case 38: return "GenericPointer";
    case 39: return "Int8";
    case 40: return "InputAttachment";
    case 41: return "SparseResidency";
    case 42: return "MinLod";
    case 43: return "Sampled1D";
    case 44: return "Image1D";
    case 45: return "SampledCubeArray";
    case 46: return "SampledBuffer";
    case 47: return "ImageBuffer";
    case 48: return "ImageMSArray";
    case 49: return "StorageImageExtendedFormats";
    case 50: return "ImageQuery";
    case 51: return "DerivativeControl";
    case 52: return "InterpolationFunction";
    case 53: return "TransformFeedback";
    case 54: return "GeometryStreams";
    case 55: return "StorageImageReadWithoutFormat";
    case 56: return "StorageImageWriteWithoutFormat";
    case 57: return "MultiViewport";

    case 4423: return "SubgroupBallotKHR";
    case 4427: return "DrawParameters";
    case 4431: return "SubgroupVoteKHR";

    case 4433: return "StorageUniformBufferBlock16";
    case 4434: return "StorageUniform16";
    case 4435: return "StoragePushConstant16";
    case 4436: return "StorageInputOutput16";

    case 4437: return "DeviceGroup";
    case 4439: return "MultiView";

#ifdef AMD_EXTENSIONS
    case 5009: return "ImageGatherBiasLodAMD";
#endif

#ifdef NV_EXTENSIONS
    case 5251: return "GeometryShaderPassthroughNV";
    case 5254: return "ShaderViewportIndexLayerNV";
    case 5255: return "ShaderViewportMaskNV";
    case 5259: return "ShaderStereoViewNV";
    case 5260: return "PerViewAttributesNV";
#endif

    case CapabilityCeiling:
    default: return "Bad";
    }
}

const char* OpcodeString(int op)
{
    switch (op) {
    case 0:   return "OpNop";
    case 1:   return "OpUndef";
    case 2:   return "OpSourceContinued";
    case 3:   return "OpSource";
    case 4:   return "OpSourceExtension";
    case 5:   return "OpName";
    case 6:   return "OpMemberName";
    case 7:   return "OpString";
    case 8:   return "OpLine";
    case 9:   return "Bad";
    case 10:  return "OpExtension";
    case 11:  return "OpExtInstImport";
    case 12:  return "OpExtInst";
    case 13:  return "Bad";
    case 14:  return "OpMemoryModel";
    case 15:  return "OpEntryPoint";
    case 16:  return "OpExecutionMode";
    case 17:  return "OpCapability";
    case 18:  return "Bad";
    case 19:  return "OpTypeVoid";
    case 20:  return "OpTypeBool";
    case 21:  return "OpTypeInt";
    case 22:  return "OpTypeFloat";
    case 23:  return "OpTypeVector";
    case 24:  return "OpTypeMatrix";
    case 25:  return "OpTypeImage";
    case 26:  return "OpTypeSampler";
    case 27:  return "OpTypeSampledImage";
    case 28:  return "OpTypeArray";
    case 29:  return "OpTypeRuntimeArray";
    case 30:  return "OpTypeStruct";
    case 31:  return "OpTypeOpaque";
    case 32:  return "OpTypePointer";
    case 33:  return "OpTypeFunction";
    case 34:  return "OpTypeEvent";
    case 35:  return "OpTypeDeviceEvent";
    case 36:  return "OpTypeReserveId";
    case 37:  return "OpTypeQueue";
    case 38:  return "OpTypePipe";
    case 39:  return "OpTypeForwardPointer";
    case 40:  return "Bad";
    case 41:  return "OpConstantTrue";
    case 42:  return "OpConstantFalse";
    case 43:  return "OpConstant";
    case 44:  return "OpConstantComposite";
    case 45:  return "OpConstantSampler";
    case 46:  return "OpConstantNull";
    case 47:  return "Bad";
    case 48:  return "OpSpecConstantTrue";
    case 49:  return "OpSpecConstantFalse";
    case 50:  return "OpSpecConstant";
    case 51:  return "OpSpecConstantComposite";
    case 52:  return "OpSpecConstantOp";
    case 53:  return "Bad";
    case 54:  return "OpFunction";
    case 55:  return "OpFunctionParameter";
    case 56:  return "OpFunctionEnd";
    case 57:  return "OpFunctionCall";
    case 58:  return "Bad";
    case 59:  return "OpVariable";
    case 60:  return "OpImageTexelPointer";
    case 61:  return "OpLoad";
    case 62:  return "OpStore";
    case 63:  return "OpCopyMemory";
    case 64:  return "OpCopyMemorySized";
    case 65:  return "OpAccessChain";
    case 66:  return "OpInBoundsAccessChain";
    case 67:  return "OpPtrAccessChain";
    case 68:  return "OpArrayLength";
    case 69:  return "OpGenericPtrMemSemantics";
    case 70:  return "OpInBoundsPtrAccessChain";
    case 71:  return "OpDecorate";
    case 72:  return "OpMemberDecorate";
    case 73:  return "OpDecorationGroup";
    case 74:  return "OpGroupDecorate";
    case 75:  return "OpGroupMemberDecorate";
    case 76:  return "Bad";
    case 77:  return "OpVectorExtractDynamic";
    case 78:  return "OpVectorInsertDynamic";
    case 79:  return "OpVectorShuffle";
    case 80:  return "OpCompositeConstruct";
    case 81:  return "OpCompositeExtract";
    case 82:  return "OpCompositeInsert";
    case 83:  return "OpCopyObject";
    case 84:  return "OpTranspose";
    case 85:  return "Bad";
    case 86:  return "OpSampledImage";
    case 87:  return "OpImageSampleImplicitLod";
    case 88:  return "OpImageSampleExplicitLod";
    case 89:  return "OpImageSampleDrefImplicitLod";
    case 90:  return "OpImageSampleDrefExplicitLod";
    case 91:  return "OpImageSampleProjImplicitLod";
    case 92:  return "OpImageSampleProjExplicitLod";
    case 93:  return "OpImageSampleProjDrefImplicitLod";
    case 94:  return "OpImageSampleProjDrefExplicitLod";
    case 95:  return "OpImageFetch";
    case 96:  return "OpImageGather";
    case 97:  return "OpImageDrefGather";
    case 98:  return "OpImageRead";
    case 99:  return "OpImageWrite";
    case 100: return "OpImage";
    case 101: return "OpImageQueryFormat";
    case 102: return "OpImageQueryOrder";
    case 103: return "OpImageQuerySizeLod";
    case 104: return "OpImageQuerySize";
    case 105: return "OpImageQueryLod";
    case 106: return "OpImageQueryLevels";
    case 107: return "OpImageQuerySamples";
    case 108: return "Bad";
    case 109: return "OpConvertFToU";
    case 110: return "OpConvertFToS";
    case 111: return "OpConvertSToF";
    case 112: return "OpConvertUToF";
    case 113: return "OpUConvert";
    case 114: return "OpSConvert";
    case 115: return "OpFConvert";
    case 116: return "OpQuantizeToF16";
    case 117: return "OpConvertPtrToU";
    case 118: return "OpSatConvertSToU";
    case 119: return "OpSatConvertUToS";
    case 120: return "OpConvertUToPtr";
    case 121: return "OpPtrCastToGeneric";
    case 122: return "OpGenericCastToPtr";
    case 123: return "OpGenericCastToPtrExplicit";
    case 124: return "OpBitcast";
    case 125: return "Bad";
    case 126: return "OpSNegate";
    case 127: return "OpFNegate";
    case 128: return "OpIAdd";
    case 129: return "OpFAdd";
    case 130: return "OpISub";
    case 131: return "OpFSub";
    case 132: return "OpIMul";
    case 133: return "OpFMul";
    case 134: return "OpUDiv";
    case 135: return "OpSDiv";
    case 136: return "OpFDiv";
    case 137: return "OpUMod";
    case 138: return "OpSRem";
    case 139: return "OpSMod";
    case 140: return "OpFRem";
    case 141: return "OpFMod";
    case 142: return "OpVectorTimesScalar";
    case 143: return "OpMatrixTimesScalar";
    case 144: return "OpVectorTimesMatrix";
    case 145: return "OpMatrixTimesVector";
    case 146: return "OpMatrixTimesMatrix";
    case 147: return "OpOuterProduct";
    case 148: return "OpDot";
    case 149: return "OpIAddCarry";
    case 150: return "OpISubBorrow";
    case 151: return "OpUMulExtended";
    case 152: return "OpSMulExtended";
    case 153: return "Bad";
    case 154: return "OpAny";
    case 155: return "OpAll";
    case 156: return "OpIsNan";
    case 157: return "OpIsInf";
    case 158: return "OpIsFinite";
    case 159: return "OpIsNormal";
    case 160: return "OpSignBitSet";
    case 161: return "OpLessOrGreater";
    case 162: return "OpOrdered";
    case 163: return "OpUnordered";
    case 164: return "OpLogicalEqual";
    case 165: return "OpLogicalNotEqual";
    case 166: return "OpLogicalOr";
    case 167: return "OpLogicalAnd";
    case 168: return "OpLogicalNot";
    case 169: return "OpSelect";
    case 170: return "OpIEqual";
    case 171: return "OpINotEqual";
    case 172: return "OpUGreaterThan";
    case 173: return "OpSGreaterThan";
    case 174: return "OpUGreaterThanEqual";
    case 175: return "OpSGreaterThanEqual";
    case 176: return "OpULessThan";
    case 177: return "OpSLessThan";
    case 178: return "OpULessThanEqual";
    case 179: return "OpSLessThanEqual";
    case 180: return "OpFOrdEqual";
    case 181: return "OpFUnordEqual";
    case 182: return "OpFOrdNotEqual";
    case 183: return "OpFUnordNotEqual";
    case 184: return "OpFOrdLessThan";
    case 185: return "OpFUnordLessThan";
    case 186: return "OpFOrdGreaterThan";
    case 187: return "OpFUnordGreaterThan";
    case 188: return "OpFOrdLessThanEqual";
    case 189: return "OpFUnordLessThanEqual";
    case 190: return "OpFOrdGreaterThanEqual";
    case 191: return "OpFUnordGreaterThanEqual";
    case 192: return "Bad";
    case 193: return "Bad";
    case 194: return "OpShiftRightLogical";
    case 195: return "OpShiftRightArithmetic";
    case 196: return "OpShiftLeftLogical";
    case 197: return "OpBitwiseOr";
    case 198: return "OpBitwiseXor";
    case 199: return "OpBitwiseAnd";
    case 200: return "OpNot";
    case 201: return "OpBitFieldInsert";
    case 202: return "OpBitFieldSExtract";
    case 203: return "OpBitFieldUExtract";
    case 204: return "OpBitReverse";
    case 205: return "OpBitCount";
    case 206: return "Bad";
    case 207: return "OpDPdx";
    case 208: return "OpDPdy";
    case 209: return "OpFwidth";
    case 210: return "OpDPdxFine";
    case 211: return "OpDPdyFine";
    case 212: return "OpFwidthFine";
    case 213: return "OpDPdxCoarse";
    case 214: return "OpDPdyCoarse";
    case 215: return "OpFwidthCoarse";
    case 216: return "Bad";
    case 217: return "Bad";
    case 218: return "OpEmitVertex";
    case 219: return "OpEndPrimitive";
    case 220: return "OpEmitStreamVertex";
    case 221: return "OpEndStreamPrimitive";
    case 222: return "Bad";
    case 223: return "Bad";
    case 224: return "OpControlBarrier";
    case 225: return "OpMemoryBarrier";
    case 226: return "Bad";
    case 227: return "OpAtomicLoad";
    case 228: return "OpAtomicStore";
    case 229: return "OpAtomicExchange";
    case 230: return "OpAtomicCompareExchange";
    case 231: return "OpAtomicCompareExchangeWeak";
    case 232: return "OpAtomicIIncrement";
    case 233: return "OpAtomicIDecrement";
    case 234: return "OpAtomicIAdd";
    case 235: return "OpAtomicISub";
    case 236: return "OpAtomicSMin";
    case 237: return "OpAtomicUMin";
    case 238: return "OpAtomicSMax";
    case 239: return "OpAtomicUMax";
    case 240: return "OpAtomicAnd";
    case 241: return "OpAtomicOr";
    case 242: return "OpAtomicXor";
    case 243: return "Bad";
    case 244: return "Bad";
    case 245: return "OpPhi";
    case 246: return "OpLoopMerge";
    case 247: return "OpSelectionMerge";
    case 248: return "OpLabel";
    case 249: return "OpBranch";
    case 250: return "OpBranchConditional";
    case 251: return "OpSwitch";
    case 252: return "OpKill";
    case 253: return "OpReturn";
    case 254: return "OpReturnValue";
    case 255: return "OpUnreachable";
    case 256: return "OpLifetimeStart";
    case 257: return "OpLifetimeStop";
    case 258: return "Bad";
    case 259: return "OpGroupAsyncCopy";
    case 260: return "OpGroupWaitEvents";
    case 261: return "OpGroupAll";
    case 262: return "OpGroupAny";
    case 263: return "OpGroupBroadcast";
    case 264: return "OpGroupIAdd";
    case 265: return "OpGroupFAdd";
    case 266: return "OpGroupFMin";
    case 267: return "OpGroupUMin";
    case 268: return "OpGroupSMin";
    case 269: return "OpGroupFMax";
    case 270: return "OpGroupUMax";
    case 271: return "OpGroupSMax";
    case 272: return "Bad";
    case 273: return "Bad";
    case 274: return "OpReadPipe";
    case 275: return "OpWritePipe";
    case 276: return "OpReservedReadPipe";
    case 277: return "OpReservedWritePipe";
    case 278: return "OpReserveReadPipePackets";
    case 279: return "OpReserveWritePipePackets";
    case 280: return "OpCommitReadPipe";
    case 281: return "OpCommitWritePipe";
    case 282: return "OpIsValidReserveId";
    case 283: return "OpGetNumPipePackets";
    case 284: return "OpGetMaxPipePackets";
    case 285: return "OpGroupReserveReadPipePackets";
    case 286: return "OpGroupReserveWritePipePackets";
    case 287: return "OpGroupCommitReadPipe";
    case 288: return "OpGroupCommitWritePipe";
    case 289: return "Bad";
    case 290: return "Bad";
    case 291: return "OpEnqueueMarker";
    case 292: return "OpEnqueueKernel";
    case 293: return "OpGetKernelNDrangeSubGroupCount";
    case 294: return "OpGetKernelNDrangeMaxSubGroupSize";
    case 295: return "OpGetKernelWorkGroupSize";
    case 296: return "OpGetKernelPreferredWorkGroupSizeMultiple";
    case 297: return "OpRetainEvent";
    case 298: return "OpReleaseEvent";
    case 299: return "OpCreateUserEvent";
    case 300: return "OpIsValidEvent";
    case 301: return "OpSetUserEventStatus";
    case 302: return "OpCaptureEventProfilingInfo";
    case 303: return "OpGetDefaultQueue";
    case 304: return "OpBuildNDRange";
    case 305: return "OpImageSparseSampleImplicitLod";
    case 306: return "OpImageSparseSampleExplicitLod";
    case 307: return "OpImageSparseSampleDrefImplicitLod";
    case 308: return "OpImageSparseSampleDrefExplicitLod";
    case 309: return "OpImageSparseSampleProjImplicitLod";
    case 310: return "OpImageSparseSampleProjExplicitLod";
    case 311: return "OpImageSparseSampleProjDrefImplicitLod";
    case 312: return "OpImageSparseSampleProjDrefExplicitLod";
    case 313: return "OpImageSparseFetch";
    case 314: return "OpImageSparseGather";
    case 315: return "OpImageSparseDrefGather";
    case 316: return "OpImageSparseTexelsResident";
    case 317: return "OpNoLine";
    case 318: return "OpAtomicFlagTestAndSet";
    case 319: return "OpAtomicFlagClear";
    case 320: return "OpImageSparseRead";

    case 4421: return "OpSubgroupBallotKHR";
    case 4422: return "OpSubgroupFirstInvocationKHR";
    case 4428: return "OpSubgroupAllKHR";
    case 4429: return "OpSubgroupAnyKHR";
    case 4430: return "OpSubgroupAllEqualKHR";
    case 4432: return "OpSubgroupReadInvocationKHR";

#ifdef AMD_EXTENSIONS
    case 5000: return "OpGroupIAddNonUniformAMD";
    case 5001: return "OpGroupFAddNonUniformAMD";
    case 5002: return "OpGroupFMinNonUniformAMD";
    case 5003: return "OpGroupUMinNonUniformAMD";
    case 5004: return "OpGroupSMinNonUniformAMD";
    case 5005: return "OpGroupFMaxNonUniformAMD";
    case 5006: return "OpGroupUMaxNonUniformAMD";
    case 5007: return "OpGroupSMaxNonUniformAMD";
#endif

    case OpcodeCeiling:
    default:
        return "Bad";
    }
}

// The set of objects that hold all the instruction/operand
// parameterization information.
InstructionParameters InstructionDesc[OpCodeMask + 1];
OperandParameters ExecutionModeOperands[ExecutionModeCeiling];
OperandParameters DecorationOperands[DecorationCeiling];

EnumDefinition OperandClassParams[OperandCount];
EnumParameters ExecutionModelParams[ExecutionModelCeiling];
EnumParameters AddressingParams[AddressingModelCeiling];
EnumParameters MemoryParams[MemoryModelCeiling];
EnumParameters ExecutionModeParams[ExecutionModeCeiling];
EnumParameters StorageParams[StorageClassCeiling];
EnumParameters SamplerAddressingModeParams[SamplerAddressingModeCeiling];
EnumParameters SamplerFilterModeParams[SamplerFilterModeCeiling];
EnumParameters ImageFormatParams[ImageFormatCeiling];
EnumParameters ImageChannelOrderParams[ImageChannelOrderCeiling];
EnumParameters ImageChannelDataTypeParams[ImageChannelDataTypeCeiling];
EnumParameters ImageOperandsParams[ImageOperandsCeiling];
EnumParameters FPFastMathParams[FPFastMathCeiling];
EnumParameters FPRoundingModeParams[FPRoundingModeCeiling];
EnumParameters LinkageTypeParams[LinkageTypeCeiling];
EnumParameters DecorationParams[DecorationCeiling];
EnumParameters BuiltInParams[BuiltInCeiling];
EnumParameters DimensionalityParams[DimensionCeiling];
EnumParameters FuncParamAttrParams[FuncParamAttrCeiling];
EnumParameters AccessQualifierParams[AccessQualifierCeiling];
EnumParameters GroupOperationParams[GroupOperationCeiling];
EnumParameters LoopControlParams[FunctionControlCeiling];
EnumParameters SelectionControlParams[SelectControlCeiling];
EnumParameters FunctionControlParams[FunctionControlCeiling];
EnumParameters MemorySemanticsParams[MemorySemanticsCeiling];
EnumParameters MemoryAccessParams[MemoryAccessCeiling];
EnumParameters ScopeParams[ScopeCeiling];
EnumParameters KernelEnqueueFlagsParams[KernelEnqueueFlagsCeiling];
EnumParameters KernelProfilingInfoParams[KernelProfilingInfoCeiling];
EnumParameters CapabilityParams[CapabilityCeiling];

// Set up all the parameterizing descriptions of the opcodes, operands, etc.
void Parameterize()
{
    // only do this once.
    static bool initialized = false;
    if (initialized)
        return;
    initialized = true;

    // Exceptions to having a result <id> and a resulting type <id>.
    // (Everything is initialized to have both).

    InstructionDesc[OpNop].setResultAndType(false, false);
    InstructionDesc[OpSource].setResultAndType(false, false);
    InstructionDesc[OpSourceContinued].setResultAndType(false, false);
    InstructionDesc[OpSourceExtension].setResultAndType(false, false);
    InstructionDesc[OpExtension].setResultAndType(false, false);
    InstructionDesc[OpExtInstImport].setResultAndType(true, false);
    InstructionDesc[OpCapability].setResultAndType(false, false);
    InstructionDesc[OpMemoryModel].setResultAndType(false, false);
    InstructionDesc[OpEntryPoint].setResultAndType(false, false);
    InstructionDesc[OpExecutionMode].setResultAndType(false, false);
    InstructionDesc[OpTypeVoid].setResultAndType(true, false);
    InstructionDesc[OpTypeBool].setResultAndType(true, false);
    InstructionDesc[OpTypeInt].setResultAndType(true, false);
    InstructionDesc[OpTypeFloat].setResultAndType(true, false);
    InstructionDesc[OpTypeVector].setResultAndType(true, false);
    InstructionDesc[OpTypeMatrix].setResultAndType(true, false);
    InstructionDesc[OpTypeImage].setResultAndType(true, false);
    InstructionDesc[OpTypeSampler].setResultAndType(true, false);
    InstructionDesc[OpTypeSampledImage].setResultAndType(true, false);
    InstructionDesc[OpTypeArray].setResultAndType(true, false);
    InstructionDesc[OpTypeRuntimeArray].setResultAndType(true, false);
    InstructionDesc[OpTypeStruct].setResultAndType(true, false);
    InstructionDesc[OpTypeOpaque].setResultAndType(true, false);
    InstructionDesc[OpTypePointer].setResultAndType(true, false);
    InstructionDesc[OpTypeForwardPointer].setResultAndType(false, false);
    InstructionDesc[OpTypeFunction].setResultAndType(true, false);
    InstructionDesc[OpTypeEvent].setResultAndType(true, false);
    InstructionDesc[OpTypeDeviceEvent].setResultAndType(true, false);
    InstructionDesc[OpTypeReserveId].setResultAndType(true, false);
    InstructionDesc[OpTypeQueue].setResultAndType(true, false);
    InstructionDesc[OpTypePipe].setResultAndType(true, false);
    InstructionDesc[OpFunctionEnd].setResultAndType(false, false);
    InstructionDesc[OpStore].setResultAndType(false, false);
    InstructionDesc[OpImageWrite].setResultAndType(false, false);
    InstructionDesc[OpDecorationGroup].setResultAndType(true, false);
    InstructionDesc[OpDecorate].setResultAndType(false, false);
    InstructionDesc[OpMemberDecorate].setResultAndType(false, false);
    InstructionDesc[OpGroupDecorate].setResultAndType(false, false);
    InstructionDesc[OpGroupMemberDecorate].setResultAndType(false, false);
    InstructionDesc[OpName].setResultAndType(false, false);
    InstructionDesc[OpMemberName].setResultAndType(false, false);
    InstructionDesc[OpString].setResultAndType(true, false);
    InstructionDesc[OpLine].setResultAndType(false, false);
    InstructionDesc[OpNoLine].setResultAndType(false, false);
    InstructionDesc[OpCopyMemory].setResultAndType(false, false);
    InstructionDesc[OpCopyMemorySized].setResultAndType(false, false);
    InstructionDesc[OpEmitVertex].setResultAndType(false, false);
    InstructionDesc[OpEndPrimitive].setResultAndType(false, false);
    InstructionDesc[OpEmitStreamVertex].setResultAndType(false, false);
    InstructionDesc[OpEndStreamPrimitive].setResultAndType(false, false);
    InstructionDesc[OpControlBarrier].setResultAndType(false, false);
    InstructionDesc[OpMemoryBarrier].setResultAndType(false, false);
    InstructionDesc[OpAtomicStore].setResultAndType(false, false);
    InstructionDesc[OpLoopMerge].setResultAndType(false, false);
    InstructionDesc[OpSelectionMerge].setResultAndType(false, false);
    InstructionDesc[OpLabel].setResultAndType(true, false);
    InstructionDesc[OpBranch].setResultAndType(false, false);
    InstructionDesc[OpBranchConditional].setResultAndType(false, false);
    InstructionDesc[OpSwitch].setResultAndType(false, false);
    InstructionDesc[OpKill].setResultAndType(false, false);
    InstructionDesc[OpReturn].setResultAndType(false, false);
    InstructionDesc[OpReturnValue].setResultAndType(false, false);
    InstructionDesc[OpUnreachable].setResultAndType(false, false);
    InstructionDesc[OpLifetimeStart].setResultAndType(false, false);
    InstructionDesc[OpLifetimeStop].setResultAndType(false, false);
    InstructionDesc[OpCommitReadPipe].setResultAndType(false, false);
    InstructionDesc[OpCommitWritePipe].setResultAndType(false, false);
    InstructionDesc[OpGroupCommitWritePipe].setResultAndType(false, false);
    InstructionDesc[OpGroupCommitReadPipe].setResultAndType(false, false);
    InstructionDesc[OpCaptureEventProfilingInfo].setResultAndType(false, false);
    InstructionDesc[OpSetUserEventStatus].setResultAndType(false, false);
    InstructionDesc[OpRetainEvent].setResultAndType(false, false);
    InstructionDesc[OpReleaseEvent].setResultAndType(false, false);
    InstructionDesc[OpGroupWaitEvents].setResultAndType(false, false);
    InstructionDesc[OpAtomicFlagClear].setResultAndType(false, false);

    // Specific additional context-dependent operands

    ExecutionModeOperands[ExecutionModeInvocations].push(OperandLiteralNumber, "'Number of <<Invocation,invocations>>'");

    ExecutionModeOperands[ExecutionModeLocalSize].push(OperandLiteralNumber, "'x size'");
    ExecutionModeOperands[ExecutionModeLocalSize].push(OperandLiteralNumber, "'y size'");
    ExecutionModeOperands[ExecutionModeLocalSize].push(OperandLiteralNumber, "'z size'");

    ExecutionModeOperands[ExecutionModeLocalSizeHint].push(OperandLiteralNumber, "'x size'");
    ExecutionModeOperands[ExecutionModeLocalSizeHint].push(OperandLiteralNumber, "'y size'");
    ExecutionModeOperands[ExecutionModeLocalSizeHint].push(OperandLiteralNumber, "'z size'");

    ExecutionModeOperands[ExecutionModeOutputVertices].push(OperandLiteralNumber, "'Vertex count'");
    ExecutionModeOperands[ExecutionModeVecTypeHint].push(OperandLiteralNumber, "'Vector type'");

    DecorationOperands[DecorationStream].push(OperandLiteralNumber, "'Stream Number'");
    DecorationOperands[DecorationLocation].push(OperandLiteralNumber, "'Location'");
    DecorationOperands[DecorationComponent].push(OperandLiteralNumber, "'Component'");
    DecorationOperands[DecorationIndex].push(OperandLiteralNumber, "'Index'");
    DecorationOperands[DecorationBinding].push(OperandLiteralNumber, "'Binding Point'");
    DecorationOperands[DecorationDescriptorSet].push(OperandLiteralNumber, "'Descriptor Set'");
    DecorationOperands[DecorationOffset].push(OperandLiteralNumber, "'Byte Offset'");
    DecorationOperands[DecorationXfbBuffer].push(OperandLiteralNumber, "'XFB Buffer Number'");
    DecorationOperands[DecorationXfbStride].push(OperandLiteralNumber, "'XFB Stride'");
    DecorationOperands[DecorationArrayStride].push(OperandLiteralNumber, "'Array Stride'");
    DecorationOperands[DecorationMatrixStride].push(OperandLiteralNumber, "'Matrix Stride'");
    DecorationOperands[DecorationBuiltIn].push(OperandLiteralNumber, "See <<BuiltIn,*BuiltIn*>>");
    DecorationOperands[DecorationFPRoundingMode].push(OperandFPRoundingMode, "'Floating-Point Rounding Mode'");
    DecorationOperands[DecorationFPFastMathMode].push(OperandFPFastMath, "'Fast-Math Mode'");
    DecorationOperands[DecorationLinkageAttributes].push(OperandLiteralString, "'Name'");
    DecorationOperands[DecorationLinkageAttributes].push(OperandLinkageType, "'Linkage Type'");
    DecorationOperands[DecorationFuncParamAttr].push(OperandFuncParamAttr, "'Function Parameter Attribute'");
    DecorationOperands[DecorationSpecId].push(OperandLiteralNumber, "'Specialization Constant ID'");
    DecorationOperands[DecorationInputAttachmentIndex].push(OperandLiteralNumber, "'Attachment Index'");
    DecorationOperands[DecorationAlignment].push(OperandLiteralNumber, "'Alignment'");

    OperandClassParams[OperandSource].set(SourceLanguageCeiling, SourceString, 0);
    OperandClassParams[OperandExecutionModel].set(ExecutionModelCeiling, ExecutionModelString, ExecutionModelParams);
    OperandClassParams[OperandAddressing].set(AddressingModelCeiling, AddressingString, AddressingParams);
    OperandClassParams[OperandMemory].set(MemoryModelCeiling, MemoryString, MemoryParams);
    OperandClassParams[OperandExecutionMode].set(ExecutionModeCeiling, ExecutionModeString, ExecutionModeParams);
    OperandClassParams[OperandExecutionMode].setOperands(ExecutionModeOperands);
    OperandClassParams[OperandStorage].set(StorageClassCeiling, StorageClassString, StorageParams);
    OperandClassParams[OperandDimensionality].set(DimensionCeiling, DimensionString, DimensionalityParams);
    OperandClassParams[OperandSamplerAddressingMode].set(SamplerAddressingModeCeiling, SamplerAddressingModeString, SamplerAddressingModeParams);
    OperandClassParams[OperandSamplerFilterMode].set(SamplerFilterModeCeiling, SamplerFilterModeString, SamplerFilterModeParams);
    OperandClassParams[OperandSamplerImageFormat].set(ImageFormatCeiling, ImageFormatString, ImageFormatParams);
    OperandClassParams[OperandImageChannelOrder].set(ImageChannelOrderCeiling, ImageChannelOrderString, ImageChannelOrderParams);
    OperandClassParams[OperandImageChannelDataType].set(ImageChannelDataTypeCeiling, ImageChannelDataTypeString, ImageChannelDataTypeParams);
    OperandClassParams[OperandImageOperands].set(ImageOperandsCeiling, ImageOperandsString, ImageOperandsParams, true);
    OperandClassParams[OperandFPFastMath].set(FPFastMathCeiling, FPFastMathString, FPFastMathParams, true);
    OperandClassParams[OperandFPRoundingMode].set(FPRoundingModeCeiling, FPRoundingModeString, FPRoundingModeParams);
    OperandClassParams[OperandLinkageType].set(LinkageTypeCeiling, LinkageTypeString, LinkageTypeParams);
    OperandClassParams[OperandFuncParamAttr].set(FuncParamAttrCeiling, FuncParamAttrString, FuncParamAttrParams);
    OperandClassParams[OperandAccessQualifier].set(AccessQualifierCeiling, AccessQualifierString, AccessQualifierParams);
    OperandClassParams[OperandDecoration].set(DecorationCeiling, DecorationString, DecorationParams);
    OperandClassParams[OperandDecoration].setOperands(DecorationOperands);
    OperandClassParams[OperandBuiltIn].set(BuiltInCeiling, BuiltInString, BuiltInParams);
    OperandClassParams[OperandSelect].set(SelectControlCeiling, SelectControlString, SelectionControlParams, true);
    OperandClassParams[OperandLoop].set(LoopControlCeiling, LoopControlString, LoopControlParams, true);
    OperandClassParams[OperandFunction].set(FunctionControlCeiling, FunctionControlString, FunctionControlParams, true);
    OperandClassParams[OperandMemorySemantics].set(MemorySemanticsCeiling, MemorySemanticsString, MemorySemanticsParams, true);
    OperandClassParams[OperandMemoryAccess].set(MemoryAccessCeiling, MemoryAccessString, MemoryAccessParams, true);
    OperandClassParams[OperandScope].set(ScopeCeiling, ScopeString, ScopeParams);
    OperandClassParams[OperandGroupOperation].set(GroupOperationCeiling, GroupOperationString, GroupOperationParams);
    OperandClassParams[OperandKernelEnqueueFlags].set(KernelEnqueueFlagsCeiling, KernelEnqueueFlagsString, KernelEnqueueFlagsParams);
    OperandClassParams[OperandKernelProfilingInfo].set(KernelProfilingInfoCeiling, KernelProfilingInfoString, KernelProfilingInfoParams, true);
    OperandClassParams[OperandCapability].set(CapabilityCeiling, CapabilityString, CapabilityParams);
    OperandClassParams[OperandOpcode].set(OpcodeCeiling, OpcodeString, 0);

    CapabilityParams[CapabilityShader].caps.push_back(CapabilityMatrix);
    CapabilityParams[CapabilityGeometry].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityTessellation].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityVector16].caps.push_back(CapabilityKernel);
    CapabilityParams[CapabilityFloat16Buffer].caps.push_back(CapabilityKernel);
    CapabilityParams[CapabilityInt64Atomics].caps.push_back(CapabilityInt64);
    CapabilityParams[CapabilityImageBasic].caps.push_back(CapabilityKernel);
    CapabilityParams[CapabilityImageReadWrite].caps.push_back(CapabilityImageBasic);
    CapabilityParams[CapabilityImageMipmap].caps.push_back(CapabilityImageBasic);
    CapabilityParams[CapabilityPipes].caps.push_back(CapabilityKernel);
    CapabilityParams[CapabilityDeviceEnqueue].caps.push_back(CapabilityKernel);
    CapabilityParams[CapabilityLiteralSampler].caps.push_back(CapabilityKernel);
    CapabilityParams[CapabilityAtomicStorage].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilitySampleRateShading].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityTessellationPointSize].caps.push_back(CapabilityTessellation);
    CapabilityParams[CapabilityGeometryPointSize].caps.push_back(CapabilityGeometry);
    CapabilityParams[CapabilityImageGatherExtended].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityStorageImageExtendedFormats].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityStorageImageMultisample].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityUniformBufferArrayDynamicIndexing].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilitySampledImageArrayDynamicIndexing].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityStorageBufferArrayDynamicIndexing].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityStorageImageArrayDynamicIndexing].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityClipDistance].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityCullDistance].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityGenericPointer].caps.push_back(CapabilityAddresses);
    CapabilityParams[CapabilityInt8].caps.push_back(CapabilityKernel);
    CapabilityParams[CapabilityInputAttachment].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityMinLod].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilitySparseResidency].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilitySampled1D].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilitySampledRect].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilitySampledBuffer].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilitySampledCubeArray].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityImageMSArray].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityImage1D].caps.push_back(CapabilitySampled1D);
    CapabilityParams[CapabilityImageRect].caps.push_back(CapabilitySampledRect);
    CapabilityParams[CapabilityImageBuffer].caps.push_back(CapabilitySampledBuffer);
    CapabilityParams[CapabilityImageCubeArray].caps.push_back(CapabilitySampledCubeArray);
    CapabilityParams[CapabilityImageQuery].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityDerivativeControl].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityInterpolationFunction].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityTransformFeedback].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityGeometryStreams].caps.push_back(CapabilityGeometry);
    CapabilityParams[CapabilityStorageImageReadWithoutFormat].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityStorageImageWriteWithoutFormat].caps.push_back(CapabilityShader);
    CapabilityParams[CapabilityMultiViewport].caps.push_back(CapabilityGeometry);

    AddressingParams[AddressingModelPhysical32].caps.push_back(CapabilityAddresses);
    AddressingParams[AddressingModelPhysical64].caps.push_back(CapabilityAddresses);

    MemoryParams[MemoryModelSimple].caps.push_back(CapabilityShader);
    MemoryParams[MemoryModelGLSL450].caps.push_back(CapabilityShader);
    MemoryParams[MemoryModelOpenCL].caps.push_back(CapabilityKernel);

    MemorySemanticsParams[MemorySemanticsUniformMemoryShift].caps.push_back(CapabilityShader);
    MemorySemanticsParams[MemorySemanticsAtomicCounterMemoryShift].caps.push_back(CapabilityAtomicStorage);

    ExecutionModelParams[ExecutionModelVertex].caps.push_back(CapabilityShader);
    ExecutionModelParams[ExecutionModelTessellationControl].caps.push_back(CapabilityTessellation);
    ExecutionModelParams[ExecutionModelTessellationEvaluation].caps.push_back(CapabilityTessellation);
    ExecutionModelParams[ExecutionModelGeometry].caps.push_back(CapabilityGeometry);
    ExecutionModelParams[ExecutionModelFragment].caps.push_back(CapabilityShader);
    ExecutionModelParams[ExecutionModelGLCompute].caps.push_back(CapabilityShader);
    ExecutionModelParams[ExecutionModelKernel].caps.push_back(CapabilityKernel);

    // Storage capabilites
    StorageParams[StorageClassInput].caps.push_back(CapabilityShader);
    StorageParams[StorageClassUniform].caps.push_back(CapabilityShader);
    StorageParams[StorageClassOutput].caps.push_back(CapabilityShader);
    StorageParams[StorageClassPrivate].caps.push_back(CapabilityShader);
    StorageParams[StorageClassGeneric].caps.push_back(CapabilityKernel);
    StorageParams[StorageClassAtomicCounter].caps.push_back(CapabilityAtomicStorage);
    StorageParams[StorageClassPushConstant].caps.push_back(CapabilityShader);

    // Sampler Filter & Addressing mode capabilities
    SamplerAddressingModeParams[SamplerAddressingModeNone].caps.push_back(CapabilityKernel);
    SamplerAddressingModeParams[SamplerAddressingModeClampToEdge].caps.push_back(CapabilityKernel);
    SamplerAddressingModeParams[SamplerAddressingModeClamp].caps.push_back(CapabilityKernel);
    SamplerAddressingModeParams[SamplerAddressingModeRepeat].caps.push_back(CapabilityKernel);
    SamplerAddressingModeParams[SamplerAddressingModeRepeatMirrored].caps.push_back(CapabilityKernel);

    SamplerFilterModeParams[SamplerFilterModeNearest].caps.push_back(CapabilityKernel);
    SamplerFilterModeParams[SamplerFilterModeLinear].caps.push_back(CapabilityKernel);

    // image format capabilities

    // ES/Desktop float
    ImageFormatParams[ImageFormatRgba32f].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatRgba16f].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatR32f].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatRgba8].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatRgba8Snorm].caps.push_back(CapabilityShader);

    // Desktop float
    ImageFormatParams[ImageFormatRg32f].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg16f].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR11fG11fB10f].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR16f].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRgba16].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRgb10A2].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg16].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg8].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR16].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR8].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRgba16Snorm].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg16Snorm].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg8Snorm].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR16Snorm].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR8Snorm].caps.push_back(CapabilityStorageImageExtendedFormats);

    // ES/Desktop int
    ImageFormatParams[ImageFormatRgba32i].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatRgba16i].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatRgba8i].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatR32i].caps.push_back(CapabilityShader);

    // Desktop int
    ImageFormatParams[ImageFormatRg32i].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg16i].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg8i].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR16i].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR8i].caps.push_back(CapabilityStorageImageExtendedFormats);

    // ES/Desktop uint
    ImageFormatParams[ImageFormatRgba32ui].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatRgba16ui].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatRgba8ui].caps.push_back(CapabilityShader);
    ImageFormatParams[ImageFormatR32ui].caps.push_back(CapabilityShader);

    // Desktop uint
    ImageFormatParams[ImageFormatRgb10a2ui].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg32ui].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg16ui].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatRg8ui].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR16ui].caps.push_back(CapabilityStorageImageExtendedFormats);
    ImageFormatParams[ImageFormatR8ui].caps.push_back(CapabilityStorageImageExtendedFormats);

    // image channel order capabilities
    for (int i = 0; i < ImageChannelOrderCeiling; ++i) {
        ImageChannelOrderParams[i].caps.push_back(CapabilityKernel);
    }

    // image channel type capabilities
    for (int i = 0; i < ImageChannelDataTypeCeiling; ++i) {
        ImageChannelDataTypeParams[i].caps.push_back(CapabilityKernel);
    }

    // image lookup operands
    ImageOperandsParams[ImageOperandsBiasShift].caps.push_back(CapabilityShader);
    ImageOperandsParams[ImageOperandsOffsetShift].caps.push_back(CapabilityImageGatherExtended);
    ImageOperandsParams[ImageOperandsMinLodShift].caps.push_back(CapabilityMinLod);

    // fast math flags capabilities
    for (int i = 0; i < FPFastMathCeiling; ++i) {
        FPFastMathParams[i].caps.push_back(CapabilityKernel);
    }

    // fp rounding mode capabilities
    for (int i = 0; i < FPRoundingModeCeiling; ++i) {
        FPRoundingModeParams[i].caps.push_back(CapabilityKernel);
    }

    // linkage types
    for (int i = 0; i < LinkageTypeCeiling; ++i) {
        LinkageTypeParams[i].caps.push_back(CapabilityLinkage);
    }

    // function argument types
    for (int i = 0; i < FuncParamAttrCeiling; ++i) {
        FuncParamAttrParams[i].caps.push_back(CapabilityKernel);
    }

    // function argument types
    for (int i = 0; i < AccessQualifierCeiling; ++i) {
        AccessQualifierParams[i].caps.push_back(CapabilityKernel);
    }

    ExecutionModeParams[ExecutionModeInvocations].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeSpacingEqual].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModeSpacingFractionalEven].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModeSpacingFractionalOdd].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModeVertexOrderCw].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModeVertexOrderCcw].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModePixelCenterInteger].caps.push_back(CapabilityShader);
    ExecutionModeParams[ExecutionModeOriginUpperLeft].caps.push_back(CapabilityShader);
    ExecutionModeParams[ExecutionModeOriginLowerLeft].caps.push_back(CapabilityShader);
    ExecutionModeParams[ExecutionModeEarlyFragmentTests].caps.push_back(CapabilityShader);
    ExecutionModeParams[ExecutionModePointMode].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModeXfb].caps.push_back(CapabilityTransformFeedback);
    ExecutionModeParams[ExecutionModeDepthReplacing].caps.push_back(CapabilityShader);
    ExecutionModeParams[ExecutionModeDepthGreater].caps.push_back(CapabilityShader);
    ExecutionModeParams[ExecutionModeDepthLess].caps.push_back(CapabilityShader);
    ExecutionModeParams[ExecutionModeDepthUnchanged].caps.push_back(CapabilityShader);
    ExecutionModeParams[ExecutionModeLocalSizeHint].caps.push_back(CapabilityKernel);
    ExecutionModeParams[ExecutionModeInputPoints].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeInputLines].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeInputLinesAdjacency].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeTriangles].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeTriangles].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModeInputTrianglesAdjacency].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeQuads].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModeIsolines].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModeOutputVertices].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeOutputVertices].caps.push_back(CapabilityTessellation);
    ExecutionModeParams[ExecutionModeOutputPoints].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeOutputLineStrip].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeOutputTriangleStrip].caps.push_back(CapabilityGeometry);
    ExecutionModeParams[ExecutionModeVecTypeHint].caps.push_back(CapabilityKernel);
    ExecutionModeParams[ExecutionModeContractionOff].caps.push_back(CapabilityKernel);

    DecorationParams[DecorationRelaxedPrecision].caps.push_back(CapabilityShader);
    DecorationParams[DecorationBlock].caps.push_back(CapabilityShader);
    DecorationParams[DecorationBufferBlock].caps.push_back(CapabilityShader);
    DecorationParams[DecorationRowMajor].caps.push_back(CapabilityMatrix);
    DecorationParams[DecorationColMajor].caps.push_back(CapabilityMatrix);
    DecorationParams[DecorationGLSLShared].caps.push_back(CapabilityShader);
    DecorationParams[DecorationGLSLPacked].caps.push_back(CapabilityShader);
    DecorationParams[DecorationNoPerspective].caps.push_back(CapabilityShader);
    DecorationParams[DecorationFlat].caps.push_back(CapabilityShader);
    DecorationParams[DecorationPatch].caps.push_back(CapabilityTessellation);
    DecorationParams[DecorationCentroid].caps.push_back(CapabilityShader);
    DecorationParams[DecorationSample].caps.push_back(CapabilitySampleRateShading);
    DecorationParams[DecorationInvariant].caps.push_back(CapabilityShader);
    DecorationParams[DecorationConstant].caps.push_back(CapabilityKernel);
    DecorationParams[DecorationUniform].caps.push_back(CapabilityShader);
    DecorationParams[DecorationCPacked].caps.push_back(CapabilityKernel);
    DecorationParams[DecorationSaturatedConversion].caps.push_back(CapabilityKernel);
    DecorationParams[DecorationStream].caps.push_back(CapabilityGeometryStreams);
    DecorationParams[DecorationLocation].caps.push_back(CapabilityShader);
    DecorationParams[DecorationComponent].caps.push_back(CapabilityShader);
    DecorationParams[DecorationOffset].caps.push_back(CapabilityShader);
    DecorationParams[DecorationIndex].caps.push_back(CapabilityShader);
    DecorationParams[DecorationBinding].caps.push_back(CapabilityShader);
    DecorationParams[DecorationDescriptorSet].caps.push_back(CapabilityShader);
    DecorationParams[DecorationXfbBuffer].caps.push_back(CapabilityTransformFeedback);
    DecorationParams[DecorationXfbStride].caps.push_back(CapabilityTransformFeedback);
    DecorationParams[DecorationArrayStride].caps.push_back(CapabilityShader);
    DecorationParams[DecorationMatrixStride].caps.push_back(CapabilityMatrix);
    DecorationParams[DecorationFuncParamAttr].caps.push_back(CapabilityKernel);
    DecorationParams[DecorationFPRoundingMode].caps.push_back(CapabilityKernel);
    DecorationParams[DecorationFPFastMathMode].caps.push_back(CapabilityKernel);
    DecorationParams[DecorationLinkageAttributes].caps.push_back(CapabilityLinkage);
    DecorationParams[DecorationSpecId].caps.push_back(CapabilityShader);
    DecorationParams[DecorationNoContraction].caps.push_back(CapabilityShader);
    DecorationParams[DecorationInputAttachmentIndex].caps.push_back(CapabilityInputAttachment);
    DecorationParams[DecorationAlignment].caps.push_back(CapabilityKernel);

    BuiltInParams[BuiltInPosition].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInPointSize].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInClipDistance].caps.push_back(CapabilityClipDistance);
    BuiltInParams[BuiltInCullDistance].caps.push_back(CapabilityCullDistance);

    BuiltInParams[BuiltInVertexId].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInVertexId].desc = "Vertex ID, which takes on values 0, 1, 2, . . . .";

    BuiltInParams[BuiltInInstanceId].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInInstanceId].desc = "Instance ID, which takes on values 0, 1, 2, . . . .";

    BuiltInParams[BuiltInVertexIndex].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInVertexIndex].desc = "Vertex index, which takes on values base, base+1, base+2, . . . .";

    BuiltInParams[BuiltInInstanceIndex].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInInstanceIndex].desc = "Instance index, which takes on values base, base+1, base+2, . . . .";

    BuiltInParams[BuiltInPrimitiveId].caps.push_back(CapabilityGeometry);
    BuiltInParams[BuiltInPrimitiveId].caps.push_back(CapabilityTessellation);
    BuiltInParams[BuiltInInvocationId].caps.push_back(CapabilityGeometry);
    BuiltInParams[BuiltInInvocationId].caps.push_back(CapabilityTessellation);
    BuiltInParams[BuiltInLayer].caps.push_back(CapabilityGeometry);
    BuiltInParams[BuiltInViewportIndex].caps.push_back(CapabilityMultiViewport);
    BuiltInParams[BuiltInTessLevelOuter].caps.push_back(CapabilityTessellation);
    BuiltInParams[BuiltInTessLevelInner].caps.push_back(CapabilityTessellation);
    BuiltInParams[BuiltInTessCoord].caps.push_back(CapabilityTessellation);
    BuiltInParams[BuiltInPatchVertices].caps.push_back(CapabilityTessellation);
    BuiltInParams[BuiltInFragCoord].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInPointCoord].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInFrontFacing].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInSampleId].caps.push_back(CapabilitySampleRateShading);
    BuiltInParams[BuiltInSamplePosition].caps.push_back(CapabilitySampleRateShading);
    BuiltInParams[BuiltInSampleMask].caps.push_back(CapabilitySampleRateShading);
    BuiltInParams[BuiltInFragDepth].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInHelperInvocation].caps.push_back(CapabilityShader);
    BuiltInParams[BuiltInWorkDim].caps.push_back(CapabilityKernel);
    BuiltInParams[BuiltInGlobalSize].caps.push_back(CapabilityKernel);
    BuiltInParams[BuiltInEnqueuedWorkgroupSize].caps.push_back(CapabilityKernel);
    BuiltInParams[BuiltInGlobalOffset].caps.push_back(CapabilityKernel);
    BuiltInParams[BuiltInGlobalLinearId].caps.push_back(CapabilityKernel);

    BuiltInParams[BuiltInSubgroupSize].caps.push_back(CapabilityKernel);
    BuiltInParams[BuiltInSubgroupMaxSize].caps.push_back(CapabilityKernel);
    BuiltInParams[BuiltInNumSubgroups].caps.push_back(CapabilityKernel);
    BuiltInParams[BuiltInNumEnqueuedSubgroups].caps.push_back(CapabilityKernel);
    BuiltInParams[BuiltInSubgroupId].caps.push_back(CapabilityKernel);
    BuiltInParams[BuiltInSubgroupLocalInvocationId].caps.push_back(CapabilityKernel);

    DimensionalityParams[Dim1D].caps.push_back(CapabilitySampled1D);
    DimensionalityParams[DimCube].caps.push_back(CapabilityShader);
    DimensionalityParams[DimRect].caps.push_back(CapabilitySampledRect);
    DimensionalityParams[DimBuffer].caps.push_back(CapabilitySampledBuffer);
    DimensionalityParams[DimSubpassData].caps.push_back(CapabilityInputAttachment);

    // Group Operations
    for (int i = 0; i < GroupOperationCeiling; ++i) {
        GroupOperationParams[i].caps.push_back(CapabilityKernel);
    }

    // Enqueue flags
    for (int i = 0; i < KernelEnqueueFlagsCeiling; ++i) {
        KernelEnqueueFlagsParams[i].caps.push_back(CapabilityKernel);
    }

    // Profiling info
    KernelProfilingInfoParams[0].caps.push_back(CapabilityKernel);

    // set name of operator, an initial set of <id> style operands, and the description

    InstructionDesc[OpSource].operands.push(OperandSource, "");
    InstructionDesc[OpSource].operands.push(OperandLiteralNumber, "'Version'");
    InstructionDesc[OpSource].operands.push(OperandId, "'File'", true);
    InstructionDesc[OpSource].operands.push(OperandLiteralString, "'Source'", true);

    InstructionDesc[OpSourceContinued].operands.push(OperandLiteralString, "'Continued Source'");

    InstructionDesc[OpSourceExtension].operands.push(OperandLiteralString, "'Extension'");

    InstructionDesc[OpName].operands.push(OperandId, "'Target'");
    InstructionDesc[OpName].operands.push(OperandLiteralString, "'Name'");

    InstructionDesc[OpMemberName].operands.push(OperandId, "'Type'");
    InstructionDesc[OpMemberName].operands.push(OperandLiteralNumber, "'Member'");
    InstructionDesc[OpMemberName].operands.push(OperandLiteralString, "'Name'");

    InstructionDesc[OpString].operands.push(OperandLiteralString, "'String'");

    InstructionDesc[OpLine].operands.push(OperandId, "'File'");
    InstructionDesc[OpLine].operands.push(OperandLiteralNumber, "'Line'");
    InstructionDesc[OpLine].operands.push(OperandLiteralNumber, "'Column'");

    InstructionDesc[OpExtension].operands.push(OperandLiteralString, "'Name'");

    InstructionDesc[OpExtInstImport].operands.push(OperandLiteralString, "'Name'");

    InstructionDesc[OpCapability].operands.push(OperandCapability, "'Capability'");

    InstructionDesc[OpMemoryModel].operands.push(OperandAddressing, "");
    InstructionDesc[OpMemoryModel].operands.push(OperandMemory, "");

    InstructionDesc[OpEntryPoint].operands.push(OperandExecutionModel, "");
    InstructionDesc[OpEntryPoint].operands.push(OperandId, "'Entry Point'");
    InstructionDesc[OpEntryPoint].operands.push(OperandLiteralString, "'Name'");
    InstructionDesc[OpEntryPoint].operands.push(OperandVariableIds, "'Interface'");

    InstructionDesc[OpExecutionMode].operands.push(OperandId, "'Entry Point'");
    InstructionDesc[OpExecutionMode].operands.push(OperandExecutionMode, "'Mode'");
    InstructionDesc[OpExecutionMode].operands.push(OperandOptionalLiteral, "See <<Execution_Mode,Execution Mode>>");

    InstructionDesc[OpTypeInt].operands.push(OperandLiteralNumber, "'Width'");
    InstructionDesc[OpTypeInt].operands.push(OperandLiteralNumber, "'Signedness'");

    InstructionDesc[OpTypeFloat].operands.push(OperandLiteralNumber, "'Width'");

    InstructionDesc[OpTypeVector].operands.push(OperandId, "'Component Type'");
    InstructionDesc[OpTypeVector].operands.push(OperandLiteralNumber, "'Component Count'");

    InstructionDesc[OpTypeMatrix].capabilities.push_back(CapabilityMatrix);
    InstructionDesc[OpTypeMatrix].operands.push(OperandId, "'Column Type'");
    InstructionDesc[OpTypeMatrix].operands.push(OperandLiteralNumber, "'Column Count'");

    InstructionDesc[OpTypeImage].operands.push(OperandId, "'Sampled Type'");
    InstructionDesc[OpTypeImage].operands.push(OperandDimensionality, "");
    InstructionDesc[OpTypeImage].operands.push(OperandLiteralNumber, "'Depth'");
    InstructionDesc[OpTypeImage].operands.push(OperandLiteralNumber, "'Arrayed'");
    InstructionDesc[OpTypeImage].operands.push(OperandLiteralNumber, "'MS'");
    InstructionDesc[OpTypeImage].operands.push(OperandLiteralNumber, "'Sampled'");
    InstructionDesc[OpTypeImage].operands.push(OperandSamplerImageFormat, "");
    InstructionDesc[OpTypeImage].operands.push(OperandAccessQualifier, "", true);

    InstructionDesc[OpTypeSampledImage].operands.push(OperandId, "'Image Type'");

    InstructionDesc[OpTypeArray].operands.push(OperandId, "'Element Type'");
    InstructionDesc[OpTypeArray].operands.push(OperandId, "'Length'");

    InstructionDesc[OpTypeRuntimeArray].capabilities.push_back(CapabilityShader);
    InstructionDesc[OpTypeRuntimeArray].operands.push(OperandId, "'Element Type'");

    InstructionDesc[OpTypeStruct].operands.push(OperandVariableIds, "'Member 0 type', +\n'member 1 type', +\n...");

    InstructionDesc[OpTypeOpaque].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpTypeOpaque].operands.push(OperandLiteralString, "The name of the opaque type.");

    InstructionDesc[OpTypePointer].operands.push(OperandStorage, "");
    InstructionDesc[OpTypePointer].operands.push(OperandId, "'Type'");

    InstructionDesc[OpTypeForwardPointer].capabilities.push_back(CapabilityAddresses);
    InstructionDesc[OpTypeForwardPointer].operands.push(OperandId, "'Pointer Type'");
    InstructionDesc[OpTypeForwardPointer].operands.push(OperandStorage, "");

    InstructionDesc[OpTypeEvent].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpTypeDeviceEvent].capabilities.push_back(CapabilityDeviceEnqueue);

    InstructionDesc[OpTypeReserveId].capabilities.push_back(CapabilityPipes);

    InstructionDesc[OpTypeQueue].capabilities.push_back(CapabilityDeviceEnqueue);

    InstructionDesc[OpTypePipe].operands.push(OperandAccessQualifier, "'Qualifier'");
    InstructionDesc[OpTypePipe].capabilities.push_back(CapabilityPipes);

    InstructionDesc[OpTypeFunction].operands.push(OperandId, "'Return Type'");
    InstructionDesc[OpTypeFunction].operands.push(OperandVariableIds, "'Parameter 0 Type', +\n'Parameter 1 Type', +\n...");

    InstructionDesc[OpConstant].operands.push(OperandVariableLiterals, "'Value'");

    InstructionDesc[OpConstantComposite].operands.push(OperandVariableIds, "'Constituents'");

    InstructionDesc[OpConstantSampler].capabilities.push_back(CapabilityLiteralSampler);
    InstructionDesc[OpConstantSampler].operands.push(OperandSamplerAddressingMode, "");
    InstructionDesc[OpConstantSampler].operands.push(OperandLiteralNumber, "'Param'");
    InstructionDesc[OpConstantSampler].operands.push(OperandSamplerFilterMode, "");

    InstructionDesc[OpSpecConstant].operands.push(OperandVariableLiterals, "'Value'");

    InstructionDesc[OpSpecConstantComposite].operands.push(OperandVariableIds, "'Constituents'");

    InstructionDesc[OpSpecConstantOp].operands.push(OperandLiteralNumber, "'Opcode'");
    InstructionDesc[OpSpecConstantOp].operands.push(OperandVariableIds, "'Operands'");

    InstructionDesc[OpVariable].operands.push(OperandStorage, "");
    InstructionDesc[OpVariable].operands.push(OperandId, "'Initializer'", true);

    InstructionDesc[OpFunction].operands.push(OperandFunction, "");
    InstructionDesc[OpFunction].operands.push(OperandId, "'Function Type'");

    InstructionDesc[OpFunctionCall].operands.push(OperandId, "'Function'");
    InstructionDesc[OpFunctionCall].operands.push(OperandVariableIds, "'Argument 0', +\n'Argument 1', +\n...");

    InstructionDesc[OpExtInst].operands.push(OperandId, "'Set'");
    InstructionDesc[OpExtInst].operands.push(OperandLiteralNumber, "'Instruction'");
    InstructionDesc[OpExtInst].operands.push(OperandVariableIds, "'Operand 1', +\n'Operand 2', +\n...");

    InstructionDesc[OpLoad].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpLoad].operands.push(OperandMemoryAccess, "", true);

    InstructionDesc[OpStore].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpStore].operands.push(OperandId, "'Object'");
    InstructionDesc[OpStore].operands.push(OperandMemoryAccess, "", true);

    InstructionDesc[OpPhi].operands.push(OperandVariableIds, "'Variable, Parent, ...'");

    InstructionDesc[OpDecorate].operands.push(OperandId, "'Target'");
    InstructionDesc[OpDecorate].operands.push(OperandDecoration, "");
    InstructionDesc[OpDecorate].operands.push(OperandVariableLiterals, "See <<Decoration,'Decoration'>>.");

    InstructionDesc[OpMemberDecorate].operands.push(OperandId, "'Structure Type'");
    InstructionDesc[OpMemberDecorate].operands.push(OperandLiteralNumber, "'Member'");
    InstructionDesc[OpMemberDecorate].operands.push(OperandDecoration, "");
    InstructionDesc[OpMemberDecorate].operands.push(OperandVariableLiterals, "See <<Decoration,'Decoration'>>.");

    InstructionDesc[OpGroupDecorate].operands.push(OperandId, "'Decoration Group'");
    InstructionDesc[OpGroupDecorate].operands.push(OperandVariableIds, "'Targets'");

    InstructionDesc[OpGroupMemberDecorate].operands.push(OperandId, "'Decoration Group'");
    InstructionDesc[OpGroupMemberDecorate].operands.push(OperandVariableIdLiteral, "'Targets'");

    InstructionDesc[OpVectorExtractDynamic].operands.push(OperandId, "'Vector'");
    InstructionDesc[OpVectorExtractDynamic].operands.push(OperandId, "'Index'");

    InstructionDesc[OpVectorInsertDynamic].operands.push(OperandId, "'Vector'");
    InstructionDesc[OpVectorInsertDynamic].operands.push(OperandId, "'Component'");
    InstructionDesc[OpVectorInsertDynamic].operands.push(OperandId, "'Index'");

    InstructionDesc[OpVectorShuffle].operands.push(OperandId, "'Vector 1'");
    InstructionDesc[OpVectorShuffle].operands.push(OperandId, "'Vector 2'");
    InstructionDesc[OpVectorShuffle].operands.push(OperandVariableLiterals, "'Components'");

    InstructionDesc[OpCompositeConstruct].operands.push(OperandVariableIds, "'Constituents'");

    InstructionDesc[OpCompositeExtract].operands.push(OperandId, "'Composite'");
    InstructionDesc[OpCompositeExtract].operands.push(OperandVariableLiterals, "'Indexes'");

    InstructionDesc[OpCompositeInsert].operands.push(OperandId, "'Object'");
    InstructionDesc[OpCompositeInsert].operands.push(OperandId, "'Composite'");
    InstructionDesc[OpCompositeInsert].operands.push(OperandVariableLiterals, "'Indexes'");

    InstructionDesc[OpCopyObject].operands.push(OperandId, "'Operand'");

    InstructionDesc[OpCopyMemory].operands.push(OperandId, "'Target'");
    InstructionDesc[OpCopyMemory].operands.push(OperandId, "'Source'");
    InstructionDesc[OpCopyMemory].operands.push(OperandMemoryAccess, "", true);

    InstructionDesc[OpCopyMemorySized].operands.push(OperandId, "'Target'");
    InstructionDesc[OpCopyMemorySized].operands.push(OperandId, "'Source'");
    InstructionDesc[OpCopyMemorySized].operands.push(OperandId, "'Size'");
    InstructionDesc[OpCopyMemorySized].operands.push(OperandMemoryAccess, "", true);

    InstructionDesc[OpCopyMemorySized].capabilities.push_back(CapabilityAddresses);

    InstructionDesc[OpSampledImage].operands.push(OperandId, "'Image'");
    InstructionDesc[OpSampledImage].operands.push(OperandId, "'Sampler'");

    InstructionDesc[OpImage].operands.push(OperandId, "'Sampled Image'");

    InstructionDesc[OpImageRead].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageRead].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageRead].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageRead].operands.push(OperandVariableIds, "", true);

    InstructionDesc[OpImageWrite].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageWrite].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageWrite].operands.push(OperandId, "'Texel'");
    InstructionDesc[OpImageWrite].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageWrite].operands.push(OperandVariableIds, "", true);

    InstructionDesc[OpImageSampleImplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSampleImplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSampleImplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSampleImplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSampleImplicitLod].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpImageSampleExplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSampleExplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSampleExplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSampleExplicitLod].operands.push(OperandVariableIds, "", true);

    InstructionDesc[OpImageSampleDrefImplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSampleDrefImplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSampleDrefImplicitLod].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageSampleDrefImplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSampleDrefImplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSampleDrefImplicitLod].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpImageSampleDrefExplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSampleDrefExplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSampleDrefExplicitLod].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageSampleDrefExplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSampleDrefExplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSampleDrefExplicitLod].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpImageSampleProjImplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSampleProjImplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSampleProjImplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSampleProjImplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSampleProjImplicitLod].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpImageSampleProjExplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSampleProjExplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSampleProjExplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSampleProjExplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSampleProjExplicitLod].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpImageSampleProjDrefImplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSampleProjDrefImplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSampleProjDrefImplicitLod].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageSampleProjDrefImplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSampleProjDrefImplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSampleProjDrefImplicitLod].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpImageSampleProjDrefExplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSampleProjDrefExplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSampleProjDrefExplicitLod].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageSampleProjDrefExplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSampleProjDrefExplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSampleProjDrefExplicitLod].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpImageFetch].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageFetch].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageFetch].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageFetch].operands.push(OperandVariableIds, "", true);

    InstructionDesc[OpImageGather].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageGather].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageGather].operands.push(OperandId, "'Component'");
    InstructionDesc[OpImageGather].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageGather].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageGather].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpImageDrefGather].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageDrefGather].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageDrefGather].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageDrefGather].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageDrefGather].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageDrefGather].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpImageSparseSampleImplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseSampleImplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseSampleImplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseSampleImplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseSampleImplicitLod].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseSampleExplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseSampleExplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseSampleExplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseSampleExplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseSampleExplicitLod].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseSampleDrefImplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseSampleDrefImplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseSampleDrefImplicitLod].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageSparseSampleDrefImplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseSampleDrefImplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseSampleDrefImplicitLod].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseSampleDrefExplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseSampleDrefExplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseSampleDrefExplicitLod].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageSparseSampleDrefExplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseSampleDrefExplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseSampleDrefExplicitLod].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseSampleProjImplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseSampleProjImplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseSampleProjImplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseSampleProjImplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseSampleProjImplicitLod].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseSampleProjExplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseSampleProjExplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseSampleProjExplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseSampleProjExplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseSampleProjExplicitLod].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseSampleProjDrefImplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseSampleProjDrefImplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseSampleProjDrefImplicitLod].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageSparseSampleProjDrefImplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseSampleProjDrefImplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseSampleProjDrefImplicitLod].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseSampleProjDrefExplicitLod].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseSampleProjDrefExplicitLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseSampleProjDrefExplicitLod].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageSparseSampleProjDrefExplicitLod].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseSampleProjDrefExplicitLod].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseSampleProjDrefExplicitLod].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseFetch].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageSparseFetch].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseFetch].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseFetch].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseFetch].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseGather].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseGather].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseGather].operands.push(OperandId, "'Component'");
    InstructionDesc[OpImageSparseGather].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseGather].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseGather].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseDrefGather].operands.push(OperandId, "'Sampled Image'");
    InstructionDesc[OpImageSparseDrefGather].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseDrefGather].operands.push(OperandId, "'D~ref~'");
    InstructionDesc[OpImageSparseDrefGather].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseDrefGather].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseDrefGather].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseRead].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageSparseRead].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageSparseRead].operands.push(OperandImageOperands, "", true);
    InstructionDesc[OpImageSparseRead].operands.push(OperandVariableIds, "", true);
    InstructionDesc[OpImageSparseRead].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageSparseTexelsResident].operands.push(OperandId, "'Resident Code'");
    InstructionDesc[OpImageSparseTexelsResident].capabilities.push_back(CapabilitySparseResidency);

    InstructionDesc[OpImageQuerySizeLod].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageQuerySizeLod].operands.push(OperandId, "'Level of Detail'");
    InstructionDesc[OpImageQuerySizeLod].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpImageQuerySizeLod].capabilities.push_back(CapabilityImageQuery);

    InstructionDesc[OpImageQuerySize].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageQuerySize].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpImageQuerySize].capabilities.push_back(CapabilityImageQuery);

    InstructionDesc[OpImageQueryLod].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageQueryLod].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageQueryLod].capabilities.push_back(CapabilityImageQuery);

    InstructionDesc[OpImageQueryLevels].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageQueryLevels].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpImageQueryLevels].capabilities.push_back(CapabilityImageQuery);

    InstructionDesc[OpImageQuerySamples].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageQuerySamples].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpImageQuerySamples].capabilities.push_back(CapabilityImageQuery);

    InstructionDesc[OpImageQueryFormat].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageQueryFormat].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpImageQueryOrder].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageQueryOrder].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpAccessChain].operands.push(OperandId, "'Base'");
    InstructionDesc[OpAccessChain].operands.push(OperandVariableIds, "'Indexes'");

    InstructionDesc[OpInBoundsAccessChain].operands.push(OperandId, "'Base'");
    InstructionDesc[OpInBoundsAccessChain].operands.push(OperandVariableIds, "'Indexes'");

    InstructionDesc[OpPtrAccessChain].operands.push(OperandId, "'Base'");
    InstructionDesc[OpPtrAccessChain].operands.push(OperandId, "'Element'");
    InstructionDesc[OpPtrAccessChain].operands.push(OperandVariableIds, "'Indexes'");
    InstructionDesc[OpPtrAccessChain].capabilities.push_back(CapabilityAddresses);

    InstructionDesc[OpInBoundsPtrAccessChain].operands.push(OperandId, "'Base'");
    InstructionDesc[OpInBoundsPtrAccessChain].operands.push(OperandId, "'Element'");
    InstructionDesc[OpInBoundsPtrAccessChain].operands.push(OperandVariableIds, "'Indexes'");
    InstructionDesc[OpInBoundsPtrAccessChain].capabilities.push_back(CapabilityAddresses);

    InstructionDesc[OpSNegate].operands.push(OperandId, "'Operand'");

    InstructionDesc[OpFNegate].operands.push(OperandId, "'Operand'");

    InstructionDesc[OpNot].operands.push(OperandId, "'Operand'");

    InstructionDesc[OpAny].operands.push(OperandId, "'Vector'");

    InstructionDesc[OpAll].operands.push(OperandId, "'Vector'");

    InstructionDesc[OpConvertFToU].operands.push(OperandId, "'Float Value'");

    InstructionDesc[OpConvertFToS].operands.push(OperandId, "'Float Value'");

    InstructionDesc[OpConvertSToF].operands.push(OperandId, "'Signed Value'");

    InstructionDesc[OpConvertUToF].operands.push(OperandId, "'Unsigned Value'");

    InstructionDesc[OpUConvert].operands.push(OperandId, "'Unsigned Value'");

    InstructionDesc[OpSConvert].operands.push(OperandId, "'Signed Value'");

    InstructionDesc[OpFConvert].operands.push(OperandId, "'Float Value'");

    InstructionDesc[OpSatConvertSToU].operands.push(OperandId, "'Signed Value'");
    InstructionDesc[OpSatConvertSToU].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpSatConvertUToS].operands.push(OperandId, "'Unsigned Value'");
    InstructionDesc[OpSatConvertUToS].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpConvertPtrToU].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpConvertPtrToU].capabilities.push_back(CapabilityAddresses);

    InstructionDesc[OpConvertUToPtr].operands.push(OperandId, "'Integer Value'");
    InstructionDesc[OpConvertUToPtr].capabilities.push_back(CapabilityAddresses);

    InstructionDesc[OpPtrCastToGeneric].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpPtrCastToGeneric].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpGenericCastToPtr].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpGenericCastToPtr].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpGenericCastToPtrExplicit].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpGenericCastToPtrExplicit].operands.push(OperandStorage, "'Storage'");
    InstructionDesc[OpGenericCastToPtrExplicit].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpGenericPtrMemSemantics].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpGenericPtrMemSemantics].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpBitcast].operands.push(OperandId, "'Operand'");

    InstructionDesc[OpQuantizeToF16].operands.push(OperandId, "'Value'");

    InstructionDesc[OpTranspose].capabilities.push_back(CapabilityMatrix);
    InstructionDesc[OpTranspose].operands.push(OperandId, "'Matrix'");

    InstructionDesc[OpIsNan].operands.push(OperandId, "'x'");

    InstructionDesc[OpIsInf].operands.push(OperandId, "'x'");

    InstructionDesc[OpIsFinite].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpIsFinite].operands.push(OperandId, "'x'");

    InstructionDesc[OpIsNormal].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpIsNormal].operands.push(OperandId, "'x'");

    InstructionDesc[OpSignBitSet].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpSignBitSet].operands.push(OperandId, "'x'");

    InstructionDesc[OpLessOrGreater].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpLessOrGreater].operands.push(OperandId, "'x'");
    InstructionDesc[OpLessOrGreater].operands.push(OperandId, "'y'");

    InstructionDesc[OpOrdered].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpOrdered].operands.push(OperandId, "'x'");
    InstructionDesc[OpOrdered].operands.push(OperandId, "'y'");

    InstructionDesc[OpUnordered].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpUnordered].operands.push(OperandId, "'x'");
    InstructionDesc[OpUnordered].operands.push(OperandId, "'y'");

    InstructionDesc[OpArrayLength].operands.push(OperandId, "'Structure'");
    InstructionDesc[OpArrayLength].operands.push(OperandLiteralNumber, "'Array member'");
    InstructionDesc[OpArrayLength].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpIAdd].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpIAdd].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFAdd].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFAdd].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpISub].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpISub].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFSub].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFSub].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpIMul].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpIMul].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFMul].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFMul].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpUDiv].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpUDiv].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpSDiv].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpSDiv].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFDiv].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFDiv].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpUMod].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpUMod].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpSRem].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpSRem].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpSMod].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpSMod].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFRem].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFRem].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFMod].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFMod].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpVectorTimesScalar].operands.push(OperandId, "'Vector'");
    InstructionDesc[OpVectorTimesScalar].operands.push(OperandId, "'Scalar'");

    InstructionDesc[OpMatrixTimesScalar].capabilities.push_back(CapabilityMatrix);
    InstructionDesc[OpMatrixTimesScalar].operands.push(OperandId, "'Matrix'");
    InstructionDesc[OpMatrixTimesScalar].operands.push(OperandId, "'Scalar'");

    InstructionDesc[OpVectorTimesMatrix].capabilities.push_back(CapabilityMatrix);
    InstructionDesc[OpVectorTimesMatrix].operands.push(OperandId, "'Vector'");
    InstructionDesc[OpVectorTimesMatrix].operands.push(OperandId, "'Matrix'");

    InstructionDesc[OpMatrixTimesVector].capabilities.push_back(CapabilityMatrix);
    InstructionDesc[OpMatrixTimesVector].operands.push(OperandId, "'Matrix'");
    InstructionDesc[OpMatrixTimesVector].operands.push(OperandId, "'Vector'");

    InstructionDesc[OpMatrixTimesMatrix].capabilities.push_back(CapabilityMatrix);
    InstructionDesc[OpMatrixTimesMatrix].operands.push(OperandId, "'LeftMatrix'");
    InstructionDesc[OpMatrixTimesMatrix].operands.push(OperandId, "'RightMatrix'");

    InstructionDesc[OpOuterProduct].capabilities.push_back(CapabilityMatrix);
    InstructionDesc[OpOuterProduct].operands.push(OperandId, "'Vector 1'");
    InstructionDesc[OpOuterProduct].operands.push(OperandId, "'Vector 2'");

    InstructionDesc[OpDot].operands.push(OperandId, "'Vector 1'");
    InstructionDesc[OpDot].operands.push(OperandId, "'Vector 2'");

    InstructionDesc[OpIAddCarry].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpIAddCarry].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpISubBorrow].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpISubBorrow].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpUMulExtended].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpUMulExtended].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpSMulExtended].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpSMulExtended].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpShiftRightLogical].operands.push(OperandId, "'Base'");
    InstructionDesc[OpShiftRightLogical].operands.push(OperandId, "'Shift'");

    InstructionDesc[OpShiftRightArithmetic].operands.push(OperandId, "'Base'");
    InstructionDesc[OpShiftRightArithmetic].operands.push(OperandId, "'Shift'");

    InstructionDesc[OpShiftLeftLogical].operands.push(OperandId, "'Base'");
    InstructionDesc[OpShiftLeftLogical].operands.push(OperandId, "'Shift'");

    InstructionDesc[OpLogicalOr].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpLogicalOr].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpLogicalAnd].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpLogicalAnd].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpLogicalEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpLogicalEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpLogicalNotEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpLogicalNotEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpLogicalNot].operands.push(OperandId, "'Operand'");

    InstructionDesc[OpBitwiseOr].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpBitwiseOr].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpBitwiseXor].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpBitwiseXor].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpBitwiseAnd].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpBitwiseAnd].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpBitFieldInsert].capabilities.push_back(CapabilityShader);
    InstructionDesc[OpBitFieldInsert].operands.push(OperandId, "'Base'");
    InstructionDesc[OpBitFieldInsert].operands.push(OperandId, "'Insert'");
    InstructionDesc[OpBitFieldInsert].operands.push(OperandId, "'Offset'");
    InstructionDesc[OpBitFieldInsert].operands.push(OperandId, "'Count'");
    
    InstructionDesc[OpBitFieldSExtract].capabilities.push_back(CapabilityShader);
    InstructionDesc[OpBitFieldSExtract].operands.push(OperandId, "'Base'");
    InstructionDesc[OpBitFieldSExtract].operands.push(OperandId, "'Offset'");
    InstructionDesc[OpBitFieldSExtract].operands.push(OperandId, "'Count'");
    
    InstructionDesc[OpBitFieldUExtract].capabilities.push_back(CapabilityShader);
    InstructionDesc[OpBitFieldUExtract].operands.push(OperandId, "'Base'");
    InstructionDesc[OpBitFieldUExtract].operands.push(OperandId, "'Offset'");
    InstructionDesc[OpBitFieldUExtract].operands.push(OperandId, "'Count'");
    
    InstructionDesc[OpBitReverse].capabilities.push_back(CapabilityShader);
    InstructionDesc[OpBitReverse].operands.push(OperandId, "'Base'");

    InstructionDesc[OpBitCount].operands.push(OperandId, "'Base'");

    InstructionDesc[OpSelect].operands.push(OperandId, "'Condition'");
    InstructionDesc[OpSelect].operands.push(OperandId, "'Object 1'");
    InstructionDesc[OpSelect].operands.push(OperandId, "'Object 2'");

    InstructionDesc[OpIEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpIEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFOrdEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFOrdEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFUnordEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFUnordEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpINotEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpINotEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFOrdNotEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFOrdNotEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFUnordNotEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFUnordNotEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpULessThan].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpULessThan].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpSLessThan].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpSLessThan].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFOrdLessThan].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFOrdLessThan].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFUnordLessThan].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFUnordLessThan].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpUGreaterThan].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpUGreaterThan].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpSGreaterThan].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpSGreaterThan].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFOrdGreaterThan].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFOrdGreaterThan].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFUnordGreaterThan].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFUnordGreaterThan].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpULessThanEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpULessThanEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpSLessThanEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpSLessThanEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFOrdLessThanEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFOrdLessThanEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFUnordLessThanEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFUnordLessThanEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpUGreaterThanEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpUGreaterThanEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpSGreaterThanEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpSGreaterThanEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFOrdGreaterThanEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFOrdGreaterThanEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpFUnordGreaterThanEqual].operands.push(OperandId, "'Operand 1'");
    InstructionDesc[OpFUnordGreaterThanEqual].operands.push(OperandId, "'Operand 2'");

    InstructionDesc[OpDPdx].capabilities.push_back(CapabilityShader);
    InstructionDesc[OpDPdx].operands.push(OperandId, "'P'");

    InstructionDesc[OpDPdy].capabilities.push_back(CapabilityShader);
    InstructionDesc[OpDPdy].operands.push(OperandId, "'P'");

    InstructionDesc[OpFwidth].capabilities.push_back(CapabilityShader);
    InstructionDesc[OpFwidth].operands.push(OperandId, "'P'");

    InstructionDesc[OpDPdxFine].capabilities.push_back(CapabilityDerivativeControl);
    InstructionDesc[OpDPdxFine].operands.push(OperandId, "'P'");

    InstructionDesc[OpDPdyFine].capabilities.push_back(CapabilityDerivativeControl);
    InstructionDesc[OpDPdyFine].operands.push(OperandId, "'P'");

    InstructionDesc[OpFwidthFine].capabilities.push_back(CapabilityDerivativeControl);
    InstructionDesc[OpFwidthFine].operands.push(OperandId, "'P'");

    InstructionDesc[OpDPdxCoarse].capabilities.push_back(CapabilityDerivativeControl);
    InstructionDesc[OpDPdxCoarse].operands.push(OperandId, "'P'");

    InstructionDesc[OpDPdyCoarse].capabilities.push_back(CapabilityDerivativeControl);
    InstructionDesc[OpDPdyCoarse].operands.push(OperandId, "'P'");

    InstructionDesc[OpFwidthCoarse].capabilities.push_back(CapabilityDerivativeControl);
    InstructionDesc[OpFwidthCoarse].operands.push(OperandId, "'P'");

    InstructionDesc[OpEmitVertex].capabilities.push_back(CapabilityGeometry);

    InstructionDesc[OpEndPrimitive].capabilities.push_back(CapabilityGeometry);

    InstructionDesc[OpEmitStreamVertex].operands.push(OperandId, "'Stream'");
    InstructionDesc[OpEmitStreamVertex].capabilities.push_back(CapabilityGeometryStreams);

    InstructionDesc[OpEndStreamPrimitive].operands.push(OperandId, "'Stream'");
    InstructionDesc[OpEndStreamPrimitive].capabilities.push_back(CapabilityGeometryStreams);

    InstructionDesc[OpControlBarrier].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpControlBarrier].operands.push(OperandScope, "'Memory'");
    InstructionDesc[OpControlBarrier].operands.push(OperandMemorySemantics, "'Semantics'");

    InstructionDesc[OpMemoryBarrier].operands.push(OperandScope, "'Memory'");
    InstructionDesc[OpMemoryBarrier].operands.push(OperandMemorySemantics, "'Semantics'");

    InstructionDesc[OpImageTexelPointer].operands.push(OperandId, "'Image'");
    InstructionDesc[OpImageTexelPointer].operands.push(OperandId, "'Coordinate'");
    InstructionDesc[OpImageTexelPointer].operands.push(OperandId, "'Sample'");

    InstructionDesc[OpAtomicLoad].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicLoad].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicLoad].operands.push(OperandMemorySemantics, "'Semantics'");

    InstructionDesc[OpAtomicStore].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicStore].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicStore].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicStore].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicExchange].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicExchange].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicExchange].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicExchange].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicCompareExchange].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicCompareExchange].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicCompareExchange].operands.push(OperandMemorySemantics, "'Equal'");
    InstructionDesc[OpAtomicCompareExchange].operands.push(OperandMemorySemantics, "'Unequal'");
    InstructionDesc[OpAtomicCompareExchange].operands.push(OperandId, "'Value'");
    InstructionDesc[OpAtomicCompareExchange].operands.push(OperandId, "'Comparator'");

    InstructionDesc[OpAtomicCompareExchangeWeak].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicCompareExchangeWeak].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicCompareExchangeWeak].operands.push(OperandMemorySemantics, "'Equal'");
    InstructionDesc[OpAtomicCompareExchangeWeak].operands.push(OperandMemorySemantics, "'Unequal'");
    InstructionDesc[OpAtomicCompareExchangeWeak].operands.push(OperandId, "'Value'");
    InstructionDesc[OpAtomicCompareExchangeWeak].operands.push(OperandId, "'Comparator'");
    InstructionDesc[OpAtomicCompareExchangeWeak].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpAtomicIIncrement].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicIIncrement].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicIIncrement].operands.push(OperandMemorySemantics, "'Semantics'");

    InstructionDesc[OpAtomicIDecrement].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicIDecrement].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicIDecrement].operands.push(OperandMemorySemantics, "'Semantics'");

    InstructionDesc[OpAtomicIAdd].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicIAdd].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicIAdd].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicIAdd].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicISub].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicISub].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicISub].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicISub].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicUMin].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicUMin].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicUMin].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicUMin].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicUMax].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicUMax].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicUMax].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicUMax].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicSMin].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicSMin].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicSMin].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicSMin].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicSMax].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicSMax].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicSMax].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicSMax].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicAnd].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicAnd].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicAnd].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicAnd].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicOr].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicOr].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicOr].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicOr].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicXor].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicXor].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicXor].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicXor].operands.push(OperandId, "'Value'");

    InstructionDesc[OpAtomicFlagTestAndSet].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicFlagTestAndSet].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicFlagTestAndSet].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicFlagTestAndSet].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpAtomicFlagClear].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpAtomicFlagClear].operands.push(OperandScope, "'Scope'");
    InstructionDesc[OpAtomicFlagClear].operands.push(OperandMemorySemantics, "'Semantics'");
    InstructionDesc[OpAtomicFlagClear].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpLoopMerge].operands.push(OperandId, "'Merge Block'");
    InstructionDesc[OpLoopMerge].operands.push(OperandId, "'Continue Target'");
    InstructionDesc[OpLoopMerge].operands.push(OperandLoop, "");

    InstructionDesc[OpSelectionMerge].operands.push(OperandId, "'Merge Block'");
    InstructionDesc[OpSelectionMerge].operands.push(OperandSelect, "");

    InstructionDesc[OpBranch].operands.push(OperandId, "'Target Label'");

    InstructionDesc[OpBranchConditional].operands.push(OperandId, "'Condition'");
    InstructionDesc[OpBranchConditional].operands.push(OperandId, "'True Label'");
    InstructionDesc[OpBranchConditional].operands.push(OperandId, "'False Label'");
    InstructionDesc[OpBranchConditional].operands.push(OperandVariableLiterals, "'Branch weights'");

    InstructionDesc[OpSwitch].operands.push(OperandId, "'Selector'");
    InstructionDesc[OpSwitch].operands.push(OperandId, "'Default'");
    InstructionDesc[OpSwitch].operands.push(OperandVariableLiteralId, "'Target'");

    InstructionDesc[OpKill].capabilities.push_back(CapabilityShader);

    InstructionDesc[OpReturnValue].operands.push(OperandId, "'Value'");

    InstructionDesc[OpLifetimeStart].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpLifetimeStart].operands.push(OperandLiteralNumber, "'Size'");
    InstructionDesc[OpLifetimeStart].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpLifetimeStop].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpLifetimeStop].operands.push(OperandLiteralNumber, "'Size'");
    InstructionDesc[OpLifetimeStop].capabilities.push_back(CapabilityKernel);

    InstructionDesc[OpGroupAsyncCopy].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpGroupAsyncCopy].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupAsyncCopy].operands.push(OperandId, "'Destination'");
    InstructionDesc[OpGroupAsyncCopy].operands.push(OperandId, "'Source'");
    InstructionDesc[OpGroupAsyncCopy].operands.push(OperandId, "'Num Elements'");
    InstructionDesc[OpGroupAsyncCopy].operands.push(OperandId, "'Stride'");
    InstructionDesc[OpGroupAsyncCopy].operands.push(OperandId, "'Event'");

    InstructionDesc[OpGroupWaitEvents].capabilities.push_back(CapabilityKernel);
    InstructionDesc[OpGroupWaitEvents].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupWaitEvents].operands.push(OperandId, "'Num Events'");
    InstructionDesc[OpGroupWaitEvents].operands.push(OperandId, "'Events List'");

    InstructionDesc[OpGroupAll].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupAll].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupAll].operands.push(OperandId, "'Predicate'");

    InstructionDesc[OpGroupAny].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupAny].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupAny].operands.push(OperandId, "'Predicate'");

    InstructionDesc[OpGroupBroadcast].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupBroadcast].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupBroadcast].operands.push(OperandId, "'Value'");
    InstructionDesc[OpGroupBroadcast].operands.push(OperandId, "'LocalId'");

    InstructionDesc[OpGroupIAdd].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupIAdd].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupIAdd].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupIAdd].operands.push(OperandId, "'X'");

    InstructionDesc[OpGroupFAdd].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupFAdd].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupFAdd].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupFAdd].operands.push(OperandId, "'X'");

    InstructionDesc[OpGroupUMin].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupUMin].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupUMin].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupUMin].operands.push(OperandId, "'X'");

    InstructionDesc[OpGroupSMin].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupSMin].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupSMin].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupSMin].operands.push(OperandId, "X");

    InstructionDesc[OpGroupFMin].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupFMin].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupFMin].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupFMin].operands.push(OperandId, "X");

    InstructionDesc[OpGroupUMax].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupUMax].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupUMax].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupUMax].operands.push(OperandId, "X");

    InstructionDesc[OpGroupSMax].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupSMax].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupSMax].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupSMax].operands.push(OperandId, "X");

    InstructionDesc[OpGroupFMax].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupFMax].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupFMax].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupFMax].operands.push(OperandId, "X");

    InstructionDesc[OpReadPipe].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpReadPipe].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpReadPipe].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpReadPipe].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpReadPipe].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpWritePipe].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpWritePipe].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpWritePipe].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpWritePipe].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpWritePipe].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpReservedReadPipe].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpReservedReadPipe].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpReservedReadPipe].operands.push(OperandId, "'Reserve Id'");
    InstructionDesc[OpReservedReadPipe].operands.push(OperandId, "'Index'");
    InstructionDesc[OpReservedReadPipe].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpReservedReadPipe].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpReservedReadPipe].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpReservedWritePipe].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpReservedWritePipe].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpReservedWritePipe].operands.push(OperandId, "'Reserve Id'");
    InstructionDesc[OpReservedWritePipe].operands.push(OperandId, "'Index'");
    InstructionDesc[OpReservedWritePipe].operands.push(OperandId, "'Pointer'");
    InstructionDesc[OpReservedWritePipe].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpReservedWritePipe].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpReserveReadPipePackets].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpReserveReadPipePackets].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpReserveReadPipePackets].operands.push(OperandId, "'Num Packets'");
    InstructionDesc[OpReserveReadPipePackets].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpReserveReadPipePackets].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpReserveWritePipePackets].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpReserveWritePipePackets].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpReserveWritePipePackets].operands.push(OperandId, "'Num Packets'");
    InstructionDesc[OpReserveWritePipePackets].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpReserveWritePipePackets].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpCommitReadPipe].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpCommitReadPipe].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpCommitReadPipe].operands.push(OperandId, "'Reserve Id'");
    InstructionDesc[OpCommitReadPipe].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpCommitReadPipe].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpCommitWritePipe].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpCommitWritePipe].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpCommitWritePipe].operands.push(OperandId, "'Reserve Id'");
    InstructionDesc[OpCommitWritePipe].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpCommitWritePipe].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpIsValidReserveId].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpIsValidReserveId].operands.push(OperandId, "'Reserve Id'");

    InstructionDesc[OpGetNumPipePackets].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpGetNumPipePackets].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpGetNumPipePackets].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpGetNumPipePackets].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpGetMaxPipePackets].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpGetMaxPipePackets].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpGetMaxPipePackets].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpGetMaxPipePackets].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpGroupReserveReadPipePackets].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpGroupReserveReadPipePackets].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupReserveReadPipePackets].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpGroupReserveReadPipePackets].operands.push(OperandId, "'Num Packets'");
    InstructionDesc[OpGroupReserveReadPipePackets].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpGroupReserveReadPipePackets].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpGroupReserveWritePipePackets].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpGroupReserveWritePipePackets].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupReserveWritePipePackets].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpGroupReserveWritePipePackets].operands.push(OperandId, "'Num Packets'");
    InstructionDesc[OpGroupReserveWritePipePackets].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpGroupReserveWritePipePackets].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpGroupCommitReadPipe].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpGroupCommitReadPipe].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupCommitReadPipe].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpGroupCommitReadPipe].operands.push(OperandId, "'Reserve Id'");
    InstructionDesc[OpGroupCommitReadPipe].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpGroupCommitReadPipe].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpGroupCommitWritePipe].capabilities.push_back(CapabilityPipes);
    InstructionDesc[OpGroupCommitWritePipe].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupCommitWritePipe].operands.push(OperandId, "'Pipe'");
    InstructionDesc[OpGroupCommitWritePipe].operands.push(OperandId, "'Reserve Id'");
    InstructionDesc[OpGroupCommitWritePipe].operands.push(OperandId, "'Packet Size'");
    InstructionDesc[OpGroupCommitWritePipe].operands.push(OperandId, "'Packet Alignment'");

    InstructionDesc[OpBuildNDRange].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpBuildNDRange].operands.push(OperandId, "'GlobalWorkSize'");
    InstructionDesc[OpBuildNDRange].operands.push(OperandId, "'LocalWorkSize'");
    InstructionDesc[OpBuildNDRange].operands.push(OperandId, "'GlobalWorkOffset'");

    InstructionDesc[OpGetDefaultQueue].capabilities.push_back(CapabilityDeviceEnqueue);

    InstructionDesc[OpCaptureEventProfilingInfo].capabilities.push_back(CapabilityDeviceEnqueue);

    InstructionDesc[OpCaptureEventProfilingInfo].operands.push(OperandId, "'Event'");
    InstructionDesc[OpCaptureEventProfilingInfo].operands.push(OperandId, "'Profiling Info'");
    InstructionDesc[OpCaptureEventProfilingInfo].operands.push(OperandId, "'Value'");

    InstructionDesc[OpSetUserEventStatus].capabilities.push_back(CapabilityDeviceEnqueue);

    InstructionDesc[OpSetUserEventStatus].operands.push(OperandId, "'Event'");
    InstructionDesc[OpSetUserEventStatus].operands.push(OperandId, "'Status'");

    InstructionDesc[OpIsValidEvent].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpIsValidEvent].operands.push(OperandId, "'Event'");

    InstructionDesc[OpCreateUserEvent].capabilities.push_back(CapabilityDeviceEnqueue);

    InstructionDesc[OpRetainEvent].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpRetainEvent].operands.push(OperandId, "'Event'");

    InstructionDesc[OpReleaseEvent].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpReleaseEvent].operands.push(OperandId, "'Event'");

    InstructionDesc[OpGetKernelWorkGroupSize].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpGetKernelWorkGroupSize].operands.push(OperandId, "'Invoke'");
    InstructionDesc[OpGetKernelWorkGroupSize].operands.push(OperandId, "'Param'");
    InstructionDesc[OpGetKernelWorkGroupSize].operands.push(OperandId, "'Param Size'");
    InstructionDesc[OpGetKernelWorkGroupSize].operands.push(OperandId, "'Param Align'");

    InstructionDesc[OpGetKernelPreferredWorkGroupSizeMultiple].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpGetKernelPreferredWorkGroupSizeMultiple].operands.push(OperandId, "'Invoke'");
    InstructionDesc[OpGetKernelPreferredWorkGroupSizeMultiple].operands.push(OperandId, "'Param'");
    InstructionDesc[OpGetKernelPreferredWorkGroupSizeMultiple].operands.push(OperandId, "'Param Size'");
    InstructionDesc[OpGetKernelPreferredWorkGroupSizeMultiple].operands.push(OperandId, "'Param Align'");

    InstructionDesc[OpGetKernelNDrangeSubGroupCount].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpGetKernelNDrangeSubGroupCount].operands.push(OperandId, "'ND Range'");
    InstructionDesc[OpGetKernelNDrangeSubGroupCount].operands.push(OperandId, "'Invoke'");
    InstructionDesc[OpGetKernelNDrangeSubGroupCount].operands.push(OperandId, "'Param'");
    InstructionDesc[OpGetKernelNDrangeSubGroupCount].operands.push(OperandId, "'Param Size'");
    InstructionDesc[OpGetKernelNDrangeSubGroupCount].operands.push(OperandId, "'Param Align'");

    InstructionDesc[OpGetKernelNDrangeMaxSubGroupSize].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpGetKernelNDrangeMaxSubGroupSize].operands.push(OperandId, "'ND Range'");
    InstructionDesc[OpGetKernelNDrangeMaxSubGroupSize].operands.push(OperandId, "'Invoke'");
    InstructionDesc[OpGetKernelNDrangeMaxSubGroupSize].operands.push(OperandId, "'Param'");
    InstructionDesc[OpGetKernelNDrangeMaxSubGroupSize].operands.push(OperandId, "'Param Size'");
    InstructionDesc[OpGetKernelNDrangeMaxSubGroupSize].operands.push(OperandId, "'Param Align'");

    InstructionDesc[OpEnqueueKernel].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'Queue'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'Flags'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'ND Range'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'Num Events'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'Wait Events'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'Ret Event'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'Invoke'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'Param'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'Param Size'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandId, "'Param Align'");
    InstructionDesc[OpEnqueueKernel].operands.push(OperandVariableIds, "'Local Size'");

    InstructionDesc[OpEnqueueMarker].capabilities.push_back(CapabilityDeviceEnqueue);
    InstructionDesc[OpEnqueueMarker].operands.push(OperandId, "'Queue'");
    InstructionDesc[OpEnqueueMarker].operands.push(OperandId, "'Num Events'");
    InstructionDesc[OpEnqueueMarker].operands.push(OperandId, "'Wait Events'");
    InstructionDesc[OpEnqueueMarker].operands.push(OperandId, "'Ret Event'");

    InstructionDesc[OpSubgroupBallotKHR].operands.push(OperandId, "'Predicate'");

    InstructionDesc[OpSubgroupFirstInvocationKHR].operands.push(OperandId, "'Value'");

    InstructionDesc[OpSubgroupAnyKHR].capabilities.push_back(CapabilitySubgroupVoteKHR);
    InstructionDesc[OpSubgroupAnyKHR].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpSubgroupAnyKHR].operands.push(OperandId, "'Predicate'");

    InstructionDesc[OpSubgroupAllKHR].capabilities.push_back(CapabilitySubgroupVoteKHR);
    InstructionDesc[OpSubgroupAllKHR].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpSubgroupAllKHR].operands.push(OperandId, "'Predicate'");

    InstructionDesc[OpSubgroupAllEqualKHR].capabilities.push_back(CapabilitySubgroupVoteKHR);
    InstructionDesc[OpSubgroupAllEqualKHR].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpSubgroupAllEqualKHR].operands.push(OperandId, "'Predicate'");

    InstructionDesc[OpSubgroupReadInvocationKHR].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpSubgroupReadInvocationKHR].operands.push(OperandId, "'Value'");
    InstructionDesc[OpSubgroupReadInvocationKHR].operands.push(OperandId, "'Index'");

#ifdef AMD_EXTENSIONS
    InstructionDesc[OpGroupIAddNonUniformAMD].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupIAddNonUniformAMD].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupIAddNonUniformAMD].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupIAddNonUniformAMD].operands.push(OperandId, "'X'");

    InstructionDesc[OpGroupFAddNonUniformAMD].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupFAddNonUniformAMD].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupFAddNonUniformAMD].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupFAddNonUniformAMD].operands.push(OperandId, "'X'");

    InstructionDesc[OpGroupUMinNonUniformAMD].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupUMinNonUniformAMD].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupUMinNonUniformAMD].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupUMinNonUniformAMD].operands.push(OperandId, "'X'");

    InstructionDesc[OpGroupSMinNonUniformAMD].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupSMinNonUniformAMD].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupSMinNonUniformAMD].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupSMinNonUniformAMD].operands.push(OperandId, "X");

    InstructionDesc[OpGroupFMinNonUniformAMD].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupFMinNonUniformAMD].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupFMinNonUniformAMD].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupFMinNonUniformAMD].operands.push(OperandId, "X");

    InstructionDesc[OpGroupUMaxNonUniformAMD].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupUMaxNonUniformAMD].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupUMaxNonUniformAMD].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupUMaxNonUniformAMD].operands.push(OperandId, "X");

    InstructionDesc[OpGroupSMaxNonUniformAMD].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupSMaxNonUniformAMD].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupSMaxNonUniformAMD].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupSMaxNonUniformAMD].operands.push(OperandId, "X");

    InstructionDesc[OpGroupFMaxNonUniformAMD].capabilities.push_back(CapabilityGroups);
    InstructionDesc[OpGroupFMaxNonUniformAMD].operands.push(OperandScope, "'Execution'");
    InstructionDesc[OpGroupFMaxNonUniformAMD].operands.push(OperandGroupOperation, "'Operation'");
    InstructionDesc[OpGroupFMaxNonUniformAMD].operands.push(OperandId, "X");
#endif
}

}; // end spv namespace
