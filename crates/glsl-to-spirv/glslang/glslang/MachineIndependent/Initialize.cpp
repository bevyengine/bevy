//
// Copyright (C) 2002-2005  3Dlabs Inc. Ltd.
// Copyright (C) 2012-2016 LunarG, Inc.
// Copyright (C) 2015-2016 Google, Inc.
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

//
// Create strings that declare built-in definitions, add built-ins programmatically
// that cannot be expressed in the strings, and establish mappings between
// built-in functions and operators.
//
// Where to put a built-in:
//   TBuiltIns::initialize(version,profile)       context-independent textual built-ins; add them to the right string
//   TBuiltIns::initialize(resources,...)         context-dependent textual built-ins; add them to the right string
//   TBuiltIns::identifyBuiltIns(...,symbolTable) context-independent programmatic additions/mappings to the symbol table,
//                                                including identifying what extensions are needed if a version does not allow a symbol
//   TBuiltIns::identifyBuiltIns(...,symbolTable, resources) context-dependent programmatic additions/mappings to the symbol table,
//                                                including identifying what extensions are needed if a version does not allow a symbol
//

#include "../Include/intermediate.h"
#include "Initialize.h"

namespace glslang {

// TODO: ARB_Compatability: do full extension support
const bool ARBCompatibility = true;

const bool ForwardCompatibility = false;

// change this back to false if depending on textual spellings of texturing calls when consuming the AST
// Using PureOperatorBuiltins=false is deprecated.
bool PureOperatorBuiltins = true;

inline bool IncludeLegacy(int version, EProfile profile, const SpvVersion& spvVersion)
{
    return profile != EEsProfile && (version <= 130 || (spvVersion.spv == 0 && ARBCompatibility) || profile == ECompatibilityProfile);
}

// Construct TBuiltInParseables base class.  This can be used for language-common constructs.
TBuiltInParseables::TBuiltInParseables()
{
}

// Destroy TBuiltInParseables.
TBuiltInParseables::~TBuiltInParseables()
{
}

TBuiltIns::TBuiltIns()
{
    // Set up textual representations for making all the permutations
    // of texturing/imaging functions.
    prefixes[EbtFloat] =  "";
    prefixes[EbtInt]   = "i";
    prefixes[EbtUint]  = "u";
    postfixes[2] = "2";
    postfixes[3] = "3";
    postfixes[4] = "4";

    // Map from symbolic class of texturing dimension to numeric dimensions.
    dimMap[Esd1D] = 1;
    dimMap[Esd2D] = 2;
    dimMap[EsdRect] = 2;
    dimMap[Esd3D] = 3;
    dimMap[EsdCube] = 3;
    dimMap[EsdBuffer] = 1;
    dimMap[EsdSubpass] = 2;  // potientially unused for now
}

TBuiltIns::~TBuiltIns()
{
}


//
// Add all context-independent built-in functions and variables that are present
// for the given version and profile.  Share common ones across stages, otherwise
// make stage-specific entries.
//
// Most built-ins variables can be added as simple text strings.  Some need to
// be added programmatically, which is done later in IdentifyBuiltIns() below.
//
void TBuiltIns::initialize(int version, EProfile profile, const SpvVersion& spvVersion)
{
    //============================================================================
    //
    // Prototypes for built-in functions seen by both vertex and fragment shaders.
    //
    //============================================================================

    //
    // Angle and Trigonometric Functions.
    //
    commonBuiltins.append(
        "float radians(float degrees);"
        "vec2  radians(vec2  degrees);"
        "vec3  radians(vec3  degrees);"
        "vec4  radians(vec4  degrees);"

        "float degrees(float radians);"
        "vec2  degrees(vec2  radians);"
        "vec3  degrees(vec3  radians);"
        "vec4  degrees(vec4  radians);"

        "float sin(float angle);"
        "vec2  sin(vec2  angle);"
        "vec3  sin(vec3  angle);"
        "vec4  sin(vec4  angle);"

        "float cos(float angle);"
        "vec2  cos(vec2  angle);"
        "vec3  cos(vec3  angle);"
        "vec4  cos(vec4  angle);"

        "float tan(float angle);"
        "vec2  tan(vec2  angle);"
        "vec3  tan(vec3  angle);"
        "vec4  tan(vec4  angle);"

        "float asin(float x);"
        "vec2  asin(vec2  x);"
        "vec3  asin(vec3  x);"
        "vec4  asin(vec4  x);"

        "float acos(float x);"
        "vec2  acos(vec2  x);"
        "vec3  acos(vec3  x);"
        "vec4  acos(vec4  x);"

        "float atan(float y, float x);"
        "vec2  atan(vec2  y, vec2  x);"
        "vec3  atan(vec3  y, vec3  x);"
        "vec4  atan(vec4  y, vec4  x);"

        "float atan(float y_over_x);"
        "vec2  atan(vec2  y_over_x);"
        "vec3  atan(vec3  y_over_x);"
        "vec4  atan(vec4  y_over_x);"

        "\n");

    if (version >= 130) {
        commonBuiltins.append(
            "float sinh(float angle);"
            "vec2  sinh(vec2  angle);"
            "vec3  sinh(vec3  angle);"
            "vec4  sinh(vec4  angle);"

            "float cosh(float angle);"
            "vec2  cosh(vec2  angle);"
            "vec3  cosh(vec3  angle);"
            "vec4  cosh(vec4  angle);"

            "float tanh(float angle);"
            "vec2  tanh(vec2  angle);"
            "vec3  tanh(vec3  angle);"
            "vec4  tanh(vec4  angle);"

            "float asinh(float x);"
            "vec2  asinh(vec2  x);"
            "vec3  asinh(vec3  x);"
            "vec4  asinh(vec4  x);"

            "float acosh(float x);"
            "vec2  acosh(vec2  x);"
            "vec3  acosh(vec3  x);"
            "vec4  acosh(vec4  x);"

            "float atanh(float y_over_x);"
            "vec2  atanh(vec2  y_over_x);"
            "vec3  atanh(vec3  y_over_x);"
            "vec4  atanh(vec4  y_over_x);"

            "\n");
    }

    //
    // Exponential Functions.
    //
    commonBuiltins.append(
        "float pow(float x, float y);"
        "vec2  pow(vec2  x, vec2  y);"
        "vec3  pow(vec3  x, vec3  y);"
        "vec4  pow(vec4  x, vec4  y);"

        "float exp(float x);"
        "vec2  exp(vec2  x);"
        "vec3  exp(vec3  x);"
        "vec4  exp(vec4  x);"

        "float log(float x);"
        "vec2  log(vec2  x);"
        "vec3  log(vec3  x);"
        "vec4  log(vec4  x);"

        "float exp2(float x);"
        "vec2  exp2(vec2  x);"
        "vec3  exp2(vec3  x);"
        "vec4  exp2(vec4  x);"

        "float log2(float x);"
        "vec2  log2(vec2  x);"
        "vec3  log2(vec3  x);"
        "vec4  log2(vec4  x);"

        "float sqrt(float x);"
        "vec2  sqrt(vec2  x);"
        "vec3  sqrt(vec3  x);"
        "vec4  sqrt(vec4  x);"

        "float inversesqrt(float x);"
        "vec2  inversesqrt(vec2  x);"
        "vec3  inversesqrt(vec3  x);"
        "vec4  inversesqrt(vec4  x);"

        "\n");

    //
    // Common Functions.
    //
    commonBuiltins.append(
        "float abs(float x);"
        "vec2  abs(vec2  x);"
        "vec3  abs(vec3  x);"
        "vec4  abs(vec4  x);"

        "float sign(float x);"
        "vec2  sign(vec2  x);"
        "vec3  sign(vec3  x);"
        "vec4  sign(vec4  x);"

        "float floor(float x);"
        "vec2  floor(vec2  x);"
        "vec3  floor(vec3  x);"
        "vec4  floor(vec4  x);"

        "float ceil(float x);"
        "vec2  ceil(vec2  x);"
        "vec3  ceil(vec3  x);"
        "vec4  ceil(vec4  x);"

        "float fract(float x);"
        "vec2  fract(vec2  x);"
        "vec3  fract(vec3  x);"
        "vec4  fract(vec4  x);"

        "float mod(float x, float y);"
        "vec2  mod(vec2  x, float y);"
        "vec3  mod(vec3  x, float y);"
        "vec4  mod(vec4  x, float y);"
        "vec2  mod(vec2  x, vec2  y);"
        "vec3  mod(vec3  x, vec3  y);"
        "vec4  mod(vec4  x, vec4  y);"

        "float min(float x, float y);"
        "vec2  min(vec2  x, float y);"
        "vec3  min(vec3  x, float y);"
        "vec4  min(vec4  x, float y);"
        "vec2  min(vec2  x, vec2  y);"
        "vec3  min(vec3  x, vec3  y);"
        "vec4  min(vec4  x, vec4  y);"

        "float max(float x, float y);"
        "vec2  max(vec2  x, float y);"
        "vec3  max(vec3  x, float y);"
        "vec4  max(vec4  x, float y);"
        "vec2  max(vec2  x, vec2  y);"
        "vec3  max(vec3  x, vec3  y);"
        "vec4  max(vec4  x, vec4  y);"

        "float clamp(float x, float minVal, float maxVal);"
        "vec2  clamp(vec2  x, float minVal, float maxVal);"
        "vec3  clamp(vec3  x, float minVal, float maxVal);"
        "vec4  clamp(vec4  x, float minVal, float maxVal);"
        "vec2  clamp(vec2  x, vec2  minVal, vec2  maxVal);"
        "vec3  clamp(vec3  x, vec3  minVal, vec3  maxVal);"
        "vec4  clamp(vec4  x, vec4  minVal, vec4  maxVal);"

        "float mix(float x, float y, float a);"
        "vec2  mix(vec2  x, vec2  y, float a);"
        "vec3  mix(vec3  x, vec3  y, float a);"
        "vec4  mix(vec4  x, vec4  y, float a);"
        "vec2  mix(vec2  x, vec2  y, vec2  a);"
        "vec3  mix(vec3  x, vec3  y, vec3  a);"
        "vec4  mix(vec4  x, vec4  y, vec4  a);"

        "float step(float edge, float x);"
        "vec2  step(vec2  edge, vec2  x);"
        "vec3  step(vec3  edge, vec3  x);"
        "vec4  step(vec4  edge, vec4  x);"
        "vec2  step(float edge, vec2  x);"
        "vec3  step(float edge, vec3  x);"
        "vec4  step(float edge, vec4  x);"

        "float smoothstep(float edge0, float edge1, float x);"
        "vec2  smoothstep(vec2  edge0, vec2  edge1, vec2  x);"
        "vec3  smoothstep(vec3  edge0, vec3  edge1, vec3  x);"
        "vec4  smoothstep(vec4  edge0, vec4  edge1, vec4  x);"
        "vec2  smoothstep(float edge0, float edge1, vec2  x);"
        "vec3  smoothstep(float edge0, float edge1, vec3  x);"
        "vec4  smoothstep(float edge0, float edge1, vec4  x);"

        "\n");

    if (version >= 130) {
        commonBuiltins.append(
            "  int abs(  int x);"
            "ivec2 abs(ivec2 x);"
            "ivec3 abs(ivec3 x);"
            "ivec4 abs(ivec4 x);"

            "  int sign(  int x);"
            "ivec2 sign(ivec2 x);"
            "ivec3 sign(ivec3 x);"
            "ivec4 sign(ivec4 x);"

            "float trunc(float x);"
            "vec2  trunc(vec2  x);"
            "vec3  trunc(vec3  x);"
            "vec4  trunc(vec4  x);"

            "float round(float x);"
            "vec2  round(vec2  x);"
            "vec3  round(vec3  x);"
            "vec4  round(vec4  x);"

            "float roundEven(float x);"
            "vec2  roundEven(vec2  x);"
            "vec3  roundEven(vec3  x);"
            "vec4  roundEven(vec4  x);"

            "float modf(float, out float);"
            "vec2  modf(vec2,  out vec2 );"
            "vec3  modf(vec3,  out vec3 );"
            "vec4  modf(vec4,  out vec4 );"

            "  int min(int    x, int y);"
            "ivec2 min(ivec2  x, int y);"
            "ivec3 min(ivec3  x, int y);"
            "ivec4 min(ivec4  x, int y);"
            "ivec2 min(ivec2  x, ivec2  y);"
            "ivec3 min(ivec3  x, ivec3  y);"
            "ivec4 min(ivec4  x, ivec4  y);"

            " uint min(uint   x, uint y);"
            "uvec2 min(uvec2  x, uint y);"
            "uvec3 min(uvec3  x, uint y);"
            "uvec4 min(uvec4  x, uint y);"
            "uvec2 min(uvec2  x, uvec2  y);"
            "uvec3 min(uvec3  x, uvec3  y);"
            "uvec4 min(uvec4  x, uvec4  y);"

            "  int max(int    x, int y);"
            "ivec2 max(ivec2  x, int y);"
            "ivec3 max(ivec3  x, int y);"
            "ivec4 max(ivec4  x, int y);"
            "ivec2 max(ivec2  x, ivec2  y);"
            "ivec3 max(ivec3  x, ivec3  y);"
            "ivec4 max(ivec4  x, ivec4  y);"

            " uint max(uint   x, uint y);"
            "uvec2 max(uvec2  x, uint y);"
            "uvec3 max(uvec3  x, uint y);"
            "uvec4 max(uvec4  x, uint y);"
            "uvec2 max(uvec2  x, uvec2  y);"
            "uvec3 max(uvec3  x, uvec3  y);"
            "uvec4 max(uvec4  x, uvec4  y);"

            "int    clamp(int x, int minVal, int maxVal);"
            "ivec2  clamp(ivec2  x, int minVal, int maxVal);"
            "ivec3  clamp(ivec3  x, int minVal, int maxVal);"
            "ivec4  clamp(ivec4  x, int minVal, int maxVal);"
            "ivec2  clamp(ivec2  x, ivec2  minVal, ivec2  maxVal);"
            "ivec3  clamp(ivec3  x, ivec3  minVal, ivec3  maxVal);"
            "ivec4  clamp(ivec4  x, ivec4  minVal, ivec4  maxVal);"

            "uint   clamp(uint x, uint minVal, uint maxVal);"
            "uvec2  clamp(uvec2  x, uint minVal, uint maxVal);"
            "uvec3  clamp(uvec3  x, uint minVal, uint maxVal);"
            "uvec4  clamp(uvec4  x, uint minVal, uint maxVal);"
            "uvec2  clamp(uvec2  x, uvec2  minVal, uvec2  maxVal);"
            "uvec3  clamp(uvec3  x, uvec3  minVal, uvec3  maxVal);"
            "uvec4  clamp(uvec4  x, uvec4  minVal, uvec4  maxVal);"

            "float mix(float x, float y, bool  a);"
            "vec2  mix(vec2  x, vec2  y, bvec2 a);"
            "vec3  mix(vec3  x, vec3  y, bvec3 a);"
            "vec4  mix(vec4  x, vec4  y, bvec4 a);"

            "bool  isnan(float x);"
            "bvec2 isnan(vec2  x);"
            "bvec3 isnan(vec3  x);"
            "bvec4 isnan(vec4  x);"

            "bool  isinf(float x);"
            "bvec2 isinf(vec2  x);"
            "bvec3 isinf(vec3  x);"
            "bvec4 isinf(vec4  x);"

            "\n");
    }

    //
    // double functions added to desktop 4.00, but not fma, frexp, ldexp, or pack/unpack
    //
    if (profile != EEsProfile && version >= 400) {
        commonBuiltins.append(

            "double sqrt(double);"
            "dvec2  sqrt(dvec2);"
            "dvec3  sqrt(dvec3);"
            "dvec4  sqrt(dvec4);"

            "double inversesqrt(double);"
            "dvec2  inversesqrt(dvec2);"
            "dvec3  inversesqrt(dvec3);"
            "dvec4  inversesqrt(dvec4);"

            "double abs(double);"
            "dvec2  abs(dvec2);"
            "dvec3  abs(dvec3);"
            "dvec4  abs(dvec4);"

            "double sign(double);"
            "dvec2  sign(dvec2);"
            "dvec3  sign(dvec3);"
            "dvec4  sign(dvec4);"

            "double floor(double);"
            "dvec2  floor(dvec2);"
            "dvec3  floor(dvec3);"
            "dvec4  floor(dvec4);"

            "double trunc(double);"
            "dvec2  trunc(dvec2);"
            "dvec3  trunc(dvec3);"
            "dvec4  trunc(dvec4);"

            "double round(double);"
            "dvec2  round(dvec2);"
            "dvec3  round(dvec3);"
            "dvec4  round(dvec4);"

            "double roundEven(double);"
            "dvec2  roundEven(dvec2);"
            "dvec3  roundEven(dvec3);"
            "dvec4  roundEven(dvec4);"

            "double ceil(double);"
            "dvec2  ceil(dvec2);"
            "dvec3  ceil(dvec3);"
            "dvec4  ceil(dvec4);"

            "double fract(double);"
            "dvec2  fract(dvec2);"
            "dvec3  fract(dvec3);"
            "dvec4  fract(dvec4);"

            "double mod(double, double);"
            "dvec2  mod(dvec2 , double);"
            "dvec3  mod(dvec3 , double);"
            "dvec4  mod(dvec4 , double);"
            "dvec2  mod(dvec2 , dvec2);"
            "dvec3  mod(dvec3 , dvec3);"
            "dvec4  mod(dvec4 , dvec4);"

            "double modf(double, out double);"
            "dvec2  modf(dvec2,  out dvec2);"
            "dvec3  modf(dvec3,  out dvec3);"
            "dvec4  modf(dvec4,  out dvec4);"

            "double min(double, double);"
            "dvec2  min(dvec2,  double);"
            "dvec3  min(dvec3,  double);"
            "dvec4  min(dvec4,  double);"
            "dvec2  min(dvec2,  dvec2);"
            "dvec3  min(dvec3,  dvec3);"
            "dvec4  min(dvec4,  dvec4);"

            "double max(double, double);"
            "dvec2  max(dvec2 , double);"
            "dvec3  max(dvec3 , double);"
            "dvec4  max(dvec4 , double);"
            "dvec2  max(dvec2 , dvec2);"
            "dvec3  max(dvec3 , dvec3);"
            "dvec4  max(dvec4 , dvec4);"

            "double clamp(double, double, double);"
            "dvec2  clamp(dvec2 , double, double);"
            "dvec3  clamp(dvec3 , double, double);"
            "dvec4  clamp(dvec4 , double, double);"
            "dvec2  clamp(dvec2 , dvec2 , dvec2);"
            "dvec3  clamp(dvec3 , dvec3 , dvec3);"
            "dvec4  clamp(dvec4 , dvec4 , dvec4);"

            "double mix(double, double, double);"
            "dvec2  mix(dvec2,  dvec2,  double);"
            "dvec3  mix(dvec3,  dvec3,  double);"
            "dvec4  mix(dvec4,  dvec4,  double);"
            "dvec2  mix(dvec2,  dvec2,  dvec2);"
            "dvec3  mix(dvec3,  dvec3,  dvec3);"
            "dvec4  mix(dvec4,  dvec4,  dvec4);"
            "double mix(double, double, bool);"
            "dvec2  mix(dvec2,  dvec2,  bvec2);"
            "dvec3  mix(dvec3,  dvec3,  bvec3);"
            "dvec4  mix(dvec4,  dvec4,  bvec4);"

            "double step(double, double);"
            "dvec2  step(dvec2 , dvec2);"
            "dvec3  step(dvec3 , dvec3);"
            "dvec4  step(dvec4 , dvec4);"
            "dvec2  step(double, dvec2);"
            "dvec3  step(double, dvec3);"
            "dvec4  step(double, dvec4);"

            "double smoothstep(double, double, double);"
            "dvec2  smoothstep(dvec2 , dvec2 , dvec2);"
            "dvec3  smoothstep(dvec3 , dvec3 , dvec3);"
            "dvec4  smoothstep(dvec4 , dvec4 , dvec4);"
            "dvec2  smoothstep(double, double, dvec2);"
            "dvec3  smoothstep(double, double, dvec3);"
            "dvec4  smoothstep(double, double, dvec4);"

            "bool  isnan(double);"
            "bvec2 isnan(dvec2);"
            "bvec3 isnan(dvec3);"
            "bvec4 isnan(dvec4);"

            "bool  isinf(double);"
            "bvec2 isinf(dvec2);"
            "bvec3 isinf(dvec3);"
            "bvec4 isinf(dvec4);"

            "double length(double);"
            "double length(dvec2);"
            "double length(dvec3);"
            "double length(dvec4);"

            "double distance(double, double);"
            "double distance(dvec2 , dvec2);"
            "double distance(dvec3 , dvec3);"
            "double distance(dvec4 , dvec4);"

            "double dot(double, double);"
            "double dot(dvec2 , dvec2);"
            "double dot(dvec3 , dvec3);"
            "double dot(dvec4 , dvec4);"

            "dvec3 cross(dvec3, dvec3);"

            "double normalize(double);"
            "dvec2  normalize(dvec2);"
            "dvec3  normalize(dvec3);"
            "dvec4  normalize(dvec4);"

            "double faceforward(double, double, double);"
            "dvec2  faceforward(dvec2,  dvec2,  dvec2);"
            "dvec3  faceforward(dvec3,  dvec3,  dvec3);"
            "dvec4  faceforward(dvec4,  dvec4,  dvec4);"

            "double reflect(double, double);"
            "dvec2  reflect(dvec2 , dvec2 );"
            "dvec3  reflect(dvec3 , dvec3 );"
            "dvec4  reflect(dvec4 , dvec4 );"

            "double refract(double, double, double);"
            "dvec2  refract(dvec2 , dvec2 , double);"
            "dvec3  refract(dvec3 , dvec3 , double);"
            "dvec4  refract(dvec4 , dvec4 , double);"

            "dmat2 matrixCompMult(dmat2, dmat2);"
            "dmat3 matrixCompMult(dmat3, dmat3);"
            "dmat4 matrixCompMult(dmat4, dmat4);"
            "dmat2x3 matrixCompMult(dmat2x3, dmat2x3);"
            "dmat2x4 matrixCompMult(dmat2x4, dmat2x4);"
            "dmat3x2 matrixCompMult(dmat3x2, dmat3x2);"
            "dmat3x4 matrixCompMult(dmat3x4, dmat3x4);"
            "dmat4x2 matrixCompMult(dmat4x2, dmat4x2);"
            "dmat4x3 matrixCompMult(dmat4x3, dmat4x3);"

            "dmat2   outerProduct(dvec2, dvec2);"
            "dmat3   outerProduct(dvec3, dvec3);"
            "dmat4   outerProduct(dvec4, dvec4);"
            "dmat2x3 outerProduct(dvec3, dvec2);"
            "dmat3x2 outerProduct(dvec2, dvec3);"
            "dmat2x4 outerProduct(dvec4, dvec2);"
            "dmat4x2 outerProduct(dvec2, dvec4);"
            "dmat3x4 outerProduct(dvec4, dvec3);"
            "dmat4x3 outerProduct(dvec3, dvec4);"

            "dmat2   transpose(dmat2);"
            "dmat3   transpose(dmat3);"
            "dmat4   transpose(dmat4);"
            "dmat2x3 transpose(dmat3x2);"
            "dmat3x2 transpose(dmat2x3);"
            "dmat2x4 transpose(dmat4x2);"
            "dmat4x2 transpose(dmat2x4);"
            "dmat3x4 transpose(dmat4x3);"
            "dmat4x3 transpose(dmat3x4);"

            "double determinant(dmat2);"
            "double determinant(dmat3);"
            "double determinant(dmat4);"

            "dmat2 inverse(dmat2);"
            "dmat3 inverse(dmat3);"
            "dmat4 inverse(dmat4);"

            "bvec2 lessThan(dvec2, dvec2);"
            "bvec3 lessThan(dvec3, dvec3);"
            "bvec4 lessThan(dvec4, dvec4);"

            "bvec2 lessThanEqual(dvec2, dvec2);"
            "bvec3 lessThanEqual(dvec3, dvec3);"
            "bvec4 lessThanEqual(dvec4, dvec4);"

            "bvec2 greaterThan(dvec2, dvec2);"
            "bvec3 greaterThan(dvec3, dvec3);"
            "bvec4 greaterThan(dvec4, dvec4);"

            "bvec2 greaterThanEqual(dvec2, dvec2);"
            "bvec3 greaterThanEqual(dvec3, dvec3);"
            "bvec4 greaterThanEqual(dvec4, dvec4);"

            "bvec2 equal(dvec2, dvec2);"
            "bvec3 equal(dvec3, dvec3);"
            "bvec4 equal(dvec4, dvec4);"

            "bvec2 notEqual(dvec2, dvec2);"
            "bvec3 notEqual(dvec3, dvec3);"
            "bvec4 notEqual(dvec4, dvec4);"

            "\n");
    }

    if (profile != EEsProfile && version >= 450) {
        commonBuiltins.append(

            "int64_t abs(int64_t);"
            "i64vec2 abs(i64vec2);"
            "i64vec3 abs(i64vec3);"
            "i64vec4 abs(i64vec4);"

            "int64_t sign(int64_t);"
            "i64vec2 sign(i64vec2);"
            "i64vec3 sign(i64vec3);"
            "i64vec4 sign(i64vec4);"

            "int64_t  min(int64_t,  int64_t);"
            "i64vec2  min(i64vec2,  int64_t);"
            "i64vec3  min(i64vec3,  int64_t);"
            "i64vec4  min(i64vec4,  int64_t);"
            "i64vec2  min(i64vec2,  i64vec2);"
            "i64vec3  min(i64vec3,  i64vec3);"
            "i64vec4  min(i64vec4,  i64vec4);"
            "uint64_t min(uint64_t, uint64_t);"
            "u64vec2  min(u64vec2,  uint64_t);"
            "u64vec3  min(u64vec3,  uint64_t);"
            "u64vec4  min(u64vec4,  uint64_t);"
            "u64vec2  min(u64vec2,  u64vec2);"
            "u64vec3  min(u64vec3,  u64vec3);"
            "u64vec4  min(u64vec4,  u64vec4);"

            "int64_t  max(int64_t,  int64_t);"
            "i64vec2  max(i64vec2,  int64_t);"
            "i64vec3  max(i64vec3,  int64_t);"
            "i64vec4  max(i64vec4,  int64_t);"
            "i64vec2  max(i64vec2,  i64vec2);"
            "i64vec3  max(i64vec3,  i64vec3);"
            "i64vec4  max(i64vec4,  i64vec4);"
            "uint64_t max(uint64_t, uint64_t);"
            "u64vec2  max(u64vec2,  uint64_t);"
            "u64vec3  max(u64vec3,  uint64_t);"
            "u64vec4  max(u64vec4,  uint64_t);"
            "u64vec2  max(u64vec2,  u64vec2);"
            "u64vec3  max(u64vec3,  u64vec3);"
            "u64vec4  max(u64vec4,  u64vec4);"

            "int64_t  clamp(int64_t,  int64_t,  int64_t);"
            "i64vec2  clamp(i64vec2,  int64_t,  int64_t);"
            "i64vec3  clamp(i64vec3,  int64_t,  int64_t);"
            "i64vec4  clamp(i64vec4,  int64_t,  int64_t);"
            "i64vec2  clamp(i64vec2,  i64vec2,  i64vec2);"
            "i64vec3  clamp(i64vec3,  i64vec3,  i64vec3);"
            "i64vec4  clamp(i64vec4,  i64vec4,  i64vec4);"
            "uint64_t clamp(uint64_t, uint64_t, uint64_t);"
            "u64vec2  clamp(u64vec2,  uint64_t, uint64_t);"
            "u64vec3  clamp(u64vec3,  uint64_t, uint64_t);"
            "u64vec4  clamp(u64vec4,  uint64_t, uint64_t);"
            "u64vec2  clamp(u64vec2,  u64vec2,  u64vec2);"
            "u64vec3  clamp(u64vec3,  u64vec3,  u64vec3);"
            "u64vec4  clamp(u64vec4,  u64vec4,  u64vec4);"

            "int64_t  mix(int64_t,  int64_t,  bool);"
            "i64vec2  mix(i64vec2,  i64vec2,  bvec2);"
            "i64vec3  mix(i64vec3,  i64vec3,  bvec3);"
            "i64vec4  mix(i64vec4,  i64vec4,  bvec4);"
            "uint64_t mix(uint64_t, uint64_t, bool);"
            "u64vec2  mix(u64vec2,  u64vec2,  bvec2);"
            "u64vec3  mix(u64vec3,  u64vec3,  bvec3);"
            "u64vec4  mix(u64vec4,  u64vec4,  bvec4);"

            "int64_t doubleBitsToInt64(double);"
            "i64vec2 doubleBitsToInt64(dvec2);"
            "i64vec3 doubleBitsToInt64(dvec3);"
            "i64vec4 doubleBitsToInt64(dvec4);"

            "uint64_t doubleBitsToUint64(double);"
            "u64vec2  doubleBitsToUint64(dvec2);"
            "u64vec3  doubleBitsToUint64(dvec3);"
            "u64vec4  doubleBitsToUint64(dvec4);"

            "double int64BitsToDouble(int64_t);"
            "dvec2  int64BitsToDouble(i64vec2);"
            "dvec3  int64BitsToDouble(i64vec3);"
            "dvec4  int64BitsToDouble(i64vec4);"

            "double uint64BitsToDouble(uint64_t);"
            "dvec2  uint64BitsToDouble(u64vec2);"
            "dvec3  uint64BitsToDouble(u64vec3);"
            "dvec4  uint64BitsToDouble(u64vec4);"

            "int64_t  packInt2x32(ivec2);"
            "uint64_t packUint2x32(uvec2);"
            "ivec2    unpackInt2x32(int64_t);"
            "uvec2    unpackUint2x32(uint64_t);"

            "bvec2 lessThan(i64vec2, i64vec2);"
            "bvec3 lessThan(i64vec3, i64vec3);"
            "bvec4 lessThan(i64vec4, i64vec4);"
            "bvec2 lessThan(u64vec2, u64vec2);"
            "bvec3 lessThan(u64vec3, u64vec3);"
            "bvec4 lessThan(u64vec4, u64vec4);"

            "bvec2 lessThanEqual(i64vec2, i64vec2);"
            "bvec3 lessThanEqual(i64vec3, i64vec3);"
            "bvec4 lessThanEqual(i64vec4, i64vec4);"
            "bvec2 lessThanEqual(u64vec2, u64vec2);"
            "bvec3 lessThanEqual(u64vec3, u64vec3);"
            "bvec4 lessThanEqual(u64vec4, u64vec4);"

            "bvec2 greaterThan(i64vec2, i64vec2);"
            "bvec3 greaterThan(i64vec3, i64vec3);"
            "bvec4 greaterThan(i64vec4, i64vec4);"
            "bvec2 greaterThan(u64vec2, u64vec2);"
            "bvec3 greaterThan(u64vec3, u64vec3);"
            "bvec4 greaterThan(u64vec4, u64vec4);"

            "bvec2 greaterThanEqual(i64vec2, i64vec2);"
            "bvec3 greaterThanEqual(i64vec3, i64vec3);"
            "bvec4 greaterThanEqual(i64vec4, i64vec4);"
            "bvec2 greaterThanEqual(u64vec2, u64vec2);"
            "bvec3 greaterThanEqual(u64vec3, u64vec3);"
            "bvec4 greaterThanEqual(u64vec4, u64vec4);"

            "bvec2 equal(i64vec2, i64vec2);"
            "bvec3 equal(i64vec3, i64vec3);"
            "bvec4 equal(i64vec4, i64vec4);"
            "bvec2 equal(u64vec2, u64vec2);"
            "bvec3 equal(u64vec3, u64vec3);"
            "bvec4 equal(u64vec4, u64vec4);"

            "bvec2 notEqual(i64vec2, i64vec2);"
            "bvec3 notEqual(i64vec3, i64vec3);"
            "bvec4 notEqual(i64vec4, i64vec4);"
            "bvec2 notEqual(u64vec2, u64vec2);"
            "bvec3 notEqual(u64vec3, u64vec3);"
            "bvec4 notEqual(u64vec4, u64vec4);"

#ifdef AMD_EXTENSIONS
            "int   findLSB(int64_t);"
            "ivec2 findLSB(i64vec2);"
            "ivec3 findLSB(i64vec3);"
            "ivec4 findLSB(i64vec4);"

            "int   findLSB(uint64_t);"
            "ivec2 findLSB(u64vec2);"
            "ivec3 findLSB(u64vec3);"
            "ivec4 findLSB(u64vec4);"

            "int   findMSB(int64_t);"
            "ivec2 findMSB(i64vec2);"
            "ivec3 findMSB(i64vec3);"
            "ivec4 findMSB(i64vec4);"

            "int   findMSB(uint64_t);"
            "ivec2 findMSB(u64vec2);"
            "ivec3 findMSB(u64vec3);"
            "ivec4 findMSB(u64vec4);"
#endif
            "\n"
        );
    }

#ifdef AMD_EXTENSIONS
    // GL_AMD_shader_trinary_minmax
    if (profile != EEsProfile && version >= 430) {
        commonBuiltins.append(
            "float min3(float, float, float);"
            "vec2  min3(vec2,  vec2,  vec2);"
            "vec3  min3(vec3,  vec3,  vec3);"
            "vec4  min3(vec4,  vec4,  vec4);"

            "int   min3(int,   int,   int);"
            "ivec2 min3(ivec2, ivec2, ivec2);"
            "ivec3 min3(ivec3, ivec3, ivec3);"
            "ivec4 min3(ivec4, ivec4, ivec4);"

            "uint  min3(uint,  uint,  uint);"
            "uvec2 min3(uvec2, uvec2, uvec2);"
            "uvec3 min3(uvec3, uvec3, uvec3);"
            "uvec4 min3(uvec4, uvec4, uvec4);"

            "float max3(float, float, float);"
            "vec2  max3(vec2,  vec2,  vec2);"
            "vec3  max3(vec3,  vec3,  vec3);"
            "vec4  max3(vec4,  vec4,  vec4);"

            "int   max3(int,   int,   int);"
            "ivec2 max3(ivec2, ivec2, ivec2);"
            "ivec3 max3(ivec3, ivec3, ivec3);"
            "ivec4 max3(ivec4, ivec4, ivec4);"

            "uint  max3(uint,  uint,  uint);"
            "uvec2 max3(uvec2, uvec2, uvec2);"
            "uvec3 max3(uvec3, uvec3, uvec3);"
            "uvec4 max3(uvec4, uvec4, uvec4);"

            "float mid3(float, float, float);"
            "vec2  mid3(vec2,  vec2,  vec2);"
            "vec3  mid3(vec3,  vec3,  vec3);"
            "vec4  mid3(vec4,  vec4,  vec4);"

            "int   mid3(int,   int,   int);"
            "ivec2 mid3(ivec2, ivec2, ivec2);"
            "ivec3 mid3(ivec3, ivec3, ivec3);"
            "ivec4 mid3(ivec4, ivec4, ivec4);"

            "uint  mid3(uint,  uint,  uint);"
            "uvec2 mid3(uvec2, uvec2, uvec2);"
            "uvec3 mid3(uvec3, uvec3, uvec3);"
            "uvec4 mid3(uvec4, uvec4, uvec4);"

            "float16_t min3(float16_t, float16_t, float16_t);"
            "f16vec2   min3(f16vec2,   f16vec2,   f16vec2);"
            "f16vec3   min3(f16vec3,   f16vec3,   f16vec3);"
            "f16vec4   min3(f16vec4,   f16vec4,   f16vec4);"

            "float16_t max3(float16_t, float16_t, float16_t);"
            "f16vec2   max3(f16vec2,   f16vec2,   f16vec2);"
            "f16vec3   max3(f16vec3,   f16vec3,   f16vec3);"
            "f16vec4   max3(f16vec4,   f16vec4,   f16vec4);"

            "float16_t mid3(float16_t, float16_t, float16_t);"
            "f16vec2   mid3(f16vec2,   f16vec2,   f16vec2);"
            "f16vec3   mid3(f16vec3,   f16vec3,   f16vec3);"
            "f16vec4   mid3(f16vec4,   f16vec4,   f16vec4);"

            "\n"
        );
    }
#endif

    if ((profile == EEsProfile && version >= 310) ||
        (profile != EEsProfile && version >= 430)) {
        commonBuiltins.append(
            "uint atomicAdd(coherent volatile inout uint, uint);"
            " int atomicAdd(coherent volatile inout  int,  int);"

            "uint atomicMin(coherent volatile inout uint, uint);"
            " int atomicMin(coherent volatile inout  int,  int);"

            "uint atomicMax(coherent volatile inout uint, uint);"
            " int atomicMax(coherent volatile inout  int,  int);"

            "uint atomicAnd(coherent volatile inout uint, uint);"
            " int atomicAnd(coherent volatile inout  int,  int);"

            "uint atomicOr (coherent volatile inout uint, uint);"
            " int atomicOr (coherent volatile inout  int,  int);"

            "uint atomicXor(coherent volatile inout uint, uint);"
            " int atomicXor(coherent volatile inout  int,  int);"

            "uint atomicExchange(coherent volatile inout uint, uint);"
            " int atomicExchange(coherent volatile inout  int,  int);"

            "uint atomicCompSwap(coherent volatile inout uint, uint, uint);"
            " int atomicCompSwap(coherent volatile inout  int,  int,  int);"

            "\n");
    }

    if ((profile == EEsProfile && version >= 310) ||
        (profile != EEsProfile && version >= 450)) {
        commonBuiltins.append(
            "int    mix(int    x, int    y, bool  a);"
            "ivec2  mix(ivec2  x, ivec2  y, bvec2 a);"
            "ivec3  mix(ivec3  x, ivec3  y, bvec3 a);"
            "ivec4  mix(ivec4  x, ivec4  y, bvec4 a);"

            "uint   mix(uint   x, uint   y, bool  a);"
            "uvec2  mix(uvec2  x, uvec2  y, bvec2 a);"
            "uvec3  mix(uvec3  x, uvec3  y, bvec3 a);"
            "uvec4  mix(uvec4  x, uvec4  y, bvec4 a);"

            "bool   mix(bool   x, bool   y, bool  a);"
            "bvec2  mix(bvec2  x, bvec2  y, bvec2 a);"
            "bvec3  mix(bvec3  x, bvec3  y, bvec3 a);"
            "bvec4  mix(bvec4  x, bvec4  y, bvec4 a);"

            "\n");
    }

    if ((profile == EEsProfile && version >= 300) ||
        (profile != EEsProfile && version >= 330)) {
        commonBuiltins.append(
            "int   floatBitsToInt(highp float value);"
            "ivec2 floatBitsToInt(highp vec2  value);"
            "ivec3 floatBitsToInt(highp vec3  value);"
            "ivec4 floatBitsToInt(highp vec4  value);"

            "uint  floatBitsToUint(highp float value);"
            "uvec2 floatBitsToUint(highp vec2  value);"
            "uvec3 floatBitsToUint(highp vec3  value);"
            "uvec4 floatBitsToUint(highp vec4  value);"

            "float intBitsToFloat(highp int   value);"
            "vec2  intBitsToFloat(highp ivec2 value);"
            "vec3  intBitsToFloat(highp ivec3 value);"
            "vec4  intBitsToFloat(highp ivec4 value);"

            "float uintBitsToFloat(highp uint  value);"
            "vec2  uintBitsToFloat(highp uvec2 value);"
            "vec3  uintBitsToFloat(highp uvec3 value);"
            "vec4  uintBitsToFloat(highp uvec4 value);"

            "\n");
    }

    if ((profile != EEsProfile && version >= 400) ||
        (profile == EEsProfile && version >= 310)) {    // GL_OES_gpu_shader5

        commonBuiltins.append(
            "float  fma(float,  float,  float );"
            "vec2   fma(vec2,   vec2,   vec2  );"
            "vec3   fma(vec3,   vec3,   vec3  );"
            "vec4   fma(vec4,   vec4,   vec4  );"
            "\n");

        if (profile != EEsProfile) {
            commonBuiltins.append(
                "double fma(double, double, double);"
                "dvec2  fma(dvec2,  dvec2,  dvec2 );"
                "dvec3  fma(dvec3,  dvec3,  dvec3 );"
                "dvec4  fma(dvec4,  dvec4,  dvec4 );"
                "\n");
        }
    }

    if ((profile == EEsProfile && version >= 310) ||
        (profile != EEsProfile && version >= 400)) {
        commonBuiltins.append(
            "float frexp(highp float, out highp int);"
            "vec2  frexp(highp vec2,  out highp ivec2);"
            "vec3  frexp(highp vec3,  out highp ivec3);"
            "vec4  frexp(highp vec4,  out highp ivec4);"

            "float ldexp(highp float, highp int);"
            "vec2  ldexp(highp vec2,  highp ivec2);"
            "vec3  ldexp(highp vec3,  highp ivec3);"
            "vec4  ldexp(highp vec4,  highp ivec4);"

            "\n");
    }

    if (profile != EEsProfile && version >= 400) {
        commonBuiltins.append(
            "double frexp(double, out int);"
            "dvec2  frexp( dvec2, out ivec2);"
            "dvec3  frexp( dvec3, out ivec3);"
            "dvec4  frexp( dvec4, out ivec4);"

            "double ldexp(double, int);"
            "dvec2  ldexp( dvec2, ivec2);"
            "dvec3  ldexp( dvec3, ivec3);"
            "dvec4  ldexp( dvec4, ivec4);"

            "double packDouble2x32(uvec2);"
            "uvec2 unpackDouble2x32(double);"

            "\n");
    }

    if ((profile == EEsProfile && version >= 300) ||
        (profile != EEsProfile && version >= 400)) {
        commonBuiltins.append(
            "highp uint packUnorm2x16(vec2);"
                  "vec2 unpackUnorm2x16(highp uint);"
            "\n");
    }

    if ((profile == EEsProfile && version >= 300) ||
        (profile != EEsProfile && version >= 420)) {
        commonBuiltins.append(
            "highp uint packSnorm2x16(vec2);"
            "      vec2 unpackSnorm2x16(highp uint);"
            "highp uint packHalf2x16(vec2);"
            "\n");
    }

    if (profile == EEsProfile && version >= 300) {
        commonBuiltins.append(
            "mediump vec2 unpackHalf2x16(highp uint);"
            "\n");
    } else if (profile != EEsProfile && version >= 420) {
        commonBuiltins.append(
            "        vec2 unpackHalf2x16(highp uint);"
            "\n");
    }

    if ((profile == EEsProfile && version >= 310) ||
        (profile != EEsProfile && version >= 400)) {
        commonBuiltins.append(
            "highp uint packSnorm4x8(vec4);"
            "highp uint packUnorm4x8(vec4);"
            "\n");
    }

    if (profile == EEsProfile && version >= 310) {
        commonBuiltins.append(
            "mediump vec4 unpackSnorm4x8(highp uint);"
            "mediump vec4 unpackUnorm4x8(highp uint);"
            "\n");
    } else if (profile != EEsProfile && version >= 400) {
        commonBuiltins.append(
                    "vec4 unpackSnorm4x8(highp uint);"
                    "vec4 unpackUnorm4x8(highp uint);"
            "\n");
    }

    //
    // Geometric Functions.
    //
    commonBuiltins.append(
        "float length(float x);"
        "float length(vec2  x);"
        "float length(vec3  x);"
        "float length(vec4  x);"

        "float distance(float p0, float p1);"
        "float distance(vec2  p0, vec2  p1);"
        "float distance(vec3  p0, vec3  p1);"
        "float distance(vec4  p0, vec4  p1);"

        "float dot(float x, float y);"
        "float dot(vec2  x, vec2  y);"
        "float dot(vec3  x, vec3  y);"
        "float dot(vec4  x, vec4  y);"

        "vec3 cross(vec3 x, vec3 y);"
        "float normalize(float x);"
        "vec2  normalize(vec2  x);"
        "vec3  normalize(vec3  x);"
        "vec4  normalize(vec4  x);"

        "float faceforward(float N, float I, float Nref);"
        "vec2  faceforward(vec2  N, vec2  I, vec2  Nref);"
        "vec3  faceforward(vec3  N, vec3  I, vec3  Nref);"
        "vec4  faceforward(vec4  N, vec4  I, vec4  Nref);"

        "float reflect(float I, float N);"
        "vec2  reflect(vec2  I, vec2  N);"
        "vec3  reflect(vec3  I, vec3  N);"
        "vec4  reflect(vec4  I, vec4  N);"

        "float refract(float I, float N, float eta);"
        "vec2  refract(vec2  I, vec2  N, float eta);"
        "vec3  refract(vec3  I, vec3  N, float eta);"
        "vec4  refract(vec4  I, vec4  N, float eta);"

        "\n");

    //
    // Matrix Functions.
    //
    commonBuiltins.append(
        "mat2 matrixCompMult(mat2 x, mat2 y);"
        "mat3 matrixCompMult(mat3 x, mat3 y);"
        "mat4 matrixCompMult(mat4 x, mat4 y);"

        "\n");

    // 120 is correct for both ES and desktop
    if (version >= 120) {
        commonBuiltins.append(
            "mat2   outerProduct(vec2 c, vec2 r);"
            "mat3   outerProduct(vec3 c, vec3 r);"
            "mat4   outerProduct(vec4 c, vec4 r);"
            "mat2x3 outerProduct(vec3 c, vec2 r);"
            "mat3x2 outerProduct(vec2 c, vec3 r);"
            "mat2x4 outerProduct(vec4 c, vec2 r);"
            "mat4x2 outerProduct(vec2 c, vec4 r);"
            "mat3x4 outerProduct(vec4 c, vec3 r);"
            "mat4x3 outerProduct(vec3 c, vec4 r);"

            "mat2   transpose(mat2   m);"
            "mat3   transpose(mat3   m);"
            "mat4   transpose(mat4   m);"
            "mat2x3 transpose(mat3x2 m);"
            "mat3x2 transpose(mat2x3 m);"
            "mat2x4 transpose(mat4x2 m);"
            "mat4x2 transpose(mat2x4 m);"
            "mat3x4 transpose(mat4x3 m);"
            "mat4x3 transpose(mat3x4 m);"

            "mat2x3 matrixCompMult(mat2x3, mat2x3);"
            "mat2x4 matrixCompMult(mat2x4, mat2x4);"
            "mat3x2 matrixCompMult(mat3x2, mat3x2);"
            "mat3x4 matrixCompMult(mat3x4, mat3x4);"
            "mat4x2 matrixCompMult(mat4x2, mat4x2);"
            "mat4x3 matrixCompMult(mat4x3, mat4x3);"

            "\n");

        // 150 is correct for both ES and desktop
        if (version >= 150) {
            commonBuiltins.append(
                "float determinant(mat2 m);"
                "float determinant(mat3 m);"
                "float determinant(mat4 m);"

                "mat2 inverse(mat2 m);"
                "mat3 inverse(mat3 m);"
                "mat4 inverse(mat4 m);"

                "\n");
        }
    }

    //
    // Vector relational functions.
    //
    commonBuiltins.append(
        "bvec2 lessThan(vec2 x, vec2 y);"
        "bvec3 lessThan(vec3 x, vec3 y);"
        "bvec4 lessThan(vec4 x, vec4 y);"

        "bvec2 lessThan(ivec2 x, ivec2 y);"
        "bvec3 lessThan(ivec3 x, ivec3 y);"
        "bvec4 lessThan(ivec4 x, ivec4 y);"

        "bvec2 lessThanEqual(vec2 x, vec2 y);"
        "bvec3 lessThanEqual(vec3 x, vec3 y);"
        "bvec4 lessThanEqual(vec4 x, vec4 y);"

        "bvec2 lessThanEqual(ivec2 x, ivec2 y);"
        "bvec3 lessThanEqual(ivec3 x, ivec3 y);"
        "bvec4 lessThanEqual(ivec4 x, ivec4 y);"

        "bvec2 greaterThan(vec2 x, vec2 y);"
        "bvec3 greaterThan(vec3 x, vec3 y);"
        "bvec4 greaterThan(vec4 x, vec4 y);"

        "bvec2 greaterThan(ivec2 x, ivec2 y);"
        "bvec3 greaterThan(ivec3 x, ivec3 y);"
        "bvec4 greaterThan(ivec4 x, ivec4 y);"

        "bvec2 greaterThanEqual(vec2 x, vec2 y);"
        "bvec3 greaterThanEqual(vec3 x, vec3 y);"
        "bvec4 greaterThanEqual(vec4 x, vec4 y);"

        "bvec2 greaterThanEqual(ivec2 x, ivec2 y);"
        "bvec3 greaterThanEqual(ivec3 x, ivec3 y);"
        "bvec4 greaterThanEqual(ivec4 x, ivec4 y);"

        "bvec2 equal(vec2 x, vec2 y);"
        "bvec3 equal(vec3 x, vec3 y);"
        "bvec4 equal(vec4 x, vec4 y);"

        "bvec2 equal(ivec2 x, ivec2 y);"
        "bvec3 equal(ivec3 x, ivec3 y);"
        "bvec4 equal(ivec4 x, ivec4 y);"

        "bvec2 equal(bvec2 x, bvec2 y);"
        "bvec3 equal(bvec3 x, bvec3 y);"
        "bvec4 equal(bvec4 x, bvec4 y);"

        "bvec2 notEqual(vec2 x, vec2 y);"
        "bvec3 notEqual(vec3 x, vec3 y);"
        "bvec4 notEqual(vec4 x, vec4 y);"

        "bvec2 notEqual(ivec2 x, ivec2 y);"
        "bvec3 notEqual(ivec3 x, ivec3 y);"
        "bvec4 notEqual(ivec4 x, ivec4 y);"

        "bvec2 notEqual(bvec2 x, bvec2 y);"
        "bvec3 notEqual(bvec3 x, bvec3 y);"
        "bvec4 notEqual(bvec4 x, bvec4 y);"

        "bool any(bvec2 x);"
        "bool any(bvec3 x);"
        "bool any(bvec4 x);"

        "bool all(bvec2 x);"
        "bool all(bvec3 x);"
        "bool all(bvec4 x);"

        "bvec2 not(bvec2 x);"
        "bvec3 not(bvec3 x);"
        "bvec4 not(bvec4 x);"

        "\n");

    if (version >= 130) {
        commonBuiltins.append(
            "bvec2 lessThan(uvec2 x, uvec2 y);"
            "bvec3 lessThan(uvec3 x, uvec3 y);"
            "bvec4 lessThan(uvec4 x, uvec4 y);"

            "bvec2 lessThanEqual(uvec2 x, uvec2 y);"
            "bvec3 lessThanEqual(uvec3 x, uvec3 y);"
            "bvec4 lessThanEqual(uvec4 x, uvec4 y);"

            "bvec2 greaterThan(uvec2 x, uvec2 y);"
            "bvec3 greaterThan(uvec3 x, uvec3 y);"
            "bvec4 greaterThan(uvec4 x, uvec4 y);"

            "bvec2 greaterThanEqual(uvec2 x, uvec2 y);"
            "bvec3 greaterThanEqual(uvec3 x, uvec3 y);"
            "bvec4 greaterThanEqual(uvec4 x, uvec4 y);"

            "bvec2 equal(uvec2 x, uvec2 y);"
            "bvec3 equal(uvec3 x, uvec3 y);"
            "bvec4 equal(uvec4 x, uvec4 y);"

            "bvec2 notEqual(uvec2 x, uvec2 y);"
            "bvec3 notEqual(uvec3 x, uvec3 y);"
            "bvec4 notEqual(uvec4 x, uvec4 y);"

            "\n");
    }

    //
    // Original-style texture functions existing in all stages.
    // (Per-stage functions below.)
    //
    if ((profile == EEsProfile && version == 100) ||
         profile == ECompatibilityProfile ||
        (profile == ECoreProfile && version < 420) ||
         profile == ENoProfile) {
        if (spvVersion.spv == 0) {
            commonBuiltins.append(
                "vec4 texture2D(sampler2D, vec2);"

                "vec4 texture2DProj(sampler2D, vec3);"
                "vec4 texture2DProj(sampler2D, vec4);"

                "vec4 texture3D(sampler3D, vec3);"     // OES_texture_3D, but caught by keyword check
                "vec4 texture3DProj(sampler3D, vec4);" // OES_texture_3D, but caught by keyword check

                "vec4 textureCube(samplerCube, vec3);"

                "\n");
        }
    }

    if ( profile == ECompatibilityProfile ||
        (profile == ECoreProfile && version < 420) ||
         profile == ENoProfile) {
        if (spvVersion.spv == 0) {
            commonBuiltins.append(
                "vec4 texture1D(sampler1D, float);"

                "vec4 texture1DProj(sampler1D, vec2);"
                "vec4 texture1DProj(sampler1D, vec4);"

                "vec4 shadow1D(sampler1DShadow, vec3);"
                "vec4 shadow2D(sampler2DShadow, vec3);"
                "vec4 shadow1DProj(sampler1DShadow, vec4);"
                "vec4 shadow2DProj(sampler2DShadow, vec4);"

                "vec4 texture2DRect(sampler2DRect, vec2);"          // GL_ARB_texture_rectangle, caught by keyword check
                "vec4 texture2DRectProj(sampler2DRect, vec3);"      // GL_ARB_texture_rectangle, caught by keyword check
                "vec4 texture2DRectProj(sampler2DRect, vec4);"      // GL_ARB_texture_rectangle, caught by keyword check
                "vec4 shadow2DRect(sampler2DRectShadow, vec3);"     // GL_ARB_texture_rectangle, caught by keyword check
                "vec4 shadow2DRectProj(sampler2DRectShadow, vec4);" // GL_ARB_texture_rectangle, caught by keyword check

                "\n");
        }
    }

    if (profile == EEsProfile) {
        if (spvVersion.spv == 0) {
            commonBuiltins.append(
                "vec4 texture2D(samplerExternalOES, vec2 coord);"  // GL_OES_EGL_image_external, caught by keyword check
                "vec4 texture2DProj(samplerExternalOES, vec3);"    // GL_OES_EGL_image_external, caught by keyword check
                "vec4 texture2DProj(samplerExternalOES, vec4);"    // GL_OES_EGL_image_external, caught by keyword check
                "vec4 texture2DGradEXT(sampler2D, vec2, vec2, vec2);"      // GL_EXT_shader_texture_lod
                "vec4 texture2DProjGradEXT(sampler2D, vec3, vec2, vec2);"  // GL_EXT_shader_texture_lod
                "vec4 texture2DProjGradEXT(sampler2D, vec4, vec2, vec2);"  // GL_EXT_shader_texture_lod
                "vec4 textureCubeGradEXT(samplerCube, vec3, vec3, vec3);"  // GL_EXT_shader_texture_lod

                "\n");
        }
    }

    //
    // Noise functions.
    //
    if (profile != EEsProfile) {
        commonBuiltins.append(
            "float noise1(float x);"
            "float noise1(vec2  x);"
            "float noise1(vec3  x);"
            "float noise1(vec4  x);"

            "vec2 noise2(float x);"
            "vec2 noise2(vec2  x);"
            "vec2 noise2(vec3  x);"
            "vec2 noise2(vec4  x);"

            "vec3 noise3(float x);"
            "vec3 noise3(vec2  x);"
            "vec3 noise3(vec3  x);"
            "vec3 noise3(vec4  x);"

            "vec4 noise4(float x);"
            "vec4 noise4(vec2  x);"
            "vec4 noise4(vec3  x);"
            "vec4 noise4(vec4  x);"

            "\n");
    }

    if (spvVersion.vulkan == 0) {
        //
        // Atomic counter functions.
        //
        if ((profile != EEsProfile && version >= 300) ||
            (profile == EEsProfile && version >= 310)) {
            commonBuiltins.append(
                "uint atomicCounterIncrement(atomic_uint x);"
                "uint atomicCounterDecrement(atomic_uint x);"
                "uint atomicCounter(atomic_uint x);"

                "\n");
        }
    }

    // Bitfield
    if ((profile == EEsProfile && version >= 310) ||
        (profile != EEsProfile && version >= 400)) {
        commonBuiltins.append(
            "  int bitfieldExtract(  int, int, int);"
            "ivec2 bitfieldExtract(ivec2, int, int);"
            "ivec3 bitfieldExtract(ivec3, int, int);"
            "ivec4 bitfieldExtract(ivec4, int, int);"

            " uint bitfieldExtract( uint, int, int);"
            "uvec2 bitfieldExtract(uvec2, int, int);"
            "uvec3 bitfieldExtract(uvec3, int, int);"
            "uvec4 bitfieldExtract(uvec4, int, int);"

            "  int bitfieldInsert(  int base,   int, int, int);"
            "ivec2 bitfieldInsert(ivec2 base, ivec2, int, int);"
            "ivec3 bitfieldInsert(ivec3 base, ivec3, int, int);"
            "ivec4 bitfieldInsert(ivec4 base, ivec4, int, int);"

            " uint bitfieldInsert( uint base,  uint, int, int);"
            "uvec2 bitfieldInsert(uvec2 base, uvec2, int, int);"
            "uvec3 bitfieldInsert(uvec3 base, uvec3, int, int);"
            "uvec4 bitfieldInsert(uvec4 base, uvec4, int, int);"

            "\n");
    }

    if (profile != EEsProfile && version >= 400) {
        commonBuiltins.append(
            "  int findLSB(  int);"
            "ivec2 findLSB(ivec2);"
            "ivec3 findLSB(ivec3);"
            "ivec4 findLSB(ivec4);"

            "  int findLSB( uint);"
            "ivec2 findLSB(uvec2);"
            "ivec3 findLSB(uvec3);"
            "ivec4 findLSB(uvec4);"

            "\n");
    } else if (profile == EEsProfile && version >= 310) {
        commonBuiltins.append(
            "lowp   int findLSB(  int);"
            "lowp ivec2 findLSB(ivec2);"
            "lowp ivec3 findLSB(ivec3);"
            "lowp ivec4 findLSB(ivec4);"

            "lowp   int findLSB( uint);"
            "lowp ivec2 findLSB(uvec2);"
            "lowp ivec3 findLSB(uvec3);"
            "lowp ivec4 findLSB(uvec4);"

            "\n");
    }

    if (profile != EEsProfile && version >= 400) {
        commonBuiltins.append(
            "  int bitCount(  int);"
            "ivec2 bitCount(ivec2);"
            "ivec3 bitCount(ivec3);"
            "ivec4 bitCount(ivec4);"

            "  int bitCount( uint);"
            "ivec2 bitCount(uvec2);"
            "ivec3 bitCount(uvec3);"
            "ivec4 bitCount(uvec4);"

            "  int findMSB(highp   int);"
            "ivec2 findMSB(highp ivec2);"
            "ivec3 findMSB(highp ivec3);"
            "ivec4 findMSB(highp ivec4);"

            "  int findMSB(highp  uint);"
            "ivec2 findMSB(highp uvec2);"
            "ivec3 findMSB(highp uvec3);"
            "ivec4 findMSB(highp uvec4);"

            "\n");
    }

    if ((profile == EEsProfile && version >= 310) ||
        (profile != EEsProfile && version >= 400)) {
        commonBuiltins.append(
            " uint uaddCarry(highp  uint, highp  uint, out lowp  uint carry);"
            "uvec2 uaddCarry(highp uvec2, highp uvec2, out lowp uvec2 carry);"
            "uvec3 uaddCarry(highp uvec3, highp uvec3, out lowp uvec3 carry);"
            "uvec4 uaddCarry(highp uvec4, highp uvec4, out lowp uvec4 carry);"

            " uint usubBorrow(highp  uint, highp  uint, out lowp  uint borrow);"
            "uvec2 usubBorrow(highp uvec2, highp uvec2, out lowp uvec2 borrow);"
            "uvec3 usubBorrow(highp uvec3, highp uvec3, out lowp uvec3 borrow);"
            "uvec4 usubBorrow(highp uvec4, highp uvec4, out lowp uvec4 borrow);"

            "void umulExtended(highp  uint, highp  uint, out highp  uint, out highp  uint lsb);"
            "void umulExtended(highp uvec2, highp uvec2, out highp uvec2, out highp uvec2 lsb);"
            "void umulExtended(highp uvec3, highp uvec3, out highp uvec3, out highp uvec3 lsb);"
            "void umulExtended(highp uvec4, highp uvec4, out highp uvec4, out highp uvec4 lsb);"

            "void imulExtended(highp   int, highp   int, out highp   int, out highp   int lsb);"
            "void imulExtended(highp ivec2, highp ivec2, out highp ivec2, out highp ivec2 lsb);"
            "void imulExtended(highp ivec3, highp ivec3, out highp ivec3, out highp ivec3 lsb);"
            "void imulExtended(highp ivec4, highp ivec4, out highp ivec4, out highp ivec4 lsb);"

            "  int bitfieldReverse(highp   int);"
            "ivec2 bitfieldReverse(highp ivec2);"
            "ivec3 bitfieldReverse(highp ivec3);"
            "ivec4 bitfieldReverse(highp ivec4);"

            " uint bitfieldReverse(highp  uint);"
            "uvec2 bitfieldReverse(highp uvec2);"
            "uvec3 bitfieldReverse(highp uvec3);"
            "uvec4 bitfieldReverse(highp uvec4);"

            "\n");
    }

    if (profile == EEsProfile && version >= 310) {
        commonBuiltins.append(
            "lowp   int bitCount(  int);"
            "lowp ivec2 bitCount(ivec2);"
            "lowp ivec3 bitCount(ivec3);"
            "lowp ivec4 bitCount(ivec4);"

            "lowp   int bitCount( uint);"
            "lowp ivec2 bitCount(uvec2);"
            "lowp ivec3 bitCount(uvec3);"
            "lowp ivec4 bitCount(uvec4);"

            "lowp   int findMSB(highp   int);"
            "lowp ivec2 findMSB(highp ivec2);"
            "lowp ivec3 findMSB(highp ivec3);"
            "lowp ivec4 findMSB(highp ivec4);"

            "lowp   int findMSB(highp  uint);"
            "lowp ivec2 findMSB(highp uvec2);"
            "lowp ivec3 findMSB(highp uvec3);"
            "lowp ivec4 findMSB(highp uvec4);"

            "\n");
    }

    // GL_ARB_shader_ballot
    if (profile != EEsProfile && version >= 450) {
        commonBuiltins.append(
            "uint64_t ballotARB(bool);"

            "float readInvocationARB(float, uint);"
            "vec2  readInvocationARB(vec2,  uint);"
            "vec3  readInvocationARB(vec3,  uint);"
            "vec4  readInvocationARB(vec4,  uint);"

            "int   readInvocationARB(int,   uint);"
            "ivec2 readInvocationARB(ivec2, uint);"
            "ivec3 readInvocationARB(ivec3, uint);"
            "ivec4 readInvocationARB(ivec4, uint);"

            "uint  readInvocationARB(uint,  uint);"
            "uvec2 readInvocationARB(uvec2, uint);"
            "uvec3 readInvocationARB(uvec3, uint);"
            "uvec4 readInvocationARB(uvec4, uint);"

            "float readFirstInvocationARB(float);"
            "vec2  readFirstInvocationARB(vec2);"
            "vec3  readFirstInvocationARB(vec3);"
            "vec4  readFirstInvocationARB(vec4);"

            "int   readFirstInvocationARB(int);"
            "ivec2 readFirstInvocationARB(ivec2);"
            "ivec3 readFirstInvocationARB(ivec3);"
            "ivec4 readFirstInvocationARB(ivec4);"

            "uint  readFirstInvocationARB(uint);"
            "uvec2 readFirstInvocationARB(uvec2);"
            "uvec3 readFirstInvocationARB(uvec3);"
            "uvec4 readFirstInvocationARB(uvec4);"

            "\n");
    }

        // GL_ARB_shader_group_vote
    if (profile != EEsProfile && version >= 430) {
        commonBuiltins.append(
            "bool anyInvocationARB(bool);"
            "bool allInvocationsARB(bool);"
            "bool allInvocationsEqualARB(bool);"

            "\n");
    }

#ifdef AMD_EXTENSIONS
    // GL_AMD_shader_ballot
    if (profile != EEsProfile && version >= 450) {
        commonBuiltins.append(
            "float minInvocationsAMD(float);"
            "vec2  minInvocationsAMD(vec2);"
            "vec3  minInvocationsAMD(vec3);"
            "vec4  minInvocationsAMD(vec4);"

            "int   minInvocationsAMD(int);"
            "ivec2 minInvocationsAMD(ivec2);"
            "ivec3 minInvocationsAMD(ivec3);"
            "ivec4 minInvocationsAMD(ivec4);"

            "uint  minInvocationsAMD(uint);"
            "uvec2 minInvocationsAMD(uvec2);"
            "uvec3 minInvocationsAMD(uvec3);"
            "uvec4 minInvocationsAMD(uvec4);"

            "double minInvocationsAMD(double);"
            "dvec2  minInvocationsAMD(dvec2);"
            "dvec3  minInvocationsAMD(dvec3);"
            "dvec4  minInvocationsAMD(dvec4);"

            "int64_t minInvocationsAMD(int64_t);"
            "i64vec2 minInvocationsAMD(i64vec2);"
            "i64vec3 minInvocationsAMD(i64vec3);"
            "i64vec4 minInvocationsAMD(i64vec4);"

            "uint64_t minInvocationsAMD(uint64_t);"
            "u64vec2  minInvocationsAMD(u64vec2);"
            "u64vec3  minInvocationsAMD(u64vec3);"
            "u64vec4  minInvocationsAMD(u64vec4);"

            "float16_t minInvocationsAMD(float16_t);"
            "f16vec2   minInvocationsAMD(f16vec2);"
            "f16vec3   minInvocationsAMD(f16vec3);"
            "f16vec4   minInvocationsAMD(f16vec4);"

            "float minInvocationsInclusiveScanAMD(float);"
            "vec2  minInvocationsInclusiveScanAMD(vec2);"
            "vec3  minInvocationsInclusiveScanAMD(vec3);"
            "vec4  minInvocationsInclusiveScanAMD(vec4);"

            "int   minInvocationsInclusiveScanAMD(int);"
            "ivec2 minInvocationsInclusiveScanAMD(ivec2);"
            "ivec3 minInvocationsInclusiveScanAMD(ivec3);"
            "ivec4 minInvocationsInclusiveScanAMD(ivec4);"

            "uint  minInvocationsInclusiveScanAMD(uint);"
            "uvec2 minInvocationsInclusiveScanAMD(uvec2);"
            "uvec3 minInvocationsInclusiveScanAMD(uvec3);"
            "uvec4 minInvocationsInclusiveScanAMD(uvec4);"

            "double minInvocationsInclusiveScanAMD(double);"
            "dvec2  minInvocationsInclusiveScanAMD(dvec2);"
            "dvec3  minInvocationsInclusiveScanAMD(dvec3);"
            "dvec4  minInvocationsInclusiveScanAMD(dvec4);"

            "int64_t minInvocationsInclusiveScanAMD(int64_t);"
            "i64vec2 minInvocationsInclusiveScanAMD(i64vec2);"
            "i64vec3 minInvocationsInclusiveScanAMD(i64vec3);"
            "i64vec4 minInvocationsInclusiveScanAMD(i64vec4);"

            "uint64_t minInvocationsInclusiveScanAMD(uint64_t);"
            "u64vec2  minInvocationsInclusiveScanAMD(u64vec2);"
            "u64vec3  minInvocationsInclusiveScanAMD(u64vec3);"
            "u64vec4  minInvocationsInclusiveScanAMD(u64vec4);"

            "float16_t minInvocationsInclusiveScanAMD(float16_t);"
            "f16vec2   minInvocationsInclusiveScanAMD(f16vec2);"
            "f16vec3   minInvocationsInclusiveScanAMD(f16vec3);"
            "f16vec4   minInvocationsInclusiveScanAMD(f16vec4);"

            "float minInvocationsExclusiveScanAMD(float);"
            "vec2  minInvocationsExclusiveScanAMD(vec2);"
            "vec3  minInvocationsExclusiveScanAMD(vec3);"
            "vec4  minInvocationsExclusiveScanAMD(vec4);"

            "int   minInvocationsExclusiveScanAMD(int);"
            "ivec2 minInvocationsExclusiveScanAMD(ivec2);"
            "ivec3 minInvocationsExclusiveScanAMD(ivec3);"
            "ivec4 minInvocationsExclusiveScanAMD(ivec4);"

            "uint  minInvocationsExclusiveScanAMD(uint);"
            "uvec2 minInvocationsExclusiveScanAMD(uvec2);"
            "uvec3 minInvocationsExclusiveScanAMD(uvec3);"
            "uvec4 minInvocationsExclusiveScanAMD(uvec4);"

            "double minInvocationsExclusiveScanAMD(double);"
            "dvec2  minInvocationsExclusiveScanAMD(dvec2);"
            "dvec3  minInvocationsExclusiveScanAMD(dvec3);"
            "dvec4  minInvocationsExclusiveScanAMD(dvec4);"

            "int64_t minInvocationsExclusiveScanAMD(int64_t);"
            "i64vec2 minInvocationsExclusiveScanAMD(i64vec2);"
            "i64vec3 minInvocationsExclusiveScanAMD(i64vec3);"
            "i64vec4 minInvocationsExclusiveScanAMD(i64vec4);"

            "uint64_t minInvocationsExclusiveScanAMD(uint64_t);"
            "u64vec2  minInvocationsExclusiveScanAMD(u64vec2);"
            "u64vec3  minInvocationsExclusiveScanAMD(u64vec3);"
            "u64vec4  minInvocationsExclusiveScanAMD(u64vec4);"

            "float16_t minInvocationsExclusiveScanAMD(float16_t);"
            "f16vec2   minInvocationsExclusiveScanAMD(f16vec2);"
            "f16vec3   minInvocationsExclusiveScanAMD(f16vec3);"
            "f16vec4   minInvocationsExclusiveScanAMD(f16vec4);"

            "float maxInvocationsAMD(float);"
            "vec2  maxInvocationsAMD(vec2);"
            "vec3  maxInvocationsAMD(vec3);"
            "vec4  maxInvocationsAMD(vec4);"

            "int   maxInvocationsAMD(int);"
            "ivec2 maxInvocationsAMD(ivec2);"
            "ivec3 maxInvocationsAMD(ivec3);"
            "ivec4 maxInvocationsAMD(ivec4);"

            "uint  maxInvocationsAMD(uint);"
            "uvec2 maxInvocationsAMD(uvec2);"
            "uvec3 maxInvocationsAMD(uvec3);"
            "uvec4 maxInvocationsAMD(uvec4);"

            "double maxInvocationsAMD(double);"
            "dvec2  maxInvocationsAMD(dvec2);"
            "dvec3  maxInvocationsAMD(dvec3);"
            "dvec4  maxInvocationsAMD(dvec4);"

            "int64_t maxInvocationsAMD(int64_t);"
            "i64vec2 maxInvocationsAMD(i64vec2);"
            "i64vec3 maxInvocationsAMD(i64vec3);"
            "i64vec4 maxInvocationsAMD(i64vec4);"

            "uint64_t maxInvocationsAMD(uint64_t);"
            "u64vec2  maxInvocationsAMD(u64vec2);"
            "u64vec3  maxInvocationsAMD(u64vec3);"
            "u64vec4  maxInvocationsAMD(u64vec4);"

            "float16_t maxInvocationsAMD(float16_t);"
            "f16vec2   maxInvocationsAMD(f16vec2);"
            "f16vec3   maxInvocationsAMD(f16vec3);"
            "f16vec4   maxInvocationsAMD(f16vec4);"

            "float maxInvocationsInclusiveScanAMD(float);"
            "vec2  maxInvocationsInclusiveScanAMD(vec2);"
            "vec3  maxInvocationsInclusiveScanAMD(vec3);"
            "vec4  maxInvocationsInclusiveScanAMD(vec4);"

            "int   maxInvocationsInclusiveScanAMD(int);"
            "ivec2 maxInvocationsInclusiveScanAMD(ivec2);"
            "ivec3 maxInvocationsInclusiveScanAMD(ivec3);"
            "ivec4 maxInvocationsInclusiveScanAMD(ivec4);"

            "uint  maxInvocationsInclusiveScanAMD(uint);"
            "uvec2 maxInvocationsInclusiveScanAMD(uvec2);"
            "uvec3 maxInvocationsInclusiveScanAMD(uvec3);"
            "uvec4 maxInvocationsInclusiveScanAMD(uvec4);"

            "double maxInvocationsInclusiveScanAMD(double);"
            "dvec2  maxInvocationsInclusiveScanAMD(dvec2);"
            "dvec3  maxInvocationsInclusiveScanAMD(dvec3);"
            "dvec4  maxInvocationsInclusiveScanAMD(dvec4);"

            "int64_t maxInvocationsInclusiveScanAMD(int64_t);"
            "i64vec2 maxInvocationsInclusiveScanAMD(i64vec2);"
            "i64vec3 maxInvocationsInclusiveScanAMD(i64vec3);"
            "i64vec4 maxInvocationsInclusiveScanAMD(i64vec4);"

            "uint64_t maxInvocationsInclusiveScanAMD(uint64_t);"
            "u64vec2  maxInvocationsInclusiveScanAMD(u64vec2);"
            "u64vec3  maxInvocationsInclusiveScanAMD(u64vec3);"
            "u64vec4  maxInvocationsInclusiveScanAMD(u64vec4);"

            "float16_t maxInvocationsInclusiveScanAMD(float16_t);"
            "f16vec2   maxInvocationsInclusiveScanAMD(f16vec2);"
            "f16vec3   maxInvocationsInclusiveScanAMD(f16vec3);"
            "f16vec4   maxInvocationsInclusiveScanAMD(f16vec4);"

            "float maxInvocationsExclusiveScanAMD(float);"
            "vec2  maxInvocationsExclusiveScanAMD(vec2);"
            "vec3  maxInvocationsExclusiveScanAMD(vec3);"
            "vec4  maxInvocationsExclusiveScanAMD(vec4);"

            "int   maxInvocationsExclusiveScanAMD(int);"
            "ivec2 maxInvocationsExclusiveScanAMD(ivec2);"
            "ivec3 maxInvocationsExclusiveScanAMD(ivec3);"
            "ivec4 maxInvocationsExclusiveScanAMD(ivec4);"

            "uint  maxInvocationsExclusiveScanAMD(uint);"
            "uvec2 maxInvocationsExclusiveScanAMD(uvec2);"
            "uvec3 maxInvocationsExclusiveScanAMD(uvec3);"
            "uvec4 maxInvocationsExclusiveScanAMD(uvec4);"

            "double maxInvocationsExclusiveScanAMD(double);"
            "dvec2  maxInvocationsExclusiveScanAMD(dvec2);"
            "dvec3  maxInvocationsExclusiveScanAMD(dvec3);"
            "dvec4  maxInvocationsExclusiveScanAMD(dvec4);"

            "int64_t maxInvocationsExclusiveScanAMD(int64_t);"
            "i64vec2 maxInvocationsExclusiveScanAMD(i64vec2);"
            "i64vec3 maxInvocationsExclusiveScanAMD(i64vec3);"
            "i64vec4 maxInvocationsExclusiveScanAMD(i64vec4);"

            "uint64_t maxInvocationsExclusiveScanAMD(uint64_t);"
            "u64vec2  maxInvocationsExclusiveScanAMD(u64vec2);"
            "u64vec3  maxInvocationsExclusiveScanAMD(u64vec3);"
            "u64vec4  maxInvocationsExclusiveScanAMD(u64vec4);"

            "float16_t maxInvocationsExclusiveScanAMD(float16_t);"
            "f16vec2   maxInvocationsExclusiveScanAMD(f16vec2);"
            "f16vec3   maxInvocationsExclusiveScanAMD(f16vec3);"
            "f16vec4   maxInvocationsExclusiveScanAMD(f16vec4);"

            "float addInvocationsAMD(float);"
            "vec2  addInvocationsAMD(vec2);"
            "vec3  addInvocationsAMD(vec3);"
            "vec4  addInvocationsAMD(vec4);"

            "int   addInvocationsAMD(int);"
            "ivec2 addInvocationsAMD(ivec2);"
            "ivec3 addInvocationsAMD(ivec3);"
            "ivec4 addInvocationsAMD(ivec4);"

            "uint  addInvocationsAMD(uint);"
            "uvec2 addInvocationsAMD(uvec2);"
            "uvec3 addInvocationsAMD(uvec3);"
            "uvec4 addInvocationsAMD(uvec4);"

            "double  addInvocationsAMD(double);"
            "dvec2   addInvocationsAMD(dvec2);"
            "dvec3   addInvocationsAMD(dvec3);"
            "dvec4   addInvocationsAMD(dvec4);"

            "int64_t addInvocationsAMD(int64_t);"
            "i64vec2 addInvocationsAMD(i64vec2);"
            "i64vec3 addInvocationsAMD(i64vec3);"
            "i64vec4 addInvocationsAMD(i64vec4);"

            "uint64_t addInvocationsAMD(uint64_t);"
            "u64vec2  addInvocationsAMD(u64vec2);"
            "u64vec3  addInvocationsAMD(u64vec3);"
            "u64vec4  addInvocationsAMD(u64vec4);"

            "float16_t addInvocationsAMD(float16_t);"
            "f16vec2   addInvocationsAMD(f16vec2);"
            "f16vec3   addInvocationsAMD(f16vec3);"
            "f16vec4   addInvocationsAMD(f16vec4);"

            "float addInvocationsInclusiveScanAMD(float);"
            "vec2  addInvocationsInclusiveScanAMD(vec2);"
            "vec3  addInvocationsInclusiveScanAMD(vec3);"
            "vec4  addInvocationsInclusiveScanAMD(vec4);"

            "int   addInvocationsInclusiveScanAMD(int);"
            "ivec2 addInvocationsInclusiveScanAMD(ivec2);"
            "ivec3 addInvocationsInclusiveScanAMD(ivec3);"
            "ivec4 addInvocationsInclusiveScanAMD(ivec4);"

            "uint  addInvocationsInclusiveScanAMD(uint);"
            "uvec2 addInvocationsInclusiveScanAMD(uvec2);"
            "uvec3 addInvocationsInclusiveScanAMD(uvec3);"
            "uvec4 addInvocationsInclusiveScanAMD(uvec4);"

            "double  addInvocationsInclusiveScanAMD(double);"
            "dvec2   addInvocationsInclusiveScanAMD(dvec2);"
            "dvec3   addInvocationsInclusiveScanAMD(dvec3);"
            "dvec4   addInvocationsInclusiveScanAMD(dvec4);"

            "int64_t addInvocationsInclusiveScanAMD(int64_t);"
            "i64vec2 addInvocationsInclusiveScanAMD(i64vec2);"
            "i64vec3 addInvocationsInclusiveScanAMD(i64vec3);"
            "i64vec4 addInvocationsInclusiveScanAMD(i64vec4);"

            "uint64_t addInvocationsInclusiveScanAMD(uint64_t);"
            "u64vec2  addInvocationsInclusiveScanAMD(u64vec2);"
            "u64vec3  addInvocationsInclusiveScanAMD(u64vec3);"
            "u64vec4  addInvocationsInclusiveScanAMD(u64vec4);"

            "float16_t addInvocationsInclusiveScanAMD(float16_t);"
            "f16vec2   addInvocationsInclusiveScanAMD(f16vec2);"
            "f16vec3   addInvocationsInclusiveScanAMD(f16vec3);"
            "f16vec4   addInvocationsInclusiveScanAMD(f16vec4);"

            "float addInvocationsExclusiveScanAMD(float);"
            "vec2  addInvocationsExclusiveScanAMD(vec2);"
            "vec3  addInvocationsExclusiveScanAMD(vec3);"
            "vec4  addInvocationsExclusiveScanAMD(vec4);"

            "int   addInvocationsExclusiveScanAMD(int);"
            "ivec2 addInvocationsExclusiveScanAMD(ivec2);"
            "ivec3 addInvocationsExclusiveScanAMD(ivec3);"
            "ivec4 addInvocationsExclusiveScanAMD(ivec4);"

            "uint  addInvocationsExclusiveScanAMD(uint);"
            "uvec2 addInvocationsExclusiveScanAMD(uvec2);"
            "uvec3 addInvocationsExclusiveScanAMD(uvec3);"
            "uvec4 addInvocationsExclusiveScanAMD(uvec4);"

            "double  addInvocationsExclusiveScanAMD(double);"
            "dvec2   addInvocationsExclusiveScanAMD(dvec2);"
            "dvec3   addInvocationsExclusiveScanAMD(dvec3);"
            "dvec4   addInvocationsExclusiveScanAMD(dvec4);"

            "int64_t addInvocationsExclusiveScanAMD(int64_t);"
            "i64vec2 addInvocationsExclusiveScanAMD(i64vec2);"
            "i64vec3 addInvocationsExclusiveScanAMD(i64vec3);"
            "i64vec4 addInvocationsExclusiveScanAMD(i64vec4);"

            "uint64_t addInvocationsExclusiveScanAMD(uint64_t);"
            "u64vec2  addInvocationsExclusiveScanAMD(u64vec2);"
            "u64vec3  addInvocationsExclusiveScanAMD(u64vec3);"
            "u64vec4  addInvocationsExclusiveScanAMD(u64vec4);"

            "float16_t addInvocationsExclusiveScanAMD(float16_t);"
            "f16vec2   addInvocationsExclusiveScanAMD(f16vec2);"
            "f16vec3   addInvocationsExclusiveScanAMD(f16vec3);"
            "f16vec4   addInvocationsExclusiveScanAMD(f16vec4);"

            "float minInvocationsNonUniformAMD(float);"
            "vec2  minInvocationsNonUniformAMD(vec2);"
            "vec3  minInvocationsNonUniformAMD(vec3);"
            "vec4  minInvocationsNonUniformAMD(vec4);"

            "int   minInvocationsNonUniformAMD(int);"
            "ivec2 minInvocationsNonUniformAMD(ivec2);"
            "ivec3 minInvocationsNonUniformAMD(ivec3);"
            "ivec4 minInvocationsNonUniformAMD(ivec4);"

            "uint  minInvocationsNonUniformAMD(uint);"
            "uvec2 minInvocationsNonUniformAMD(uvec2);"
            "uvec3 minInvocationsNonUniformAMD(uvec3);"
            "uvec4 minInvocationsNonUniformAMD(uvec4);"

            "double minInvocationsNonUniformAMD(double);"
            "dvec2  minInvocationsNonUniformAMD(dvec2);"
            "dvec3  minInvocationsNonUniformAMD(dvec3);"
            "dvec4  minInvocationsNonUniformAMD(dvec4);"

            "int64_t minInvocationsNonUniformAMD(int64_t);"
            "i64vec2 minInvocationsNonUniformAMD(i64vec2);"
            "i64vec3 minInvocationsNonUniformAMD(i64vec3);"
            "i64vec4 minInvocationsNonUniformAMD(i64vec4);"

            "uint64_t minInvocationsNonUniformAMD(uint64_t);"
            "u64vec2  minInvocationsNonUniformAMD(u64vec2);"
            "u64vec3  minInvocationsNonUniformAMD(u64vec3);"
            "u64vec4  minInvocationsNonUniformAMD(u64vec4);"

            "float16_t minInvocationsNonUniformAMD(float16_t);"
            "f16vec2   minInvocationsNonUniformAMD(f16vec2);"
            "f16vec3   minInvocationsNonUniformAMD(f16vec3);"
            "f16vec4   minInvocationsNonUniformAMD(f16vec4);"

            "float minInvocationsInclusiveScanNonUniformAMD(float);"
            "vec2  minInvocationsInclusiveScanNonUniformAMD(vec2);"
            "vec3  minInvocationsInclusiveScanNonUniformAMD(vec3);"
            "vec4  minInvocationsInclusiveScanNonUniformAMD(vec4);"

            "int   minInvocationsInclusiveScanNonUniformAMD(int);"
            "ivec2 minInvocationsInclusiveScanNonUniformAMD(ivec2);"
            "ivec3 minInvocationsInclusiveScanNonUniformAMD(ivec3);"
            "ivec4 minInvocationsInclusiveScanNonUniformAMD(ivec4);"

            "uint  minInvocationsInclusiveScanNonUniformAMD(uint);"
            "uvec2 minInvocationsInclusiveScanNonUniformAMD(uvec2);"
            "uvec3 minInvocationsInclusiveScanNonUniformAMD(uvec3);"
            "uvec4 minInvocationsInclusiveScanNonUniformAMD(uvec4);"

            "double minInvocationsInclusiveScanNonUniformAMD(double);"
            "dvec2  minInvocationsInclusiveScanNonUniformAMD(dvec2);"
            "dvec3  minInvocationsInclusiveScanNonUniformAMD(dvec3);"
            "dvec4  minInvocationsInclusiveScanNonUniformAMD(dvec4);"

            "int64_t minInvocationsInclusiveScanNonUniformAMD(int64_t);"
            "i64vec2 minInvocationsInclusiveScanNonUniformAMD(i64vec2);"
            "i64vec3 minInvocationsInclusiveScanNonUniformAMD(i64vec3);"
            "i64vec4 minInvocationsInclusiveScanNonUniformAMD(i64vec4);"

            "uint64_t minInvocationsInclusiveScanNonUniformAMD(uint64_t);"
            "u64vec2  minInvocationsInclusiveScanNonUniformAMD(u64vec2);"
            "u64vec3  minInvocationsInclusiveScanNonUniformAMD(u64vec3);"
            "u64vec4  minInvocationsInclusiveScanNonUniformAMD(u64vec4);"

            "float16_t minInvocationsInclusiveScanNonUniformAMD(float16_t);"
            "f16vec2   minInvocationsInclusiveScanNonUniformAMD(f16vec2);"
            "f16vec3   minInvocationsInclusiveScanNonUniformAMD(f16vec3);"
            "f16vec4   minInvocationsInclusiveScanNonUniformAMD(f16vec4);"

            "float minInvocationsExclusiveScanNonUniformAMD(float);"
            "vec2  minInvocationsExclusiveScanNonUniformAMD(vec2);"
            "vec3  minInvocationsExclusiveScanNonUniformAMD(vec3);"
            "vec4  minInvocationsExclusiveScanNonUniformAMD(vec4);"

            "int   minInvocationsExclusiveScanNonUniformAMD(int);"
            "ivec2 minInvocationsExclusiveScanNonUniformAMD(ivec2);"
            "ivec3 minInvocationsExclusiveScanNonUniformAMD(ivec3);"
            "ivec4 minInvocationsExclusiveScanNonUniformAMD(ivec4);"

            "uint  minInvocationsExclusiveScanNonUniformAMD(uint);"
            "uvec2 minInvocationsExclusiveScanNonUniformAMD(uvec2);"
            "uvec3 minInvocationsExclusiveScanNonUniformAMD(uvec3);"
            "uvec4 minInvocationsExclusiveScanNonUniformAMD(uvec4);"

            "double minInvocationsExclusiveScanNonUniformAMD(double);"
            "dvec2  minInvocationsExclusiveScanNonUniformAMD(dvec2);"
            "dvec3  minInvocationsExclusiveScanNonUniformAMD(dvec3);"
            "dvec4  minInvocationsExclusiveScanNonUniformAMD(dvec4);"

            "int64_t minInvocationsExclusiveScanNonUniformAMD(int64_t);"
            "i64vec2 minInvocationsExclusiveScanNonUniformAMD(i64vec2);"
            "i64vec3 minInvocationsExclusiveScanNonUniformAMD(i64vec3);"
            "i64vec4 minInvocationsExclusiveScanNonUniformAMD(i64vec4);"

            "uint64_t minInvocationsExclusiveScanNonUniformAMD(uint64_t);"
            "u64vec2  minInvocationsExclusiveScanNonUniformAMD(u64vec2);"
            "u64vec3  minInvocationsExclusiveScanNonUniformAMD(u64vec3);"
            "u64vec4  minInvocationsExclusiveScanNonUniformAMD(u64vec4);"

            "float16_t minInvocationsExclusiveScanNonUniformAMD(float16_t);"
            "f16vec2   minInvocationsExclusiveScanNonUniformAMD(f16vec2);"
            "f16vec3   minInvocationsExclusiveScanNonUniformAMD(f16vec3);"
            "f16vec4   minInvocationsExclusiveScanNonUniformAMD(f16vec4);"

            "float maxInvocationsNonUniformAMD(float);"
            "vec2  maxInvocationsNonUniformAMD(vec2);"
            "vec3  maxInvocationsNonUniformAMD(vec3);"
            "vec4  maxInvocationsNonUniformAMD(vec4);"

            "int   maxInvocationsNonUniformAMD(int);"
            "ivec2 maxInvocationsNonUniformAMD(ivec2);"
            "ivec3 maxInvocationsNonUniformAMD(ivec3);"
            "ivec4 maxInvocationsNonUniformAMD(ivec4);"

            "uint  maxInvocationsNonUniformAMD(uint);"
            "uvec2 maxInvocationsNonUniformAMD(uvec2);"
            "uvec3 maxInvocationsNonUniformAMD(uvec3);"
            "uvec4 maxInvocationsNonUniformAMD(uvec4);"

            "double maxInvocationsNonUniformAMD(double);"
            "dvec2  maxInvocationsNonUniformAMD(dvec2);"
            "dvec3  maxInvocationsNonUniformAMD(dvec3);"
            "dvec4  maxInvocationsNonUniformAMD(dvec4);"

            "int64_t maxInvocationsNonUniformAMD(int64_t);"
            "i64vec2 maxInvocationsNonUniformAMD(i64vec2);"
            "i64vec3 maxInvocationsNonUniformAMD(i64vec3);"
            "i64vec4 maxInvocationsNonUniformAMD(i64vec4);"

            "uint64_t maxInvocationsNonUniformAMD(uint64_t);"
            "u64vec2  maxInvocationsNonUniformAMD(u64vec2);"
            "u64vec3  maxInvocationsNonUniformAMD(u64vec3);"
            "u64vec4  maxInvocationsNonUniformAMD(u64vec4);"

            "float16_t maxInvocationsNonUniformAMD(float16_t);"
            "f16vec2   maxInvocationsNonUniformAMD(f16vec2);"
            "f16vec3   maxInvocationsNonUniformAMD(f16vec3);"
            "f16vec4   maxInvocationsNonUniformAMD(f16vec4);"

            "float maxInvocationsInclusiveScanNonUniformAMD(float);"
            "vec2  maxInvocationsInclusiveScanNonUniformAMD(vec2);"
            "vec3  maxInvocationsInclusiveScanNonUniformAMD(vec3);"
            "vec4  maxInvocationsInclusiveScanNonUniformAMD(vec4);"

            "int   maxInvocationsInclusiveScanNonUniformAMD(int);"
            "ivec2 maxInvocationsInclusiveScanNonUniformAMD(ivec2);"
            "ivec3 maxInvocationsInclusiveScanNonUniformAMD(ivec3);"
            "ivec4 maxInvocationsInclusiveScanNonUniformAMD(ivec4);"

            "uint  maxInvocationsInclusiveScanNonUniformAMD(uint);"
            "uvec2 maxInvocationsInclusiveScanNonUniformAMD(uvec2);"
            "uvec3 maxInvocationsInclusiveScanNonUniformAMD(uvec3);"
            "uvec4 maxInvocationsInclusiveScanNonUniformAMD(uvec4);"

            "double maxInvocationsInclusiveScanNonUniformAMD(double);"
            "dvec2  maxInvocationsInclusiveScanNonUniformAMD(dvec2);"
            "dvec3  maxInvocationsInclusiveScanNonUniformAMD(dvec3);"
            "dvec4  maxInvocationsInclusiveScanNonUniformAMD(dvec4);"

            "int64_t maxInvocationsInclusiveScanNonUniformAMD(int64_t);"
            "i64vec2 maxInvocationsInclusiveScanNonUniformAMD(i64vec2);"
            "i64vec3 maxInvocationsInclusiveScanNonUniformAMD(i64vec3);"
            "i64vec4 maxInvocationsInclusiveScanNonUniformAMD(i64vec4);"

            "uint64_t maxInvocationsInclusiveScanNonUniformAMD(uint64_t);"
            "u64vec2  maxInvocationsInclusiveScanNonUniformAMD(u64vec2);"
            "u64vec3  maxInvocationsInclusiveScanNonUniformAMD(u64vec3);"
            "u64vec4  maxInvocationsInclusiveScanNonUniformAMD(u64vec4);"

            "float16_t maxInvocationsInclusiveScanNonUniformAMD(float16_t);"
            "f16vec2   maxInvocationsInclusiveScanNonUniformAMD(f16vec2);"
            "f16vec3   maxInvocationsInclusiveScanNonUniformAMD(f16vec3);"
            "f16vec4   maxInvocationsInclusiveScanNonUniformAMD(f16vec4);"

            "float maxInvocationsExclusiveScanNonUniformAMD(float);"
            "vec2  maxInvocationsExclusiveScanNonUniformAMD(vec2);"
            "vec3  maxInvocationsExclusiveScanNonUniformAMD(vec3);"
            "vec4  maxInvocationsExclusiveScanNonUniformAMD(vec4);"

            "int   maxInvocationsExclusiveScanNonUniformAMD(int);"
            "ivec2 maxInvocationsExclusiveScanNonUniformAMD(ivec2);"
            "ivec3 maxInvocationsExclusiveScanNonUniformAMD(ivec3);"
            "ivec4 maxInvocationsExclusiveScanNonUniformAMD(ivec4);"

            "uint  maxInvocationsExclusiveScanNonUniformAMD(uint);"
            "uvec2 maxInvocationsExclusiveScanNonUniformAMD(uvec2);"
            "uvec3 maxInvocationsExclusiveScanNonUniformAMD(uvec3);"
            "uvec4 maxInvocationsExclusiveScanNonUniformAMD(uvec4);"

            "double maxInvocationsExclusiveScanNonUniformAMD(double);"
            "dvec2  maxInvocationsExclusiveScanNonUniformAMD(dvec2);"
            "dvec3  maxInvocationsExclusiveScanNonUniformAMD(dvec3);"
            "dvec4  maxInvocationsExclusiveScanNonUniformAMD(dvec4);"

            "int64_t maxInvocationsExclusiveScanNonUniformAMD(int64_t);"
            "i64vec2 maxInvocationsExclusiveScanNonUniformAMD(i64vec2);"
            "i64vec3 maxInvocationsExclusiveScanNonUniformAMD(i64vec3);"
            "i64vec4 maxInvocationsExclusiveScanNonUniformAMD(i64vec4);"

            "uint64_t maxInvocationsExclusiveScanNonUniformAMD(uint64_t);"
            "u64vec2  maxInvocationsExclusiveScanNonUniformAMD(u64vec2);"
            "u64vec3  maxInvocationsExclusiveScanNonUniformAMD(u64vec3);"
            "u64vec4  maxInvocationsExclusiveScanNonUniformAMD(u64vec4);"

            "float16_t maxInvocationsExclusiveScanNonUniformAMD(float16_t);"
            "f16vec2   maxInvocationsExclusiveScanNonUniformAMD(f16vec2);"
            "f16vec3   maxInvocationsExclusiveScanNonUniformAMD(f16vec3);"
            "f16vec4   maxInvocationsExclusiveScanNonUniformAMD(f16vec4);"

            "float addInvocationsNonUniformAMD(float);"
            "vec2  addInvocationsNonUniformAMD(vec2);"
            "vec3  addInvocationsNonUniformAMD(vec3);"
            "vec4  addInvocationsNonUniformAMD(vec4);"

            "int   addInvocationsNonUniformAMD(int);"
            "ivec2 addInvocationsNonUniformAMD(ivec2);"
            "ivec3 addInvocationsNonUniformAMD(ivec3);"
            "ivec4 addInvocationsNonUniformAMD(ivec4);"

            "uint  addInvocationsNonUniformAMD(uint);"
            "uvec2 addInvocationsNonUniformAMD(uvec2);"
            "uvec3 addInvocationsNonUniformAMD(uvec3);"
            "uvec4 addInvocationsNonUniformAMD(uvec4);"

            "double addInvocationsNonUniformAMD(double);"
            "dvec2  addInvocationsNonUniformAMD(dvec2);"
            "dvec3  addInvocationsNonUniformAMD(dvec3);"
            "dvec4  addInvocationsNonUniformAMD(dvec4);"

            "int64_t addInvocationsNonUniformAMD(int64_t);"
            "i64vec2 addInvocationsNonUniformAMD(i64vec2);"
            "i64vec3 addInvocationsNonUniformAMD(i64vec3);"
            "i64vec4 addInvocationsNonUniformAMD(i64vec4);"

            "uint64_t addInvocationsNonUniformAMD(uint64_t);"
            "u64vec2  addInvocationsNonUniformAMD(u64vec2);"
            "u64vec3  addInvocationsNonUniformAMD(u64vec3);"
            "u64vec4  addInvocationsNonUniformAMD(u64vec4);"

            "float16_t addInvocationsNonUniformAMD(float16_t);"
            "f16vec2   addInvocationsNonUniformAMD(f16vec2);"
            "f16vec3   addInvocationsNonUniformAMD(f16vec3);"
            "f16vec4   addInvocationsNonUniformAMD(f16vec4);"

            "float addInvocationsInclusiveScanNonUniformAMD(float);"
            "vec2  addInvocationsInclusiveScanNonUniformAMD(vec2);"
            "vec3  addInvocationsInclusiveScanNonUniformAMD(vec3);"
            "vec4  addInvocationsInclusiveScanNonUniformAMD(vec4);"

            "int   addInvocationsInclusiveScanNonUniformAMD(int);"
            "ivec2 addInvocationsInclusiveScanNonUniformAMD(ivec2);"
            "ivec3 addInvocationsInclusiveScanNonUniformAMD(ivec3);"
            "ivec4 addInvocationsInclusiveScanNonUniformAMD(ivec4);"

            "uint  addInvocationsInclusiveScanNonUniformAMD(uint);"
            "uvec2 addInvocationsInclusiveScanNonUniformAMD(uvec2);"
            "uvec3 addInvocationsInclusiveScanNonUniformAMD(uvec3);"
            "uvec4 addInvocationsInclusiveScanNonUniformAMD(uvec4);"

            "double addInvocationsInclusiveScanNonUniformAMD(double);"
            "dvec2  addInvocationsInclusiveScanNonUniformAMD(dvec2);"
            "dvec3  addInvocationsInclusiveScanNonUniformAMD(dvec3);"
            "dvec4  addInvocationsInclusiveScanNonUniformAMD(dvec4);"

            "int64_t addInvocationsInclusiveScanNonUniformAMD(int64_t);"
            "i64vec2 addInvocationsInclusiveScanNonUniformAMD(i64vec2);"
            "i64vec3 addInvocationsInclusiveScanNonUniformAMD(i64vec3);"
            "i64vec4 addInvocationsInclusiveScanNonUniformAMD(i64vec4);"

            "uint64_t addInvocationsInclusiveScanNonUniformAMD(uint64_t);"
            "u64vec2  addInvocationsInclusiveScanNonUniformAMD(u64vec2);"
            "u64vec3  addInvocationsInclusiveScanNonUniformAMD(u64vec3);"
            "u64vec4  addInvocationsInclusiveScanNonUniformAMD(u64vec4);"

            "float16_t addInvocationsInclusiveScanNonUniformAMD(float16_t);"
            "f16vec2   addInvocationsInclusiveScanNonUniformAMD(f16vec2);"
            "f16vec3   addInvocationsInclusiveScanNonUniformAMD(f16vec3);"
            "f16vec4   addInvocationsInclusiveScanNonUniformAMD(f16vec4);"

            "float addInvocationsExclusiveScanNonUniformAMD(float);"
            "vec2  addInvocationsExclusiveScanNonUniformAMD(vec2);"
            "vec3  addInvocationsExclusiveScanNonUniformAMD(vec3);"
            "vec4  addInvocationsExclusiveScanNonUniformAMD(vec4);"

            "int   addInvocationsExclusiveScanNonUniformAMD(int);"
            "ivec2 addInvocationsExclusiveScanNonUniformAMD(ivec2);"
            "ivec3 addInvocationsExclusiveScanNonUniformAMD(ivec3);"
            "ivec4 addInvocationsExclusiveScanNonUniformAMD(ivec4);"

            "uint  addInvocationsExclusiveScanNonUniformAMD(uint);"
            "uvec2 addInvocationsExclusiveScanNonUniformAMD(uvec2);"
            "uvec3 addInvocationsExclusiveScanNonUniformAMD(uvec3);"
            "uvec4 addInvocationsExclusiveScanNonUniformAMD(uvec4);"

            "double addInvocationsExclusiveScanNonUniformAMD(double);"
            "dvec2  addInvocationsExclusiveScanNonUniformAMD(dvec2);"
            "dvec3  addInvocationsExclusiveScanNonUniformAMD(dvec3);"
            "dvec4  addInvocationsExclusiveScanNonUniformAMD(dvec4);"

            "int64_t addInvocationsExclusiveScanNonUniformAMD(int64_t);"
            "i64vec2 addInvocationsExclusiveScanNonUniformAMD(i64vec2);"
            "i64vec3 addInvocationsExclusiveScanNonUniformAMD(i64vec3);"
            "i64vec4 addInvocationsExclusiveScanNonUniformAMD(i64vec4);"

            "uint64_t addInvocationsExclusiveScanNonUniformAMD(uint64_t);"
            "u64vec2  addInvocationsExclusiveScanNonUniformAMD(u64vec2);"
            "u64vec3  addInvocationsExclusiveScanNonUniformAMD(u64vec3);"
            "u64vec4  addInvocationsExclusiveScanNonUniformAMD(u64vec4);"

            "float16_t addInvocationsExclusiveScanNonUniformAMD(float16_t);"
            "f16vec2   addInvocationsExclusiveScanNonUniformAMD(f16vec2);"
            "f16vec3   addInvocationsExclusiveScanNonUniformAMD(f16vec3);"
            "f16vec4   addInvocationsExclusiveScanNonUniformAMD(f16vec4);"

            "float swizzleInvocationsAMD(float, uvec4);"
            "vec2  swizzleInvocationsAMD(vec2,  uvec4);"
            "vec3  swizzleInvocationsAMD(vec3,  uvec4);"
            "vec4  swizzleInvocationsAMD(vec4,  uvec4);"

            "int   swizzleInvocationsAMD(int,   uvec4);"
            "ivec2 swizzleInvocationsAMD(ivec2, uvec4);"
            "ivec3 swizzleInvocationsAMD(ivec3, uvec4);"
            "ivec4 swizzleInvocationsAMD(ivec4, uvec4);"

            "uint  swizzleInvocationsAMD(uint,  uvec4);"
            "uvec2 swizzleInvocationsAMD(uvec2, uvec4);"
            "uvec3 swizzleInvocationsAMD(uvec3, uvec4);"
            "uvec4 swizzleInvocationsAMD(uvec4, uvec4);"

            "float swizzleInvocationsMaskedAMD(float, uvec3);"
            "vec2  swizzleInvocationsMaskedAMD(vec2,  uvec3);"
            "vec3  swizzleInvocationsMaskedAMD(vec3,  uvec3);"
            "vec4  swizzleInvocationsMaskedAMD(vec4,  uvec3);"

            "int   swizzleInvocationsMaskedAMD(int,   uvec3);"
            "ivec2 swizzleInvocationsMaskedAMD(ivec2, uvec3);"
            "ivec3 swizzleInvocationsMaskedAMD(ivec3, uvec3);"
            "ivec4 swizzleInvocationsMaskedAMD(ivec4, uvec3);"

            "uint  swizzleInvocationsMaskedAMD(uint,  uvec3);"
            "uvec2 swizzleInvocationsMaskedAMD(uvec2, uvec3);"
            "uvec3 swizzleInvocationsMaskedAMD(uvec3, uvec3);"
            "uvec4 swizzleInvocationsMaskedAMD(uvec4, uvec3);"

            "float writeInvocationAMD(float, float, uint);"
            "vec2  writeInvocationAMD(vec2,  vec2,  uint);"
            "vec3  writeInvocationAMD(vec3,  vec3,  uint);"
            "vec4  writeInvocationAMD(vec4,  vec4,  uint);"

            "int   writeInvocationAMD(int,   int,   uint);"
            "ivec2 writeInvocationAMD(ivec2, ivec2, uint);"
            "ivec3 writeInvocationAMD(ivec3, ivec3, uint);"
            "ivec4 writeInvocationAMD(ivec4, ivec4, uint);"

            "uint  writeInvocationAMD(uint,  uint,  uint);"
            "uvec2 writeInvocationAMD(uvec2, uvec2, uint);"
            "uvec3 writeInvocationAMD(uvec3, uvec3, uint);"
            "uvec4 writeInvocationAMD(uvec4, uvec4, uint);"

            "uint mbcntAMD(uint64_t);"

            "\n");
    }

    // GL_AMD_gcn_shader
    if (profile != EEsProfile && version >= 450) {
        commonBuiltins.append(
            "float cubeFaceIndexAMD(vec3);"
            "vec2 cubeFaceCoordAMD(vec3);"
            "uint64_t timeAMD();"

            "\n");
    }

    // GL_AMD_gpu_shader_half_float
    if (profile != EEsProfile && version >= 450) {
        commonBuiltins.append(
            "float16_t radians(float16_t);"
            "f16vec2   radians(f16vec2);"
            "f16vec3   radians(f16vec3);"
            "f16vec4   radians(f16vec4);"

            "float16_t degrees(float16_t);"
            "f16vec2   degrees(f16vec2);"
            "f16vec3   degrees(f16vec3);"
            "f16vec4   degrees(f16vec4);"

            "float16_t sin(float16_t);"
            "f16vec2   sin(f16vec2);"
            "f16vec3   sin(f16vec3);"
            "f16vec4   sin(f16vec4);"

            "float16_t cos(float16_t);"
            "f16vec2   cos(f16vec2);"
            "f16vec3   cos(f16vec3);"
            "f16vec4   cos(f16vec4);"

            "float16_t tan(float16_t);"
            "f16vec2   tan(f16vec2);"
            "f16vec3   tan(f16vec3);"
            "f16vec4   tan(f16vec4);"

            "float16_t asin(float16_t);"
            "f16vec2   asin(f16vec2);"
            "f16vec3   asin(f16vec3);"
            "f16vec4   asin(f16vec4);"

            "float16_t acos(float16_t);"
            "f16vec2   acos(f16vec2);"
            "f16vec3   acos(f16vec3);"
            "f16vec4   acos(f16vec4);"

            "float16_t atan(float16_t, float16_t);"
            "f16vec2   atan(f16vec2,   f16vec2);"
            "f16vec3   atan(f16vec3,   f16vec3);"
            "f16vec4   atan(f16vec4,   f16vec4);"

            "float16_t atan(float16_t);"
            "f16vec2   atan(f16vec2);"
            "f16vec3   atan(f16vec3);"
            "f16vec4   atan(f16vec4);"

            "float16_t sinh(float16_t);"
            "f16vec2   sinh(f16vec2);"
            "f16vec3   sinh(f16vec3);"
            "f16vec4   sinh(f16vec4);"

            "float16_t cosh(float16_t);"
            "f16vec2   cosh(f16vec2);"
            "f16vec3   cosh(f16vec3);"
            "f16vec4   cosh(f16vec4);"

            "float16_t tanh(float16_t);"
            "f16vec2   tanh(f16vec2);"
            "f16vec3   tanh(f16vec3);"
            "f16vec4   tanh(f16vec4);"

            "float16_t asinh(float16_t);"
            "f16vec2   asinh(f16vec2);"
            "f16vec3   asinh(f16vec3);"
            "f16vec4   asinh(f16vec4);"

            "float16_t acosh(float16_t);"
            "f16vec2   acosh(f16vec2);"
            "f16vec3   acosh(f16vec3);"
            "f16vec4   acosh(f16vec4);"

            "float16_t atanh(float16_t);"
            "f16vec2   atanh(f16vec2);"
            "f16vec3   atanh(f16vec3);"
            "f16vec4   atanh(f16vec4);"

            "float16_t pow(float16_t, float16_t);"
            "f16vec2   pow(f16vec2,   f16vec2);"
            "f16vec3   pow(f16vec3,   f16vec3);"
            "f16vec4   pow(f16vec4,   f16vec4);"

            "float16_t exp(float16_t);"
            "f16vec2   exp(f16vec2);"
            "f16vec3   exp(f16vec3);"
            "f16vec4   exp(f16vec4);"

            "float16_t log(float16_t);"
            "f16vec2   log(f16vec2);"
            "f16vec3   log(f16vec3);"
            "f16vec4   log(f16vec4);"

            "float16_t exp2(float16_t);"
            "f16vec2   exp2(f16vec2);"
            "f16vec3   exp2(f16vec3);"
            "f16vec4   exp2(f16vec4);"

            "float16_t log2(float16_t);"
            "f16vec2   log2(f16vec2);"
            "f16vec3   log2(f16vec3);"
            "f16vec4   log2(f16vec4);"

            "float16_t sqrt(float16_t);"
            "f16vec2   sqrt(f16vec2);"
            "f16vec3   sqrt(f16vec3);"
            "f16vec4   sqrt(f16vec4);"

            "float16_t inversesqrt(float16_t);"
            "f16vec2   inversesqrt(f16vec2);"
            "f16vec3   inversesqrt(f16vec3);"
            "f16vec4   inversesqrt(f16vec4);"

            "float16_t abs(float16_t);"
            "f16vec2   abs(f16vec2);"
            "f16vec3   abs(f16vec3);"
            "f16vec4   abs(f16vec4);"

            "float16_t sign(float16_t);"
            "f16vec2   sign(f16vec2);"
            "f16vec3   sign(f16vec3);"
            "f16vec4   sign(f16vec4);"

            "float16_t floor(float16_t);"
            "f16vec2   floor(f16vec2);"
            "f16vec3   floor(f16vec3);"
            "f16vec4   floor(f16vec4);"

            "float16_t trunc(float16_t);"
            "f16vec2   trunc(f16vec2);"
            "f16vec3   trunc(f16vec3);"
            "f16vec4   trunc(f16vec4);"

            "float16_t round(float16_t);"
            "f16vec2   round(f16vec2);"
            "f16vec3   round(f16vec3);"
            "f16vec4   round(f16vec4);"

            "float16_t roundEven(float16_t);"
            "f16vec2   roundEven(f16vec2);"
            "f16vec3   roundEven(f16vec3);"
            "f16vec4   roundEven(f16vec4);"

            "float16_t ceil(float16_t);"
            "f16vec2   ceil(f16vec2);"
            "f16vec3   ceil(f16vec3);"
            "f16vec4   ceil(f16vec4);"

            "float16_t fract(float16_t);"
            "f16vec2   fract(f16vec2);"
            "f16vec3   fract(f16vec3);"
            "f16vec4   fract(f16vec4);"

            "float16_t mod(float16_t, float16_t);"
            "f16vec2   mod(f16vec2,   float16_t);"
            "f16vec3   mod(f16vec3,   float16_t);"
            "f16vec4   mod(f16vec4,   float16_t);"
            "f16vec2   mod(f16vec2,   f16vec2);"
            "f16vec3   mod(f16vec3,   f16vec3);"
            "f16vec4   mod(f16vec4,   f16vec4);"

            "float16_t modf(float16_t, out float16_t);"
            "f16vec2   modf(f16vec2,   out f16vec2);"
            "f16vec3   modf(f16vec3,   out f16vec3);"
            "f16vec4   modf(f16vec4,   out f16vec4);"

            "float16_t min(float16_t, float16_t);"
            "f16vec2   min(f16vec2,   float16_t);"
            "f16vec3   min(f16vec3,   float16_t);"
            "f16vec4   min(f16vec4,   float16_t);"
            "f16vec2   min(f16vec2,   f16vec2);"
            "f16vec3   min(f16vec3,   f16vec3);"
            "f16vec4   min(f16vec4,   f16vec4);"

            "float16_t max(float16_t, float16_t);"
            "f16vec2   max(f16vec2,   float16_t);"
            "f16vec3   max(f16vec3,   float16_t);"
            "f16vec4   max(f16vec4,   float16_t);"
            "f16vec2   max(f16vec2,   f16vec2);"
            "f16vec3   max(f16vec3,   f16vec3);"
            "f16vec4   max(f16vec4,   f16vec4);"

            "float16_t clamp(float16_t, float16_t, float16_t);"
            "f16vec2   clamp(f16vec2,   float16_t, float16_t);"
            "f16vec3   clamp(f16vec3,   float16_t, float16_t);"
            "f16vec4   clamp(f16vec4,   float16_t, float16_t);"
            "f16vec2   clamp(f16vec2,   f16vec2,   f16vec2);"
            "f16vec3   clamp(f16vec3,   f16vec3,   f16vec3);"
            "f16vec4   clamp(f16vec4,   f16vec4,   f16vec4);"

            "float16_t mix(float16_t, float16_t, float16_t);"
            "f16vec2   mix(f16vec2,   f16vec2,   float16_t);"
            "f16vec3   mix(f16vec3,   f16vec3,   float16_t);"
            "f16vec4   mix(f16vec4,   f16vec4,   float16_t);"
            "f16vec2   mix(f16vec2,   f16vec2,   f16vec2);"
            "f16vec3   mix(f16vec3,   f16vec3,   f16vec3);"
            "f16vec4   mix(f16vec4,   f16vec4,   f16vec4);"
            "float16_t mix(float16_t, float16_t, bool);"
            "f16vec2   mix(f16vec2,   f16vec2,   bvec2);"
            "f16vec3   mix(f16vec3,   f16vec3,   bvec3);"
            "f16vec4   mix(f16vec4,   f16vec4,   bvec4);"

            "float16_t step(float16_t, float16_t);"
            "f16vec2   step(f16vec2,   f16vec2);"
            "f16vec3   step(f16vec3,   f16vec3);"
            "f16vec4   step(f16vec4,   f16vec4);"
            "f16vec2   step(float16_t, f16vec2);"
            "f16vec3   step(float16_t, f16vec3);"
            "f16vec4   step(float16_t, f16vec4);"

            "float16_t smoothstep(float16_t, float16_t, float16_t);"
            "f16vec2   smoothstep(f16vec2,   f16vec2,   f16vec2);"
            "f16vec3   smoothstep(f16vec3,   f16vec3,   f16vec3);"
            "f16vec4   smoothstep(f16vec4,   f16vec4,   f16vec4);"
            "f16vec2   smoothstep(float16_t, float16_t, f16vec2);"
            "f16vec3   smoothstep(float16_t, float16_t, f16vec3);"
            "f16vec4   smoothstep(float16_t, float16_t, f16vec4);"

            "bool  isnan(float16_t);"
            "bvec2 isnan(f16vec2);"
            "bvec3 isnan(f16vec3);"
            "bvec4 isnan(f16vec4);"

            "bool  isinf(float16_t);"
            "bvec2 isinf(f16vec2);"
            "bvec3 isinf(f16vec3);"
            "bvec4 isinf(f16vec4);"

            "float16_t fma(float16_t, float16_t, float16_t);"
            "f16vec2   fma(f16vec2,   f16vec2,   f16vec2);"
            "f16vec3   fma(f16vec3,   f16vec3,   f16vec3);"
            "f16vec4   fma(f16vec4,   f16vec4,   f16vec4);"

            "float16_t frexp(float16_t, out int);"
            "f16vec2   frexp(f16vec2,   out ivec2);"
            "f16vec3   frexp(f16vec3,   out ivec3);"
            "f16vec4   frexp(f16vec4,   out ivec4);"

            "float16_t ldexp(float16_t, in int);"
            "f16vec2   ldexp(f16vec2,   in ivec2);"
            "f16vec3   ldexp(f16vec3,   in ivec3);"
            "f16vec4   ldexp(f16vec4,   in ivec4);"

            "uint    packFloat2x16(f16vec2);"
            "f16vec2 unpackFloat2x16(uint);"

            "float16_t length(float16_t);"
            "float16_t length(f16vec2);"
            "float16_t length(f16vec3);"
            "float16_t length(f16vec4);"

            "float16_t distance(float16_t, float16_t);"
            "float16_t distance(f16vec2,   f16vec2);"
            "float16_t distance(f16vec3,   f16vec3);"
            "float16_t distance(f16vec4,   f16vec4);"

            "float16_t dot(float16_t, float16_t);"
            "float16_t dot(f16vec2,   f16vec2);"
            "float16_t dot(f16vec3,   f16vec3);"
            "float16_t dot(f16vec4,   f16vec4);"

            "f16vec3 cross(f16vec3, f16vec3);"

            "float16_t normalize(float16_t);"
            "f16vec2   normalize(f16vec2);"
            "f16vec3   normalize(f16vec3);"
            "f16vec4   normalize(f16vec4);"

            "float16_t faceforward(float16_t, float16_t, float16_t);"
            "f16vec2   faceforward(f16vec2,   f16vec2,   f16vec2);"
            "f16vec3   faceforward(f16vec3,   f16vec3,   f16vec3);"
            "f16vec4   faceforward(f16vec4,   f16vec4,   f16vec4);"

            "float16_t reflect(float16_t, float16_t);"
            "f16vec2   reflect(f16vec2,   f16vec2);"
            "f16vec3   reflect(f16vec3,   f16vec3);"
            "f16vec4   reflect(f16vec4,   f16vec4);"

            "float16_t refract(float16_t, float16_t, float16_t);"
            "f16vec2   refract(f16vec2,   f16vec2,   float16_t);"
            "f16vec3   refract(f16vec3,   f16vec3,   float16_t);"
            "f16vec4   refract(f16vec4,   f16vec4,   float16_t);"

            "f16mat2   matrixCompMult(f16mat2,   f16mat2);"
            "f16mat3   matrixCompMult(f16mat3,   f16mat3);"
            "f16mat4   matrixCompMult(f16mat4,   f16mat4);"
            "f16mat2x3 matrixCompMult(f16mat2x3, f16mat2x3);"
            "f16mat2x4 matrixCompMult(f16mat2x4, f16mat2x4);"
            "f16mat3x2 matrixCompMult(f16mat3x2, f16mat3x2);"
            "f16mat3x4 matrixCompMult(f16mat3x4, f16mat3x4);"
            "f16mat4x2 matrixCompMult(f16mat4x2, f16mat4x2);"
            "f16mat4x3 matrixCompMult(f16mat4x3, f16mat4x3);"

            "f16mat2   outerProduct(f16vec2, f16vec2);"
            "f16mat3   outerProduct(f16vec3, f16vec3);"
            "f16mat4   outerProduct(f16vec4, f16vec4);"
            "f16mat2x3 outerProduct(f16vec3, f16vec2);"
            "f16mat3x2 outerProduct(f16vec2, f16vec3);"
            "f16mat2x4 outerProduct(f16vec4, f16vec2);"
            "f16mat4x2 outerProduct(f16vec2, f16vec4);"
            "f16mat3x4 outerProduct(f16vec4, f16vec3);"
            "f16mat4x3 outerProduct(f16vec3, f16vec4);"

            "f16mat2   transpose(f16mat2);"
            "f16mat3   transpose(f16mat3);"
            "f16mat4   transpose(f16mat4);"
            "f16mat2x3 transpose(f16mat3x2);"
            "f16mat3x2 transpose(f16mat2x3);"
            "f16mat2x4 transpose(f16mat4x2);"
            "f16mat4x2 transpose(f16mat2x4);"
            "f16mat3x4 transpose(f16mat4x3);"
            "f16mat4x3 transpose(f16mat3x4);"

            "float16_t determinant(f16mat2);"
            "float16_t determinant(f16mat3);"
            "float16_t determinant(f16mat4);"

            "f16mat2 inverse(f16mat2);"
            "f16mat3 inverse(f16mat3);"
            "f16mat4 inverse(f16mat4);"

            "bvec2 lessThan(f16vec2, f16vec2);"
            "bvec3 lessThan(f16vec3, f16vec3);"
            "bvec4 lessThan(f16vec4, f16vec4);"

            "bvec2 lessThanEqual(f16vec2, f16vec2);"
            "bvec3 lessThanEqual(f16vec3, f16vec3);"
            "bvec4 lessThanEqual(f16vec4, f16vec4);"

            "bvec2 greaterThan(f16vec2, f16vec2);"
            "bvec3 greaterThan(f16vec3, f16vec3);"
            "bvec4 greaterThan(f16vec4, f16vec4);"

            "bvec2 greaterThanEqual(f16vec2, f16vec2);"
            "bvec3 greaterThanEqual(f16vec3, f16vec3);"
            "bvec4 greaterThanEqual(f16vec4, f16vec4);"

            "bvec2 equal(f16vec2, f16vec2);"
            "bvec3 equal(f16vec3, f16vec3);"
            "bvec4 equal(f16vec4, f16vec4);"

            "bvec2 notEqual(f16vec2, f16vec2);"
            "bvec3 notEqual(f16vec3, f16vec3);"
            "bvec4 notEqual(f16vec4, f16vec4);"

            "\n");
    }

    // GL_AMD_gpu_shader_int16
    if (profile != EEsProfile && version >= 450) {
        commonBuiltins.append(
            "int16_t abs(int16_t);"
            "i16vec2 abs(i16vec2);"
            "i16vec3 abs(i16vec3);"
            "i16vec4 abs(i16vec4);"

            "int16_t sign(int16_t);"
            "i16vec2 sign(i16vec2);"
            "i16vec3 sign(i16vec3);"
            "i16vec4 sign(i16vec4);"

            "int16_t  min(int16_t,  int16_t);"
            "i16vec2  min(i16vec2,  int16_t);"
            "i16vec3  min(i16vec3,  int16_t);"
            "i16vec4  min(i16vec4,  int16_t);"
            "i16vec2  min(i16vec2,  i16vec2);"
            "i16vec3  min(i16vec3,  i16vec3);"
            "i16vec4  min(i16vec4,  i16vec4);"
            "uint16_t min(uint16_t, uint16_t);"
            "u16vec2  min(u16vec2,  uint16_t);"
            "u16vec3  min(u16vec3,  uint16_t);"
            "u16vec4  min(u16vec4,  uint16_t);"
            "u16vec2  min(u16vec2,  u16vec2);"
            "u16vec3  min(u16vec3,  u16vec3);"
            "u16vec4  min(u16vec4,  u16vec4);"

            "int16_t  max(int16_t,  int16_t);"
            "i16vec2  max(i16vec2,  int16_t);"
            "i16vec3  max(i16vec3,  int16_t);"
            "i16vec4  max(i16vec4,  int16_t);"
            "i16vec2  max(i16vec2,  i16vec2);"
            "i16vec3  max(i16vec3,  i16vec3);"
            "i16vec4  max(i16vec4,  i16vec4);"
            "uint16_t max(uint16_t, uint16_t);"
            "u16vec2  max(u16vec2,  uint16_t);"
            "u16vec3  max(u16vec3,  uint16_t);"
            "u16vec4  max(u16vec4,  uint16_t);"
            "u16vec2  max(u16vec2,  u16vec2);"
            "u16vec3  max(u16vec3,  u16vec3);"
            "u16vec4  max(u16vec4,  u16vec4);"

            "int16_t  clamp(int16_t,  int16_t,  int16_t);"
            "i16vec2  clamp(i16vec2,  int16_t,  int16_t);"
            "i16vec3  clamp(i16vec3,  int16_t,  int16_t);"
            "i16vec4  clamp(i16vec4,  int16_t,  int16_t);"
            "i16vec2  clamp(i16vec2,  i16vec2,  i16vec2);"
            "i16vec3  clamp(i16vec3,  i16vec3,  i16vec3);"
            "i16vec4  clamp(i16vec4,  i16vec4,  i16vec4);"
            "uint16_t clamp(uint16_t, uint16_t, uint16_t);"
            "u16vec2  clamp(u16vec2,  uint16_t, uint16_t);"
            "u16vec3  clamp(u16vec3,  uint16_t, uint16_t);"
            "u16vec4  clamp(u16vec4,  uint16_t, uint16_t);"
            "u16vec2  clamp(u16vec2,  u16vec2,  u16vec2);"
            "u16vec3  clamp(u16vec3,  u16vec3,  u16vec3);"
            "u16vec4  clamp(u16vec4,  u16vec4,  u16vec4);"

            "int16_t  mix(int16_t,  int16_t,  bool);"
            "i16vec2  mix(i16vec2,  i16vec2,  bvec2);"
            "i16vec3  mix(i16vec3,  i16vec3,  bvec3);"
            "i16vec4  mix(i16vec4,  i16vec4,  bvec4);"
            "uint16_t mix(uint16_t, uint16_t, bool);"
            "u16vec2  mix(u16vec2,  u16vec2,  bvec2);"
            "u16vec3  mix(u16vec3,  u16vec3,  bvec3);"
            "u16vec4  mix(u16vec4,  u16vec4,  bvec4);"

            "float16_t frexp(float16_t, out int16_t);"
            "f16vec2   frexp(f16vec2,   out i16vec2);"
            "f16vec3   frexp(f16vec3,   out i16vec3);"
            "f16vec4   frexp(f16vec4,   out i16vec4);"

            "float16_t ldexp(float16_t, int16_t);"
            "f16vec2   ldexp(f16vec2,   i16vec2);"
            "f16vec3   ldexp(f16vec3,   i16vec3);"
            "f16vec4   ldexp(f16vec4,   i16vec4);"

            "int16_t float16BitsToInt16(float16_t);"
            "i16vec2 float16BitsToInt16(f16vec2);"
            "i16vec3 float16BitsToInt16(f16vec3);"
            "i16vec4 float16BitsToInt16(f16vec4);"

            "uint16_t float16BitsToUint16(float16_t);"
            "u16vec2  float16BitsToUint16(f16vec2);"
            "u16vec3  float16BitsToUint16(f16vec3);"
            "u16vec4  float16BitsToUint16(f16vec4);"

            "float16_t int16BitsToFloat16(int16_t);"
            "f16vec2   int16BitsToFloat16(i16vec2);"
            "f16vec3   int16BitsToFloat16(i16vec3);"
            "f16vec4   int16BitsToFloat16(i16vec4);"

            "float16_t uint16BitsToFloat16(uint16_t);"
            "f16vec2   uint16BitsToFloat16(u16vec2);"
            "f16vec3   uint16BitsToFloat16(u16vec3);"
            "f16vec4   uint16BitsToFloat16(u16vec4);"

            "int      packInt2x16(i16vec2);"
            "uint     packUint2x16(u16vec2);"
            "int64_t  packInt4x16(i16vec4);"
            "uint64_t packUint4x16(u16vec4);"
            "i16vec2  unpackInt2x16(int);"
            "u16vec2  unpackUint2x16(uint);"
            "i16vec4  unpackInt4x16(int64_t);"
            "u16vec4  unpackUint4x16(uint64_t);"

            "bvec2 lessThan(i16vec2, i16vec2);"
            "bvec3 lessThan(i16vec3, i16vec3);"
            "bvec4 lessThan(i16vec4, i16vec4);"
            "bvec2 lessThan(u16vec2, u16vec2);"
            "bvec3 lessThan(u16vec3, u16vec3);"
            "bvec4 lessThan(u16vec4, u16vec4);"

            "bvec2 lessThanEqual(i16vec2, i16vec2);"
            "bvec3 lessThanEqual(i16vec3, i16vec3);"
            "bvec4 lessThanEqual(i16vec4, i16vec4);"
            "bvec2 lessThanEqual(u16vec2, u16vec2);"
            "bvec3 lessThanEqual(u16vec3, u16vec3);"
            "bvec4 lessThanEqual(u16vec4, u16vec4);"

            "bvec2 greaterThan(i16vec2, i16vec2);"
            "bvec3 greaterThan(i16vec3, i16vec3);"
            "bvec4 greaterThan(i16vec4, i16vec4);"
            "bvec2 greaterThan(u16vec2, u16vec2);"
            "bvec3 greaterThan(u16vec3, u16vec3);"
            "bvec4 greaterThan(u16vec4, u16vec4);"

            "bvec2 greaterThanEqual(i16vec2, i16vec2);"
            "bvec3 greaterThanEqual(i16vec3, i16vec3);"
            "bvec4 greaterThanEqual(i16vec4, i16vec4);"
            "bvec2 greaterThanEqual(u16vec2, u16vec2);"
            "bvec3 greaterThanEqual(u16vec3, u16vec3);"
            "bvec4 greaterThanEqual(u16vec4, u16vec4);"

            "bvec2 equal(i16vec2, i16vec2);"
            "bvec3 equal(i16vec3, i16vec3);"
            "bvec4 equal(i16vec4, i16vec4);"
            "bvec2 equal(u16vec2, u16vec2);"
            "bvec3 equal(u16vec3, u16vec3);"
            "bvec4 equal(u16vec4, u16vec4);"

            "bvec2 notEqual(i16vec2, i16vec2);"
            "bvec3 notEqual(i16vec3, i16vec3);"
            "bvec4 notEqual(i16vec4, i16vec4);"
            "bvec2 notEqual(u16vec2, u16vec2);"
            "bvec3 notEqual(u16vec3, u16vec3);"
            "bvec4 notEqual(u16vec4, u16vec4);"

            "\n");
    }
#endif

    //============================================================================
    //
    // Prototypes for built-in functions seen by vertex shaders only.
    // (Except legacy lod functions, where it depends which release they are
    // vertex only.)
    //
    //============================================================================

    //
    // Geometric Functions.
    //
    if (IncludeLegacy(version, profile, spvVersion))
        stageBuiltins[EShLangVertex].append("vec4 ftransform();");

    //
    // Original-style texture Functions with lod.
    //
    TString* s;
    if (version == 100)
        s = &stageBuiltins[EShLangVertex];
    else
        s = &commonBuiltins;
    if ((profile == EEsProfile && version == 100) ||
         profile == ECompatibilityProfile ||
        (profile == ECoreProfile && version < 420) ||
         profile == ENoProfile) {
        if (spvVersion.spv == 0) {
            s->append(
                "vec4 texture2DLod(sampler2D, vec2, float);"         // GL_ARB_shader_texture_lod
                "vec4 texture2DProjLod(sampler2D, vec3, float);"     // GL_ARB_shader_texture_lod
                "vec4 texture2DProjLod(sampler2D, vec4, float);"     // GL_ARB_shader_texture_lod
                "vec4 texture3DLod(sampler3D, vec3, float);"         // GL_ARB_shader_texture_lod  // OES_texture_3D, but caught by keyword check
                "vec4 texture3DProjLod(sampler3D, vec4, float);"     // GL_ARB_shader_texture_lod  // OES_texture_3D, but caught by keyword check
                "vec4 textureCubeLod(samplerCube, vec3, float);"     // GL_ARB_shader_texture_lod

                "\n");
        }
    }
    if ( profile == ECompatibilityProfile ||
        (profile == ECoreProfile && version < 420) ||
         profile == ENoProfile) {
        if (spvVersion.spv == 0) {
            s->append(
                "vec4 texture1DLod(sampler1D, float, float);"                          // GL_ARB_shader_texture_lod
                "vec4 texture1DProjLod(sampler1D, vec2, float);"                       // GL_ARB_shader_texture_lod
                "vec4 texture1DProjLod(sampler1D, vec4, float);"                       // GL_ARB_shader_texture_lod
                "vec4 shadow1DLod(sampler1DShadow, vec3, float);"                      // GL_ARB_shader_texture_lod
                "vec4 shadow2DLod(sampler2DShadow, vec3, float);"                      // GL_ARB_shader_texture_lod
                "vec4 shadow1DProjLod(sampler1DShadow, vec4, float);"                  // GL_ARB_shader_texture_lod
                "vec4 shadow2DProjLod(sampler2DShadow, vec4, float);"                  // GL_ARB_shader_texture_lod

                "vec4 texture1DGradARB(sampler1D, float, float, float);"               // GL_ARB_shader_texture_lod
                "vec4 texture1DProjGradARB(sampler1D, vec2, float, float);"            // GL_ARB_shader_texture_lod
                "vec4 texture1DProjGradARB(sampler1D, vec4, float, float);"            // GL_ARB_shader_texture_lod
                "vec4 texture2DGradARB(sampler2D, vec2, vec2, vec2);"                  // GL_ARB_shader_texture_lod
                "vec4 texture2DProjGradARB(sampler2D, vec3, vec2, vec2);"              // GL_ARB_shader_texture_lod
                "vec4 texture2DProjGradARB(sampler2D, vec4, vec2, vec2);"              // GL_ARB_shader_texture_lod
                "vec4 texture3DGradARB(sampler3D, vec3, vec3, vec3);"                  // GL_ARB_shader_texture_lod
                "vec4 texture3DProjGradARB(sampler3D, vec4, vec3, vec3);"              // GL_ARB_shader_texture_lod
                "vec4 textureCubeGradARB(samplerCube, vec3, vec3, vec3);"              // GL_ARB_shader_texture_lod
                "vec4 shadow1DGradARB(sampler1DShadow, vec3, float, float);"           // GL_ARB_shader_texture_lod
                "vec4 shadow1DProjGradARB( sampler1DShadow, vec4, float, float);"      // GL_ARB_shader_texture_lod
                "vec4 shadow2DGradARB(sampler2DShadow, vec3, vec2, vec2);"             // GL_ARB_shader_texture_lod
                "vec4 shadow2DProjGradARB( sampler2DShadow, vec4, vec2, vec2);"        // GL_ARB_shader_texture_lod
                "vec4 texture2DRectGradARB(sampler2DRect, vec2, vec2, vec2);"          // GL_ARB_shader_texture_lod
                "vec4 texture2DRectProjGradARB( sampler2DRect, vec3, vec2, vec2);"     // GL_ARB_shader_texture_lod
                "vec4 texture2DRectProjGradARB( sampler2DRect, vec4, vec2, vec2);"     // GL_ARB_shader_texture_lod
                "vec4 shadow2DRectGradARB( sampler2DRectShadow, vec3, vec2, vec2);"    // GL_ARB_shader_texture_lod
                "vec4 shadow2DRectProjGradARB(sampler2DRectShadow, vec4, vec2, vec2);" // GL_ARB_shader_texture_lod

                "\n");
        }
    }

    if ((profile != EEsProfile && version >= 150) ||
        (profile == EEsProfile && version >= 310)) {
        //============================================================================
        //
        // Prototypes for built-in functions seen by geometry shaders only.
        //
        //============================================================================

        if (profile != EEsProfile && version >= 400) {
            stageBuiltins[EShLangGeometry].append(
                "void EmitStreamVertex(int);"
                "void EndStreamPrimitive(int);"
                );
        }
        stageBuiltins[EShLangGeometry].append(
            "void EmitVertex();"
            "void EndPrimitive();"
            "\n");
    }

    //============================================================================
    //
    // Prototypes for all control functions.
    //
    //============================================================================
    bool esBarrier = (profile == EEsProfile && version >= 310);
    if ((profile != EEsProfile && version >= 150) || esBarrier)
        stageBuiltins[EShLangTessControl].append(
            "void barrier();"
            );
    if ((profile != EEsProfile && version >= 420) || esBarrier)
        stageBuiltins[EShLangCompute].append(
            "void barrier();"
            );
    if ((profile != EEsProfile && version >= 130) || esBarrier)
        commonBuiltins.append(
            "void memoryBarrier();"
            );
    if ((profile != EEsProfile && version >= 420) || esBarrier) {
        commonBuiltins.append(
            "void memoryBarrierAtomicCounter();"
            "void memoryBarrierBuffer();"
            "void memoryBarrierImage();"
            );
        stageBuiltins[EShLangCompute].append(
            "void memoryBarrierShared();"
            "void groupMemoryBarrier();"
            );
    }

    //============================================================================
    //
    // Prototypes for built-in functions seen by fragment shaders only.
    //
    //============================================================================

    //
    // Original-style texture Functions with bias.
    //
    if (spvVersion.spv == 0 && (profile != EEsProfile || version == 100)) {
        stageBuiltins[EShLangFragment].append(
            "vec4 texture2D(sampler2D, vec2, float);"
            "vec4 texture2DProj(sampler2D, vec3, float);"
            "vec4 texture2DProj(sampler2D, vec4, float);"
            "vec4 texture3D(sampler3D, vec3, float);"        // OES_texture_3D
            "vec4 texture3DProj(sampler3D, vec4, float);"    // OES_texture_3D
            "vec4 textureCube(samplerCube, vec3, float);"

            "\n");
    }
    if (spvVersion.spv == 0 && (profile != EEsProfile && version > 100)) {
        stageBuiltins[EShLangFragment].append(
            "vec4 texture1D(sampler1D, float, float);"
            "vec4 texture1DProj(sampler1D, vec2, float);"
            "vec4 texture1DProj(sampler1D, vec4, float);"
            "vec4 shadow1D(sampler1DShadow, vec3, float);"
            "vec4 shadow2D(sampler2DShadow, vec3, float);"
            "vec4 shadow1DProj(sampler1DShadow, vec4, float);"
            "vec4 shadow2DProj(sampler2DShadow, vec4, float);"

            "\n");
    }
    if (spvVersion.spv == 0 && profile == EEsProfile) {
        stageBuiltins[EShLangFragment].append(
            "vec4 texture2DLodEXT(sampler2D, vec2, float);"      // GL_EXT_shader_texture_lod
            "vec4 texture2DProjLodEXT(sampler2D, vec3, float);"  // GL_EXT_shader_texture_lod
            "vec4 texture2DProjLodEXT(sampler2D, vec4, float);"  // GL_EXT_shader_texture_lod
            "vec4 textureCubeLodEXT(samplerCube, vec3, float);"  // GL_EXT_shader_texture_lod

            "\n");
    }

    stageBuiltins[EShLangFragment].append(
        "float dFdx(float p);"
        "vec2  dFdx(vec2  p);"
        "vec3  dFdx(vec3  p);"
        "vec4  dFdx(vec4  p);"

        "float dFdy(float p);"
        "vec2  dFdy(vec2  p);"
        "vec3  dFdy(vec3  p);"
        "vec4  dFdy(vec4  p);"

        "float fwidth(float p);"
        "vec2  fwidth(vec2  p);"
        "vec3  fwidth(vec3  p);"
        "vec4  fwidth(vec4  p);"

        "\n");

    // GL_ARB_derivative_control
    if (profile != EEsProfile && version >= 400) {
        stageBuiltins[EShLangFragment].append(
            "float dFdxFine(float p);"
            "vec2  dFdxFine(vec2  p);"
            "vec3  dFdxFine(vec3  p);"
            "vec4  dFdxFine(vec4  p);"

            "float dFdyFine(float p);"
            "vec2  dFdyFine(vec2  p);"
            "vec3  dFdyFine(vec3  p);"
            "vec4  dFdyFine(vec4  p);"

            "float fwidthFine(float p);"
            "vec2  fwidthFine(vec2  p);"
            "vec3  fwidthFine(vec3  p);"
            "vec4  fwidthFine(vec4  p);"

            "\n");

        stageBuiltins[EShLangFragment].append(
            "float dFdxCoarse(float p);"
            "vec2  dFdxCoarse(vec2  p);"
            "vec3  dFdxCoarse(vec3  p);"
            "vec4  dFdxCoarse(vec4  p);"

            "float dFdyCoarse(float p);"
            "vec2  dFdyCoarse(vec2  p);"
            "vec3  dFdyCoarse(vec3  p);"
            "vec4  dFdyCoarse(vec4  p);"

            "float fwidthCoarse(float p);"
            "vec2  fwidthCoarse(vec2  p);"
            "vec3  fwidthCoarse(vec3  p);"
            "vec4  fwidthCoarse(vec4  p);"

            "\n");
    }

    // GL_OES_shader_multisample_interpolation
    if ((profile == EEsProfile && version >= 310) ||
        (profile != EEsProfile && version >= 400)) {
        stageBuiltins[EShLangFragment].append(
            "float interpolateAtCentroid(float);"
            "vec2  interpolateAtCentroid(vec2);"
            "vec3  interpolateAtCentroid(vec3);"
            "vec4  interpolateAtCentroid(vec4);"

            "float interpolateAtSample(float, int);"
            "vec2  interpolateAtSample(vec2,  int);"
            "vec3  interpolateAtSample(vec3,  int);"
            "vec4  interpolateAtSample(vec4,  int);"

            "float interpolateAtOffset(float, vec2);"
            "vec2  interpolateAtOffset(vec2,  vec2);"
            "vec3  interpolateAtOffset(vec3,  vec2);"
            "vec4  interpolateAtOffset(vec4,  vec2);"

            "\n");
    }

#ifdef AMD_EXTENSIONS
    // GL_AMD_shader_explicit_vertex_parameter
    if (profile != EEsProfile && version >= 450) {
        stageBuiltins[EShLangFragment].append(
            "float interpolateAtVertexAMD(float, uint);"
            "vec2  interpolateAtVertexAMD(vec2,  uint);"
            "vec3  interpolateAtVertexAMD(vec3,  uint);"
            "vec4  interpolateAtVertexAMD(vec4,  uint);"

            "int   interpolateAtVertexAMD(int,   uint);"
            "ivec2 interpolateAtVertexAMD(ivec2, uint);"
            "ivec3 interpolateAtVertexAMD(ivec3, uint);"
            "ivec4 interpolateAtVertexAMD(ivec4, uint);"

            "uint  interpolateAtVertexAMD(uint,  uint);"
            "uvec2 interpolateAtVertexAMD(uvec2, uint);"
            "uvec3 interpolateAtVertexAMD(uvec3, uint);"
            "uvec4 interpolateAtVertexAMD(uvec4, uint);"

            "float16_t interpolateAtVertexAMD(float16_t, uint);"
            "f16vec2   interpolateAtVertexAMD(f16vec2,   uint);"
            "f16vec3   interpolateAtVertexAMD(f16vec3,   uint);"
            "f16vec4   interpolateAtVertexAMD(f16vec4,   uint);"

            "\n");
    }

    // GL_AMD_gpu_shader_half_float
    if (profile != EEsProfile && version >= 450) {
        stageBuiltins[EShLangFragment].append(
            "float16_t dFdx(float16_t);"
            "f16vec2   dFdx(f16vec2);"
            "f16vec3   dFdx(f16vec3);"
            "f16vec4   dFdx(f16vec4);"

            "float16_t dFdy(float16_t);"
            "f16vec2   dFdy(f16vec2);"
            "f16vec3   dFdy(f16vec3);"
            "f16vec4   dFdy(f16vec4);"

            "float16_t dFdxFine(float16_t);"
            "f16vec2   dFdxFine(f16vec2);"
            "f16vec3   dFdxFine(f16vec3);"
            "f16vec4   dFdxFine(f16vec4);"

            "float16_t dFdyFine(float16_t);"
            "f16vec2   dFdyFine(f16vec2);"
            "f16vec3   dFdyFine(f16vec3);"
            "f16vec4   dFdyFine(f16vec4);"

            "float16_t dFdxCoarse(float16_t);"
            "f16vec2   dFdxCoarse(f16vec2);"
            "f16vec3   dFdxCoarse(f16vec3);"
            "f16vec4   dFdxCoarse(f16vec4);"

            "float16_t dFdyCoarse(float16_t);"
            "f16vec2   dFdyCoarse(f16vec2);"
            "f16vec3   dFdyCoarse(f16vec3);"
            "f16vec4   dFdyCoarse(f16vec4);"

            "float16_t fwidth(float16_t);"
            "f16vec2   fwidth(f16vec2);"
            "f16vec3   fwidth(f16vec3);"
            "f16vec4   fwidth(f16vec4);"

            "float16_t fwidthFine(float16_t);"
            "f16vec2   fwidthFine(f16vec2);"
            "f16vec3   fwidthFine(f16vec3);"
            "f16vec4   fwidthFine(f16vec4);"

            "float16_t fwidthCoarse(float16_t);"
            "f16vec2   fwidthCoarse(f16vec2);"
            "f16vec3   fwidthCoarse(f16vec3);"
            "f16vec4   fwidthCoarse(f16vec4);"

            "float16_t interpolateAtCentroid(float16_t);"
            "f16vec2   interpolateAtCentroid(f16vec2);"
            "f16vec3   interpolateAtCentroid(f16vec3);"
            "f16vec4   interpolateAtCentroid(f16vec4);"

            "float16_t interpolateAtSample(float16_t, int);"
            "f16vec2   interpolateAtSample(f16vec2,   int);"
            "f16vec3   interpolateAtSample(f16vec3,   int);"
            "f16vec4   interpolateAtSample(f16vec4,   int);"

            "float16_t interpolateAtOffset(float16_t, f16vec2);"
            "f16vec2   interpolateAtOffset(f16vec2,   f16vec2);"
            "f16vec3   interpolateAtOffset(f16vec3,   f16vec2);"
            "f16vec4   interpolateAtOffset(f16vec4,   f16vec2);"

            "\n");
    }
#endif

    //============================================================================
    //
    // Standard Uniforms
    //
    //============================================================================

    //
    // Depth range in window coordinates, p. 33
    //
    if (spvVersion.spv == 0) {
        commonBuiltins.append(
            "struct gl_DepthRangeParameters {"
            );
        if (profile == EEsProfile) {
            commonBuiltins.append(
                "highp float near;"   // n
                "highp float far;"    // f
                "highp float diff;"   // f - n
                );
        } else {
            commonBuiltins.append(
                "float near;"  // n
                "float far;"   // f
                "float diff;"  // f - n
                );
        }

        commonBuiltins.append(
            "};"
            "uniform gl_DepthRangeParameters gl_DepthRange;"
            "\n");
    }

    if (spvVersion.spv == 0 && IncludeLegacy(version, profile, spvVersion)) {
        //
        // Matrix state. p. 31, 32, 37, 39, 40.
        //
        commonBuiltins.append(
            "uniform mat4  gl_ModelViewMatrix;"
            "uniform mat4  gl_ProjectionMatrix;"
            "uniform mat4  gl_ModelViewProjectionMatrix;"

            //
            // Derived matrix state that provides inverse and transposed versions
            // of the matrices above.
            //
            "uniform mat3  gl_NormalMatrix;"

            "uniform mat4  gl_ModelViewMatrixInverse;"
            "uniform mat4  gl_ProjectionMatrixInverse;"
            "uniform mat4  gl_ModelViewProjectionMatrixInverse;"

            "uniform mat4  gl_ModelViewMatrixTranspose;"
            "uniform mat4  gl_ProjectionMatrixTranspose;"
            "uniform mat4  gl_ModelViewProjectionMatrixTranspose;"

            "uniform mat4  gl_ModelViewMatrixInverseTranspose;"
            "uniform mat4  gl_ProjectionMatrixInverseTranspose;"
            "uniform mat4  gl_ModelViewProjectionMatrixInverseTranspose;"

            //
            // Normal scaling p. 39.
            //
            "uniform float gl_NormalScale;"

            //
            // Point Size, p. 66, 67.
            //
            "struct gl_PointParameters {"
                "float size;"
                "float sizeMin;"
                "float sizeMax;"
                "float fadeThresholdSize;"
                "float distanceConstantAttenuation;"
                "float distanceLinearAttenuation;"
                "float distanceQuadraticAttenuation;"
            "};"

            "uniform gl_PointParameters gl_Point;"

            //
            // Material State p. 50, 55.
            //
            "struct gl_MaterialParameters {"
                "vec4  emission;"    // Ecm
                "vec4  ambient;"     // Acm
                "vec4  diffuse;"     // Dcm
                "vec4  specular;"    // Scm
                "float shininess;"   // Srm
            "};"
            "uniform gl_MaterialParameters  gl_FrontMaterial;"
            "uniform gl_MaterialParameters  gl_BackMaterial;"

            //
            // Light State p 50, 53, 55.
            //
            "struct gl_LightSourceParameters {"
                "vec4  ambient;"             // Acli
                "vec4  diffuse;"             // Dcli
                "vec4  specular;"            // Scli
                "vec4  position;"            // Ppli
                "vec4  halfVector;"          // Derived: Hi
                "vec3  spotDirection;"       // Sdli
                "float spotExponent;"        // Srli
                "float spotCutoff;"          // Crli
                                                        // (range: [0.0,90.0], 180.0)
                "float spotCosCutoff;"       // Derived: cos(Crli)
                                                        // (range: [1.0,0.0],-1.0)
                "float constantAttenuation;" // K0
                "float linearAttenuation;"   // K1
                "float quadraticAttenuation;"// K2
            "};"

            "struct gl_LightModelParameters {"
                "vec4  ambient;"       // Acs
            "};"

            "uniform gl_LightModelParameters  gl_LightModel;"

            //
            // Derived state from products of light and material.
            //
            "struct gl_LightModelProducts {"
                "vec4  sceneColor;"     // Derived. Ecm + Acm * Acs
            "};"

            "uniform gl_LightModelProducts gl_FrontLightModelProduct;"
            "uniform gl_LightModelProducts gl_BackLightModelProduct;"

            "struct gl_LightProducts {"
                "vec4  ambient;"        // Acm * Acli
                "vec4  diffuse;"        // Dcm * Dcli
                "vec4  specular;"       // Scm * Scli
            "};"

            //
            // Fog p. 161
            //
            "struct gl_FogParameters {"
                "vec4  color;"
                "float density;"
                "float start;"
                "float end;"
                "float scale;"   //  1 / (gl_FogEnd - gl_FogStart)
            "};"

            "uniform gl_FogParameters gl_Fog;"

            "\n");
    }

    //============================================================================
    //
    // Define the interface to the compute shader.
    //
    //============================================================================

    if ((profile != EEsProfile && version >= 420) ||
        (profile == EEsProfile && version >= 310)) {
        stageBuiltins[EShLangCompute].append(
            "in    highp uvec3 gl_NumWorkGroups;"
            "const highp uvec3 gl_WorkGroupSize = uvec3(1,1,1);"

            "in highp uvec3 gl_WorkGroupID;"
            "in highp uvec3 gl_LocalInvocationID;"

            "in highp uvec3 gl_GlobalInvocationID;"
            "in highp uint gl_LocalInvocationIndex;"

            "\n");
    }

    if ((profile != EEsProfile && version >= 140) ||
        (profile == EEsProfile && version >= 310)) {
        stageBuiltins[EShLangCompute].append(
            "in highp int gl_DeviceIndex;"     // GL_EXT_device_group
            "\n");
    }

    //============================================================================
    //
    // Define the interface to the vertex shader.
    //
    //============================================================================

    if (profile != EEsProfile) {
        if (version < 130) {
            stageBuiltins[EShLangVertex].append(
                "attribute vec4  gl_Color;"
                "attribute vec4  gl_SecondaryColor;"
                "attribute vec3  gl_Normal;"
                "attribute vec4  gl_Vertex;"
                "attribute vec4  gl_MultiTexCoord0;"
                "attribute vec4  gl_MultiTexCoord1;"
                "attribute vec4  gl_MultiTexCoord2;"
                "attribute vec4  gl_MultiTexCoord3;"
                "attribute vec4  gl_MultiTexCoord4;"
                "attribute vec4  gl_MultiTexCoord5;"
                "attribute vec4  gl_MultiTexCoord6;"
                "attribute vec4  gl_MultiTexCoord7;"
                "attribute float gl_FogCoord;"
                "\n");
        } else if (IncludeLegacy(version, profile, spvVersion)) {
            stageBuiltins[EShLangVertex].append(
                "in vec4  gl_Color;"
                "in vec4  gl_SecondaryColor;"
                "in vec3  gl_Normal;"
                "in vec4  gl_Vertex;"
                "in vec4  gl_MultiTexCoord0;"
                "in vec4  gl_MultiTexCoord1;"
                "in vec4  gl_MultiTexCoord2;"
                "in vec4  gl_MultiTexCoord3;"
                "in vec4  gl_MultiTexCoord4;"
                "in vec4  gl_MultiTexCoord5;"
                "in vec4  gl_MultiTexCoord6;"
                "in vec4  gl_MultiTexCoord7;"
                "in float gl_FogCoord;"
                "\n");
        }

        if (version < 150) {
            if (version < 130) {
                stageBuiltins[EShLangVertex].append(
                    "        vec4  gl_ClipVertex;"       // needs qualifier fixed later
                    "varying vec4  gl_FrontColor;"
                    "varying vec4  gl_BackColor;"
                    "varying vec4  gl_FrontSecondaryColor;"
                    "varying vec4  gl_BackSecondaryColor;"
                    "varying vec4  gl_TexCoord[];"
                    "varying float gl_FogFragCoord;"
                    "\n");
            } else if (IncludeLegacy(version, profile, spvVersion)) {
                stageBuiltins[EShLangVertex].append(
                    "    vec4  gl_ClipVertex;"       // needs qualifier fixed later
                    "out vec4  gl_FrontColor;"
                    "out vec4  gl_BackColor;"
                    "out vec4  gl_FrontSecondaryColor;"
                    "out vec4  gl_BackSecondaryColor;"
                    "out vec4  gl_TexCoord[];"
                    "out float gl_FogFragCoord;"
                    "\n");
            }
            stageBuiltins[EShLangVertex].append(
                "vec4 gl_Position;"   // needs qualifier fixed later
                "float gl_PointSize;" // needs qualifier fixed later
                );

            if (version == 130 || version == 140)
                stageBuiltins[EShLangVertex].append(
                    "out float gl_ClipDistance[];"
                    );
        } else {
            // version >= 150
            stageBuiltins[EShLangVertex].append(
                "out gl_PerVertex {"
                    "vec4 gl_Position;"     // needs qualifier fixed later
                    "float gl_PointSize;"   // needs qualifier fixed later
                    "float gl_ClipDistance[];"
                    );
            if (IncludeLegacy(version, profile, spvVersion))
                stageBuiltins[EShLangVertex].append(
                    "vec4 gl_ClipVertex;"   // needs qualifier fixed later
                    "vec4 gl_FrontColor;"
                    "vec4 gl_BackColor;"
                    "vec4 gl_FrontSecondaryColor;"
                    "vec4 gl_BackSecondaryColor;"
                    "vec4 gl_TexCoord[];"
                    "float gl_FogFragCoord;"
                    );
            if (version >= 450)
                stageBuiltins[EShLangVertex].append(
                    "float gl_CullDistance[];"
                    );
            stageBuiltins[EShLangVertex].append(
                "};"
                "\n");
        }
        if (version >= 130 && spvVersion.vulkan == 0)
            stageBuiltins[EShLangVertex].append(
                "int gl_VertexID;"            // needs qualifier fixed later
                );
        if (version >= 140 && spvVersion.vulkan == 0)
            stageBuiltins[EShLangVertex].append(
                "int gl_InstanceID;"          // needs qualifier fixed later
                );
        if (spvVersion.vulkan >= 100 && version >= 140)
            stageBuiltins[EShLangVertex].append(
                "in int gl_VertexIndex;"
                "in int gl_InstanceIndex;"
                );
        if (version >= 440) {
            stageBuiltins[EShLangVertex].append(
                "in int gl_BaseVertexARB;"
                "in int gl_BaseInstanceARB;"
                "in int gl_DrawIDARB;"
                );
        }

#ifdef NV_EXTENSIONS
        if (version >= 450)
            stageBuiltins[EShLangVertex].append(
                "out int gl_ViewportIndex;"
                "out int gl_Layer;"
                "out int gl_ViewportMask[];"             // GL_NV_viewport_array2
                "out int gl_SecondaryViewportMaskNV[];"  // GL_NV_stereo_view_rendering
                "out vec4 gl_SecondaryPositionNV;"       // GL_NV_stereo_view_rendering
                "out vec4 gl_PositionPerViewNV[];"       // GL_NVX_multiview_per_view_attributes
                "out int  gl_ViewportMaskPerViewNV[];"   // GL_NVX_multiview_per_view_attributes
                );
#endif

    } else {
        // ES profile
        if (version == 100) {
            stageBuiltins[EShLangVertex].append(
                "highp   vec4  gl_Position;"  // needs qualifier fixed later
                "mediump float gl_PointSize;" // needs qualifier fixed later
                );
        } else {
            if (spvVersion.vulkan == 0)
                stageBuiltins[EShLangVertex].append(
                    "in highp int gl_VertexID;"      // needs qualifier fixed later
                    "in highp int gl_InstanceID;"    // needs qualifier fixed later
                    );
            if (spvVersion.vulkan >= 100)
                stageBuiltins[EShLangVertex].append(
                    "in highp int gl_VertexIndex;"
                    "in highp int gl_InstanceIndex;"
                    );
            if (version < 310)
                stageBuiltins[EShLangVertex].append(
                    "highp vec4  gl_Position;"    // needs qualifier fixed later
                    "highp float gl_PointSize;"   // needs qualifier fixed later
                    );
            else
                stageBuiltins[EShLangVertex].append(
                    "out gl_PerVertex {"
                        "highp vec4  gl_Position;"    // needs qualifier fixed later
                        "highp float gl_PointSize;"   // needs qualifier fixed later
                    "};"
                    );
        }
    }

    if ((profile != EEsProfile && version >= 140) ||
        (profile == EEsProfile && version >= 310)) {
        stageBuiltins[EShLangVertex].append(
            "in highp int gl_DeviceIndex;"     // GL_EXT_device_group
            "in highp int gl_ViewIndex;"       // GL_EXT_multiview
            "\n");
    }


    //============================================================================
    //
    // Define the interface to the geometry shader.
    //
    //============================================================================

    if (profile == ECoreProfile || profile == ECompatibilityProfile) {
        stageBuiltins[EShLangGeometry].append(
            "in gl_PerVertex {"
                "vec4 gl_Position;"
                "float gl_PointSize;"
                "float gl_ClipDistance[];"
                );
        if (profile == ECompatibilityProfile)
            stageBuiltins[EShLangGeometry].append(
                "vec4 gl_ClipVertex;"
                "vec4 gl_FrontColor;"
                "vec4 gl_BackColor;"
                "vec4 gl_FrontSecondaryColor;"
                "vec4 gl_BackSecondaryColor;"
                "vec4 gl_TexCoord[];"
                "float gl_FogFragCoord;"
                );
        if (version >= 450)
            stageBuiltins[EShLangGeometry].append(
                "float gl_CullDistance[];"
#ifdef NV_EXTENSIONS
                "vec4 gl_SecondaryPositionNV;"   // GL_NV_stereo_view_rendering
                "vec4 gl_PositionPerViewNV[];"   // GL_NVX_multiview_per_view_attributes
#endif
                );
        stageBuiltins[EShLangGeometry].append(
            "} gl_in[];"

            "in int gl_PrimitiveIDIn;"
            "out gl_PerVertex {"
                "vec4 gl_Position;"
                "float gl_PointSize;"
                "float gl_ClipDistance[];"
                "\n");
        if (profile == ECompatibilityProfile && version >= 400)
            stageBuiltins[EShLangGeometry].append(
                "vec4 gl_ClipVertex;"
                "vec4 gl_FrontColor;"
                "vec4 gl_BackColor;"
                "vec4 gl_FrontSecondaryColor;"
                "vec4 gl_BackSecondaryColor;"
                "vec4 gl_TexCoord[];"
                "float gl_FogFragCoord;"
                );
        if (version >= 450)
            stageBuiltins[EShLangGeometry].append(
                "float gl_CullDistance[];"
                );
        stageBuiltins[EShLangGeometry].append(
            "};"

            "out int gl_PrimitiveID;"
            "out int gl_Layer;");

        if (profile == ECompatibilityProfile && version < 400)
            stageBuiltins[EShLangGeometry].append(
            "out vec4 gl_ClipVertex;"
            );

        if (version >= 400)
            stageBuiltins[EShLangGeometry].append(
            "in int gl_InvocationID;"
            );
        // GL_ARB_viewport_array
        if (version >= 150)
            stageBuiltins[EShLangGeometry].append(
            "out int gl_ViewportIndex;"
            );

#ifdef NV_EXTENSIONS
        if (version >= 450)
            stageBuiltins[EShLangGeometry].append(
                "out int gl_ViewportMask[];"               // GL_NV_viewport_array2
                "out int gl_SecondaryViewportMaskNV[];"    // GL_NV_stereo_view_rendering
                "out vec4 gl_SecondaryPositionNV;"         // GL_NV_stereo_view_rendering
                "out vec4 gl_PositionPerViewNV[];"         // GL_NVX_multiview_per_view_attributes
                "out int  gl_ViewportMaskPerViewNV[];"     // GL_NVX_multiview_per_view_attributes
            );
#endif

        stageBuiltins[EShLangGeometry].append("\n");
    } else if (profile == EEsProfile && version >= 310) {
        stageBuiltins[EShLangGeometry].append(
            "in gl_PerVertex {"
                "highp vec4 gl_Position;"
                "highp float gl_PointSize;"
            "} gl_in[];"
            "\n"
            "in highp int gl_PrimitiveIDIn;"
            "in highp int gl_InvocationID;"
            "\n"
            "out gl_PerVertex {"
                "highp vec4 gl_Position;"
                "highp float gl_PointSize;"
            "};"
            "\n"
            "out highp int gl_PrimitiveID;"
            "out highp int gl_Layer;"
            "\n"
            );
    }

    if ((profile != EEsProfile && version >= 140) ||
        (profile == EEsProfile && version >= 310)) {
        stageBuiltins[EShLangGeometry].append(
            "in highp int gl_DeviceIndex;"     // GL_EXT_device_group
            "in highp int gl_ViewIndex;"       // GL_EXT_multiview
            "\n");
    }

    //============================================================================
    //
    // Define the interface to the tessellation control shader.
    //
    //============================================================================

    if (profile != EEsProfile && version >= 150) {
        // Note:  "in gl_PerVertex {...} gl_in[gl_MaxPatchVertices];" is declared in initialize() below,
        // as it depends on the resource sizing of gl_MaxPatchVertices.

        stageBuiltins[EShLangTessControl].append(
            "in int gl_PatchVerticesIn;"
            "in int gl_PrimitiveID;"
            "in int gl_InvocationID;"

            "out gl_PerVertex {"
                "vec4 gl_Position;"
                "float gl_PointSize;"
                "float gl_ClipDistance[];"
                );
        if (profile == ECompatibilityProfile)
            stageBuiltins[EShLangTessControl].append(
                "vec4 gl_ClipVertex;"
                "vec4 gl_FrontColor;"
                "vec4 gl_BackColor;"
                "vec4 gl_FrontSecondaryColor;"
                "vec4 gl_BackSecondaryColor;"
                "vec4 gl_TexCoord[];"
                "float gl_FogFragCoord;"
                );
        if (version >= 450)
            stageBuiltins[EShLangTessControl].append(
                "float gl_CullDistance[];"
#ifdef NV_EXTENSIONS
                "int  gl_ViewportIndex;"
                "int  gl_Layer;"
                "int  gl_ViewportMask[];"             // GL_NV_viewport_array2
                "vec4 gl_SecondaryPositionNV;"        // GL_NV_stereo_view_rendering
                "int  gl_SecondaryViewportMaskNV[];"  // GL_NV_stereo_view_rendering
                "vec4 gl_PositionPerViewNV[];"        // GL_NVX_multiview_per_view_attributes
                "int  gl_ViewportMaskPerViewNV[];"    // GL_NVX_multiview_per_view_attributes
#endif
                );
        stageBuiltins[EShLangTessControl].append(
            "} gl_out[];"

            "patch out float gl_TessLevelOuter[4];"
            "patch out float gl_TessLevelInner[2];"
            "\n");
    } else {
        // Note:  "in gl_PerVertex {...} gl_in[gl_MaxPatchVertices];" is declared in initialize() below,
        // as it depends on the resource sizing of gl_MaxPatchVertices.

        stageBuiltins[EShLangTessControl].append(
            "in highp int gl_PatchVerticesIn;"
            "in highp int gl_PrimitiveID;"
            "in highp int gl_InvocationID;"

            "out gl_PerVertex {"
                "highp vec4 gl_Position;"
                "highp float gl_PointSize;"
                );
        stageBuiltins[EShLangTessControl].append(
            "} gl_out[];"

            "patch out highp float gl_TessLevelOuter[4];"
            "patch out highp float gl_TessLevelInner[2];"
            "patch out highp vec4 gl_BoundingBoxOES[2];"
            "\n");
    }

    if ((profile != EEsProfile && version >= 140) ||
        (profile == EEsProfile && version >= 310)) {
        stageBuiltins[EShLangTessControl].append(
            "in highp int gl_DeviceIndex;"     // GL_EXT_device_group
            "in highp int gl_ViewIndex;"       // GL_EXT_multiview
            "\n");
    }

    //============================================================================
    //
    // Define the interface to the tessellation evaluation shader.
    //
    //============================================================================

    if (profile != EEsProfile && version >= 150) {
        // Note:  "in gl_PerVertex {...} gl_in[gl_MaxPatchVertices];" is declared in initialize() below,
        // as it depends on the resource sizing of gl_MaxPatchVertices.

        stageBuiltins[EShLangTessEvaluation].append(
            "in int gl_PatchVerticesIn;"
            "in int gl_PrimitiveID;"
            "in vec3 gl_TessCoord;"

            "patch in float gl_TessLevelOuter[4];"
            "patch in float gl_TessLevelInner[2];"

            "out gl_PerVertex {"
                "vec4 gl_Position;"
                "float gl_PointSize;"
                "float gl_ClipDistance[];"
            );
        if (version >= 400 && profile == ECompatibilityProfile)
            stageBuiltins[EShLangTessEvaluation].append(
                "vec4 gl_ClipVertex;"
                "vec4 gl_FrontColor;"
                "vec4 gl_BackColor;"
                "vec4 gl_FrontSecondaryColor;"
                "vec4 gl_BackSecondaryColor;"
                "vec4 gl_TexCoord[];"
                "float gl_FogFragCoord;"
                );
        if (version >= 450)
            stageBuiltins[EShLangTessEvaluation].append(
                "float gl_CullDistance[];"
                );
        stageBuiltins[EShLangTessEvaluation].append(
            "};"
            "\n");

#ifdef NV_EXTENSIONS
        if (version >= 450)
            stageBuiltins[EShLangTessEvaluation].append(
                "out int  gl_ViewportIndex;"
                "out int  gl_Layer;"
                "out int  gl_ViewportMask[];"             // GL_NV_viewport_array2
                "out vec4 gl_SecondaryPositionNV;"        // GL_NV_stereo_view_rendering
                "out int  gl_SecondaryViewportMaskNV[];"  // GL_NV_stereo_view_rendering
                "out vec4 gl_PositionPerViewNV[];"        // GL_NVX_multiview_per_view_attributes
                "out int  gl_ViewportMaskPerViewNV[];"    // GL_NVX_multiview_per_view_attributes
                );
#endif

    } else if (profile == EEsProfile && version >= 310) {
        // Note:  "in gl_PerVertex {...} gl_in[gl_MaxPatchVertices];" is declared in initialize() below,
        // as it depends on the resource sizing of gl_MaxPatchVertices.

        stageBuiltins[EShLangTessEvaluation].append(
            "in highp int gl_PatchVerticesIn;"
            "in highp int gl_PrimitiveID;"
            "in highp vec3 gl_TessCoord;"

            "patch in highp float gl_TessLevelOuter[4];"
            "patch in highp float gl_TessLevelInner[2];"

            "out gl_PerVertex {"
                "highp vec4 gl_Position;"
                "highp float gl_PointSize;"
            );
        stageBuiltins[EShLangTessEvaluation].append(
            "};"
            "\n");
    }

    if ((profile != EEsProfile && version >= 140) ||
        (profile == EEsProfile && version >= 310)) {
        stageBuiltins[EShLangTessEvaluation].append(
            "in highp int gl_DeviceIndex;"     // GL_EXT_device_group
            "in highp int gl_ViewIndex;"       // GL_EXT_multiview
            "\n");
    }

    //============================================================================
    //
    // Define the interface to the fragment shader.
    //
    //============================================================================

    if (profile != EEsProfile) {

        stageBuiltins[EShLangFragment].append(
            "vec4  gl_FragCoord;"   // needs qualifier fixed later
            "bool  gl_FrontFacing;" // needs qualifier fixed later
            "float gl_FragDepth;"   // needs qualifier fixed later
            );
        if (version >= 120)
            stageBuiltins[EShLangFragment].append(
                "vec2 gl_PointCoord;"  // needs qualifier fixed later
                );
        if (IncludeLegacy(version, profile, spvVersion) || (! ForwardCompatibility && version < 420))
            stageBuiltins[EShLangFragment].append(
                "vec4 gl_FragColor;"   // needs qualifier fixed later
                );

        if (version < 130) {
            stageBuiltins[EShLangFragment].append(
                "varying vec4  gl_Color;"
                "varying vec4  gl_SecondaryColor;"
                "varying vec4  gl_TexCoord[];"
                "varying float gl_FogFragCoord;"
                );
        } else {
            stageBuiltins[EShLangFragment].append(
                "in float gl_ClipDistance[];"
                );

            if (IncludeLegacy(version, profile, spvVersion)) {
                if (version < 150)
                    stageBuiltins[EShLangFragment].append(
                        "in float gl_FogFragCoord;"
                        "in vec4  gl_TexCoord[];"
                        "in vec4  gl_Color;"
                        "in vec4  gl_SecondaryColor;"
                        );
                else
                    stageBuiltins[EShLangFragment].append(
                        "in gl_PerFragment {"
                            "in float gl_FogFragCoord;"
                            "in vec4  gl_TexCoord[];"
                            "in vec4  gl_Color;"
                            "in vec4  gl_SecondaryColor;"
                        "};"
                        );
            }
        }

        if (version >= 150)
            stageBuiltins[EShLangFragment].append(
                "flat in int gl_PrimitiveID;"
                );

        if (version >= 400) {
            stageBuiltins[EShLangFragment].append(
                "flat in  int  gl_SampleID;"
                "     in  vec2 gl_SamplePosition;"
                "flat in  int  gl_SampleMaskIn[];"
                "     out int  gl_SampleMask[];"
                );
            if (spvVersion.spv == 0)
                stageBuiltins[EShLangFragment].append(
                    "uniform int gl_NumSamples;"
                    );
        }

        if (version >= 430)
            stageBuiltins[EShLangFragment].append(
                "flat in int gl_Layer;"
                "flat in int gl_ViewportIndex;"
                );

        if (version >= 450)
            stageBuiltins[EShLangFragment].append(
                "in float gl_CullDistance[];"
                "bool gl_HelperInvocation;"     // needs qualifier fixed later
                );

#ifdef AMD_EXTENSIONS
        if (version >= 450)
            stageBuiltins[EShLangFragment].append(
                "in vec2 gl_BaryCoordNoPerspAMD;"
                "in vec2 gl_BaryCoordNoPerspCentroidAMD;"
                "in vec2 gl_BaryCoordNoPerspSampleAMD;"
                "in vec2 gl_BaryCoordSmoothAMD;"
                "in vec2 gl_BaryCoordSmoothCentroidAMD;"
                "in vec2 gl_BaryCoordSmoothSampleAMD;"
                "in vec3 gl_BaryCoordPullModelAMD;"
                );
#endif
    } else {
        // ES profile

        if (version == 100) {
            stageBuiltins[EShLangFragment].append(
                "mediump vec4 gl_FragCoord;"    // needs qualifier fixed later
                "        bool gl_FrontFacing;"  // needs qualifier fixed later
                "mediump vec4 gl_FragColor;"    // needs qualifier fixed later
                "mediump vec2 gl_PointCoord;"   // needs qualifier fixed later
                );
        }
        if (version >= 300) {
            stageBuiltins[EShLangFragment].append(
                "highp   vec4  gl_FragCoord;"    // needs qualifier fixed later
                "        bool  gl_FrontFacing;"  // needs qualifier fixed later
                "mediump vec2  gl_PointCoord;"   // needs qualifier fixed later
                "highp   float gl_FragDepth;"    // needs qualifier fixed later
                );
        }
        if (version >= 310) {
            stageBuiltins[EShLangFragment].append(
                "bool gl_HelperInvocation;"          // needs qualifier fixed later
                "flat in highp int gl_PrimitiveID;"  // needs qualifier fixed later
                "flat in highp int gl_Layer;"        // needs qualifier fixed later
                );

            stageBuiltins[EShLangFragment].append(  // GL_OES_sample_variables
                "flat  in lowp     int gl_SampleID;"
                "      in mediump vec2 gl_SamplePosition;"
                "flat  in highp    int gl_SampleMaskIn[];"
                "     out highp    int gl_SampleMask[];"
                );
            if (spvVersion.spv == 0)
                stageBuiltins[EShLangFragment].append(  // GL_OES_sample_variables
                    "uniform lowp int gl_NumSamples;"
                    );
        }
        stageBuiltins[EShLangFragment].append(
            "highp float gl_FragDepthEXT;"       // GL_EXT_frag_depth
            );
    }
    stageBuiltins[EShLangFragment].append("\n");

    if (version >= 130)
        add2ndGenerationSamplingImaging(version, profile, spvVersion);

    // GL_ARB_shader_ballot
    if (profile != EEsProfile && version >= 450) {
        commonBuiltins.append(
            "uniform uint gl_SubGroupSizeARB;"

            "in uint     gl_SubGroupInvocationARB;"
            "in uint64_t gl_SubGroupEqMaskARB;"
            "in uint64_t gl_SubGroupGeMaskARB;"
            "in uint64_t gl_SubGroupGtMaskARB;"
            "in uint64_t gl_SubGroupLeMaskARB;"
            "in uint64_t gl_SubGroupLtMaskARB;"

            "\n");
    }

    if ((profile != EEsProfile && version >= 140) ||
        (profile == EEsProfile && version >= 310)) {
        stageBuiltins[EShLangFragment].append(
            "flat in highp int gl_DeviceIndex;"     // GL_EXT_device_group
            "flat in highp int gl_ViewIndex;"       // GL_EXT_multiview
            "\n");
    }

    // printf("%s\n", commonBuiltins.c_str());
    // printf("%s\n", stageBuiltins[EShLangFragment].c_str());
}

//
// Helper function for initialize(), to add the second set of names for texturing,
// when adding context-independent built-in functions.
//
void TBuiltIns::add2ndGenerationSamplingImaging(int version, EProfile profile, const SpvVersion& spvVersion)
{
    //
    // In this function proper, enumerate the types, then calls the next set of functions
    // to enumerate all the uses for that type.
    //

    TBasicType bTypes[3] = { EbtFloat, EbtInt, EbtUint };
    bool skipBuffer = (profile == EEsProfile && version < 310) || (profile != EEsProfile && version < 140);
    bool skipCubeArrayed = (profile == EEsProfile && version < 310) || (profile != EEsProfile && version < 130);

    // enumerate all the types
    for (int image = 0; image <= 1; ++image) { // loop over "bool" image vs sampler

        for (int shadow = 0; shadow <= 1; ++shadow) { // loop over "bool" shadow or not
            for (int ms = 0; ms <=1; ++ms) {
                if ((ms || image) && shadow)
                    continue;
                if (ms && profile != EEsProfile && version < 150)
                    continue;
                if (ms && image && profile == EEsProfile)
                    continue;
                if (ms && profile == EEsProfile && version < 310)
                    continue;

                for (int arrayed = 0; arrayed <= 1; ++arrayed) { // loop over "bool" arrayed or not
                    for (int dim = Esd1D; dim < EsdNumDims; ++dim) { // 1D, 2D, ..., buffer
                        if (dim == EsdSubpass && spvVersion.vulkan == 0)
                            continue;
                        if (dim == EsdSubpass && (image || shadow || arrayed))
                            continue;
                        if ((dim == Esd1D || dim == EsdRect) && profile == EEsProfile)
                            continue;
                        if (dim != Esd2D && dim != EsdSubpass && ms)
                            continue;
                        if ((dim == Esd3D || dim == EsdRect) && arrayed)
                            continue;
                        if (dim == Esd3D && shadow)
                            continue;
                        if (dim == EsdCube && arrayed && skipCubeArrayed)
                            continue;
                        if (dim == EsdBuffer && skipBuffer)
                            continue;
                        if (dim == EsdBuffer && (shadow || arrayed || ms))
                            continue;
                        if (ms && arrayed && profile == EEsProfile && version < 310)
                            continue;

                        for (int bType = 0; bType < 3; ++bType) { // float, int, uint results

                            if (shadow && bType > 0)
                                continue;

                            if (dim == EsdRect && version < 140 && bType > 0)
                                continue;

                            //
                            // Now, make all the function prototypes for the type we just built...
                            //

                            TSampler sampler;
                            if (dim == EsdSubpass) {
                                sampler.setSubpass(bTypes[bType], ms ? true : false);
                            } else if (image) {
                                sampler.setImage(bTypes[bType], (TSamplerDim)dim, arrayed ? true : false,
                                                                                  shadow  ? true : false,
                                                                                  ms      ? true : false);
                            } else {
                                sampler.set(bTypes[bType], (TSamplerDim)dim, arrayed ? true : false,
                                                                             shadow  ? true : false,
                                                                             ms      ? true : false);
                            }

                            TString typeName = sampler.getString();

                            if (dim == EsdSubpass) {
                                addSubpassSampling(sampler, typeName, version, profile);
                                continue;
                            }

                            addQueryFunctions(sampler, typeName, version, profile);

                            if (image)
                                addImageFunctions(sampler, typeName, version, profile);
                            else {
                                addSamplingFunctions(sampler, typeName, version, profile);
                                addGatherFunctions(sampler, typeName, version, profile);

                                if (spvVersion.vulkan > 0 && sampler.dim == EsdBuffer && sampler.isCombined()) {
                                    // Vulkan wants a textureBuffer to allow texelFetch() --
                                    // a sampled image with no sampler.
                                    // So, add sampling functions for both the
                                    // samplerBuffer and textureBuffer types.
                                    sampler.setTexture(sampler.type, sampler.dim, sampler.arrayed, sampler.shadow,
                                                       sampler.ms);
                                    TString textureTypeName = sampler.getString();
                                    addSamplingFunctions(sampler, textureTypeName, version, profile);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    //
    // sparseTexelsResidentARB()
    //

    if (profile != EEsProfile && version >= 450) {
        commonBuiltins.append("bool sparseTexelsResidentARB(int code);\n");
    }
}

//
// Helper function for add2ndGenerationSamplingImaging(),
// when adding context-independent built-in functions.
//
// Add all the query functions for the given type.
//
void TBuiltIns::addQueryFunctions(TSampler sampler, const TString& typeName, int version, EProfile profile)
{
    if (sampler.image && ((profile == EEsProfile && version < 310) || (profile != EEsProfile && version < 430)))
        return;

    //
    // textureSize() and imageSize()
    //

    int sizeDims = dimMap[sampler.dim] + (sampler.arrayed ? 1 : 0) - (sampler.dim == EsdCube ? 1 : 0);
    if (profile == EEsProfile)
        commonBuiltins.append("highp ");
    if (sizeDims == 1)
        commonBuiltins.append("int");
    else {
        commonBuiltins.append("ivec");
        commonBuiltins.append(postfixes[sizeDims]);
    }
    if (sampler.image)
        commonBuiltins.append(" imageSize(readonly writeonly volatile coherent ");
    else
        commonBuiltins.append(" textureSize(");
    commonBuiltins.append(typeName);
    if (! sampler.image && sampler.dim != EsdRect && sampler.dim != EsdBuffer && ! sampler.ms)
        commonBuiltins.append(",int);\n");
    else
        commonBuiltins.append(");\n");

    //
    // textureSamples() and imageSamples()
    //

    // GL_ARB_shader_texture_image_samples
    // TODO: spec issue? there are no memory qualifiers; how to query a writeonly/readonly image, etc?
    if (profile != EEsProfile && version >= 430 && sampler.ms) {
        commonBuiltins.append("int ");
        if (sampler.image)
            commonBuiltins.append("imageSamples(readonly writeonly volatile coherent ");
        else
            commonBuiltins.append("textureSamples(");
        commonBuiltins.append(typeName);
        commonBuiltins.append(");\n");
    }

    //
    // textureQueryLod(), fragment stage only
    //

    if (profile != EEsProfile && version >= 400 && ! sampler.image && sampler.dim != EsdRect && ! sampler.ms && sampler.dim != EsdBuffer) {
        stageBuiltins[EShLangFragment].append("vec2 textureQueryLod(");
        stageBuiltins[EShLangFragment].append(typeName);
        if (dimMap[sampler.dim] == 1)
            stageBuiltins[EShLangFragment].append(", float");
        else {
            stageBuiltins[EShLangFragment].append(", vec");
            stageBuiltins[EShLangFragment].append(postfixes[dimMap[sampler.dim]]);
        }
        stageBuiltins[EShLangFragment].append(");\n");
    }

    //
    // textureQueryLevels()
    //

    if (profile != EEsProfile && version >= 430 && ! sampler.image && sampler.dim != EsdRect && ! sampler.ms && sampler.dim != EsdBuffer) {
        commonBuiltins.append("int textureQueryLevels(");
        commonBuiltins.append(typeName);
        commonBuiltins.append(");\n");
    }
}

//
// Helper function for add2ndGenerationSamplingImaging(),
// when adding context-independent built-in functions.
//
// Add all the image access functions for the given type.
//
void TBuiltIns::addImageFunctions(TSampler sampler, const TString& typeName, int version, EProfile profile)
{
    int dims = dimMap[sampler.dim];
    // most things with an array add a dimension, except for cubemaps
    if (sampler.arrayed && sampler.dim != EsdCube)
        ++dims;

    TString imageParams = typeName;
    if (dims == 1)
        imageParams.append(", int");
    else {
        imageParams.append(", ivec");
        imageParams.append(postfixes[dims]);
    }
    if (sampler.ms)
        imageParams.append(", int");

    if (profile == EEsProfile)
        commonBuiltins.append("highp ");
    commonBuiltins.append(prefixes[sampler.type]);
    commonBuiltins.append("vec4 imageLoad(readonly volatile coherent ");
    commonBuiltins.append(imageParams);
    commonBuiltins.append(");\n");

    commonBuiltins.append("void imageStore(writeonly volatile coherent ");
    commonBuiltins.append(imageParams);
    commonBuiltins.append(", ");
    commonBuiltins.append(prefixes[sampler.type]);
    commonBuiltins.append("vec4);\n");

    if (sampler.dim != Esd1D && sampler.dim != EsdBuffer && profile != EEsProfile && version >= 450) {
        commonBuiltins.append("int sparseImageLoadARB(readonly volatile coherent ");
        commonBuiltins.append(imageParams);
        commonBuiltins.append(", out ");
        commonBuiltins.append(prefixes[sampler.type]);
        commonBuiltins.append("vec4");
        commonBuiltins.append(");\n");
    }

    if ( profile != EEsProfile ||
        (profile == EEsProfile && version >= 310)) {
        if (sampler.type == EbtInt || sampler.type == EbtUint) {
            const char* dataType = sampler.type == EbtInt ? "highp int" : "highp uint";

            const int numBuiltins = 7;

            static const char* atomicFunc[numBuiltins] = {
                " imageAtomicAdd(volatile coherent ",
                " imageAtomicMin(volatile coherent ",
                " imageAtomicMax(volatile coherent ",
                " imageAtomicAnd(volatile coherent ",
                " imageAtomicOr(volatile coherent ",
                " imageAtomicXor(volatile coherent ",
                " imageAtomicExchange(volatile coherent "
            };

            for (size_t i = 0; i < numBuiltins; ++i) {
                commonBuiltins.append(dataType);
                commonBuiltins.append(atomicFunc[i]);
                commonBuiltins.append(imageParams);
                commonBuiltins.append(", ");
                commonBuiltins.append(dataType);
                commonBuiltins.append(");\n");
            }

            commonBuiltins.append(dataType);
            commonBuiltins.append(" imageAtomicCompSwap(volatile coherent ");
            commonBuiltins.append(imageParams);
            commonBuiltins.append(", ");
            commonBuiltins.append(dataType);
            commonBuiltins.append(", ");
            commonBuiltins.append(dataType);
            commonBuiltins.append(");\n");
        } else {
            // not int or uint
            // GL_ARB_ES3_1_compatibility
            // TODO: spec issue: are there restrictions on the kind of layout() that can be used?  what about dropping memory qualifiers?
            if ((profile != EEsProfile && version >= 450) ||
                (profile == EEsProfile && version >= 310)) {
                commonBuiltins.append("float imageAtomicExchange(volatile coherent ");
                commonBuiltins.append(imageParams);
                commonBuiltins.append(", float);\n");
            }
        }
    }
}

//
// Helper function for initialize(),
// when adding context-independent built-in functions.
//
// Add all the subpass access functions for the given type.
//
void TBuiltIns::addSubpassSampling(TSampler sampler, const TString& typeName, int /*version*/, EProfile /*profile*/)
{
    stageBuiltins[EShLangFragment].append(prefixes[sampler.type]);
    stageBuiltins[EShLangFragment].append("vec4 subpassLoad");
    stageBuiltins[EShLangFragment].append("(");
    stageBuiltins[EShLangFragment].append(typeName.c_str());
    if (sampler.ms)
        stageBuiltins[EShLangFragment].append(", int");
    stageBuiltins[EShLangFragment].append(");\n");
}

//
// Helper function for add2ndGenerationSamplingImaging(),
// when adding context-independent built-in functions.
//
// Add all the texture lookup functions for the given type.
//
void TBuiltIns::addSamplingFunctions(TSampler sampler, const TString& typeName, int version, EProfile profile)
{
    //
    // texturing
    //
    for (int proj = 0; proj <= 1; ++proj) { // loop over "bool" projective or not

        if (proj && (sampler.dim == EsdCube || sampler.dim == EsdBuffer || sampler.arrayed || sampler.ms))
            continue;

        for (int lod = 0; lod <= 1; ++lod) {

            if (lod && (sampler.dim == EsdBuffer || sampler.dim == EsdRect || sampler.ms))
                continue;
            if (lod && sampler.dim == Esd2D && sampler.arrayed && sampler.shadow)
                continue;
            if (lod && sampler.dim == EsdCube && sampler.shadow)
                continue;

            for (int bias = 0; bias <= 1; ++bias) {

                if (bias && (lod || sampler.ms))
                    continue;
                if (bias && sampler.dim == Esd2D && sampler.shadow && sampler.arrayed)
                    continue;
                if (bias && (sampler.dim == EsdRect || sampler.dim == EsdBuffer))
                    continue;

                for (int offset = 0; offset <= 1; ++offset) { // loop over "bool" offset or not

                    if (proj + offset + bias + lod > 3)
                        continue;
                    if (offset && (sampler.dim == EsdCube || sampler.dim == EsdBuffer || sampler.ms))
                        continue;

                    for (int fetch = 0; fetch <= 1; ++fetch) { // loop over "bool" fetch or not

                        if (proj + offset + fetch + bias + lod > 3)
                            continue;
                        if (fetch && (lod || bias))
                            continue;
                        if (fetch && (sampler.shadow || sampler.dim == EsdCube))
                            continue;
                        if (fetch == 0 && (sampler.ms || sampler.dim == EsdBuffer))
                            continue;

                        for (int grad = 0; grad <= 1; ++grad) { // loop over "bool" grad or not

                            if (grad && (lod || bias || sampler.ms))
                                continue;
                            if (grad && sampler.dim == EsdBuffer)
                                continue;
                            if (proj + offset + fetch + grad + bias + lod > 3)
                                continue;

                            for (int extraProj = 0; extraProj <= 1; ++extraProj) {
                                bool compare = false;
                                int totalDims = dimMap[sampler.dim] + (sampler.arrayed ? 1 : 0);
                                // skip dummy unused second component for 1D non-array shadows
                                if (sampler.shadow && totalDims < 2)
                                    totalDims = 2;
                                totalDims += (sampler.shadow ? 1 : 0) + proj;
                                if (totalDims > 4 && sampler.shadow) {
                                    compare = true;
                                    totalDims = 4;
                                }
                                assert(totalDims <= 4);

                                if (extraProj && ! proj)
                                    continue;
                                if (extraProj && (sampler.dim == Esd3D || sampler.shadow))
                                    continue;

                                for (int lodClamp = 0; lodClamp <= 1 ;++lodClamp) { // loop over "bool" lod clamp

                                    if (lodClamp && (profile == EEsProfile || version < 450))
                                        continue;
                                    if (lodClamp && (proj || lod || fetch))
                                        continue;

                                    for (int sparse = 0; sparse <= 1; ++sparse) { // loop over "bool" sparse or not

                                        if (sparse && (profile == EEsProfile || version < 450))
                                            continue;
                                        // Sparse sampling is not for 1D/1D array texture, buffer texture, and projective texture
                                        if (sparse && (sampler.dim == Esd1D || sampler.dim == EsdBuffer || proj))
                                            continue;

                                        TString s;

                                        // return type
                                        if (sparse)
                                            s.append("int ");
                                        else {
                                            if (sampler.shadow)
                                                s.append("float ");
                                            else {
                                                s.append(prefixes[sampler.type]);
                                                s.append("vec4 ");
                                            }
                                        }

                                        // name
                                        if (sparse) {
                                            if (fetch)
                                                s.append("sparseTexel");
                                            else
                                                s.append("sparseTexture");
                                        } else {
                                            if (fetch)
                                                s.append("texel");
                                            else
                                                s.append("texture");
                                        }
                                        if (proj)
                                            s.append("Proj");
                                        if (lod)
                                            s.append("Lod");
                                        if (grad)
                                            s.append("Grad");
                                        if (fetch)
                                            s.append("Fetch");
                                        if (offset)
                                            s.append("Offset");
                                        if (lodClamp)
                                            s.append("Clamp");
                                        if (lodClamp || sparse)
                                            s.append("ARB");
                                        s.append("(");

                                        // sampler type
                                        s.append(typeName);

                                        // P coordinate
                                        if (extraProj)
                                            s.append(",vec4");
                                        else {
                                            s.append(",");
                                            TBasicType t = fetch ? EbtInt : EbtFloat;
                                            if (totalDims == 1)
                                                s.append(TType::getBasicString(t));
                                            else {
                                                s.append(prefixes[t]);
                                                s.append("vec");
                                                s.append(postfixes[totalDims]);
                                            }
                                        }

                                        if (bias && compare)
                                            continue;

                                        // non-optional lod argument (lod that's not driven by lod loop) or sample
                                        if ((fetch && sampler.dim != EsdBuffer && sampler.dim != EsdRect && !sampler.ms) ||
                                            (sampler.ms && fetch))
                                            s.append(",int");

                                        // non-optional lod
                                        if (lod)
                                            s.append(",float");

                                        // gradient arguments
                                        if (grad) {
                                            if (dimMap[sampler.dim] == 1)
                                                s.append(",float,float");
                                            else {
                                                s.append(",vec");
                                                s.append(postfixes[dimMap[sampler.dim]]);
                                                s.append(",vec");
                                                s.append(postfixes[dimMap[sampler.dim]]);
                                            }
                                        }

                                        // offset
                                        if (offset) {
                                            if (dimMap[sampler.dim] == 1)
                                                s.append(",int");
                                            else {
                                                s.append(",ivec");
                                                s.append(postfixes[dimMap[sampler.dim]]);
                                            }
                                        }

                                        // non-optional compare
                                        if (compare)
                                            s.append(",float");

                                        // lod clamp
                                        if (lodClamp)
                                            s.append(",float");

                                        // texel out (for sparse texture)
                                        if (sparse) {
                                            s.append(",out ");
                                            if (sampler.shadow)
                                                s.append("float ");
                                            else {
                                                s.append(prefixes[sampler.type]);
                                                s.append("vec4 ");
                                            }
                                        }

                                        // optional bias
                                        if (bias)
                                            s.append(",float");

                                        s.append(");\n");

                                        // Add to the per-language set of built-ins

                                        if (bias || lodClamp)
                                            stageBuiltins[EShLangFragment].append(s);
                                        else
                                            commonBuiltins.append(s);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

//
// Helper function for add2ndGenerationSamplingImaging(),
// when adding context-independent built-in functions.
//
// Add all the texture gather functions for the given type.
//
void TBuiltIns::addGatherFunctions(TSampler sampler, const TString& typeName, int version, EProfile profile)
{
    switch (sampler.dim) {
    case Esd2D:
    case EsdRect:
    case EsdCube:
        break;
    default:
        return;
    }

    if (sampler.ms)
        return;

    if (version < 140 && sampler.dim == EsdRect && sampler.type != EbtFloat)
        return;

    for (int offset = 0; offset < 3; ++offset) { // loop over three forms of offset in the call name:  none, Offset, and Offsets

        for (int comp = 0; comp < 2; ++comp) { // loop over presence of comp argument

            if (comp > 0 && sampler.shadow)
                continue;

            if (offset > 0 && sampler.dim == EsdCube)
                continue;

            for (int sparse = 0; sparse <= 1; ++sparse) { // loop over "bool" sparse or not
                if (sparse && (profile == EEsProfile || version < 450))
                    continue;

                TString s;

                // return type
                if (sparse)
                    s.append("int ");
                else {
                    s.append(prefixes[sampler.type]);
                    s.append("vec4 ");
                }

                // name
                if (sparse)
                    s.append("sparseTextureGather");
                else
                    s.append("textureGather");
                switch (offset) {
                case 1:
                    s.append("Offset");
                    break;
                case 2:
                    s.append("Offsets");
                default:
                    break;
                }

                if (sparse)
                    s.append("ARB");
                s.append("(");

                // sampler type argument
                s.append(typeName);

                // P coordinate argument
                s.append(",vec");
                int totalDims = dimMap[sampler.dim] + (sampler.arrayed ? 1 : 0);
                s.append(postfixes[totalDims]);

                // refZ argument
                if (sampler.shadow)
                    s.append(",float");

                // offset argument
                if (offset > 0) {
                    s.append(",ivec2");
                    if (offset == 2)
                        s.append("[4]");
                }

                // texel out (for sparse texture)
                if (sparse) {
                    s.append(",out ");
                    s.append(prefixes[sampler.type]);
                    s.append("vec4 ");
                }

                // comp argument
                if (comp)
                    s.append(",int");

                s.append(");\n");
                commonBuiltins.append(s);
            }
        }
    }

#ifdef AMD_EXTENSIONS
    if (sampler.dim == EsdRect || sampler.shadow)
        return;

    if (profile == EEsProfile || version < 450)
        return;

    for (int bias = 0; bias < 2; ++bias) { // loop over presence of bias argument

        for (int lod = 0; lod < 2; ++lod) { // loop over presence of lod argument

            if ((lod && bias) || (lod == 0 && bias == 0))
                continue;

            for (int offset = 0; offset < 3; ++offset) { // loop over three forms of offset in the call name:  none, Offset, and Offsets

                for (int comp = 0; comp < 2; ++comp) { // loop over presence of comp argument

                    if (comp == 0 && bias)
                        continue;

                    if (offset > 0 && sampler.dim == EsdCube)
                        continue;

                    for (int sparse = 0; sparse <= 1; ++sparse) { // loop over "bool" sparse or not
                        if (sparse && (profile == EEsProfile || version < 450))
                            continue;

                        TString s;

                        // return type
                        if (sparse)
                            s.append("int ");
                        else {
                            s.append(prefixes[sampler.type]);
                            s.append("vec4 ");
                        }

                        // name
                        if (sparse)
                            s.append("sparseTextureGather");
                        else
                            s.append("textureGather");

                        if (lod)
                            s.append("Lod");

                        switch (offset) {
                        case 1:
                            s.append("Offset");
                            break;
                        case 2:
                            s.append("Offsets");
                        default:
                            break;
                        }

                        if (lod)
                            s.append("AMD");
                        else if (sparse)
                            s.append("ARB");

                        s.append("(");

                        // sampler type argument
                        s.append(typeName);

                        // P coordinate argument
                        s.append(",vec");
                        int totalDims = dimMap[sampler.dim] + (sampler.arrayed ? 1 : 0);
                        s.append(postfixes[totalDims]);

                        // lod argument
                        if (lod)
                            s.append(",float");

                        // offset argument
                        if (offset > 0) {
                            s.append(",ivec2");
                            if (offset == 2)
                                s.append("[4]");
                        }

                        // texel out (for sparse texture)
                        if (sparse) {
                            s.append(",out ");
                            s.append(prefixes[sampler.type]);
                            s.append("vec4 ");
                        }

                        // comp argument
                        if (comp)
                            s.append(",int");

                        // bias argument
                        if (bias)
                            s.append(",float");

                        s.append(");\n");
                        if (bias)
                            stageBuiltins[EShLangFragment].append(s);
                        else
                            commonBuiltins.append(s);
                    }
                }
            }
        }
    }
#endif
}

//
// Add context-dependent built-in functions and variables that are present
// for the given version and profile.  All the results are put into just the
// commonBuiltins, because it is called for just a specific stage.  So,
// add stage-specific entries to the commonBuiltins, and only if that stage
// was requested.
//
void TBuiltIns::initialize(const TBuiltInResource &resources, int version, EProfile profile, const SpvVersion& spvVersion, EShLanguage language)
{
    //
    // Initialize the context-dependent (resource-dependent) built-in strings for parsing.
    //

    //============================================================================
    //
    // Standard Uniforms
    //
    //============================================================================

    TString& s = commonBuiltins;
    const int maxSize = 80;
    char builtInConstant[maxSize];

    //
    // Build string of implementation dependent constants.
    //

    if (profile == EEsProfile) {
        snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxVertexAttribs = %d;", resources.maxVertexAttribs);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxVertexUniformVectors = %d;", resources.maxVertexUniformVectors);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxVertexTextureImageUnits = %d;", resources.maxVertexTextureImageUnits);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxCombinedTextureImageUnits = %d;", resources.maxCombinedTextureImageUnits);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxTextureImageUnits = %d;", resources.maxTextureImageUnits);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxFragmentUniformVectors = %d;", resources.maxFragmentUniformVectors);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxDrawBuffers = %d;", resources.maxDrawBuffers);
        s.append(builtInConstant);

        if (version == 100) {
            snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxVaryingVectors = %d;", resources.maxVaryingVectors);
            s.append(builtInConstant);
        } else {
            snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxVertexOutputVectors = %d;", resources.maxVertexOutputVectors);
            s.append(builtInConstant);

            snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxFragmentInputVectors = %d;", resources.maxFragmentInputVectors);
            s.append(builtInConstant);

            snprintf(builtInConstant, maxSize, "const mediump int  gl_MinProgramTexelOffset = %d;", resources.minProgramTexelOffset);
            s.append(builtInConstant);

            snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxProgramTexelOffset = %d;", resources.maxProgramTexelOffset);
            s.append(builtInConstant);
        }

        if (version >= 310) {
            // geometry

            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryInputComponents = %d;", resources.maxGeometryInputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryOutputComponents = %d;", resources.maxGeometryOutputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryImageUniforms = %d;", resources.maxGeometryImageUniforms);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryTextureImageUnits = %d;", resources.maxGeometryTextureImageUnits);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryOutputVertices = %d;", resources.maxGeometryOutputVertices);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryTotalOutputComponents = %d;", resources.maxGeometryTotalOutputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryUniformComponents = %d;", resources.maxGeometryUniformComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryAtomicCounters = %d;", resources.maxGeometryAtomicCounters);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryAtomicCounterBuffers = %d;", resources.maxGeometryAtomicCounterBuffers);
            s.append(builtInConstant);

            // tessellation

            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlInputComponents = %d;", resources.maxTessControlInputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlOutputComponents = %d;", resources.maxTessControlOutputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlTextureImageUnits = %d;", resources.maxTessControlTextureImageUnits);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlUniformComponents = %d;", resources.maxTessControlUniformComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlTotalOutputComponents = %d;", resources.maxTessControlTotalOutputComponents);
            s.append(builtInConstant);

            snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationInputComponents = %d;", resources.maxTessEvaluationInputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationOutputComponents = %d;", resources.maxTessEvaluationOutputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationTextureImageUnits = %d;", resources.maxTessEvaluationTextureImageUnits);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationUniformComponents = %d;", resources.maxTessEvaluationUniformComponents);
            s.append(builtInConstant);

            snprintf(builtInConstant, maxSize, "const int gl_MaxTessPatchComponents = %d;", resources.maxTessPatchComponents);
            s.append(builtInConstant);

            snprintf(builtInConstant, maxSize, "const int gl_MaxPatchVertices = %d;", resources.maxPatchVertices);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessGenLevel = %d;", resources.maxTessGenLevel);
            s.append(builtInConstant);

            // this is here instead of with the others in initialize(version, profile) due to the dependence on gl_MaxPatchVertices
            if (language == EShLangTessControl || language == EShLangTessEvaluation) {
                s.append(
                    "in gl_PerVertex {"
                        "highp vec4 gl_Position;"
                        "highp float gl_PointSize;"
#ifdef NV_EXTENSIONS
                        "highp vec4 gl_SecondaryPositionNV;"  // GL_NV_stereo_view_rendering
                        "highp vec4 gl_PositionPerViewNV[];"  // GL_NVX_multiview_per_view_attributes
#endif
                    "} gl_in[gl_MaxPatchVertices];"
                    "\n");
            }
        }

    } else {
        // non-ES profile

        snprintf(builtInConstant, maxSize, "const int  gl_MaxVertexAttribs = %d;", resources.maxVertexAttribs);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int  gl_MaxVertexTextureImageUnits = %d;", resources.maxVertexTextureImageUnits);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int  gl_MaxCombinedTextureImageUnits = %d;", resources.maxCombinedTextureImageUnits);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int  gl_MaxTextureImageUnits = %d;", resources.maxTextureImageUnits);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int  gl_MaxDrawBuffers = %d;", resources.maxDrawBuffers);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int  gl_MaxLights = %d;", resources.maxLights);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int  gl_MaxClipPlanes = %d;", resources.maxClipPlanes);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int  gl_MaxTextureUnits = %d;", resources.maxTextureUnits);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int  gl_MaxTextureCoords = %d;", resources.maxTextureCoords);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int  gl_MaxVertexUniformComponents = %d;", resources.maxVertexUniformComponents);
        s.append(builtInConstant);

        if (version < 150 || ARBCompatibility) {
            snprintf(builtInConstant, maxSize, "const int  gl_MaxVaryingFloats = %d;", resources.maxVaryingFloats);
            s.append(builtInConstant);
        }

        snprintf(builtInConstant, maxSize, "const int  gl_MaxFragmentUniformComponents = %d;", resources.maxFragmentUniformComponents);
        s.append(builtInConstant);

        if (spvVersion.spv == 0 && IncludeLegacy(version, profile, spvVersion)) {
            //
            // OpenGL'uniform' state.  Page numbers are in reference to version
            // 1.4 of the OpenGL specification.
            //

            //
            // Matrix state. p. 31, 32, 37, 39, 40.
            //
            s.append("uniform mat4  gl_TextureMatrix[gl_MaxTextureCoords];"

            //
            // Derived matrix state that provides inverse and transposed versions
            // of the matrices above.
            //
                        "uniform mat4  gl_TextureMatrixInverse[gl_MaxTextureCoords];"

                        "uniform mat4  gl_TextureMatrixTranspose[gl_MaxTextureCoords];"

                        "uniform mat4  gl_TextureMatrixInverseTranspose[gl_MaxTextureCoords];"

            //
            // Clip planes p. 42.
            //
                        "uniform vec4  gl_ClipPlane[gl_MaxClipPlanes];"

            //
            // Light State p 50, 53, 55.
            //
                        "uniform gl_LightSourceParameters  gl_LightSource[gl_MaxLights];"

            //
            // Derived state from products of light.
            //
                        "uniform gl_LightProducts gl_FrontLightProduct[gl_MaxLights];"
                        "uniform gl_LightProducts gl_BackLightProduct[gl_MaxLights];"

            //
            // Texture Environment and Generation, p. 152, p. 40-42.
            //
                        "uniform vec4  gl_TextureEnvColor[gl_MaxTextureImageUnits];"
                        "uniform vec4  gl_EyePlaneS[gl_MaxTextureCoords];"
                        "uniform vec4  gl_EyePlaneT[gl_MaxTextureCoords];"
                        "uniform vec4  gl_EyePlaneR[gl_MaxTextureCoords];"
                        "uniform vec4  gl_EyePlaneQ[gl_MaxTextureCoords];"
                        "uniform vec4  gl_ObjectPlaneS[gl_MaxTextureCoords];"
                        "uniform vec4  gl_ObjectPlaneT[gl_MaxTextureCoords];"
                        "uniform vec4  gl_ObjectPlaneR[gl_MaxTextureCoords];"
                        "uniform vec4  gl_ObjectPlaneQ[gl_MaxTextureCoords];");
        }

        if (version >= 130) {
            snprintf(builtInConstant, maxSize, "const int gl_MaxClipDistances = %d;", resources.maxClipDistances);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxVaryingComponents = %d;", resources.maxVaryingComponents);
            s.append(builtInConstant);

            // GL_ARB_shading_language_420pack
            snprintf(builtInConstant, maxSize, "const mediump int  gl_MinProgramTexelOffset = %d;", resources.minProgramTexelOffset);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const mediump int  gl_MaxProgramTexelOffset = %d;", resources.maxProgramTexelOffset);
            s.append(builtInConstant);
        }

        // geometry
        if (version >= 150) {
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryInputComponents = %d;", resources.maxGeometryInputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryOutputComponents = %d;", resources.maxGeometryOutputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryTextureImageUnits = %d;", resources.maxGeometryTextureImageUnits);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryOutputVertices = %d;", resources.maxGeometryOutputVertices);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryTotalOutputComponents = %d;", resources.maxGeometryTotalOutputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryUniformComponents = %d;", resources.maxGeometryUniformComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryVaryingComponents = %d;", resources.maxGeometryVaryingComponents);
            s.append(builtInConstant);

        }

        if (version >= 150) {
            snprintf(builtInConstant, maxSize, "const int gl_MaxVertexOutputComponents = %d;", resources.maxVertexOutputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxFragmentInputComponents = %d;", resources.maxFragmentInputComponents);
            s.append(builtInConstant);
        }

        // tessellation
        if (version >= 150) {
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlInputComponents = %d;", resources.maxTessControlInputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlOutputComponents = %d;", resources.maxTessControlOutputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlTextureImageUnits = %d;", resources.maxTessControlTextureImageUnits);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlUniformComponents = %d;", resources.maxTessControlUniformComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlTotalOutputComponents = %d;", resources.maxTessControlTotalOutputComponents);
            s.append(builtInConstant);

            snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationInputComponents = %d;", resources.maxTessEvaluationInputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationOutputComponents = %d;", resources.maxTessEvaluationOutputComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationTextureImageUnits = %d;", resources.maxTessEvaluationTextureImageUnits);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationUniformComponents = %d;", resources.maxTessEvaluationUniformComponents);
            s.append(builtInConstant);

            snprintf(builtInConstant, maxSize, "const int gl_MaxTessPatchComponents = %d;", resources.maxTessPatchComponents);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessGenLevel = %d;", resources.maxTessGenLevel);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxPatchVertices = %d;", resources.maxPatchVertices);
            s.append(builtInConstant);

            // this is here instead of with the others in initialize(version, profile) due to the dependence on gl_MaxPatchVertices
            if (language == EShLangTessControl || language == EShLangTessEvaluation) {
                s.append(
                    "in gl_PerVertex {"
                        "vec4 gl_Position;"
                        "float gl_PointSize;"
                        "float gl_ClipDistance[];"
                    );
                if (profile == ECompatibilityProfile)
                    s.append(
                        "vec4 gl_ClipVertex;"
                        "vec4 gl_FrontColor;"
                        "vec4 gl_BackColor;"
                        "vec4 gl_FrontSecondaryColor;"
                        "vec4 gl_BackSecondaryColor;"
                        "vec4 gl_TexCoord[];"
                        "float gl_FogFragCoord;"
                        );
                if (profile != EEsProfile && version >= 450)
                    s.append(
                        "float gl_CullDistance[];"
#ifdef NV_EXTENSIONS
                        "vec4 gl_SecondaryPositionNV;"  // GL_NV_stereo_view_rendering
                        "vec4 gl_PositionPerViewNV[];"  // GL_NVX_multiview_per_view_attributes
#endif
                       );
                s.append(
                    "} gl_in[gl_MaxPatchVertices];"
                    "\n");
            }
        }

        if (version >= 150) {
            snprintf(builtInConstant, maxSize, "const int gl_MaxViewports = %d;", resources.maxViewports);
            s.append(builtInConstant);
        }

        // images
        if (version >= 130) {
            snprintf(builtInConstant, maxSize, "const int gl_MaxCombinedImageUnitsAndFragmentOutputs = %d;", resources.maxCombinedImageUnitsAndFragmentOutputs);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxImageSamples = %d;", resources.maxImageSamples);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlImageUniforms = %d;", resources.maxTessControlImageUniforms);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationImageUniforms = %d;", resources.maxTessEvaluationImageUniforms);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryImageUniforms = %d;", resources.maxGeometryImageUniforms);
            s.append(builtInConstant);
        }

        // enhanced layouts
        if (version >= 430) {
            snprintf(builtInConstant, maxSize, "const int gl_MaxTransformFeedbackBuffers = %d;", resources.maxTransformFeedbackBuffers);
            s.append(builtInConstant);
            snprintf(builtInConstant, maxSize, "const int gl_MaxTransformFeedbackInterleavedComponents = %d;", resources.maxTransformFeedbackInterleavedComponents);
            s.append(builtInConstant);
        }
    }

    // images (some in compute below)
    if ((profile == EEsProfile && version >= 310) ||
        (profile != EEsProfile && version >= 130)) {
        snprintf(builtInConstant, maxSize, "const int gl_MaxImageUnits = %d;", resources.maxImageUnits);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxCombinedShaderOutputResources = %d;", resources.maxCombinedShaderOutputResources);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxVertexImageUniforms = %d;", resources.maxVertexImageUniforms);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxFragmentImageUniforms = %d;", resources.maxFragmentImageUniforms);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxCombinedImageUniforms = %d;", resources.maxCombinedImageUniforms);
        s.append(builtInConstant);
    }

    // atomic counters (some in compute below)
    if ((profile == EEsProfile && version >= 310) ||
        (profile != EEsProfile && version >= 420)) {
        snprintf(builtInConstant, maxSize, "const int gl_MaxVertexAtomicCounters = %d;", resources.               maxVertexAtomicCounters);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxFragmentAtomicCounters = %d;", resources.             maxFragmentAtomicCounters);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxCombinedAtomicCounters = %d;", resources.             maxCombinedAtomicCounters);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxAtomicCounterBindings = %d;", resources.              maxAtomicCounterBindings);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxVertexAtomicCounterBuffers = %d;", resources.         maxVertexAtomicCounterBuffers);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxFragmentAtomicCounterBuffers = %d;", resources.       maxFragmentAtomicCounterBuffers);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxCombinedAtomicCounterBuffers = %d;", resources.       maxCombinedAtomicCounterBuffers);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxAtomicCounterBufferSize = %d;", resources.            maxAtomicCounterBufferSize);
        s.append(builtInConstant);
    }
    if (profile != EEsProfile && version >= 420) {
        snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlAtomicCounters = %d;", resources.          maxTessControlAtomicCounters);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationAtomicCounters = %d;", resources.       maxTessEvaluationAtomicCounters);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryAtomicCounters = %d;", resources.             maxGeometryAtomicCounters);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxTessControlAtomicCounterBuffers = %d;", resources.    maxTessControlAtomicCounterBuffers);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxTessEvaluationAtomicCounterBuffers = %d;", resources. maxTessEvaluationAtomicCounterBuffers);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxGeometryAtomicCounterBuffers = %d;", resources.       maxGeometryAtomicCounterBuffers);
        s.append(builtInConstant);

        s.append("\n");
    }

    // compute
    if ((profile == EEsProfile && version >= 310) || (profile != EEsProfile && version >= 420)) {
        snprintf(builtInConstant, maxSize, "const ivec3 gl_MaxComputeWorkGroupCount = ivec3(%d,%d,%d);", resources.maxComputeWorkGroupCountX,
                                                                                                         resources.maxComputeWorkGroupCountY,
                                                                                                         resources.maxComputeWorkGroupCountZ);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const ivec3 gl_MaxComputeWorkGroupSize = ivec3(%d,%d,%d);", resources.maxComputeWorkGroupSizeX,
                                                                                                        resources.maxComputeWorkGroupSizeY,
                                                                                                        resources.maxComputeWorkGroupSizeZ);
        s.append(builtInConstant);

        snprintf(builtInConstant, maxSize, "const int gl_MaxComputeUniformComponents = %d;", resources.maxComputeUniformComponents);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxComputeTextureImageUnits = %d;", resources.maxComputeTextureImageUnits);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxComputeImageUniforms = %d;", resources.maxComputeImageUniforms);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxComputeAtomicCounters = %d;", resources.maxComputeAtomicCounters);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxComputeAtomicCounterBuffers = %d;", resources.maxComputeAtomicCounterBuffers);
        s.append(builtInConstant);

        s.append("\n");
    }

    // GL_ARB_cull_distance
    if (profile != EEsProfile && version >= 450) {
        snprintf(builtInConstant, maxSize, "const int gl_MaxCullDistances = %d;",                resources.maxCullDistances);
        s.append(builtInConstant);
        snprintf(builtInConstant, maxSize, "const int gl_MaxCombinedClipAndCullDistances = %d;", resources.maxCombinedClipAndCullDistances);
        s.append(builtInConstant);
    }

    // GL_ARB_ES3_1_compatibility
    if ((profile != EEsProfile && version >= 450) ||
        (profile == EEsProfile && version >= 310)) {
        snprintf(builtInConstant, maxSize, "const int gl_MaxSamples = %d;", resources.maxSamples);
        s.append(builtInConstant);
    }

#ifdef AMD_EXTENSIONS
    // GL_AMD_gcn_shader
    if (profile != EEsProfile && version >= 450) {
        snprintf(builtInConstant, maxSize, "const int gl_SIMDGroupSizeAMD = 64;");
        s.append(builtInConstant);
    }
#endif

    s.append("\n");
}

//
// To support special built-ins that have a special qualifier that cannot be declared textually
// in a shader, like gl_Position.
//
// This lets the type of the built-in be declared textually, and then have just its qualifier be
// updated afterward.
//
// Safe to call even if name is not present.
//
// Only use this for built-in variables that have a special qualifier in TStorageQualifier.
// New built-in variables should use a generic (textually declarable) qualifier in
// TStoraregQualifier and only call BuiltInVariable().
//
static void SpecialQualifier(const char* name, TStorageQualifier qualifier, TBuiltInVariable builtIn, TSymbolTable& symbolTable)
{
    TSymbol* symbol = symbolTable.find(name);
    if (symbol) {
        TQualifier& symQualifier = symbol->getWritableType().getQualifier();
        symQualifier.storage = qualifier;
        symQualifier.builtIn = builtIn;
    }
}

//
// To tag built-in variables with their TBuiltInVariable enum.  Use this when the
// normal declaration text already gets the qualifier right, and all that's needed
// is setting the builtIn field.  This should be the normal way for all new
// built-in variables.
//
// If SpecialQualifier() was called, this does not need to be called.
//
// Safe to call even if name is not present.
//
static void BuiltInVariable(const char* name, TBuiltInVariable builtIn, TSymbolTable& symbolTable)
{
    TSymbol* symbol = symbolTable.find(name);
    if (! symbol)
        return;

    TQualifier& symQualifier = symbol->getWritableType().getQualifier();
    symQualifier.builtIn = builtIn;
}

//
// For built-in variables inside a named block.
// SpecialQualifier() won't ever go inside a block; their member's qualifier come
// from the qualification of the block.
//
// See comments above for other detail.
//
static void BuiltInVariable(const char* blockName, const char* name, TBuiltInVariable builtIn, TSymbolTable& symbolTable)
{
    TSymbol* symbol = symbolTable.find(blockName);
    if (! symbol)
        return;

    TTypeList& structure = *symbol->getWritableType().getWritableStruct();
    for (int i = 0; i < (int)structure.size(); ++i) {
        if (structure[i].type->getFieldName().compare(name) == 0) {
            structure[i].type->getQualifier().builtIn = builtIn;
            return;
        }
    }
}

//
// Finish adding/processing context-independent built-in symbols.
// 1) Programmatically add symbols that could not be added by simple text strings above.
// 2) Map built-in functions to operators, for those that will turn into an operation node
//    instead of remaining a function call.
// 3) Tag extension-related symbols added to their base version with their extensions, so
//    that if an early version has the extension turned off, there is an error reported on use.
//
void TBuiltIns::identifyBuiltIns(int version, EProfile profile, const SpvVersion& spvVersion, EShLanguage language, TSymbolTable& symbolTable)
{
    //
    // Tag built-in variables and functions with additional qualifier and extension information
    // that cannot be declared with the text strings.
    //

    // N.B.: a symbol should only be tagged once, and this function is called multiple times, once
    // per stage that's used for this profile.  So
    //  - generally, stick common ones in the fragment stage to ensure they are tagged exactly once
    //  - for ES, which has different precisions for different stages, the coarsest-grained tagging
    //    for a built-in used in many stages needs to be once for the fragment stage and once for
    //    the vertex stage

    switch(language) {
    case EShLangVertex:
        if (profile != EEsProfile) {
            symbolTable.setVariableExtensions("gl_BaseVertexARB",   1, &E_GL_ARB_shader_draw_parameters);
            symbolTable.setVariableExtensions("gl_BaseInstanceARB", 1, &E_GL_ARB_shader_draw_parameters);
            symbolTable.setVariableExtensions("gl_DrawIDARB",       1, &E_GL_ARB_shader_draw_parameters);

            BuiltInVariable("gl_BaseVertexARB",   EbvBaseVertex,   symbolTable);
            BuiltInVariable("gl_BaseInstanceARB", EbvBaseInstance, symbolTable);
            BuiltInVariable("gl_DrawIDARB",       EbvDrawId,       symbolTable);
        }

        if (profile != EEsProfile) {
            symbolTable.setVariableExtensions("gl_SubGroupSizeARB",       1, &E_GL_ARB_shader_ballot);
            symbolTable.setVariableExtensions("gl_SubGroupInvocationARB", 1, &E_GL_ARB_shader_ballot);
            symbolTable.setVariableExtensions("gl_SubGroupEqMaskARB",     1, &E_GL_ARB_shader_ballot);
            symbolTable.setVariableExtensions("gl_SubGroupGeMaskARB",     1, &E_GL_ARB_shader_ballot);
            symbolTable.setVariableExtensions("gl_SubGroupGtMaskARB",     1, &E_GL_ARB_shader_ballot);
            symbolTable.setVariableExtensions("gl_SubGroupLeMaskARB",     1, &E_GL_ARB_shader_ballot);
            symbolTable.setVariableExtensions("gl_SubGroupLtMaskARB",     1, &E_GL_ARB_shader_ballot);

            symbolTable.setFunctionExtensions("ballotARB",              1, &E_GL_ARB_shader_ballot);
            symbolTable.setFunctionExtensions("readInvocationARB",      1, &E_GL_ARB_shader_ballot);
            symbolTable.setFunctionExtensions("readFirstInvocationARB", 1, &E_GL_ARB_shader_ballot);

            BuiltInVariable("gl_SubGroupInvocationARB", EbvSubGroupInvocation, symbolTable);
            BuiltInVariable("gl_SubGroupEqMaskARB",     EbvSubGroupEqMask,     symbolTable);
            BuiltInVariable("gl_SubGroupGeMaskARB",     EbvSubGroupGeMask,     symbolTable);
            BuiltInVariable("gl_SubGroupGtMaskARB",     EbvSubGroupGtMask,     symbolTable);
            BuiltInVariable("gl_SubGroupLeMaskARB",     EbvSubGroupLeMask,     symbolTable);
            BuiltInVariable("gl_SubGroupLtMaskARB",     EbvSubGroupLtMask,     symbolTable);

            if (spvVersion.vulkan >= 100)
                // Treat "gl_SubGroupSizeARB" as shader input instead of uniform for Vulkan
                SpecialQualifier("gl_SubGroupSizeARB", EvqVaryingIn, EbvSubGroupSize, symbolTable);

            symbolTable.setFunctionExtensions("anyInvocationARB",       1, &E_GL_ARB_shader_group_vote);
            symbolTable.setFunctionExtensions("allInvocationsARB",      1, &E_GL_ARB_shader_group_vote);
            symbolTable.setFunctionExtensions("allInvocationsEqualARB", 1, &E_GL_ARB_shader_group_vote);
        }

#ifdef AMD_EXTENSIONS
        if (profile != EEsProfile) {
            symbolTable.setFunctionExtensions("minInvocationsAMD",                1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("maxInvocationsAMD",                1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("addInvocationsAMD",                1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("minInvocationsNonUniformAMD",      1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("maxInvocationsNonUniformAMD",      1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("addInvocationsNonUniformAMD",      1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("swizzleInvocationsAMD",            1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("swizzleInvocationsWithPatternAMD", 1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("writeInvocationAMD",               1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("mbcntAMD",                         1, &E_GL_AMD_shader_ballot);

            symbolTable.setFunctionExtensions("minInvocationsInclusiveScanAMD",             1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("maxInvocationsInclusiveScanAMD",             1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("addInvocationsInclusiveScanAMD",             1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("minInvocationsInclusiveScanNonUniformAMD",   1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("maxInvocationsInclusiveScanNonUniformAMD",   1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("addInvocationsInclusiveScanNonUniformAMD",   1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("minInvocationsExclusiveScanAMD",             1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("maxInvocationsExclusiveScanAMD",             1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("addInvocationsExclusiveScanAMD",             1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("minInvocationsExclusiveScanNonUniformAMD",   1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("maxInvocationsExclusiveScanNonUniformAMD",   1, &E_GL_AMD_shader_ballot);
            symbolTable.setFunctionExtensions("addInvocationsExclusiveScanNonUniformAMD",   1, &E_GL_AMD_shader_ballot);
        }

        if (profile != EEsProfile) {
            symbolTable.setFunctionExtensions("min3", 1, &E_GL_AMD_shader_trinary_minmax);
            symbolTable.setFunctionExtensions("max3", 1, &E_GL_AMD_shader_trinary_minmax);
            symbolTable.setFunctionExtensions("mid3", 1, &E_GL_AMD_shader_trinary_minmax);
        }

        if (profile != EEsProfile) {
            symbolTable.setFunctionExtensions("cubeFaceIndexAMD", 1, &E_GL_AMD_gcn_shader);
            symbolTable.setFunctionExtensions("cubeFaceCoordAMD", 1, &E_GL_AMD_gcn_shader);
            symbolTable.setFunctionExtensions("timeAMD",          1, &E_GL_AMD_gcn_shader);
        }
#endif

        // Compatibility variables, vertex only
        if (spvVersion.spv == 0) {
            BuiltInVariable("gl_Color",          EbvColor,          symbolTable);
            BuiltInVariable("gl_SecondaryColor", EbvSecondaryColor, symbolTable);
            BuiltInVariable("gl_Normal",         EbvNormal,         symbolTable);
            BuiltInVariable("gl_Vertex",         EbvVertex,         symbolTable);
            BuiltInVariable("gl_MultiTexCoord0", EbvMultiTexCoord0, symbolTable);
            BuiltInVariable("gl_MultiTexCoord1", EbvMultiTexCoord1, symbolTable);
            BuiltInVariable("gl_MultiTexCoord2", EbvMultiTexCoord2, symbolTable);
            BuiltInVariable("gl_MultiTexCoord3", EbvMultiTexCoord3, symbolTable);
            BuiltInVariable("gl_MultiTexCoord4", EbvMultiTexCoord4, symbolTable);
            BuiltInVariable("gl_MultiTexCoord5", EbvMultiTexCoord5, symbolTable);
            BuiltInVariable("gl_MultiTexCoord6", EbvMultiTexCoord6, symbolTable);
            BuiltInVariable("gl_MultiTexCoord7", EbvMultiTexCoord7, symbolTable);
            BuiltInVariable("gl_FogCoord",       EbvFogFragCoord,   symbolTable);
        }

        if (profile == EEsProfile) {
            if (spvVersion.spv == 0) {
                symbolTable.setFunctionExtensions("texture2DGradEXT",     1, &E_GL_EXT_shader_texture_lod);
                symbolTable.setFunctionExtensions("texture2DProjGradEXT", 1, &E_GL_EXT_shader_texture_lod);
                symbolTable.setFunctionExtensions("textureCubeGradEXT",   1, &E_GL_EXT_shader_texture_lod);
                symbolTable.setFunctionExtensions("textureGatherOffsets", Num_AEP_gpu_shader5, AEP_gpu_shader5);
            }
            if (version >= 310)
                symbolTable.setFunctionExtensions("fma", Num_AEP_gpu_shader5, AEP_gpu_shader5);
        }

        if (profile == EEsProfile) {
            symbolTable.setFunctionExtensions("imageAtomicAdd",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicMin",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicMax",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicAnd",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicOr",       1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicXor",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicExchange", 1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicCompSwap", 1, &E_GL_OES_shader_image_atomic);
        }

        if (spvVersion.vulkan == 0) {
            SpecialQualifier("gl_VertexID",   EvqVertexId,   EbvVertexId,   symbolTable);
            SpecialQualifier("gl_InstanceID", EvqInstanceId, EbvInstanceId, symbolTable);
        }

        if (spvVersion.vulkan >= 100) {
            BuiltInVariable("gl_VertexIndex",   EbvVertexIndex,   symbolTable);
            BuiltInVariable("gl_InstanceIndex", EbvInstanceIndex, symbolTable);
        }

        // Fall through

    case EShLangTessControl:
        if (profile == EEsProfile && version >= 310) {
            symbolTable.setVariableExtensions("gl_BoundingBoxOES", Num_AEP_primitive_bounding_box, AEP_primitive_bounding_box);
            BuiltInVariable("gl_BoundingBoxOES", EbvBoundingBox, symbolTable);
        }

        // Fall through

    case EShLangTessEvaluation:
    case EShLangGeometry:
        SpecialQualifier("gl_Position",   EvqPosition,   EbvPosition,   symbolTable);
        SpecialQualifier("gl_PointSize",  EvqPointSize,  EbvPointSize,  symbolTable);
        SpecialQualifier("gl_ClipVertex", EvqClipVertex, EbvClipVertex, symbolTable);

        BuiltInVariable("gl_in",  "gl_Position",     EbvPosition,     symbolTable);
        BuiltInVariable("gl_in",  "gl_PointSize",    EbvPointSize,    symbolTable);
        BuiltInVariable("gl_in",  "gl_ClipDistance", EbvClipDistance, symbolTable);
        BuiltInVariable("gl_in",  "gl_CullDistance", EbvCullDistance, symbolTable);

        BuiltInVariable("gl_out", "gl_Position",     EbvPosition,     symbolTable);
        BuiltInVariable("gl_out", "gl_PointSize",    EbvPointSize,    symbolTable);
        BuiltInVariable("gl_out", "gl_ClipDistance", EbvClipDistance, symbolTable);
        BuiltInVariable("gl_out", "gl_CullDistance", EbvCullDistance, symbolTable);

        BuiltInVariable("gl_ClipDistance",    EbvClipDistance,   symbolTable);
        BuiltInVariable("gl_CullDistance",    EbvCullDistance,   symbolTable);
        BuiltInVariable("gl_PrimitiveIDIn",   EbvPrimitiveId,    symbolTable);
        BuiltInVariable("gl_PrimitiveID",     EbvPrimitiveId,    symbolTable);
        BuiltInVariable("gl_InvocationID",    EbvInvocationId,   symbolTable);
        BuiltInVariable("gl_Layer",           EbvLayer,          symbolTable);
        BuiltInVariable("gl_ViewportIndex",   EbvViewportIndex,  symbolTable);

#ifdef NV_EXTENSIONS
        if (language != EShLangGeometry) {
            symbolTable.setVariableExtensions("gl_Layer",         Num_viewportEXTs, viewportEXTs);
            symbolTable.setVariableExtensions("gl_ViewportIndex", Num_viewportEXTs, viewportEXTs);
        }
        symbolTable.setVariableExtensions("gl_ViewportMask",            1, &E_GL_NV_viewport_array2);
        symbolTable.setVariableExtensions("gl_SecondaryPositionNV",     1, &E_GL_NV_stereo_view_rendering);
        symbolTable.setVariableExtensions("gl_SecondaryViewportMaskNV", 1, &E_GL_NV_stereo_view_rendering);
        symbolTable.setVariableExtensions("gl_PositionPerViewNV",       1, &E_GL_NVX_multiview_per_view_attributes);
        symbolTable.setVariableExtensions("gl_ViewportMaskPerViewNV",   1, &E_GL_NVX_multiview_per_view_attributes);

        BuiltInVariable("gl_ViewportMask",              EbvViewportMaskNV,          symbolTable);
        BuiltInVariable("gl_SecondaryPositionNV",       EbvSecondaryPositionNV,     symbolTable);
        BuiltInVariable("gl_SecondaryViewportMaskNV",   EbvSecondaryViewportMaskNV, symbolTable);
        BuiltInVariable("gl_PositionPerViewNV",         EbvPositionPerViewNV,       symbolTable);
        BuiltInVariable("gl_ViewportMaskPerViewNV",     EbvViewportMaskPerViewNV,   symbolTable);

        if (language != EShLangVertex) {
            BuiltInVariable("gl_in", "gl_SecondaryPositionNV", EbvSecondaryPositionNV, symbolTable);
            BuiltInVariable("gl_in", "gl_PositionPerViewNV",   EbvPositionPerViewNV,   symbolTable);
        }
        BuiltInVariable("gl_out", "gl_Layer",                   EbvLayer,                   symbolTable);
        BuiltInVariable("gl_out", "gl_ViewportIndex",           EbvViewportIndex,           symbolTable);
        BuiltInVariable("gl_out", "gl_ViewportMask",            EbvViewportMaskNV,          symbolTable);
        BuiltInVariable("gl_out", "gl_SecondaryPositionNV",     EbvSecondaryPositionNV,     symbolTable);
        BuiltInVariable("gl_out", "gl_SecondaryViewportMaskNV", EbvSecondaryViewportMaskNV, symbolTable);
        BuiltInVariable("gl_out", "gl_PositionPerViewNV",       EbvPositionPerViewNV,       symbolTable);
        BuiltInVariable("gl_out", "gl_ViewportMaskPerViewNV",   EbvViewportMaskPerViewNV,   symbolTable);
#endif

        BuiltInVariable("gl_PatchVerticesIn", EbvPatchVertices,  symbolTable);
        BuiltInVariable("gl_TessLevelOuter",  EbvTessLevelOuter, symbolTable);
        BuiltInVariable("gl_TessLevelInner",  EbvTessLevelInner, symbolTable);
        BuiltInVariable("gl_TessCoord",       EbvTessCoord,      symbolTable);

        if (version < 410)
            symbolTable.setVariableExtensions("gl_ViewportIndex", 1, &E_GL_ARB_viewport_array);

        // Compatibility variables

        BuiltInVariable("gl_in", "gl_ClipVertex",          EbvClipVertex,          symbolTable);
        BuiltInVariable("gl_in", "gl_FrontColor",          EbvFrontColor,          symbolTable);
        BuiltInVariable("gl_in", "gl_BackColor",           EbvBackColor,           symbolTable);
        BuiltInVariable("gl_in", "gl_FrontSecondaryColor", EbvFrontSecondaryColor, symbolTable);
        BuiltInVariable("gl_in", "gl_BackSecondaryColor",  EbvBackSecondaryColor,  symbolTable);
        BuiltInVariable("gl_in", "gl_TexCoord",            EbvTexCoord,            symbolTable);
        BuiltInVariable("gl_in", "gl_FogFragCoord",        EbvFogFragCoord,        symbolTable);

        BuiltInVariable("gl_out", "gl_ClipVertex",          EbvClipVertex,          symbolTable);
        BuiltInVariable("gl_out", "gl_FrontColor",          EbvFrontColor,          symbolTable);
        BuiltInVariable("gl_out", "gl_BackColor",           EbvBackColor,           symbolTable);
        BuiltInVariable("gl_out", "gl_FrontSecondaryColor", EbvFrontSecondaryColor, symbolTable);
        BuiltInVariable("gl_out", "gl_BackSecondaryColor",  EbvBackSecondaryColor,  symbolTable);
        BuiltInVariable("gl_out", "gl_TexCoord",            EbvTexCoord,            symbolTable);
        BuiltInVariable("gl_out", "gl_FogFragCoord",        EbvFogFragCoord,        symbolTable);

        BuiltInVariable("gl_ClipVertex",          EbvClipVertex,          symbolTable);
        BuiltInVariable("gl_FrontColor",          EbvFrontColor,          symbolTable);
        BuiltInVariable("gl_BackColor",           EbvBackColor,           symbolTable);
        BuiltInVariable("gl_FrontSecondaryColor", EbvFrontSecondaryColor, symbolTable);
        BuiltInVariable("gl_BackSecondaryColor",  EbvBackSecondaryColor,  symbolTable);
        BuiltInVariable("gl_TexCoord",            EbvTexCoord,            symbolTable);
        BuiltInVariable("gl_FogFragCoord",        EbvFogFragCoord,        symbolTable);

        // gl_PointSize, when it needs to be tied to an extension, is always a member of a block.
        // (Sometimes with an instance name, sometimes anonymous).
        // However, the current automatic extension scheme does not work per block member,
        // so for now check when parsing.
        //
        // if (profile == EEsProfile) {
        //    if (language == EShLangGeometry)
        //        symbolTable.setVariableExtensions("gl_PointSize", Num_AEP_geometry_point_size, AEP_geometry_point_size);
        //    else if (language == EShLangTessEvaluation || language == EShLangTessControl)
        //        symbolTable.setVariableExtensions("gl_PointSize", Num_AEP_tessellation_point_size, AEP_tessellation_point_size);
        //}

        if ((profile != EEsProfile && version >= 140) ||
            (profile == EEsProfile && version >= 310)) {
            symbolTable.setVariableExtensions("gl_DeviceIndex",  1, &E_GL_EXT_device_group);
            BuiltInVariable("gl_DeviceIndex", EbvDeviceIndex, symbolTable);
            symbolTable.setVariableExtensions("gl_ViewIndex", 1, &E_GL_EXT_multiview);
            BuiltInVariable("gl_ViewIndex", EbvViewIndex, symbolTable);
        }

        break;

    case EShLangFragment:
        SpecialQualifier("gl_FrontFacing",      EvqFace,       EbvFace,             symbolTable);
        SpecialQualifier("gl_FragCoord",        EvqFragCoord,  EbvFragCoord,        symbolTable);
        SpecialQualifier("gl_PointCoord",       EvqPointCoord, EbvPointCoord,       symbolTable);
        if (spvVersion.spv == 0)
            SpecialQualifier("gl_FragColor",    EvqFragColor,  EbvFragColor,        symbolTable);
        else {
            TSymbol* symbol = symbolTable.find("gl_FragColor");
            if (symbol) {
                symbol->getWritableType().getQualifier().storage = EvqVaryingOut;
                symbol->getWritableType().getQualifier().layoutLocation = 0;
            }
        }
        SpecialQualifier("gl_FragDepth",        EvqFragDepth,  EbvFragDepth,        symbolTable);
        SpecialQualifier("gl_FragDepthEXT",     EvqFragDepth,  EbvFragDepth,        symbolTable);
        SpecialQualifier("gl_HelperInvocation", EvqVaryingIn,  EbvHelperInvocation, symbolTable);

        BuiltInVariable("gl_ClipDistance",    EbvClipDistance,   symbolTable);
        BuiltInVariable("gl_CullDistance",    EbvCullDistance,   symbolTable);
        BuiltInVariable("gl_PrimitiveID",     EbvPrimitiveId,    symbolTable);

        if ((profile != EEsProfile && version >= 400) ||
            (profile == EEsProfile && version >= 310)) {
            BuiltInVariable("gl_SampleID",        EbvSampleId,       symbolTable);
            BuiltInVariable("gl_SamplePosition",  EbvSamplePosition, symbolTable);
            BuiltInVariable("gl_SampleMaskIn",    EbvSampleMask,     symbolTable);
            BuiltInVariable("gl_SampleMask",      EbvSampleMask,     symbolTable);
            if (profile == EEsProfile) {
                symbolTable.setVariableExtensions("gl_SampleID",       1, &E_GL_OES_sample_variables);
                symbolTable.setVariableExtensions("gl_SamplePosition", 1, &E_GL_OES_sample_variables);
                symbolTable.setVariableExtensions("gl_SampleMaskIn",   1, &E_GL_OES_sample_variables);
                symbolTable.setVariableExtensions("gl_SampleMask",     1, &E_GL_OES_sample_variables);
                symbolTable.setVariableExtensions("gl_NumSamples",     1, &E_GL_OES_sample_variables);
            }
        }

        BuiltInVariable("gl_Layer",           EbvLayer,          symbolTable);
        BuiltInVariable("gl_ViewportIndex",   EbvViewportIndex,  symbolTable);

        // Compatibility variables

        BuiltInVariable("gl_in", "gl_FogFragCoord",   EbvFogFragCoord,   symbolTable);
        BuiltInVariable("gl_in", "gl_TexCoord",       EbvTexCoord,       symbolTable);
        BuiltInVariable("gl_in", "gl_Color",          EbvColor,          symbolTable);
        BuiltInVariable("gl_in", "gl_SecondaryColor", EbvSecondaryColor, symbolTable);

        BuiltInVariable("gl_FogFragCoord",   EbvFogFragCoord,   symbolTable);
        BuiltInVariable("gl_TexCoord",       EbvTexCoord,       symbolTable);
        BuiltInVariable("gl_Color",          EbvColor,          symbolTable);
        BuiltInVariable("gl_SecondaryColor", EbvSecondaryColor, symbolTable);

        // built-in functions

        if (profile == EEsProfile) {
            if (spvVersion.spv == 0) {
                symbolTable.setFunctionExtensions("texture2DLodEXT",      1, &E_GL_EXT_shader_texture_lod);
                symbolTable.setFunctionExtensions("texture2DProjLodEXT",  1, &E_GL_EXT_shader_texture_lod);
                symbolTable.setFunctionExtensions("textureCubeLodEXT",    1, &E_GL_EXT_shader_texture_lod);
                symbolTable.setFunctionExtensions("texture2DGradEXT",     1, &E_GL_EXT_shader_texture_lod);
                symbolTable.setFunctionExtensions("texture2DProjGradEXT", 1, &E_GL_EXT_shader_texture_lod);
                symbolTable.setFunctionExtensions("textureCubeGradEXT",   1, &E_GL_EXT_shader_texture_lod);
                symbolTable.setFunctionExtensions("textureGatherOffsets", Num_AEP_gpu_shader5, AEP_gpu_shader5);
            }
            if (version == 100) {
                symbolTable.setFunctionExtensions("dFdx",   1, &E_GL_OES_standard_derivatives);
                symbolTable.setFunctionExtensions("dFdy",   1, &E_GL_OES_standard_derivatives);
                symbolTable.setFunctionExtensions("fwidth", 1, &E_GL_OES_standard_derivatives);
            }
            if (version >= 310) {
                symbolTable.setFunctionExtensions("fma", Num_AEP_gpu_shader5, AEP_gpu_shader5);
                symbolTable.setFunctionExtensions("interpolateAtCentroid", 1, &E_GL_OES_shader_multisample_interpolation);
                symbolTable.setFunctionExtensions("interpolateAtSample",   1, &E_GL_OES_shader_multisample_interpolation);
                symbolTable.setFunctionExtensions("interpolateAtOffset",   1, &E_GL_OES_shader_multisample_interpolation);
            }
        } else if (version < 130) {
            if (spvVersion.spv == 0) {
                symbolTable.setFunctionExtensions("texture1DLod",        1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("texture2DLod",        1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("texture3DLod",        1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("textureCubeLod",      1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("texture1DProjLod",    1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("texture2DProjLod",    1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("texture3DProjLod",    1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("shadow1DLod",         1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("shadow2DLod",         1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("shadow1DProjLod",     1, &E_GL_ARB_shader_texture_lod);
                symbolTable.setFunctionExtensions("shadow2DProjLod",     1, &E_GL_ARB_shader_texture_lod);
            }
        }

        // E_GL_ARB_shader_texture_lod functions usable only with the extension enabled
        if (profile != EEsProfile && spvVersion.spv == 0) {
            symbolTable.setFunctionExtensions("texture1DGradARB",         1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("texture1DProjGradARB",     1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("texture2DGradARB",         1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("texture2DProjGradARB",     1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("texture3DGradARB",         1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("texture3DProjGradARB",     1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("textureCubeGradARB",       1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("shadow1DGradARB",          1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("shadow1DProjGradARB",      1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("shadow2DGradARB",          1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("shadow2DProjGradARB",      1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("texture2DRectGradARB",     1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("texture2DRectProjGradARB", 1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("shadow2DRectGradARB",      1, &E_GL_ARB_shader_texture_lod);
            symbolTable.setFunctionExtensions("shadow2DRectProjGradARB",  1, &E_GL_ARB_shader_texture_lod);
        }

        // E_GL_ARB_shader_image_load_store
        if (profile != EEsProfile && version < 420)
            symbolTable.setFunctionExtensions("memoryBarrier", 1, &E_GL_ARB_shader_image_load_store);
        // All the image access functions are protected by checks on the type of the first argument.

        // E_GL_ARB_shader_atomic_counters
        if (profile != EEsProfile && version < 420) {
            symbolTable.setFunctionExtensions("atomicCounterIncrement", 1, &E_GL_ARB_shader_atomic_counters);
            symbolTable.setFunctionExtensions("atomicCounterDecrement", 1, &E_GL_ARB_shader_atomic_counters);
            symbolTable.setFunctionExtensions("atomicCounter"         , 1, &E_GL_ARB_shader_atomic_counters);
        }

        // E_GL_ARB_derivative_control
        if (profile != EEsProfile && version < 450) {
            symbolTable.setFunctionExtensions("dFdxFine",     1, &E_GL_ARB_derivative_control);
            symbolTable.setFunctionExtensions("dFdyFine",     1, &E_GL_ARB_derivative_control);
            symbolTable.setFunctionExtensions("fwidthFine",   1, &E_GL_ARB_derivative_control);
            symbolTable.setFunctionExtensions("dFdxCoarse",   1, &E_GL_ARB_derivative_control);
            symbolTable.setFunctionExtensions("dFdyCoarse",   1, &E_GL_ARB_derivative_control);
            symbolTable.setFunctionExtensions("fwidthCoarse", 1, &E_GL_ARB_derivative_control);
        }

        // E_GL_ARB_sparse_texture2
        if (profile != EEsProfile)
        {
            symbolTable.setFunctionExtensions("sparseTextureARB",              1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTextureLodARB",           1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTextureOffsetARB",        1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTexelFetchARB",           1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTexelFetchOffsetARB",     1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTextureLodOffsetARB",     1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTextureGradARB",          1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTextureGradOffsetARB",    1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTextureGatherARB",        1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTextureGatherOffsetARB",  1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTextureGatherOffsetsARB", 1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseImageLoadARB",            1, &E_GL_ARB_sparse_texture2);
            symbolTable.setFunctionExtensions("sparseTexelsResident",          1, &E_GL_ARB_sparse_texture2);
        }

        // E_GL_ARB_sparse_texture_clamp
        if (profile != EEsProfile)
        {
            symbolTable.setFunctionExtensions("sparseTextureClampARB",              1, &E_GL_ARB_sparse_texture_clamp);
            symbolTable.setFunctionExtensions("sparseTextureOffsetClampARB",        1, &E_GL_ARB_sparse_texture_clamp);
            symbolTable.setFunctionExtensions("sparseTextureGradClampARB",          1, &E_GL_ARB_sparse_texture_clamp);
            symbolTable.setFunctionExtensions("sparseTextureGradOffsetClampARB",    1, &E_GL_ARB_sparse_texture_clamp);
            symbolTable.setFunctionExtensions("textureClampARB",                    1, &E_GL_ARB_sparse_texture_clamp);
            symbolTable.setFunctionExtensions("textureOffsetClampARB",              1, &E_GL_ARB_sparse_texture_clamp);
            symbolTable.setFunctionExtensions("textureGradClampARB",                1, &E_GL_ARB_sparse_texture_clamp);
            symbolTable.setFunctionExtensions("textureGradOffsetClampARB",          1, &E_GL_ARB_sparse_texture_clamp);
        }

#ifdef AMD_EXTENSIONS
        // E_GL_AMD_shader_explicit_vertex_parameter
        if (profile != EEsProfile) {
            symbolTable.setVariableExtensions("gl_BaryCoordNoPerspAMD",         1, &E_GL_AMD_shader_explicit_vertex_parameter);
            symbolTable.setVariableExtensions("gl_BaryCoordNoPerspCentroidAMD", 1, &E_GL_AMD_shader_explicit_vertex_parameter);
            symbolTable.setVariableExtensions("gl_BaryCoordNoPerspSampleAMD",   1, &E_GL_AMD_shader_explicit_vertex_parameter);
            symbolTable.setVariableExtensions("gl_BaryCoordSmoothAMD",          1, &E_GL_AMD_shader_explicit_vertex_parameter);
            symbolTable.setVariableExtensions("gl_BaryCoordSmoothCentroidAMD",  1, &E_GL_AMD_shader_explicit_vertex_parameter);
            symbolTable.setVariableExtensions("gl_BaryCoordSmoothSampleAMD",    1, &E_GL_AMD_shader_explicit_vertex_parameter);
            symbolTable.setVariableExtensions("gl_BaryCoordPullModelAMD",       1, &E_GL_AMD_shader_explicit_vertex_parameter);

            symbolTable.setFunctionExtensions("interpolateAtVertexAMD",         1, &E_GL_AMD_shader_explicit_vertex_parameter);

            BuiltInVariable("gl_BaryCoordNoPerspAMD",           EbvBaryCoordNoPersp,         symbolTable);
            BuiltInVariable("gl_BaryCoordNoPerspCentroidAMD",   EbvBaryCoordNoPerspCentroid, symbolTable);
            BuiltInVariable("gl_BaryCoordNoPerspSampleAMD",     EbvBaryCoordNoPerspSample,   symbolTable);
            BuiltInVariable("gl_BaryCoordSmoothAMD",            EbvBaryCoordSmooth,          symbolTable);
            BuiltInVariable("gl_BaryCoordSmoothCentroidAMD",    EbvBaryCoordSmoothCentroid,  symbolTable);
            BuiltInVariable("gl_BaryCoordSmoothSampleAMD",      EbvBaryCoordSmoothSample,    symbolTable);
            BuiltInVariable("gl_BaryCoordPullModelAMD",         EbvBaryCoordPullModel,       symbolTable);
        }

        // E_GL_AMD_texture_gather_bias_lod
        if (profile != EEsProfile) {
            symbolTable.setFunctionExtensions("textureGatherLodAMD",                1, &E_GL_AMD_texture_gather_bias_lod);
            symbolTable.setFunctionExtensions("textureGatherLodOffsetAMD",          1, &E_GL_AMD_texture_gather_bias_lod);
            symbolTable.setFunctionExtensions("textureGatherLodOffsetsAMD",         1, &E_GL_AMD_texture_gather_bias_lod);
            symbolTable.setFunctionExtensions("sparseTextureGatherLodAMD",          1, &E_GL_AMD_texture_gather_bias_lod);
            symbolTable.setFunctionExtensions("sparseTextureGatherLodOffsetAMD",    1, &E_GL_AMD_texture_gather_bias_lod);
            symbolTable.setFunctionExtensions("sparseTextureGatherLodOffsetsAMD",   1, &E_GL_AMD_texture_gather_bias_lod);
        }
#endif

        symbolTable.setVariableExtensions("gl_FragDepthEXT", 1, &E_GL_EXT_frag_depth);

        if (profile == EEsProfile) {
            symbolTable.setVariableExtensions("gl_PrimitiveID",  Num_AEP_geometry_shader, AEP_geometry_shader);
            symbolTable.setVariableExtensions("gl_Layer",        Num_AEP_geometry_shader, AEP_geometry_shader);
        }

        if (profile == EEsProfile) {
            symbolTable.setFunctionExtensions("imageAtomicAdd",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicMin",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicMax",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicAnd",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicOr",       1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicXor",      1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicExchange", 1, &E_GL_OES_shader_image_atomic);
            symbolTable.setFunctionExtensions("imageAtomicCompSwap", 1, &E_GL_OES_shader_image_atomic);
        }

        symbolTable.setVariableExtensions("gl_DeviceIndex",  1, &E_GL_EXT_device_group);
        BuiltInVariable("gl_DeviceIndex", EbvDeviceIndex, symbolTable);
        symbolTable.setVariableExtensions("gl_ViewIndex", 1, &E_GL_EXT_multiview);
        BuiltInVariable("gl_ViewIndex", EbvViewIndex, symbolTable);
        break;

    case EShLangCompute:
        BuiltInVariable("gl_NumWorkGroups",         EbvNumWorkGroups,        symbolTable);
        BuiltInVariable("gl_WorkGroupSize",         EbvWorkGroupSize,        symbolTable);
        BuiltInVariable("gl_WorkGroupID",           EbvWorkGroupId,          symbolTable);
        BuiltInVariable("gl_LocalInvocationID",     EbvLocalInvocationId,    symbolTable);
        BuiltInVariable("gl_GlobalInvocationID",    EbvGlobalInvocationId,   symbolTable);
        BuiltInVariable("gl_LocalInvocationIndex",  EbvLocalInvocationIndex, symbolTable);

        if (profile != EEsProfile && version < 430) {
            symbolTable.setVariableExtensions("gl_NumWorkGroups",        1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_WorkGroupSize",        1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_WorkGroupID",          1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_LocalInvocationID",    1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_GlobalInvocationID",   1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_LocalInvocationIndex", 1, &E_GL_ARB_compute_shader);

            symbolTable.setVariableExtensions("gl_MaxComputeWorkGroupCount",       1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_MaxComputeWorkGroupSize",        1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_MaxComputeUniformComponents",    1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_MaxComputeTextureImageUnits",    1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_MaxComputeImageUniforms",        1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_MaxComputeAtomicCounters",       1, &E_GL_ARB_compute_shader);
            symbolTable.setVariableExtensions("gl_MaxComputeAtomicCounterBuffers", 1, &E_GL_ARB_compute_shader);

            symbolTable.setFunctionExtensions("barrier",                    1, &E_GL_ARB_compute_shader);
            symbolTable.setFunctionExtensions("memoryBarrierAtomicCounter", 1, &E_GL_ARB_compute_shader);
            symbolTable.setFunctionExtensions("memoryBarrierBuffer",        1, &E_GL_ARB_compute_shader);
            symbolTable.setFunctionExtensions("memoryBarrierImage",         1, &E_GL_ARB_compute_shader);
            symbolTable.setFunctionExtensions("memoryBarrierShared",        1, &E_GL_ARB_compute_shader);
            symbolTable.setFunctionExtensions("groupMemoryBarrier",         1, &E_GL_ARB_compute_shader);
        }

        if ((profile != EEsProfile && version >= 140) ||
            (profile == EEsProfile && version >= 310)) {
            symbolTable.setVariableExtensions("gl_DeviceIndex",  1, &E_GL_EXT_device_group);
            BuiltInVariable("gl_DeviceIndex", EbvDeviceIndex, symbolTable);
            symbolTable.setVariableExtensions("gl_ViewIndex", 1, &E_GL_EXT_multiview);
            BuiltInVariable("gl_ViewIndex", EbvViewIndex, symbolTable);
        }

        break;

    default:
        assert(false && "Language not supported");
        break;
    }

    //
    // Next, identify which built-ins have a mapping to an operator.
    // If PureOperatorBuiltins is false, those that are not identified as such are
    // expected to be resolved through a library of functions, versus as
    // operations.
    //
    symbolTable.relateToOperator("not",              EOpVectorLogicalNot);

    symbolTable.relateToOperator("matrixCompMult",   EOpMul);
    // 120 and 150 are correct for both ES and desktop
    if (version >= 120) {
        symbolTable.relateToOperator("outerProduct", EOpOuterProduct);
        symbolTable.relateToOperator("transpose", EOpTranspose);
        if (version >= 150) {
            symbolTable.relateToOperator("determinant", EOpDeterminant);
            symbolTable.relateToOperator("inverse", EOpMatrixInverse);
        }
    }

    symbolTable.relateToOperator("mod",              EOpMod);
    symbolTable.relateToOperator("modf",             EOpModf);

    symbolTable.relateToOperator("equal",            EOpVectorEqual);
    symbolTable.relateToOperator("notEqual",         EOpVectorNotEqual);
    symbolTable.relateToOperator("lessThan",         EOpLessThan);
    symbolTable.relateToOperator("greaterThan",      EOpGreaterThan);
    symbolTable.relateToOperator("lessThanEqual",    EOpLessThanEqual);
    symbolTable.relateToOperator("greaterThanEqual", EOpGreaterThanEqual);

    symbolTable.relateToOperator("radians",      EOpRadians);
    symbolTable.relateToOperator("degrees",      EOpDegrees);
    symbolTable.relateToOperator("sin",          EOpSin);
    symbolTable.relateToOperator("cos",          EOpCos);
    symbolTable.relateToOperator("tan",          EOpTan);
    symbolTable.relateToOperator("asin",         EOpAsin);
    symbolTable.relateToOperator("acos",         EOpAcos);
    symbolTable.relateToOperator("atan",         EOpAtan);
    symbolTable.relateToOperator("sinh",         EOpSinh);
    symbolTable.relateToOperator("cosh",         EOpCosh);
    symbolTable.relateToOperator("tanh",         EOpTanh);
    symbolTable.relateToOperator("asinh",        EOpAsinh);
    symbolTable.relateToOperator("acosh",        EOpAcosh);
    symbolTable.relateToOperator("atanh",        EOpAtanh);

    symbolTable.relateToOperator("pow",          EOpPow);
    symbolTable.relateToOperator("exp2",         EOpExp2);
    symbolTable.relateToOperator("log",          EOpLog);
    symbolTable.relateToOperator("exp",          EOpExp);
    symbolTable.relateToOperator("log2",         EOpLog2);
    symbolTable.relateToOperator("sqrt",         EOpSqrt);
    symbolTable.relateToOperator("inversesqrt",  EOpInverseSqrt);

    symbolTable.relateToOperator("abs",          EOpAbs);
    symbolTable.relateToOperator("sign",         EOpSign);
    symbolTable.relateToOperator("floor",        EOpFloor);
    symbolTable.relateToOperator("trunc",        EOpTrunc);
    symbolTable.relateToOperator("round",        EOpRound);
    symbolTable.relateToOperator("roundEven",    EOpRoundEven);
    symbolTable.relateToOperator("ceil",         EOpCeil);
    symbolTable.relateToOperator("fract",        EOpFract);
    symbolTable.relateToOperator("min",          EOpMin);
    symbolTable.relateToOperator("max",          EOpMax);
    symbolTable.relateToOperator("clamp",        EOpClamp);
    symbolTable.relateToOperator("mix",          EOpMix);
    symbolTable.relateToOperator("step",         EOpStep);
    symbolTable.relateToOperator("smoothstep",   EOpSmoothStep);

    symbolTable.relateToOperator("isnan",  EOpIsNan);
    symbolTable.relateToOperator("isinf",  EOpIsInf);

    symbolTable.relateToOperator("floatBitsToInt",  EOpFloatBitsToInt);
    symbolTable.relateToOperator("floatBitsToUint", EOpFloatBitsToUint);
    symbolTable.relateToOperator("intBitsToFloat",  EOpIntBitsToFloat);
    symbolTable.relateToOperator("uintBitsToFloat", EOpUintBitsToFloat);
    symbolTable.relateToOperator("doubleBitsToInt64",  EOpDoubleBitsToInt64);
    symbolTable.relateToOperator("doubleBitsToUint64", EOpDoubleBitsToUint64);
    symbolTable.relateToOperator("int64BitsToDouble",  EOpInt64BitsToDouble);
    symbolTable.relateToOperator("uint64BitsToDouble", EOpUint64BitsToDouble);
#ifdef AMD_EXTENSIONS
    symbolTable.relateToOperator("float16BitsToInt16",  EOpFloat16BitsToInt16);
    symbolTable.relateToOperator("float16BitsToUint16", EOpFloat16BitsToUint16);
    symbolTable.relateToOperator("int16BitsToFloat16",  EOpInt16BitsToFloat16);
    symbolTable.relateToOperator("uint16BitsToFloat16", EOpUint16BitsToFloat16);
#endif

    symbolTable.relateToOperator("packSnorm2x16",   EOpPackSnorm2x16);
    symbolTable.relateToOperator("unpackSnorm2x16", EOpUnpackSnorm2x16);
    symbolTable.relateToOperator("packUnorm2x16",   EOpPackUnorm2x16);
    symbolTable.relateToOperator("unpackUnorm2x16", EOpUnpackUnorm2x16);

    symbolTable.relateToOperator("packSnorm4x8",    EOpPackSnorm4x8);
    symbolTable.relateToOperator("unpackSnorm4x8",  EOpUnpackSnorm4x8);
    symbolTable.relateToOperator("packUnorm4x8",    EOpPackUnorm4x8);
    symbolTable.relateToOperator("unpackUnorm4x8",  EOpUnpackUnorm4x8);

    symbolTable.relateToOperator("packDouble2x32",    EOpPackDouble2x32);
    symbolTable.relateToOperator("unpackDouble2x32",  EOpUnpackDouble2x32);

    symbolTable.relateToOperator("packHalf2x16",    EOpPackHalf2x16);
    symbolTable.relateToOperator("unpackHalf2x16",  EOpUnpackHalf2x16);

    symbolTable.relateToOperator("packInt2x32",     EOpPackInt2x32);
    symbolTable.relateToOperator("unpackInt2x32",   EOpUnpackInt2x32);
    symbolTable.relateToOperator("packUint2x32",    EOpPackUint2x32);
    symbolTable.relateToOperator("unpackUint2x32",  EOpUnpackUint2x32);

#ifdef AMD_EXTENSIONS
    symbolTable.relateToOperator("packInt2x16",     EOpPackInt2x16);
    symbolTable.relateToOperator("unpackInt2x16",   EOpUnpackInt2x16);
    symbolTable.relateToOperator("packUint2x16",    EOpPackUint2x16);
    symbolTable.relateToOperator("unpackUint2x16",  EOpUnpackUint2x16);

    symbolTable.relateToOperator("packInt4x16",     EOpPackInt4x16);
    symbolTable.relateToOperator("unpackInt4x16",   EOpUnpackInt4x16);
    symbolTable.relateToOperator("packUint4x16",    EOpPackUint4x16);
    symbolTable.relateToOperator("unpackUint4x16",  EOpUnpackUint4x16);

    symbolTable.relateToOperator("packFloat2x16",   EOpPackFloat2x16);
    symbolTable.relateToOperator("unpackFloat2x16", EOpUnpackFloat2x16);
#endif

    symbolTable.relateToOperator("length",       EOpLength);
    symbolTable.relateToOperator("distance",     EOpDistance);
    symbolTable.relateToOperator("dot",          EOpDot);
    symbolTable.relateToOperator("cross",        EOpCross);
    symbolTable.relateToOperator("normalize",    EOpNormalize);
    symbolTable.relateToOperator("faceforward",  EOpFaceForward);
    symbolTable.relateToOperator("reflect",      EOpReflect);
    symbolTable.relateToOperator("refract",      EOpRefract);

    symbolTable.relateToOperator("any",          EOpAny);
    symbolTable.relateToOperator("all",          EOpAll);

    symbolTable.relateToOperator("barrier",                    EOpBarrier);
    symbolTable.relateToOperator("memoryBarrier",              EOpMemoryBarrier);
    symbolTable.relateToOperator("memoryBarrierAtomicCounter", EOpMemoryBarrierAtomicCounter);
    symbolTable.relateToOperator("memoryBarrierBuffer",        EOpMemoryBarrierBuffer);
    symbolTable.relateToOperator("memoryBarrierImage",         EOpMemoryBarrierImage);

    symbolTable.relateToOperator("atomicAdd",      EOpAtomicAdd);
    symbolTable.relateToOperator("atomicMin",      EOpAtomicMin);
    symbolTable.relateToOperator("atomicMax",      EOpAtomicMax);
    symbolTable.relateToOperator("atomicAnd",      EOpAtomicAnd);
    symbolTable.relateToOperator("atomicOr",       EOpAtomicOr);
    symbolTable.relateToOperator("atomicXor",      EOpAtomicXor);
    symbolTable.relateToOperator("atomicExchange", EOpAtomicExchange);
    symbolTable.relateToOperator("atomicCompSwap", EOpAtomicCompSwap);

    symbolTable.relateToOperator("atomicCounterIncrement", EOpAtomicCounterIncrement);
    symbolTable.relateToOperator("atomicCounterDecrement", EOpAtomicCounterDecrement);
    symbolTable.relateToOperator("atomicCounter",          EOpAtomicCounter);

    symbolTable.relateToOperator("fma",               EOpFma);
    symbolTable.relateToOperator("frexp",             EOpFrexp);
    symbolTable.relateToOperator("ldexp",             EOpLdexp);
    symbolTable.relateToOperator("uaddCarry",         EOpAddCarry);
    symbolTable.relateToOperator("usubBorrow",        EOpSubBorrow);
    symbolTable.relateToOperator("umulExtended",      EOpUMulExtended);
    symbolTable.relateToOperator("imulExtended",      EOpIMulExtended);
    symbolTable.relateToOperator("bitfieldExtract",   EOpBitfieldExtract);
    symbolTable.relateToOperator("bitfieldInsert",    EOpBitfieldInsert);
    symbolTable.relateToOperator("bitfieldReverse",   EOpBitFieldReverse);
    symbolTable.relateToOperator("bitCount",          EOpBitCount);
    symbolTable.relateToOperator("findLSB",           EOpFindLSB);
    symbolTable.relateToOperator("findMSB",           EOpFindMSB);

    if (PureOperatorBuiltins) {
        symbolTable.relateToOperator("imageSize",               EOpImageQuerySize);
        symbolTable.relateToOperator("imageSamples",            EOpImageQuerySamples);
        symbolTable.relateToOperator("imageLoad",               EOpImageLoad);
        symbolTable.relateToOperator("imageStore",              EOpImageStore);
        symbolTable.relateToOperator("imageAtomicAdd",          EOpImageAtomicAdd);
        symbolTable.relateToOperator("imageAtomicMin",          EOpImageAtomicMin);
        symbolTable.relateToOperator("imageAtomicMax",          EOpImageAtomicMax);
        symbolTable.relateToOperator("imageAtomicAnd",          EOpImageAtomicAnd);
        symbolTable.relateToOperator("imageAtomicOr",           EOpImageAtomicOr);
        symbolTable.relateToOperator("imageAtomicXor",          EOpImageAtomicXor);
        symbolTable.relateToOperator("imageAtomicExchange",     EOpImageAtomicExchange);
        symbolTable.relateToOperator("imageAtomicCompSwap",     EOpImageAtomicCompSwap);

        symbolTable.relateToOperator("subpassLoad",             EOpSubpassLoad);
        symbolTable.relateToOperator("subpassLoadMS",           EOpSubpassLoadMS);

        symbolTable.relateToOperator("textureSize",             EOpTextureQuerySize);
        symbolTable.relateToOperator("textureQueryLod",         EOpTextureQueryLod);
        symbolTable.relateToOperator("textureQueryLevels",      EOpTextureQueryLevels);
        symbolTable.relateToOperator("textureSamples",          EOpTextureQuerySamples);
        symbolTable.relateToOperator("texture",                 EOpTexture);
        symbolTable.relateToOperator("textureProj",             EOpTextureProj);
        symbolTable.relateToOperator("textureLod",              EOpTextureLod);
        symbolTable.relateToOperator("textureOffset",           EOpTextureOffset);
        symbolTable.relateToOperator("texelFetch",              EOpTextureFetch);
        symbolTable.relateToOperator("texelFetchOffset",        EOpTextureFetchOffset);
        symbolTable.relateToOperator("textureProjOffset",       EOpTextureProjOffset);
        symbolTable.relateToOperator("textureLodOffset",        EOpTextureLodOffset);
        symbolTable.relateToOperator("textureProjLod",          EOpTextureProjLod);
        symbolTable.relateToOperator("textureProjLodOffset",    EOpTextureProjLodOffset);
        symbolTable.relateToOperator("textureGrad",             EOpTextureGrad);
        symbolTable.relateToOperator("textureGradOffset",       EOpTextureGradOffset);
        symbolTable.relateToOperator("textureProjGrad",         EOpTextureProjGrad);
        symbolTable.relateToOperator("textureProjGradOffset",   EOpTextureProjGradOffset);
        symbolTable.relateToOperator("textureGather",           EOpTextureGather);
        symbolTable.relateToOperator("textureGatherOffset",     EOpTextureGatherOffset);
        symbolTable.relateToOperator("textureGatherOffsets",    EOpTextureGatherOffsets);

        symbolTable.relateToOperator("noise1", EOpNoise);
        symbolTable.relateToOperator("noise2", EOpNoise);
        symbolTable.relateToOperator("noise3", EOpNoise);
        symbolTable.relateToOperator("noise4", EOpNoise);

        if (spvVersion.spv == 0 && (IncludeLegacy(version, profile, spvVersion) ||
            (profile == EEsProfile && version == 100))) {
            symbolTable.relateToOperator("ftransform",               EOpFtransform);

            symbolTable.relateToOperator("texture1D",                EOpTexture);
            symbolTable.relateToOperator("texture1DGradARB",         EOpTextureGrad);
            symbolTable.relateToOperator("texture1DProj",            EOpTextureProj);
            symbolTable.relateToOperator("texture1DProjGradARB",     EOpTextureProjGrad);
            symbolTable.relateToOperator("texture1DLod",             EOpTextureLod);
            symbolTable.relateToOperator("texture1DProjLod",         EOpTextureProjLod);

            symbolTable.relateToOperator("texture2DRect",            EOpTexture);
            symbolTable.relateToOperator("texture2DRectProj",        EOpTextureProj);
            symbolTable.relateToOperator("texture2DRectGradARB",     EOpTextureGrad);
            symbolTable.relateToOperator("texture2DRectProjGradARB", EOpTextureProjGrad);
            symbolTable.relateToOperator("shadow2DRect",             EOpTexture);
            symbolTable.relateToOperator("shadow2DRectProj",         EOpTextureProj);
            symbolTable.relateToOperator("shadow2DRectGradARB",      EOpTextureGrad);
            symbolTable.relateToOperator("shadow2DRectProjGradARB",  EOpTextureProjGrad);

            symbolTable.relateToOperator("texture2D",                EOpTexture);
            symbolTable.relateToOperator("texture2DProj",            EOpTextureProj);
            symbolTable.relateToOperator("texture2DGradEXT",         EOpTextureGrad);
            symbolTable.relateToOperator("texture2DGradARB",         EOpTextureGrad);
            symbolTable.relateToOperator("texture2DProjGradEXT",     EOpTextureProjGrad);
            symbolTable.relateToOperator("texture2DProjGradARB",     EOpTextureProjGrad);
            symbolTable.relateToOperator("texture2DLod",             EOpTextureLod);
            symbolTable.relateToOperator("texture2DLodEXT",          EOpTextureLod);
            symbolTable.relateToOperator("texture2DProjLod",         EOpTextureProjLod);
            symbolTable.relateToOperator("texture2DProjLodEXT",      EOpTextureProjLod);

            symbolTable.relateToOperator("texture3D",                EOpTexture);
            symbolTable.relateToOperator("texture3DGradARB",         EOpTextureGrad);
            symbolTable.relateToOperator("texture3DProj",            EOpTextureProj);
            symbolTable.relateToOperator("texture3DProjGradARB",     EOpTextureProjGrad);
            symbolTable.relateToOperator("texture3DLod",             EOpTextureLod);
            symbolTable.relateToOperator("texture3DProjLod",         EOpTextureProjLod);
            symbolTable.relateToOperator("textureCube",              EOpTexture);
            symbolTable.relateToOperator("textureCubeGradEXT",       EOpTextureGrad);
            symbolTable.relateToOperator("textureCubeGradARB",       EOpTextureGrad);
            symbolTable.relateToOperator("textureCubeLod",           EOpTextureLod);
            symbolTable.relateToOperator("textureCubeLodEXT",        EOpTextureLod);
            symbolTable.relateToOperator("shadow1D",                 EOpTexture);
            symbolTable.relateToOperator("shadow1DGradARB",          EOpTextureGrad);
            symbolTable.relateToOperator("shadow2D",                 EOpTexture);
            symbolTable.relateToOperator("shadow2DGradARB",          EOpTextureGrad);
            symbolTable.relateToOperator("shadow1DProj",             EOpTextureProj);
            symbolTable.relateToOperator("shadow2DProj",             EOpTextureProj);
            symbolTable.relateToOperator("shadow1DProjGradARB",      EOpTextureProjGrad);
            symbolTable.relateToOperator("shadow2DProjGradARB",      EOpTextureProjGrad);
            symbolTable.relateToOperator("shadow1DLod",              EOpTextureLod);
            symbolTable.relateToOperator("shadow2DLod",              EOpTextureLod);
            symbolTable.relateToOperator("shadow1DProjLod",          EOpTextureProjLod);
            symbolTable.relateToOperator("shadow2DProjLod",          EOpTextureProjLod);
        }

        if (profile != EEsProfile) {
            symbolTable.relateToOperator("sparseTextureARB",                EOpSparseTexture);
            symbolTable.relateToOperator("sparseTextureLodARB",             EOpSparseTextureLod);
            symbolTable.relateToOperator("sparseTextureOffsetARB",          EOpSparseTextureOffset);
            symbolTable.relateToOperator("sparseTexelFetchARB",             EOpSparseTextureFetch);
            symbolTable.relateToOperator("sparseTexelFetchOffsetARB",       EOpSparseTextureFetchOffset);
            symbolTable.relateToOperator("sparseTextureLodOffsetARB",       EOpSparseTextureLodOffset);
            symbolTable.relateToOperator("sparseTextureGradARB",            EOpSparseTextureGrad);
            symbolTable.relateToOperator("sparseTextureGradOffsetARB",      EOpSparseTextureGradOffset);
            symbolTable.relateToOperator("sparseTextureGatherARB",          EOpSparseTextureGather);
            symbolTable.relateToOperator("sparseTextureGatherOffsetARB",    EOpSparseTextureGatherOffset);
            symbolTable.relateToOperator("sparseTextureGatherOffsetsARB",   EOpSparseTextureGatherOffsets);
            symbolTable.relateToOperator("sparseImageLoadARB",              EOpSparseImageLoad);
            symbolTable.relateToOperator("sparseTexelsResidentARB",         EOpSparseTexelsResident);

            symbolTable.relateToOperator("sparseTextureClampARB",           EOpSparseTextureClamp);
            symbolTable.relateToOperator("sparseTextureOffsetClampARB",     EOpSparseTextureOffsetClamp);
            symbolTable.relateToOperator("sparseTextureGradClampARB",       EOpSparseTextureGradClamp);
            symbolTable.relateToOperator("sparseTextureGradOffsetClampARB", EOpSparseTextureGradOffsetClamp);
            symbolTable.relateToOperator("textureClampARB",                 EOpTextureClamp);
            symbolTable.relateToOperator("textureOffsetClampARB",           EOpTextureOffsetClamp);
            symbolTable.relateToOperator("textureGradClampARB",             EOpTextureGradClamp);
            symbolTable.relateToOperator("textureGradOffsetClampARB",       EOpTextureGradOffsetClamp);

            symbolTable.relateToOperator("ballotARB",                       EOpBallot);
            symbolTable.relateToOperator("readInvocationARB",               EOpReadInvocation);
            symbolTable.relateToOperator("readFirstInvocationARB",          EOpReadFirstInvocation);

            symbolTable.relateToOperator("anyInvocationARB",                EOpAnyInvocation);
            symbolTable.relateToOperator("allInvocationsARB",               EOpAllInvocations);
            symbolTable.relateToOperator("allInvocationsEqualARB",          EOpAllInvocationsEqual);

#ifdef AMD_EXTENSIONS
            symbolTable.relateToOperator("minInvocationsAMD",                           EOpMinInvocations);
            symbolTable.relateToOperator("maxInvocationsAMD",                           EOpMaxInvocations);
            symbolTable.relateToOperator("addInvocationsAMD",                           EOpAddInvocations);
            symbolTable.relateToOperator("minInvocationsNonUniformAMD",                 EOpMinInvocationsNonUniform);
            symbolTable.relateToOperator("maxInvocationsNonUniformAMD",                 EOpMaxInvocationsNonUniform);
            symbolTable.relateToOperator("addInvocationsNonUniformAMD",                 EOpAddInvocationsNonUniform);
            symbolTable.relateToOperator("minInvocationsInclusiveScanAMD",              EOpMinInvocationsInclusiveScan);
            symbolTable.relateToOperator("maxInvocationsInclusiveScanAMD",              EOpMaxInvocationsInclusiveScan);
            symbolTable.relateToOperator("addInvocationsInclusiveScanAMD",              EOpAddInvocationsInclusiveScan);
            symbolTable.relateToOperator("minInvocationsInclusiveScanNonUniformAMD",    EOpMinInvocationsInclusiveScanNonUniform);
            symbolTable.relateToOperator("maxInvocationsInclusiveScanNonUniformAMD",    EOpMaxInvocationsInclusiveScanNonUniform);
            symbolTable.relateToOperator("addInvocationsInclusiveScanNonUniformAMD",    EOpAddInvocationsInclusiveScanNonUniform);
            symbolTable.relateToOperator("minInvocationsExclusiveScanAMD",              EOpMinInvocationsExclusiveScan);
            symbolTable.relateToOperator("maxInvocationsExclusiveScanAMD",              EOpMaxInvocationsExclusiveScan);
            symbolTable.relateToOperator("addInvocationsExclusiveScanAMD",              EOpAddInvocationsExclusiveScan);
            symbolTable.relateToOperator("minInvocationsExclusiveScanNonUniformAMD",    EOpMinInvocationsExclusiveScanNonUniform);
            symbolTable.relateToOperator("maxInvocationsExclusiveScanNonUniformAMD",    EOpMaxInvocationsExclusiveScanNonUniform);
            symbolTable.relateToOperator("addInvocationsExclusiveScanNonUniformAMD",    EOpAddInvocationsExclusiveScanNonUniform);
            symbolTable.relateToOperator("swizzleInvocationsAMD",                       EOpSwizzleInvocations);
            symbolTable.relateToOperator("swizzleInvocationsMaskedAMD",                 EOpSwizzleInvocationsMasked);
            symbolTable.relateToOperator("writeInvocationAMD",                          EOpWriteInvocation);
            symbolTable.relateToOperator("mbcntAMD",                                    EOpMbcnt);

            symbolTable.relateToOperator("min3",    EOpMin3);
            symbolTable.relateToOperator("max3",    EOpMax3);
            symbolTable.relateToOperator("mid3",    EOpMid3);

            symbolTable.relateToOperator("cubeFaceIndexAMD",    EOpCubeFaceIndex);
            symbolTable.relateToOperator("cubeFaceCoordAMD",    EOpCubeFaceCoord);
            symbolTable.relateToOperator("timeAMD",             EOpTime);

            symbolTable.relateToOperator("textureGatherLodAMD",                 EOpTextureGatherLod);
            symbolTable.relateToOperator("textureGatherLodOffsetAMD",           EOpTextureGatherLodOffset);
            symbolTable.relateToOperator("textureGatherLodOffsetsAMD",          EOpTextureGatherLodOffsets);
            symbolTable.relateToOperator("sparseTextureGatherLodAMD",           EOpSparseTextureGatherLod);
            symbolTable.relateToOperator("sparseTextureGatherLodOffsetAMD",     EOpSparseTextureGatherLodOffset);
            symbolTable.relateToOperator("sparseTextureGatherLodOffsetsAMD",    EOpSparseTextureGatherLodOffsets);
#endif
        }
    }

    switch(language) {
    case EShLangVertex:
        break;

    case EShLangTessControl:
    case EShLangTessEvaluation:
        break;

    case EShLangGeometry:
        symbolTable.relateToOperator("EmitStreamVertex",   EOpEmitStreamVertex);
        symbolTable.relateToOperator("EndStreamPrimitive", EOpEndStreamPrimitive);
        symbolTable.relateToOperator("EmitVertex",         EOpEmitVertex);
        symbolTable.relateToOperator("EndPrimitive",       EOpEndPrimitive);
        break;

    case EShLangFragment:
        symbolTable.relateToOperator("dFdx",         EOpDPdx);
        symbolTable.relateToOperator("dFdy",         EOpDPdy);
        symbolTable.relateToOperator("fwidth",       EOpFwidth);
        if (profile != EEsProfile && version >= 400) {
            symbolTable.relateToOperator("dFdxFine",     EOpDPdxFine);
            symbolTable.relateToOperator("dFdyFine",     EOpDPdyFine);
            symbolTable.relateToOperator("fwidthFine",   EOpFwidthFine);
            symbolTable.relateToOperator("dFdxCoarse",   EOpDPdxCoarse);
            symbolTable.relateToOperator("dFdyCoarse",   EOpDPdyCoarse);
            symbolTable.relateToOperator("fwidthCoarse", EOpFwidthCoarse);
        }
        symbolTable.relateToOperator("interpolateAtCentroid", EOpInterpolateAtCentroid);
        symbolTable.relateToOperator("interpolateAtSample",   EOpInterpolateAtSample);
        symbolTable.relateToOperator("interpolateAtOffset",   EOpInterpolateAtOffset);

#ifdef AMD_EXTENSIONS
        if (profile != EEsProfile)
            symbolTable.relateToOperator("interpolateAtVertexAMD", EOpInterpolateAtVertex);
        break;
#endif

    case EShLangCompute:
        symbolTable.relateToOperator("memoryBarrierShared",     EOpMemoryBarrierShared);
        symbolTable.relateToOperator("groupMemoryBarrier",      EOpGroupMemoryBarrier);
        break;

    default:
        assert(false && "Language not supported");
    }
}

//
// Add context-dependent (resource-specific) built-ins not handled by the above.  These
// would be ones that need to be programmatically added because they cannot
// be added by simple text strings.  For these, also
// 1) Map built-in functions to operators, for those that will turn into an operation node
//    instead of remaining a function call.
// 2) Tag extension-related symbols added to their base version with their extensions, so
//    that if an early version has the extension turned off, there is an error reported on use.
//
void TBuiltIns::identifyBuiltIns(int version, EProfile profile, const SpvVersion& spvVersion, EShLanguage language, TSymbolTable& symbolTable, const TBuiltInResource &resources)
{
    if (profile != EEsProfile && version >= 430 && version < 440) {
        symbolTable.setVariableExtensions("gl_MaxTransformFeedbackBuffers", 1, &E_GL_ARB_enhanced_layouts);
        symbolTable.setVariableExtensions("gl_MaxTransformFeedbackInterleavedComponents", 1, &E_GL_ARB_enhanced_layouts);
    }
    if (profile != EEsProfile && version >= 130 && version < 420) {
        symbolTable.setVariableExtensions("gl_MinProgramTexelOffset", 1, &E_GL_ARB_shading_language_420pack);
        symbolTable.setVariableExtensions("gl_MaxProgramTexelOffset", 1, &E_GL_ARB_shading_language_420pack);
    }
    if (profile != EEsProfile && version >= 150 && version < 410)
        symbolTable.setVariableExtensions("gl_MaxViewports", 1, &E_GL_ARB_viewport_array);

    switch(language) {
    case EShLangFragment:
        // Set up gl_FragData based on current array size.
        if (version == 100 || IncludeLegacy(version, profile, spvVersion) || (! ForwardCompatibility && profile != EEsProfile && version < 420)) {
            TPrecisionQualifier pq = profile == EEsProfile ? EpqMedium : EpqNone;
            TType fragData(EbtFloat, EvqFragColor, pq, 4);
            TArraySizes& arraySizes = *new TArraySizes;
            arraySizes.addInnerSize(resources.maxDrawBuffers);
            fragData.newArraySizes(arraySizes);
            symbolTable.insert(*new TVariable(NewPoolTString("gl_FragData"), fragData));
            SpecialQualifier("gl_FragData", EvqFragColor, EbvFragData, symbolTable);
        }
        break;

    case EShLangTessControl:
    case EShLangTessEvaluation:
        // Because of the context-dependent array size (gl_MaxPatchVertices),
        // these variables were added later than the others and need to be mapped now.

        // standard members
        BuiltInVariable("gl_in", "gl_Position",     EbvPosition,     symbolTable);
        BuiltInVariable("gl_in", "gl_PointSize",    EbvPointSize,    symbolTable);
        BuiltInVariable("gl_in", "gl_ClipDistance", EbvClipDistance, symbolTable);
        BuiltInVariable("gl_in", "gl_CullDistance", EbvCullDistance, symbolTable);

        // compatibility members
        BuiltInVariable("gl_in", "gl_ClipVertex",          EbvClipVertex,          symbolTable);
        BuiltInVariable("gl_in", "gl_FrontColor",          EbvFrontColor,          symbolTable);
        BuiltInVariable("gl_in", "gl_BackColor",           EbvBackColor,           symbolTable);
        BuiltInVariable("gl_in", "gl_FrontSecondaryColor", EbvFrontSecondaryColor, symbolTable);
        BuiltInVariable("gl_in", "gl_BackSecondaryColor",  EbvBackSecondaryColor,  symbolTable);
        BuiltInVariable("gl_in", "gl_TexCoord",            EbvTexCoord,            symbolTable);
        BuiltInVariable("gl_in", "gl_FogFragCoord",        EbvFogFragCoord,        symbolTable);
        break;

    default:
        break;
    }
}

} // end namespace glslang
