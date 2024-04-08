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

#ifndef FFX_FSR2_RESOURCES_H
#define FFX_FSR2_RESOURCES_H

#if defined(FFX_CPU) || defined(FFX_GPU)
#define FFX_FSR2_RESOURCE_IDENTIFIER_NULL                                           0
#define FFX_FSR2_RESOURCE_IDENTIFIER_INPUT_COLOR                                    1
#define FFX_FSR2_RESOURCE_IDENTIFIER_INPUT_MOTION_VECTORS                           2
#define FFX_FSR2_RESOURCE_IDENTIFIER_INPUT_DEPTH                                    3
#define FFX_FSR2_RESOURCE_IDENTIFIER_INPUT_EXPOSURE                                 4
#define FFX_FSR2_RESOURCE_IDENTIFIER_INPUT_REACTIVE_MASK                            5
#define FFX_FSR2_RESOURCE_IDENTIFIER_INPUT_TRANSPARENCY_AND_COMPOSITION_MASK        6
#define FFX_FSR2_RESOURCE_IDENTIFIER_RECONSTRUCTED_PREVIOUS_NEAREST_DEPTH           7
#define FFX_FSR2_RESOURCE_IDENTIFIER_DILATED_MOTION_VECTORS                         8
#define FFX_FSR2_RESOURCE_IDENTIFIER_DILATED_DEPTH                                  9
#define FFX_FSR2_RESOURCE_IDENTIFIER_INTERNAL_UPSCALED_COLOR                        10
#define FFX_FSR2_RESOURCE_IDENTIFIER_LOCK_STATUS                                    11
#define FFX_FSR2_RESOURCE_IDENTIFIER_DEPTH_CLIP                                     12
#define FFX_FSR2_RESOURCE_IDENTIFIER_PREPARED_INPUT_COLOR                           13
#define FFX_FSR2_RESOURCE_IDENTIFIER_LUMA_HISTORY                                   14
#define FFX_FSR2_RESOURCE_IDENTIFIER_DEBUG_OUTPUT                                   15
#define FFX_FSR2_RESOURCE_IDENTIFIER_LANCZOS_LUT                                    16
#define FFX_FSR2_RESOURCE_IDENTIFIER_SPD_ATOMIC_COUNT                               17
#define FFX_FSR2_RESOURCE_IDENTIFIER_UPSCALED_OUTPUT                                18
#define FFX_FSR2_RESOURCE_IDENTIFIER_RCAS_INPUT                                     19
#define FFX_FSR2_RESOURCE_IDENTIFIER_LOCK_STATUS_1                                  20
#define FFX_FSR2_RESOURCE_IDENTIFIER_LOCK_STATUS_2                                  21
#define FFX_FSR2_RESOURCE_IDENTIFIER_INTERNAL_UPSCALED_COLOR_1                      22
#define FFX_FSR2_RESOURCE_IDENTIFIER_INTERNAL_UPSCALED_COLOR_2                      23
#define FFX_FSR2_RESOURCE_IDENTIFIER_INTERNAL_DEFAULT_REACTIVITY                    24
#define FFX_FSR2_RESOURCE_IDENTIFIER_INTERNAL_DEFAULT_TRANSPARENCY_AND_COMPOSITION  25
#define FFX_FSR2_RESOURCE_IDENTITIER_UPSAMPLE_MAXIMUM_BIAS_LUT                      26
#define FFX_FSR2_RESOURCE_IDENTIFIER_DILATED_REACTIVE_MASKS                         27
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE                                  28 // same as FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_0
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_0                         28
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_1                         29
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_2                         30
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_3                         31
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_4                         32
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_5                         33
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_6                         34
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_7                         35
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_8                         36
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_9                         37
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_10                        38
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_11                        39
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_12                        40
#define FFX_FSR2_RESOURCE_IDENTIFIER_INTERNAL_DEFAULT_EXPOSURE                      41
#define FFX_FSR2_RESOURCE_IDENTIFIER_EXPOSURE                                       42

// Shading change detection mip level setting, value must be in the range [FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_0, FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_12]
#define FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_SHADING_CHANGE            FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_4
#define FFX_FSR2_SHADING_CHANGE_MIP_LEVEL                                           (FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE_MIPMAP_SHADING_CHANGE - FFX_FSR2_RESOURCE_IDENTIFIER_AUTO_EXPOSURE)

#define FFX_FSR2_RESOURCE_IDENTIFIER_COUNT                                          43

#define FFX_FSR2_CONSTANTBUFFER_IDENTIFIER_FSR2                                      0
#define FFX_FSR2_CONSTANTBUFFER_IDENTIFIER_SPD                                       1
#define FFX_FSR2_CONSTANTBUFFER_IDENTIFIER_RCAS                                      2

#define FFX_FSR2_AUTOREACTIVEFLAGS_APPLY_TONEMAP                                    1
#define FFX_FSR2_AUTOREACTIVEFLAGS_APPLY_INVERSETONEMAP                             2
#define FFX_FSR2_AUTOREACTIVEFLAGS_APPLY_THRESHOLD                                  4
#define FFX_FSR2_AUTOREACTIVEFLAGS_USE_COMPONENTS_MAX                               8

#endif // #if defined(FFX_CPU) || defined(FFX_GPU)

#endif //!defined( FFX_FSR2_RESOURCES_H )
