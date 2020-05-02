//
// Copyright (C) 2013-2016 LunarG, Inc.
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

#include "../Include/Common.h"
#include "reflection.h"
#include "LiveTraverser.h"
#include "localintermediate.h"

#include "gl_types.h"

//
// Grow the reflection database through a friend traverser class of TReflection and a
// collection of functions to do a liveness traversal that note what uniforms are used
// in semantically non-dead code.
//
// Can be used multiple times, once per stage, to grow a program reflection.
//
// High-level algorithm for one stage:
//
// 1. Put the entry point on the list of live functions.
//
// 2. Traverse any live function, while skipping if-tests with a compile-time constant
//    condition of false, and while adding any encountered function calls to the live
//    function list.
//
//    Repeat until the live function list is empty.
//
// 3. Add any encountered uniform variables and blocks to the reflection database.
//
// Can be attempted with a failed link, but will return false if recursion had been detected, or
// there wasn't exactly one entry point.
//

namespace glslang {

//
// The traverser: mostly pass through, except
//  - processing binary nodes to see if they are dereferences of an aggregates to track
//  - processing symbol nodes to see if they are non-aggregate objects to track
//
// This ignores semantically dead code by using TLiveTraverser.
//
// This is in the glslang namespace directly so it can be a friend of TReflection.
//

class TReflectionTraverser : public TLiveTraverser {
public:
    TReflectionTraverser(const TIntermediate& i, TReflection& r) :
         TLiveTraverser(i), reflection(r) { }

    virtual bool visitBinary(TVisit, TIntermBinary* node);
    virtual void visitSymbol(TIntermSymbol* base);

    // Add a simple reference to a uniform variable to the uniform database, no dereference involved.
    // However, no dereference doesn't mean simple... it could be a complex aggregate.
    void addUniform(const TIntermSymbol& base)
    {
        if (processedDerefs.find(&base) == processedDerefs.end()) {
            processedDerefs.insert(&base);

            // Use a degenerate (empty) set of dereferences to immediately put as at the end of
            // the dereference change expected by blowUpActiveAggregate.
            TList<TIntermBinary*> derefs;
            blowUpActiveAggregate(base.getType(), base.getName(), derefs, derefs.end(), -1, -1, 0);
        }
    }

    void addAttribute(const TIntermSymbol& base)
    {
        if (processedDerefs.find(&base) == processedDerefs.end()) {
            processedDerefs.insert(&base);

            const TString &name = base.getName();
            const TType &type = base.getType();

            TReflection::TNameToIndex::const_iterator it = reflection.nameToIndex.find(name);
            if (it == reflection.nameToIndex.end()) {
                reflection.nameToIndex[name] = (int)reflection.indexToAttribute.size();
                reflection.indexToAttribute.push_back(TObjectReflection(name, type, 0, mapToGlType(type), 0, 0));
            }
        }
    }

    // Lookup or calculate the offset of a block member, using the recursively
    // defined block offset rules.
    int getOffset(const TType& type, int index)
    {
        const TTypeList& memberList = *type.getStruct();

        // Don't calculate offset if one is present, it could be user supplied
        // and different than what would be calculated.  That is, this is faster,
        // but not just an optimization.
        if (memberList[index].type->getQualifier().hasOffset())
            return memberList[index].type->getQualifier().layoutOffset;

        int memberSize;
        int dummyStride;
        int offset = 0;
        for (int m = 0; m <= index; ++m) {
            // modify just the children's view of matrix layout, if there is one for this member
            TLayoutMatrix subMatrixLayout = memberList[m].type->getQualifier().layoutMatrix;
            int memberAlignment = intermediate.getBaseAlignment(*memberList[m].type, memberSize, dummyStride, type.getQualifier().layoutPacking == ElpStd140,
                                                                subMatrixLayout != ElmNone ? subMatrixLayout == ElmRowMajor : type.getQualifier().layoutMatrix == ElmRowMajor);
            RoundToPow2(offset, memberAlignment);
            if (m < index)
                offset += memberSize;
        }

        return offset;
    }

    // Calculate the block data size.
    // Block arrayness is not taken into account, each element is backed by a separate buffer.
    int getBlockSize(const TType& blockType)
    {
        const TTypeList& memberList = *blockType.getStruct();
        int lastIndex = (int)memberList.size() - 1;
        int lastOffset = getOffset(blockType, lastIndex);

        int lastMemberSize;
        int dummyStride;
        intermediate.getBaseAlignment(*memberList[lastIndex].type, lastMemberSize, dummyStride, blockType.getQualifier().layoutPacking == ElpStd140,
                                      blockType.getQualifier().layoutMatrix == ElmRowMajor);

        return lastOffset + lastMemberSize;
    }

    // Traverse the provided deref chain, including the base, and
    // - build a full reflection-granularity name, array size, etc. entry out of it, if it goes down to that granularity
    // - recursively expand any variable array index in the middle of that traversal
    // - recursively expand what's left at the end if the deref chain did not reach down to reflection granularity
    //
    // arraySize tracks, just for the final dereference in the chain, if there was a specific known size.
    // A value of 0 for arraySize will mean to use the full array's size.
    void blowUpActiveAggregate(const TType& baseType, const TString& baseName, const TList<TIntermBinary*>& derefs,
                               TList<TIntermBinary*>::const_iterator deref, int offset, int blockIndex, int arraySize)
    {
        // process the part of the derefence chain that was explicit in the shader
        TString name = baseName;
        const TType* terminalType = &baseType;
        for (; deref != derefs.end(); ++deref) {
            TIntermBinary* visitNode = *deref;
            terminalType = &visitNode->getType();
            int index;
            switch (visitNode->getOp()) {
            case EOpIndexIndirect:
                // Visit all the indices of this array, and for each one add on the remaining dereferencing
                for (int i = 0; i < visitNode->getLeft()->getType().getOuterArraySize(); ++i) {
                    TString newBaseName = name;
                    if (baseType.getBasicType() != EbtBlock)
                        newBaseName.append(TString("[") + String(i) + "]");
                    TList<TIntermBinary*>::const_iterator nextDeref = deref;
                    ++nextDeref;
                    TType derefType(*terminalType, 0);
                    blowUpActiveAggregate(derefType, newBaseName, derefs, nextDeref, offset, blockIndex, arraySize);
                }

                // it was all completed in the recursive calls above
                return;
            case EOpIndexDirect:
                index = visitNode->getRight()->getAsConstantUnion()->getConstArray()[0].getIConst();
                if (baseType.getBasicType() != EbtBlock)
                    name.append(TString("[") + String(index) + "]");
                break;
            case EOpIndexDirectStruct:
                index = visitNode->getRight()->getAsConstantUnion()->getConstArray()[0].getIConst();
                if (offset >= 0)
                    offset += getOffset(visitNode->getLeft()->getType(), index);
                if (name.size() > 0)
                    name.append(".");
                name.append((*visitNode->getLeft()->getType().getStruct())[index].type->getFieldName());
                break;
            default:
                break;
            }
        }

        // if the terminalType is still too coarse a granularity, this is still an aggregate to expand, expand it...
        if (! isReflectionGranularity(*terminalType)) {
            if (terminalType->isArray()) {
                // Visit all the indices of this array, and for each one,
                // fully explode the remaining aggregate to dereference
                for (int i = 0; i < terminalType->getOuterArraySize(); ++i) {
                    TString newBaseName = name;
                    newBaseName.append(TString("[") + String(i) + "]");
                    TType derefType(*terminalType, 0);
                    blowUpActiveAggregate(derefType, newBaseName, derefs, derefs.end(), offset, blockIndex, 0);
                }
            } else {
                // Visit all members of this aggregate, and for each one,
                // fully explode the remaining aggregate to dereference
                const TTypeList& typeList = *terminalType->getStruct();
                for (int i = 0; i < (int)typeList.size(); ++i) {
                    TString newBaseName = name;
                    newBaseName.append(TString(".") + typeList[i].type->getFieldName());
                    TType derefType(*terminalType, i);
                    blowUpActiveAggregate(derefType, newBaseName, derefs, derefs.end(), offset, blockIndex, 0);
                }
            }

            // it was all completed in the recursive calls above
            return;
        }

        // Finally, add a full string to the reflection database, and update the array size if necessary.
        // If the derefenced entity to record is an array, compute the size and update the maximum size.

        // there might not be a final array dereference, it could have been copied as an array object
        if (arraySize == 0)
            arraySize = mapToGlArraySize(*terminalType);

        TReflection::TNameToIndex::const_iterator it = reflection.nameToIndex.find(name);
        if (it == reflection.nameToIndex.end()) {
            reflection.nameToIndex[name] = (int)reflection.indexToUniform.size();
            reflection.indexToUniform.push_back(TObjectReflection(name, *terminalType, offset, mapToGlType(*terminalType),
                                                                  arraySize, blockIndex));
        } else if (arraySize > 1) {
            int& reflectedArraySize = reflection.indexToUniform[it->second].size;
            reflectedArraySize = std::max(arraySize, reflectedArraySize);
        }
    }

    // Add a uniform dereference where blocks/struct/arrays are involved in the access.
    // Handles the situation where the left node is at the correct or too coarse a
    // granularity for reflection.  (That is, further dereferences up the tree will be
    // skipped.) Earlier dereferences, down the tree, will be handled
    // at the same time, and logged to prevent reprocessing as the tree is traversed.
    //
    // Note: Other things like the following must be caught elsewhere:
    //  - a simple non-array, non-struct variable (no dereference even conceivable)
    //  - an aggregrate consumed en masse, without a dereference
    //
    // So, this code is for cases like
    //   - a struct/block dereferencing a member (whether the member is array or not)
    //   - an array of struct
    //   - structs/arrays containing the above
    //
    void addDereferencedUniform(TIntermBinary* topNode)
    {
        // See if too fine-grained to process (wait to get further down the tree)
        const TType& leftType = topNode->getLeft()->getType();
        if ((leftType.isVector() || leftType.isMatrix()) && ! leftType.isArray())
            return;

        // We have an array or structure or block dereference, see if it's a uniform
        // based dereference (if not, skip it).
        TIntermSymbol* base = findBase(topNode);
        if (! base || ! base->getQualifier().isUniformOrBuffer())
            return;

        // See if we've already processed this (e.g., in the middle of something
        // we did earlier), and if so skip it
        if (processedDerefs.find(topNode) != processedDerefs.end())
            return;

        // Process this uniform dereference

        int offset = -1;
        int blockIndex = -1;
        bool anonymous = false;

        // See if we need to record the block itself
        bool block = base->getBasicType() == EbtBlock;
        if (block) {
            offset = 0;
            anonymous = IsAnonymous(base->getName());

            const TString& blockName = base->getType().getTypeName();

            if (base->getType().isArray()) {
                TType derefType(base->getType(), 0);

                assert(! anonymous);
                for (int e = 0; e < base->getType().getCumulativeArraySize(); ++e)
                    blockIndex = addBlockName(blockName + "[" + String(e) + "]", derefType,
                                              getBlockSize(base->getType()));
            } else
                blockIndex = addBlockName(blockName, base->getType(), getBlockSize(base->getType()));
        }

        // Process the dereference chain, backward, accumulating the pieces for later forward traversal.
        // If the topNode is a reflection-granularity-array dereference, don't include that last dereference.
        TList<TIntermBinary*> derefs;
        for (TIntermBinary* visitNode = topNode; visitNode; visitNode = visitNode->getLeft()->getAsBinaryNode()) {
            if (isReflectionGranularity(visitNode->getLeft()->getType()))
                continue;

            derefs.push_front(visitNode);
            processedDerefs.insert(visitNode);
        }
        processedDerefs.insert(base);

        // See if we have a specific array size to stick to while enumerating the explosion of the aggregate
        int arraySize = 0;
        if (isReflectionGranularity(topNode->getLeft()->getType()) && topNode->getLeft()->isArray()) {
            if (topNode->getOp() == EOpIndexDirect)
                arraySize = topNode->getRight()->getAsConstantUnion()->getConstArray()[0].getIConst() + 1;
        }

        // Put the dereference chain together, forward
        TString baseName;
        if (! anonymous) {
            if (block)
                baseName = base->getType().getTypeName();
            else
                baseName = base->getName();
        }
        blowUpActiveAggregate(base->getType(), baseName, derefs, derefs.begin(), offset, blockIndex, arraySize);
    }

    int addBlockName(const TString& name, const TType& type, int size)
    {
        int blockIndex;
        TReflection::TNameToIndex::const_iterator it = reflection.nameToIndex.find(name);
        if (reflection.nameToIndex.find(name) == reflection.nameToIndex.end()) {
            blockIndex = (int)reflection.indexToUniformBlock.size();
            reflection.nameToIndex[name] = blockIndex;
            reflection.indexToUniformBlock.push_back(TObjectReflection(name, type, -1, -1, size, -1));
        } else
            blockIndex = it->second;

        return blockIndex;
    }

    // Are we at a level in a dereference chain at which individual active uniform queries are made?
    bool isReflectionGranularity(const TType& type)
    {
        return type.getBasicType() != EbtBlock && type.getBasicType() != EbtStruct;
    }

    // For a binary operation indexing into an aggregate, chase down the base of the aggregate.
    // Return 0 if the topology does not fit this situation.
    TIntermSymbol* findBase(const TIntermBinary* node)
    {
        TIntermSymbol *base = node->getLeft()->getAsSymbolNode();
        if (base)
            return base;
        TIntermBinary* left = node->getLeft()->getAsBinaryNode();
        if (! left)
            return nullptr;

        return findBase(left);
    }

    //
    // Translate a glslang sampler type into the GL API #define number.
    //
    int mapSamplerToGlType(TSampler sampler)
    {
        if (! sampler.image) {
            // a sampler...
            switch (sampler.type) {
            case EbtFloat:
                switch ((int)sampler.dim) {
                case Esd1D:
                    switch ((int)sampler.shadow) {
                    case false: return sampler.arrayed ? GL_SAMPLER_1D_ARRAY : GL_SAMPLER_1D;
                    case true:  return sampler.arrayed ? GL_SAMPLER_1D_ARRAY_SHADOW : GL_SAMPLER_1D_SHADOW;
                    }
                case Esd2D:
                    switch ((int)sampler.ms) {
                    case false:
                        switch ((int)sampler.shadow) {
                        case false: return sampler.arrayed ? GL_SAMPLER_2D_ARRAY : GL_SAMPLER_2D;
                        case true:  return sampler.arrayed ? GL_SAMPLER_2D_ARRAY_SHADOW : GL_SAMPLER_2D_SHADOW;
                        }
                    case true:      return sampler.arrayed ? GL_SAMPLER_2D_MULTISAMPLE_ARRAY : GL_SAMPLER_2D_MULTISAMPLE;
                    }
                case Esd3D:
                    return GL_SAMPLER_3D;
                case EsdCube:
                    switch ((int)sampler.shadow) {
                    case false: return sampler.arrayed ? GL_SAMPLER_CUBE_MAP_ARRAY : GL_SAMPLER_CUBE;
                    case true:  return sampler.arrayed ? GL_SAMPLER_CUBE_MAP_ARRAY_SHADOW : GL_SAMPLER_CUBE_SHADOW;
                    }
                case EsdRect:
                    return sampler.shadow ? GL_SAMPLER_2D_RECT_SHADOW : GL_SAMPLER_2D_RECT;
                case EsdBuffer:
                    return GL_SAMPLER_BUFFER;
                }
            case EbtInt:
                switch ((int)sampler.dim) {
                case Esd1D:
                    return sampler.arrayed ? GL_INT_SAMPLER_1D_ARRAY : GL_INT_SAMPLER_1D;
                case Esd2D:
                    switch ((int)sampler.ms) {
                    case false:  return sampler.arrayed ? GL_INT_SAMPLER_2D_ARRAY : GL_INT_SAMPLER_2D;
                    case true:   return sampler.arrayed ? GL_INT_SAMPLER_2D_MULTISAMPLE_ARRAY : GL_INT_SAMPLER_2D_MULTISAMPLE;
                    }
                case Esd3D:
                    return GL_INT_SAMPLER_3D;
                case EsdCube:
                    return sampler.arrayed ? GL_INT_SAMPLER_CUBE_MAP_ARRAY : GL_INT_SAMPLER_CUBE;
                case EsdRect:
                    return GL_INT_SAMPLER_2D_RECT;
                case EsdBuffer:
                    return GL_INT_SAMPLER_BUFFER;
                }
            case EbtUint:
                switch ((int)sampler.dim) {
                case Esd1D:
                    return sampler.arrayed ? GL_UNSIGNED_INT_SAMPLER_1D_ARRAY : GL_UNSIGNED_INT_SAMPLER_1D;
                case Esd2D:
                    switch ((int)sampler.ms) {
                    case false:  return sampler.arrayed ? GL_UNSIGNED_INT_SAMPLER_2D_ARRAY : GL_UNSIGNED_INT_SAMPLER_2D;
                    case true:   return sampler.arrayed ? GL_UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE_ARRAY : GL_UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE;
                    }
                case Esd3D:
                    return GL_UNSIGNED_INT_SAMPLER_3D;
                case EsdCube:
                    return sampler.arrayed ? GL_UNSIGNED_INT_SAMPLER_CUBE_MAP_ARRAY : GL_UNSIGNED_INT_SAMPLER_CUBE;
                case EsdRect:
                    return GL_UNSIGNED_INT_SAMPLER_2D_RECT;
                case EsdBuffer:
                    return GL_UNSIGNED_INT_SAMPLER_BUFFER;
                }
            default:
                return 0;
            }
        } else {
            // an image...
            switch (sampler.type) {
            case EbtFloat:
                switch ((int)sampler.dim) {
                case Esd1D:
                    return sampler.arrayed ? GL_IMAGE_1D_ARRAY : GL_IMAGE_1D;
                case Esd2D:
                    switch ((int)sampler.ms) {
                    case false:     return sampler.arrayed ? GL_IMAGE_2D_ARRAY : GL_IMAGE_2D;
                    case true:      return sampler.arrayed ? GL_IMAGE_2D_MULTISAMPLE_ARRAY : GL_IMAGE_2D_MULTISAMPLE;
                    }
                case Esd3D:
                    return GL_IMAGE_3D;
                case EsdCube:
                    return sampler.arrayed ? GL_IMAGE_CUBE_MAP_ARRAY : GL_IMAGE_CUBE;
                case EsdRect:
                    return GL_IMAGE_2D_RECT;
                case EsdBuffer:
                    return GL_IMAGE_BUFFER;
                }
            case EbtInt:
                switch ((int)sampler.dim) {
                case Esd1D:
                    return sampler.arrayed ? GL_INT_IMAGE_1D_ARRAY : GL_INT_IMAGE_1D;
                case Esd2D:
                    switch ((int)sampler.ms) {
                    case false:  return sampler.arrayed ? GL_INT_IMAGE_2D_ARRAY : GL_INT_IMAGE_2D;
                    case true:   return sampler.arrayed ? GL_INT_IMAGE_2D_MULTISAMPLE_ARRAY : GL_INT_IMAGE_2D_MULTISAMPLE;
                    }
                case Esd3D:
                    return GL_INT_IMAGE_3D;
                case EsdCube:
                    return sampler.arrayed ? GL_INT_IMAGE_CUBE_MAP_ARRAY : GL_INT_IMAGE_CUBE;
                case EsdRect:
                    return GL_INT_IMAGE_2D_RECT;
                case EsdBuffer:
                    return GL_INT_IMAGE_BUFFER;
                }
            case EbtUint:
                switch ((int)sampler.dim) {
                case Esd1D:
                    return sampler.arrayed ? GL_UNSIGNED_INT_IMAGE_1D_ARRAY : GL_UNSIGNED_INT_IMAGE_1D;
                case Esd2D:
                    switch ((int)sampler.ms) {
                    case false:  return sampler.arrayed ? GL_UNSIGNED_INT_IMAGE_2D_ARRAY : GL_UNSIGNED_INT_IMAGE_2D;
                    case true:   return sampler.arrayed ? GL_UNSIGNED_INT_IMAGE_2D_MULTISAMPLE_ARRAY : GL_UNSIGNED_INT_IMAGE_2D_MULTISAMPLE;
                    }
                case Esd3D:
                    return GL_UNSIGNED_INT_IMAGE_3D;
                case EsdCube:
                    return sampler.arrayed ? GL_UNSIGNED_INT_IMAGE_CUBE_MAP_ARRAY : GL_UNSIGNED_INT_IMAGE_CUBE;
                case EsdRect:
                    return GL_UNSIGNED_INT_IMAGE_2D_RECT;
                case EsdBuffer:
                    return GL_UNSIGNED_INT_IMAGE_BUFFER;
                }
            default:
                return 0;
            }
        }
    }

    //
    // Translate a glslang type into the GL API #define number.
    // Ignores arrayness.
    //
    int mapToGlType(const TType& type)
    {
        switch (type.getBasicType()) {
        case EbtSampler:
            return mapSamplerToGlType(type.getSampler());
        case EbtStruct:
        case EbtBlock:
        case EbtVoid:
            return 0;
        default:
            break;
        }

        if (type.isVector()) {
            int offset = type.getVectorSize() - 2;
            switch (type.getBasicType()) {
            case EbtFloat:      return GL_FLOAT_VEC2                  + offset;
            case EbtDouble:     return GL_DOUBLE_VEC2                 + offset;
#ifdef AMD_EXTENSIONS
            case EbtFloat16:    return GL_FLOAT16_VEC2_NV             + offset;
#endif
            case EbtInt:        return GL_INT_VEC2                    + offset;
            case EbtUint:       return GL_UNSIGNED_INT_VEC2           + offset;
            case EbtInt64:      return GL_INT64_ARB                   + offset;
            case EbtUint64:     return GL_UNSIGNED_INT64_ARB          + offset;
            case EbtBool:       return GL_BOOL_VEC2                   + offset;
            case EbtAtomicUint: return GL_UNSIGNED_INT_ATOMIC_COUNTER + offset;
            default:            return 0;
            }
        }
        if (type.isMatrix()) {
            switch (type.getBasicType()) {
            case EbtFloat:
                switch (type.getMatrixCols()) {
                case 2:
                    switch (type.getMatrixRows()) {
                    case 2:    return GL_FLOAT_MAT2;
                    case 3:    return GL_FLOAT_MAT2x3;
                    case 4:    return GL_FLOAT_MAT2x4;
                    default:   return 0;
                    }
                case 3:
                    switch (type.getMatrixRows()) {
                    case 2:    return GL_FLOAT_MAT3x2;
                    case 3:    return GL_FLOAT_MAT3;
                    case 4:    return GL_FLOAT_MAT3x4;
                    default:   return 0;
                    }
                case 4:
                    switch (type.getMatrixRows()) {
                    case 2:    return GL_FLOAT_MAT4x2;
                    case 3:    return GL_FLOAT_MAT4x3;
                    case 4:    return GL_FLOAT_MAT4;
                    default:   return 0;
                    }
                }
            case EbtDouble:
                switch (type.getMatrixCols()) {
                case 2:
                    switch (type.getMatrixRows()) {
                    case 2:    return GL_DOUBLE_MAT2;
                    case 3:    return GL_DOUBLE_MAT2x3;
                    case 4:    return GL_DOUBLE_MAT2x4;
                    default:   return 0;
                    }
                case 3:
                    switch (type.getMatrixRows()) {
                    case 2:    return GL_DOUBLE_MAT3x2;
                    case 3:    return GL_DOUBLE_MAT3;
                    case 4:    return GL_DOUBLE_MAT3x4;
                    default:   return 0;
                    }
                case 4:
                    switch (type.getMatrixRows()) {
                    case 2:    return GL_DOUBLE_MAT4x2;
                    case 3:    return GL_DOUBLE_MAT4x3;
                    case 4:    return GL_DOUBLE_MAT4;
                    default:   return 0;
                    }
                }
#ifdef AMD_EXTENSIONS
            case EbtFloat16:
                switch (type.getMatrixCols()) {
                case 2:
                    switch (type.getMatrixRows()) {
                    case 2:    return GL_FLOAT16_MAT2_AMD;
                    case 3:    return GL_FLOAT16_MAT2x3_AMD;
                    case 4:    return GL_FLOAT16_MAT2x4_AMD;
                    default:   return 0;
                    }
                case 3:
                    switch (type.getMatrixRows()) {
                    case 2:    return GL_FLOAT16_MAT3x2_AMD;
                    case 3:    return GL_FLOAT16_MAT3_AMD;
                    case 4:    return GL_FLOAT16_MAT3x4_AMD;
                    default:   return 0;
                    }
                case 4:
                    switch (type.getMatrixRows()) {
                    case 2:    return GL_FLOAT16_MAT4x2_AMD;
                    case 3:    return GL_FLOAT16_MAT4x3_AMD;
                    case 4:    return GL_FLOAT16_MAT4_AMD;
                    default:   return 0;
                    }
                }
#endif
            default:
                return 0;
            }
        }
        if (type.getVectorSize() == 1) {
            switch (type.getBasicType()) {
            case EbtFloat:      return GL_FLOAT;
            case EbtDouble:     return GL_DOUBLE;
#ifdef AMD_EXTENSIONS
            case EbtFloat16:    return GL_FLOAT16_NV;
#endif
            case EbtInt:        return GL_INT;
            case EbtUint:       return GL_UNSIGNED_INT;
            case EbtInt64:      return GL_INT64_ARB;
            case EbtUint64:     return GL_UNSIGNED_INT64_ARB;
            case EbtBool:       return GL_BOOL;
            case EbtAtomicUint: return GL_UNSIGNED_INT_ATOMIC_COUNTER;
            default:            return 0;
            }
        }

        return 0;
    }

    int mapToGlArraySize(const TType& type)
    {
        return type.isArray() ? type.getOuterArraySize() : 1;
    }

    TReflection& reflection;
    std::set<const TIntermNode*> processedDerefs;

protected:
    TReflectionTraverser(TReflectionTraverser&);
    TReflectionTraverser& operator=(TReflectionTraverser&);
};

//
// Implement the traversal functions of interest.
//

// To catch dereferenced aggregates that must be reflected.
// This catches them at the highest level possible in the tree.
bool TReflectionTraverser::visitBinary(TVisit /* visit */, TIntermBinary* node)
{
    switch (node->getOp()) {
    case EOpIndexDirect:
    case EOpIndexIndirect:
    case EOpIndexDirectStruct:
        addDereferencedUniform(node);
        break;
    default:
        break;
    }

    // still need to visit everything below, which could contain sub-expressions
    // containing different uniforms
    return true;
}

// To reflect non-dereferenced objects.
void TReflectionTraverser::visitSymbol(TIntermSymbol* base)
{
    if (base->getQualifier().storage == EvqUniform)
        addUniform(*base);

    if (intermediate.getStage() == EShLangVertex && base->getQualifier().isPipeInput())
        addAttribute(*base);
}

//
// Implement TReflection methods.
//

// Track any required attribute reflection, such as compute shader numthreads.
//
void TReflection::buildAttributeReflection(EShLanguage stage, const TIntermediate& intermediate)
{
    if (stage == EShLangCompute) {
        // Remember thread dimensions
        for (int dim=0; dim<3; ++dim)
            localSize[dim] = intermediate.getLocalSize(dim);
    }
}

// build counter block index associations for buffers
void TReflection::buildCounterIndices()
{
    // search for ones that have counters
    for (int i = 0; i < int(indexToUniformBlock.size()); ++i) {
        const TString counterName(indexToUniformBlock[i].name + "@count");
        const int index = getIndex(counterName);

        if (index >= 0)
            indexToUniformBlock[i].counterIndex = index;
    }
}

// Merge live symbols from 'intermediate' into the existing reflection database.
//
// Returns false if the input is too malformed to do this.
bool TReflection::addStage(EShLanguage stage, const TIntermediate& intermediate)
{
    if (intermediate.getTreeRoot() == nullptr ||
        intermediate.getNumEntryPoints() != 1 ||
        intermediate.isRecursive())
        return false;

    buildAttributeReflection(stage, intermediate);

    TReflectionTraverser it(intermediate, *this);

    // put the entry point on the list of functions to process
    it.pushFunction(intermediate.getEntryPointMangledName().c_str());

    // process all the functions
    while (! it.functions.empty()) {
        TIntermNode* function = it.functions.back();
        it.functions.pop_back();
        function->traverse(&it);
    }

    buildCounterIndices();

    return true;
}

void TReflection::dump()
{
    printf("Uniform reflection:\n");
    for (size_t i = 0; i < indexToUniform.size(); ++i)
        indexToUniform[i].dump();
    printf("\n");

    printf("Uniform block reflection:\n");
    for (size_t i = 0; i < indexToUniformBlock.size(); ++i)
        indexToUniformBlock[i].dump();
    printf("\n");

    printf("Vertex attribute reflection:\n");
    for (size_t i = 0; i < indexToAttribute.size(); ++i)
        indexToAttribute[i].dump();
    printf("\n");

    if (getLocalSize(0) > 1) {
        static const char* axis[] = { "X", "Y", "Z" };

        for (int dim=0; dim<3; ++dim)
            if (getLocalSize(dim) > 1)
                printf("Local size %s: %d\n", axis[dim], getLocalSize(dim));

        printf("\n");
    }

    // printf("Live names\n");
    // for (TNameToIndex::const_iterator it = nameToIndex.begin(); it != nameToIndex.end(); ++it)
    //    printf("%s: %d\n", it->first.c_str(), it->second);
    // printf("\n");
}

} // end namespace glslang
