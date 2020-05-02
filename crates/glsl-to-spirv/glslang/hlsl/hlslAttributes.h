//
// Copyright (C) 2016 LunarG, Inc.
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
//    Neither the name of Google, Inc., nor the names of its
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

#ifndef HLSLATTRIBUTES_H_
#define HLSLATTRIBUTES_H_

#include <unordered_map>
#include <functional>
#include "hlslScanContext.h"
#include "../glslang/Include/Common.h"

namespace glslang {
    enum TAttributeType {
        EatNone,
        EatAllow_uav_condition,
        EatBranch,
        EatCall,
        EatDomain,
        EatEarlyDepthStencil,
        EatFastOpt,
        EatFlatten,
        EatForceCase,
        EatInstance,
        EatMaxTessFactor,
        EatNumThreads,
        EatMaxVertexCount,
        EatOutputControlPoints,
        EatOutputTopology,
        EatPartitioning,
        EatPatchConstantFunc,
        EatPatchSize,
        EatUnroll,
        EatLoop,
    };
}

namespace std {
    // Allow use of TAttributeType enum in hash_map without calling code having to cast.
    template <> struct hash<glslang::TAttributeType> {
        std::size_t operator()(glslang::TAttributeType attr) const {
            return std::hash<int>()(int(attr));
        }
    };
} // end namespace std

namespace glslang {
    class TIntermAggregate;

    class TAttributeMap {
    public:
        // Search for and potentially add the attribute into the map.  Return the
        // attribute type enum for it, if found, else EatNone.
        TAttributeType setAttribute(const TString* name, TIntermAggregate* value);

        // Const lookup: search for (but do not modify) the attribute in the map.
        const TIntermAggregate* operator[](TAttributeType) const;

        // True if entry exists in map (even if value is nullptr)
        bool contains(TAttributeType) const;

    protected:
        // Find an attribute enum given its name.
        static TAttributeType attributeFromName(const TString&);

        std::unordered_map<TAttributeType, TIntermAggregate*> attributes;
    };

    class TFunctionDeclarator {
    public:
        TFunctionDeclarator() : function(nullptr), body(nullptr) { }
        TSourceLoc loc;
        TFunction* function;
        TAttributeMap attributes;
        TVector<HlslToken>* body;
    };

} // end namespace glslang


#endif // HLSLATTRIBUTES_H_
