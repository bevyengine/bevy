// This file is part of the FidelityFX SDK.
//
// Copyright (c) 2022 Advanced Micro Devices, Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#if !defined(FFX_FSR2_COMMON_H)
#define FFX_FSR2_COMMON_H

#if defined(FFX_CPU) || defined(FFX_GPU)
//Locks
#define LOCK_LIFETIME_REMAINING 0
#define LOCK_TEMPORAL_LUMA 1
#define LOCK_TRUST 2
#endif // #if defined(FFX_CPU) || defined(FFX_GPU)

#if defined(FFX_GPU)
FFX_STATIC const FfxFloat32 FSR2_EPSILON = 1e-03f;
FFX_STATIC const FfxFloat32 FSR2_TONEMAP_EPSILON = 1e-03f;
FFX_STATIC const FfxFloat32 FSR2_FLT_MAX = 3.402823466e+38f;
FFX_STATIC const FfxFloat32 FSR2_FLT_MIN = 1.175494351e-38f;

// treat vector truncation warnings as errors
#pragma warning(error: 3206)

// suppress warnings
#pragma warning(disable: 3205)  // conversion from larger type to smaller
#pragma warning(disable: 3571)  // in ffxPow(f, e), f could be negative

// Reconstructed depth usage
FFX_STATIC const FfxFloat32 reconstructedDepthBilinearWeightThreshold = 0.05f;

// Accumulation
FFX_STATIC const FfxFloat32 averageLanczosWeightPerFrame = 0.74f; // Average lanczos weight for jitter accumulated samples
FFX_STATIC const FfxFloat32 accumulationMaxOnMotion = 4.0f;

// Auto exposure
FFX_STATIC const FfxFloat32 resetAutoExposureAverageSmoothing = 1e8f;

struct LockState
{
    FfxBoolean NewLock; //Set for both unique new and re-locked new
    FfxBoolean WasLockedPrevFrame; //Set to identify if the pixel was already locked (relock)
};

FfxFloat32 GetNormalizedRemainingLockLifetime(FfxFloat32x3 fLockStatus)
{
    const FfxFloat32 fTrust = fLockStatus[LOCK_TRUST];

    return ffxSaturate(fLockStatus[LOCK_LIFETIME_REMAINING] - LockInitialLifetime()) / LockInitialLifetime() * fTrust;
}

#if FFX_HALF
FFX_MIN16_F GetNormalizedRemainingLockLifetime(FFX_MIN16_F3 fLockStatus)
{
    const FFX_MIN16_F fTrust = fLockStatus[LOCK_TRUST];
    const FFX_MIN16_F fInitialLockLifetime = FFX_MIN16_F(LockInitialLifetime());

    return ffxSaturate(fLockStatus[LOCK_LIFETIME_REMAINING] - fInitialLockLifetime) / fInitialLockLifetime * fTrust;
}
#endif

void InitializeNewLockSample(FFX_PARAMETER_OUT FfxFloat32x3 fLockStatus)
{
    fLockStatus = FfxFloat32x3(0, 0, 1); // LOCK_TRUST to 1
}

#if FFX_HALF
void InitializeNewLockSample(FFX_PARAMETER_OUT FFX_MIN16_F3 fLockStatus)
{
    fLockStatus = FFX_MIN16_F3(0, 0, 1); // LOCK_TRUST to 1
}
#endif


void KillLock(FFX_PARAMETER_INOUT FfxFloat32x3 fLockStatus)
{
    fLockStatus[LOCK_LIFETIME_REMAINING] = 0;
}

#if FFX_HALF
void KillLock(FFX_PARAMETER_INOUT FFX_MIN16_F3 fLockStatus)
{
    fLockStatus[LOCK_LIFETIME_REMAINING] = FFX_MIN16_F(0);
}
#endif

struct RectificationBoxData
{
    FfxFloat32x3 boxCenter;
    FfxFloat32x3 boxVec;
    FfxFloat32x3 aabbMin;
    FfxFloat32x3 aabbMax;
};
#if FFX_HALF
struct RectificationBoxDataMin16
{
    FFX_MIN16_F3 boxCenter;
    FFX_MIN16_F3 boxVec;
    FFX_MIN16_F3 aabbMin;
    FFX_MIN16_F3 aabbMax;
};
#endif

struct RectificationBox
{
    RectificationBoxData data_;
    FfxFloat32 fBoxCenterWeight;
};
#if FFX_HALF
struct RectificationBoxMin16
{
    RectificationBoxDataMin16 data_;
    FFX_MIN16_F fBoxCenterWeight;
};
#endif

void RectificationBoxReset(FFX_PARAMETER_INOUT RectificationBox rectificationBox, const FfxFloat32x3 initialColorSample)
{
    rectificationBox.fBoxCenterWeight = FfxFloat32(0);

    rectificationBox.data_.boxCenter = FfxFloat32x3(0, 0, 0);
    rectificationBox.data_.boxVec = FfxFloat32x3(0, 0, 0);
    rectificationBox.data_.aabbMin = initialColorSample;
    rectificationBox.data_.aabbMax = initialColorSample;
}
#if FFX_HALF
void RectificationBoxReset(FFX_PARAMETER_INOUT RectificationBoxMin16 rectificationBox, const FFX_MIN16_F3 initialColorSample)
{
    rectificationBox.fBoxCenterWeight = FFX_MIN16_F(0);

    rectificationBox.data_.boxCenter = FFX_MIN16_F3(0, 0, 0);
    rectificationBox.data_.boxVec = FFX_MIN16_F3(0, 0, 0);
    rectificationBox.data_.aabbMin = initialColorSample;
    rectificationBox.data_.aabbMax = initialColorSample;
}
#endif

void RectificationBoxAddSample(FFX_PARAMETER_INOUT RectificationBox rectificationBox, const FfxFloat32x3 colorSample, const FfxFloat32 fSampleWeight)
{
    rectificationBox.data_.aabbMin = ffxMin(rectificationBox.data_.aabbMin, colorSample);
    rectificationBox.data_.aabbMax = ffxMax(rectificationBox.data_.aabbMax, colorSample);
    FfxFloat32x3 weightedSample = colorSample * fSampleWeight;
    rectificationBox.data_.boxCenter += weightedSample;
    rectificationBox.data_.boxVec += colorSample * weightedSample;
    rectificationBox.fBoxCenterWeight += fSampleWeight;
}
#if FFX_HALF
void RectificationBoxAddSample(FFX_PARAMETER_INOUT RectificationBoxMin16 rectificationBox, const FFX_MIN16_F3 colorSample, const FFX_MIN16_F fSampleWeight)
{
    rectificationBox.data_.aabbMin = ffxMin(rectificationBox.data_.aabbMin, colorSample);
    rectificationBox.data_.aabbMax = ffxMax(rectificationBox.data_.aabbMax, colorSample);
    FFX_MIN16_F3 weightedSample = colorSample * fSampleWeight;
    rectificationBox.data_.boxCenter += weightedSample;
    rectificationBox.data_.boxVec += colorSample * weightedSample;
    rectificationBox.fBoxCenterWeight += fSampleWeight;
}
#endif

void RectificationBoxComputeVarianceBoxData(FFX_PARAMETER_INOUT RectificationBox rectificationBox)
{
    rectificationBox.fBoxCenterWeight = (abs(rectificationBox.fBoxCenterWeight) > FfxFloat32(FSR2_EPSILON) ? rectificationBox.fBoxCenterWeight : FfxFloat32(1.f));
    rectificationBox.data_.boxCenter /= rectificationBox.fBoxCenterWeight;
    rectificationBox.data_.boxVec /= rectificationBox.fBoxCenterWeight;
    FfxFloat32x3 stdDev = sqrt(abs(rectificationBox.data_.boxVec - rectificationBox.data_.boxCenter * rectificationBox.data_.boxCenter));
    rectificationBox.data_.boxVec = stdDev;
}
#if FFX_HALF
void RectificationBoxComputeVarianceBoxData(FFX_PARAMETER_INOUT RectificationBoxMin16 rectificationBox)
{
    rectificationBox.fBoxCenterWeight = (abs(rectificationBox.fBoxCenterWeight) > FFX_MIN16_F(FSR2_EPSILON) ? rectificationBox.fBoxCenterWeight : FFX_MIN16_F(1.f));
    rectificationBox.data_.boxCenter /= rectificationBox.fBoxCenterWeight;
    rectificationBox.data_.boxVec /= rectificationBox.fBoxCenterWeight;
    FFX_MIN16_F3 stdDev = sqrt(abs(rectificationBox.data_.boxVec - rectificationBox.data_.boxCenter * rectificationBox.data_.boxCenter));
    rectificationBox.data_.boxVec = stdDev;
}
#endif

RectificationBoxData RectificationBoxGetData(FFX_PARAMETER_INOUT RectificationBox rectificationBox)
{
    return rectificationBox.data_;
}
#if FFX_HALF
RectificationBoxDataMin16 RectificationBoxGetData(FFX_PARAMETER_INOUT RectificationBoxMin16 rectificationBox)
{
    return rectificationBox.data_;
}
#endif

FfxFloat32x3 SafeRcp3(FfxFloat32x3 v)
{
    return (all(FFX_NOT_EQUAL(v, FfxFloat32x3(0, 0, 0)))) ? (FfxFloat32x3(1, 1, 1) / v) : FfxFloat32x3(0, 0, 0);
}
#if FFX_HALF
FFX_MIN16_F3 SafeRcp3(FFX_MIN16_F3 v)
{
    return (all(FFX_NOT_EQUAL(v, FFX_MIN16_F3(0, 0, 0)))) ? (FFX_MIN16_F3(1, 1, 1) / v) : FFX_MIN16_F3(0, 0, 0);
}
#endif

FfxFloat32 MinDividedByMax(const FfxFloat32 v0, const FfxFloat32 v1)
{
    const FfxFloat32 m = ffxMax(v0, v1);
    return m != 0 ? ffxMin(v0, v1) / m : 0;
}

#if FFX_HALF
FFX_MIN16_F MinDividedByMax(const FFX_MIN16_F v0, const FFX_MIN16_F v1)
{
    const FFX_MIN16_F m = ffxMax(v0, v1);
    return m != FFX_MIN16_F(0) ? ffxMin(v0, v1) / m : FFX_MIN16_F(0);
}
#endif

FfxFloat32x3 YCoCgToRGB(FfxFloat32x3 fYCoCg)
{
    FfxFloat32x3 fRgb;

    fYCoCg.yz -= FfxFloat32x2(0.5f, 0.5f);  // [0,1] -> [-0.5,0.5]

    fRgb = FfxFloat32x3(
        fYCoCg.x + fYCoCg.y - fYCoCg.z,
        fYCoCg.x + fYCoCg.z,
        fYCoCg.x - fYCoCg.y - fYCoCg.z);

    return fRgb;
}
#if FFX_HALF
FFX_MIN16_F3 YCoCgToRGB(FFX_MIN16_F3 fYCoCg)
{
    FFX_MIN16_F3 fRgb;

    fYCoCg.yz -= FFX_MIN16_F2(0.5f, 0.5f);  // [0,1] -> [-0.5,0.5]

    fRgb = FFX_MIN16_F3(
        fYCoCg.x + fYCoCg.y - fYCoCg.z,
        fYCoCg.x + fYCoCg.z,
        fYCoCg.x - fYCoCg.y - fYCoCg.z);

    return fRgb;
}
#endif

FfxFloat32x3 RGBToYCoCg(FfxFloat32x3 fRgb)
{
    FfxFloat32x3 fYCoCg;

    fYCoCg = FfxFloat32x3(
        0.25f * fRgb.r + 0.5f * fRgb.g + 0.25f * fRgb.b,
        0.5f * fRgb.r - 0.5f * fRgb.b,
        -0.25f * fRgb.r + 0.5f * fRgb.g - 0.25f * fRgb.b);

    fYCoCg.yz += FfxFloat32x2(0.5f, 0.5f); // [-0.5,0.5] -> [0,1]

    return fYCoCg;
}
#if FFX_HALF
FFX_MIN16_F3 RGBToYCoCg(FFX_MIN16_F3 fRgb)
{
    FFX_MIN16_F3 fYCoCg;

    fYCoCg = FFX_MIN16_F3(
        0.25 * fRgb.r + 0.5 * fRgb.g + 0.25 * fRgb.b,
        0.5 * fRgb.r - 0.5 * fRgb.b,
        -0.25 * fRgb.r + 0.5 * fRgb.g - 0.25 * fRgb.b);

    fYCoCg.yz += FFX_MIN16_F2(0.5, 0.5); // [-0.5,0.5] -> [0,1]

    return fYCoCg;
}
#endif

FfxFloat32 RGBToLuma(FfxFloat32x3 fLinearRgb)
{
    return dot(fLinearRgb, FfxFloat32x3(0.2126f, 0.7152f, 0.0722f));
}
#if FFX_HALF
FFX_MIN16_F RGBToLuma(FFX_MIN16_F3 fLinearRgb)
{
    return dot(fLinearRgb, FFX_MIN16_F3(0.2126f, 0.7152f, 0.0722f));
}
#endif

FfxFloat32 RGBToPerceivedLuma(FfxFloat32x3 fLinearRgb)
{
    FfxFloat32 fLuminance = RGBToLuma(fLinearRgb);

    FfxFloat32 fPercievedLuminance = 0;
    if (fLuminance <= 216.0f / 24389.0f) {
        fPercievedLuminance = fLuminance * (24389.0f / 27.0f);
    } else {
        fPercievedLuminance = ffxPow(fLuminance, 1.0f / 3.0f) * 116.0f - 16.0f;
    }

    return fPercievedLuminance * 0.01f;
}
#if FFX_HALF
FFX_MIN16_F RGBToPerceivedLuma(FFX_MIN16_F3 fLinearRgb)
{
    FFX_MIN16_F fLuminance = RGBToLuma(fLinearRgb);

    FFX_MIN16_F fPercievedLuminance = FFX_MIN16_F(0);
    if (fLuminance <= FFX_MIN16_F(216.0f / 24389.0f)) {
        fPercievedLuminance = fLuminance * FFX_MIN16_F(24389.0f / 27.0f);
    }
    else {
        fPercievedLuminance = ffxPow(fLuminance, FFX_MIN16_F(1.0f / 3.0f)) * FFX_MIN16_F(116.0f) - FFX_MIN16_F(16.0f);
    }

    return fPercievedLuminance * FFX_MIN16_F(0.01f);
}
#endif


FfxFloat32x3 Tonemap(FfxFloat32x3 fRgb)
{
    return fRgb / (ffxMax(ffxMax(0.f, fRgb.r), ffxMax(fRgb.g, fRgb.b)) + 1.f).xxx;
}

FfxFloat32x3 InverseTonemap(FfxFloat32x3 fRgb)
{
    return fRgb / ffxMax(FSR2_TONEMAP_EPSILON, 1.f - ffxMax(fRgb.r, ffxMax(fRgb.g, fRgb.b))).xxx;
}

#if FFX_HALF
FFX_MIN16_F3 Tonemap(FFX_MIN16_F3 fRgb)
{
    return fRgb / (ffxMax(ffxMax(FFX_MIN16_F(0.f), fRgb.r), ffxMax(fRgb.g, fRgb.b)) + FFX_MIN16_F(1.f)).xxx;
}

FFX_MIN16_F3 InverseTonemap(FFX_MIN16_F3 fRgb)
{
    return fRgb / ffxMax(FFX_MIN16_F(FSR2_TONEMAP_EPSILON), FFX_MIN16_F(1.f) - ffxMax(fRgb.r, ffxMax(fRgb.g, fRgb.b))).xxx;
}
#endif

FfxInt32x2 ClampLoad(FfxInt32x2 iPxSample, FfxInt32x2 iPxOffset, FfxInt32x2 iTextureSize)
{
    return clamp(iPxSample + iPxOffset, FfxInt32x2(0, 0), iTextureSize - FfxInt32x2(1, 1));
}
#if FFX_HALF
FFX_MIN16_I2 ClampLoad(FFX_MIN16_I2 iPxSample, FFX_MIN16_I2 iPxOffset, FFX_MIN16_I2 iTextureSize)
{
    return clamp(iPxSample + iPxOffset, FFX_MIN16_I2(0, 0), iTextureSize - FFX_MIN16_I2(1, 1));
}
#endif

FfxBoolean IsOnScreen(FfxInt32x2 pos, FfxInt32x2 size)
{
    return all(FFX_GREATER_THAN_EQUAL(pos, FfxInt32x2(0, 0))) && all(FFX_LESS_THAN(pos, size));
}
#if FFX_HALF
FfxBoolean IsOnScreen(FFX_MIN16_I2 pos, FFX_MIN16_I2 size)
{
    return all(FFX_GREATER_THAN_EQUAL(pos, FFX_MIN16_I2(0, 0))) && all(FFX_LESS_THAN(pos, size));
}
#endif

FfxFloat32 ComputeAutoExposureFromLavg(FfxFloat32 Lavg)
{
    Lavg = exp(Lavg);

    const FfxFloat32 S = 100.0f; //ISO arithmetic speed
    const FfxFloat32 K = 12.5f;
    FfxFloat32 ExposureISO100 = log2((Lavg * S) / K);

    const FfxFloat32 q = 0.65f;
    FfxFloat32 Lmax = (78.0f / (q * S)) * ffxPow(2.0f, ExposureISO100);

    return 1 / Lmax;
}
#if FFX_HALF
FFX_MIN16_F ComputeAutoExposureFromLavg(FFX_MIN16_F Lavg)
{
    Lavg = exp(Lavg);

    const FFX_MIN16_F S = FFX_MIN16_F(100.0f); //ISO arithmetic speed
    const FFX_MIN16_F K = FFX_MIN16_F(12.5f);
    const FFX_MIN16_F ExposureISO100 = log2((Lavg * S) / K);

    const FFX_MIN16_F q = FFX_MIN16_F(0.65f);
    const FFX_MIN16_F Lmax = (FFX_MIN16_F(78.0f) / (q * S)) * ffxPow(FFX_MIN16_F(2.0f), ExposureISO100);

    return FFX_MIN16_F(1) / Lmax;
}
#endif

FfxInt32x2 ComputeHrPosFromLrPos(FfxInt32x2 iPxLrPos)
{
    FfxFloat32x2 fSrcJitteredPos = FfxFloat32x2(iPxLrPos) + 0.5f - Jitter();
    FfxFloat32x2 fLrPosInHr = (fSrcJitteredPos / RenderSize()) * DisplaySize();
    FfxFloat32x2 fHrPos = floor(fLrPosInHr) + 0.5f;
    return FfxInt32x2(fHrPos);
}
#if FFX_HALF
FFX_MIN16_I2 ComputeHrPosFromLrPos(FFX_MIN16_I2 iPxLrPos)
{
    FFX_MIN16_F2 fSrcJitteredPos = FFX_MIN16_F2(iPxLrPos) + FFX_MIN16_F(0.5f) - FFX_MIN16_F2(Jitter());
    FFX_MIN16_F2 fLrPosInHr = (fSrcJitteredPos / FFX_MIN16_F2(RenderSize())) * FFX_MIN16_F2(DisplaySize());
    FFX_MIN16_F2 fHrPos = floor(fLrPosInHr) + FFX_MIN16_F(0.5);
    return FFX_MIN16_I2(fHrPos);
}
#endif

#endif // #if defined(FFX_GPU)

#endif //!defined(FFX_FSR2_COMMON_H)
