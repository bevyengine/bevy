//
// Copyright (C) 2016-2017 LunarG, Inc.
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
#include "../Include/InfoSink.h"
#include "iomapper.h"
#include "LiveTraverser.h"
#include "localintermediate.h"

#include "gl_types.h"

#include <unordered_set>
#include <unordered_map>

//
// Map IO bindings.
//
// High-level algorithm for one stage:
//
// 1. Traverse all code (live+dead) to find the explicitly provided bindings.
//
// 2. Traverse (just) the live code to determine which non-provided bindings
//    require auto-numbering.  We do not auto-number dead ones.
//
// 3. Traverse all the code to apply the bindings:
//    a. explicitly given bindings are offset according to their type
//    b. implicit live bindings are auto-numbered into the holes, using
//       any open binding slot.
//    c. implicit dead bindings are left un-bound.
//


namespace glslang {

struct TVarEntryInfo
{
    int               id;
    TIntermSymbol*    symbol;
    bool              live;
    int               newBinding;
    int               newSet;
    int               newLocation;
    int               newComponent;
    int               newIndex;

    struct TOrderById
    {
      inline bool operator()(const TVarEntryInfo& l, const TVarEntryInfo& r)
      {
        return l.id < r.id;
      }
    };

    struct TOrderByPriority
    {
        // ordering:
        // 1) has both binding and set
        // 2) has binding but no set
        // 3) has no binding but set
        // 4) has no binding and no set
        inline bool operator()(const TVarEntryInfo& l, const TVarEntryInfo& r)
        {
            const TQualifier& lq = l.symbol->getQualifier();
            const TQualifier& rq = r.symbol->getQualifier();

            // simple rules:
            // has binding gives 2 points
            // has set gives 1 point
            // who has the most points is more important.
            int lPoints = (lq.hasBinding() ? 2 : 0) + (lq.hasSet() ? 1 : 0);
            int rPoints = (rq.hasBinding() ? 2 : 0) + (rq.hasSet() ? 1 : 0);

            if (lPoints == rPoints)
              return l.id < r.id;
            return lPoints > rPoints;
        }
    };
};



typedef std::vector<TVarEntryInfo> TVarLiveMap;

class TVarGatherTraverser : public TLiveTraverser
{
public:
    TVarGatherTraverser(const TIntermediate& i, bool traverseDeadCode, TVarLiveMap& inList, TVarLiveMap& outList, TVarLiveMap& uniformList)
      : TLiveTraverser(i, traverseDeadCode, true, true, false)
      , inputList(inList)
      , outputList(outList)
      , uniformList(uniformList)
    {
    }


    virtual void visitSymbol(TIntermSymbol* base)
    {
        TVarLiveMap* target = nullptr;
        if (base->getQualifier().storage == EvqVaryingIn)
            target = &inputList;
        else if (base->getQualifier().storage == EvqVaryingOut)
            target = &outputList;
        else if (base->getQualifier().isUniformOrBuffer())
            target = &uniformList;

        if (target) {
            TVarEntryInfo ent = { base->getId(), base, !traverseAll };
            TVarLiveMap::iterator at = std::lower_bound(target->begin(), target->end(), ent, TVarEntryInfo::TOrderById());
            if (at != target->end() && at->id == ent.id)
              at->live = at->live || !traverseAll; // update live state
            else
              target->insert(at, ent);
        }
    }

private:
    TVarLiveMap&    inputList;
    TVarLiveMap&    outputList;
    TVarLiveMap&    uniformList;
};

class TVarSetTraverser : public TLiveTraverser
{
public:
    TVarSetTraverser(const TIntermediate& i, const TVarLiveMap& inList, const TVarLiveMap& outList, const TVarLiveMap& uniformList)
      : TLiveTraverser(i, true, true, true, false)
      , inputList(inList)
      , outputList(outList)
      , uniformList(uniformList)
    {
    }


    virtual void visitSymbol(TIntermSymbol* base)
    {
        const TVarLiveMap* source;
        if (base->getQualifier().storage == EvqVaryingIn)
            source = &inputList;
        else if (base->getQualifier().storage == EvqVaryingOut)
            source = &outputList;
        else if (base->getQualifier().isUniformOrBuffer())
            source = &uniformList;
        else
            return;

        TVarEntryInfo ent = { base->getId() };
        TVarLiveMap::const_iterator at = std::lower_bound(source->begin(), source->end(), ent, TVarEntryInfo::TOrderById());
        if (at == source->end())
            return;

        if (at->id != ent.id)
            return;

        if (at->newBinding != -1)
            base->getWritableType().getQualifier().layoutBinding = at->newBinding;
        if (at->newSet != -1)
            base->getWritableType().getQualifier().layoutSet = at->newSet;
        if (at->newLocation != -1)
            base->getWritableType().getQualifier().layoutLocation = at->newLocation;
        if (at->newComponent != -1)
            base->getWritableType().getQualifier().layoutComponent = at->newComponent;
        if (at->newIndex != -1)
            base->getWritableType().getQualifier().layoutIndex = at->newIndex;
    }

  private:
    const TVarLiveMap&    inputList;
    const TVarLiveMap&    outputList;
    const TVarLiveMap&    uniformList;
};

struct TNotifyUniformAdaptor
{
    EShLanguage stage;
    TIoMapResolver& resolver;
    inline TNotifyUniformAdaptor(EShLanguage s, TIoMapResolver& r)
      : stage(s)
      , resolver(r)
    {
    }
    inline void operator()(TVarEntryInfo& ent)
    {
        resolver.notifyBinding(stage, ent.symbol->getName().c_str(), ent.symbol->getType(), ent.live);
    }
private:
    TNotifyUniformAdaptor& operator=(TNotifyUniformAdaptor&);
};

struct TNotifyInOutAdaptor
{
    EShLanguage stage;
    TIoMapResolver& resolver;
    inline TNotifyInOutAdaptor(EShLanguage s, TIoMapResolver& r)
      : stage(s)
      , resolver(r)
    {
    }
    inline void operator()(TVarEntryInfo& ent)
    {
        resolver.notifyInOut(stage, ent.symbol->getName().c_str(), ent.symbol->getType(), ent.live);
    }
private:
    TNotifyInOutAdaptor& operator=(TNotifyInOutAdaptor&);
};

struct TResolverUniformAdaptor
{
    TResolverUniformAdaptor(EShLanguage s, TIoMapResolver& r, TInfoSink& i, bool& e, TIntermediate& interm)
      : stage(s)
      , resolver(r)
      , infoSink(i)
      , error(e)
      , intermediate(interm)
    {
    }

    inline void operator()(TVarEntryInfo& ent)
    {
        ent.newLocation = -1;
        ent.newComponent = -1;
        ent.newBinding = -1;
        ent.newSet = -1;
        ent.newIndex = -1;
        const bool isValid = resolver.validateBinding(stage, ent.symbol->getName().c_str(), ent.symbol->getType(), ent.live);
        if (isValid) {
            ent.newBinding = resolver.resolveBinding(stage, ent.symbol->getName().c_str(), ent.symbol->getType(), ent.live);
            ent.newSet = resolver.resolveSet(stage, ent.symbol->getName().c_str(), ent.symbol->getType(), ent.live);

            if (ent.newBinding != -1) {
                if (ent.newBinding >= int(TQualifier::layoutBindingEnd)) {
                    TString err = "mapped binding out of range: " + ent.symbol->getName();

                    infoSink.info.message(EPrefixInternalError, err.c_str());
                    error = true;
                }
            }
            if (ent.newSet != -1) {
                if (ent.newSet >= int(TQualifier::layoutSetEnd)) {
                    TString err = "mapped set out of range: " + ent.symbol->getName();

                    infoSink.info.message(EPrefixInternalError, err.c_str());
                    error = true;
                }
            }
        } else {
            TString errorMsg = "Invalid binding: " + ent.symbol->getName();
            infoSink.info.message(EPrefixInternalError, errorMsg.c_str());
            error = true;
        }
    }

    EShLanguage     stage;
    TIoMapResolver& resolver;
    TInfoSink&      infoSink;
    bool&           error;
    TIntermediate&  intermediate;

private:
    TResolverUniformAdaptor& operator=(TResolverUniformAdaptor&);
};

struct TResolverInOutAdaptor
{
    TResolverInOutAdaptor(EShLanguage s, TIoMapResolver& r, TInfoSink& i, bool& e, TIntermediate& interm)
      : stage(s)
      , resolver(r)
      , infoSink(i)
      , error(e)
      , intermediate(interm)
    {
    }

    inline void operator()(TVarEntryInfo& ent)
    {
        ent.newLocation = -1;
        ent.newComponent = -1;
        ent.newBinding = -1;
        ent.newSet = -1;
        ent.newIndex = -1;
        const bool isValid = resolver.validateInOut(stage,
                                                    ent.symbol->getName().c_str(),
                                                    ent.symbol->getType(),
                                                    ent.live);
        if (isValid) {
            ent.newLocation = resolver.resolveInOutLocation(stage,
                                                            ent.symbol->getName().c_str(),
                                                            ent.symbol->getType(),
                                                            ent.live);
            ent.newComponent = resolver.resolveInOutComponent(stage,
                                                              ent.symbol->getName().c_str(),
                                                              ent.symbol->getType(),
                                                              ent.live);
            ent.newIndex = resolver.resolveInOutIndex(stage,
                                                      ent.symbol->getName().c_str(),
                                                      ent.symbol->getType(),
                                                      ent.live);
        } else {
            TString errorMsg = "Invalid shader In/Out variable semantic: ";
            errorMsg += ent.symbol->getType().getQualifier().semanticName;
            infoSink.info.message(EPrefixInternalError, errorMsg.c_str());
            error = true;
        }
    }

    EShLanguage     stage;
    TIoMapResolver& resolver;
    TInfoSink&      infoSink;
    bool&           error;
    TIntermediate&  intermediate;

private:
    TResolverInOutAdaptor& operator=(TResolverInOutAdaptor&);
};

// Base class for shared TIoMapResolver services, used by several derivations.
struct TDefaultIoResolverBase : public glslang::TIoMapResolver
{
    int baseSamplerBinding;
    int baseTextureBinding;
    int baseImageBinding;
    int baseUboBinding;
    int baseSsboBinding;
    int baseUavBinding;
    std::vector<std::string> baseResourceSetBinding;
    bool doAutoBindingMapping;
    bool doAutoLocationMapping;
    typedef std::vector<int> TSlotSet;
    typedef std::unordered_map<int, TSlotSet> TSlotSetMap;
    TSlotSetMap slots;

    TSlotSet::iterator findSlot(int set, int slot)
    {
        return std::lower_bound(slots[set].begin(), slots[set].end(), slot);
    }

    bool checkEmpty(int set, int slot)
    {
        TSlotSet::iterator at = findSlot(set, slot);
        return !(at != slots[set].end() && *at == slot);
    }

    int reserveSlot(int set, int slot)
    {
        TSlotSet::iterator at = findSlot(set, slot);

        // tolerate aliasing, by not double-recording aliases
        // (policy about appropriateness of the alias is higher up)
        if (at == slots[set].end() || *at != slot)
            slots[set].insert(at, slot);

        return slot;
    }

    int getFreeSlot(int set, int base)
    {
        TSlotSet::iterator at = findSlot(set, base);
        if (at == slots[set].end())
            return reserveSlot(set, base);

        // look in locksteps, if they not match, then there is a free slot
        for (; at != slots[set].end(); ++at, ++base)
            if (*at != base)
                break;
        return reserveSlot(set, base);
    }

    virtual bool validateBinding(EShLanguage /*stage*/, const char* /*name*/, const glslang::TType& type, bool /*is_live*/) override = 0;

    virtual int resolveBinding(EShLanguage /*stage*/, const char* /*name*/, const glslang::TType& type, bool is_live) override = 0;

    int resolveSet(EShLanguage /*stage*/, const char* /*name*/, const glslang::TType& type, bool /*is_live*/) override
    {
        if (type.getQualifier().hasSet())
            return type.getQualifier().layoutSet;
        return 0;
    }

    bool validateInOut(EShLanguage /*stage*/, const char* /*name*/, const TType& /*type*/, bool /*is_live*/) override
    {
        return true;
    }
    int resolveInOutLocation(EShLanguage /*stage*/, const char* /*name*/, const TType& type, bool /*is_live*/) override
    {
        if (!doAutoLocationMapping || type.getQualifier().hasLocation())
            return -1;

        // Placeholder.
        // TODO: It would be nice to flesh this out using 
        // intermediate->computeTypeLocationSize(type), or functions that call it like
        // intermediate->addUsedLocation()
        // These in turn would want the intermediate, which is not available here, but
        // is available in many places, and a lot of copying from it could be saved if
        // it were just available.
        return 0;
    }
    int resolveInOutComponent(EShLanguage /*stage*/, const char* /*name*/, const TType& /*type*/, bool /*is_live*/) override
    {
        return -1;
    }
    int resolveInOutIndex(EShLanguage /*stage*/, const char* /*name*/, const TType& /*type*/, bool /*is_live*/) override
    {
        return -1;
    }

    void notifyBinding(EShLanguage, const char* /*name*/, const TType&, bool /*is_live*/) override {}
    void notifyInOut(EShLanguage, const char* /*name*/, const TType&, bool /*is_live*/) override {}
    void endNotifications() override {}

protected:
    static int getLayoutSet(const glslang::TType& type) {
        if (type.getQualifier().hasSet())
            return type.getQualifier().layoutSet;
        else
            return 0;
    }

    static bool isSamplerType(const glslang::TType& type) {
        return type.getBasicType() == glslang::EbtSampler && type.getSampler().isPureSampler();
    }

    static bool isTextureType(const glslang::TType& type) {
        return type.getBasicType() == glslang::EbtSampler && type.getSampler().isTexture();
    }

    static bool isUboType(const glslang::TType& type) {
        return type.getQualifier().storage == EvqUniform;
    }
};

/*
 * Basic implementation of glslang::TIoMapResolver that replaces the
 * previous offset behavior.
 * It does the same, uses the offsets for the corresponding uniform
 * types. Also respects the EOptionAutoMapBindings flag and binds
 * them if needed.
 */
/*
 * Default resolver
 */
struct TDefaultIoResolver : public TDefaultIoResolverBase
{
    bool validateBinding(EShLanguage /*stage*/, const char* /*name*/, const glslang::TType& /*type*/, bool /*is_live*/) override
    {
        return true;
    }

    int resolveBinding(EShLanguage /*stage*/, const char* /*name*/, const glslang::TType& type, bool is_live) override
    {
        const int set = getLayoutSet(type);

        if (type.getQualifier().hasBinding()) {
            if (isImageType(type))
                return reserveSlot(set, baseImageBinding + type.getQualifier().layoutBinding);

            if (isTextureType(type))
                return reserveSlot(set, baseTextureBinding + type.getQualifier().layoutBinding);

            if (isSsboType(type))
                return reserveSlot(set, baseSsboBinding + type.getQualifier().layoutBinding);

            if (isSamplerType(type))
                return reserveSlot(set, baseSamplerBinding + type.getQualifier().layoutBinding);

            if (isUboType(type))
                return reserveSlot(set, baseUboBinding + type.getQualifier().layoutBinding);
        } else if (is_live && doAutoBindingMapping) {
            // find free slot, the caller did make sure it passes all vars with binding
            // first and now all are passed that do not have a binding and needs one

            if (isImageType(type))
                return getFreeSlot(set, baseImageBinding);

            if (isTextureType(type))
                return getFreeSlot(set, baseTextureBinding);

            if (isSsboType(type))
                return getFreeSlot(set, baseSsboBinding);

            if (isSamplerType(type))
                return getFreeSlot(set, baseSamplerBinding);

            if (isUboType(type))
                return getFreeSlot(set, baseUboBinding);
        }

        return -1;
    }

protected:
    static bool isImageType(const glslang::TType& type) {
        return type.getBasicType() == glslang::EbtSampler && type.getSampler().isImage();
    }

    static bool isSsboType(const glslang::TType& type) {
        return type.getQualifier().storage == EvqBuffer;
    }
};

/********************************************************************************
The following IO resolver maps types in HLSL register space, as follows:

t – for shader resource views (SRV)
   TEXTURE1D
   TEXTURE1DARRAY
   TEXTURE2D
   TEXTURE2DARRAY
   TEXTURE3D
   TEXTURECUBE
   TEXTURECUBEARRAY
   TEXTURE2DMS
   TEXTURE2DMSARRAY
   STRUCTUREDBUFFER
   BYTEADDRESSBUFFER
   BUFFER
   TBUFFER
    
s – for samplers
   SAMPLER
   SAMPLER1D
   SAMPLER2D
   SAMPLER3D
   SAMPLERCUBE
   SAMPLERSTATE
   SAMPLERCOMPARISONSTATE

u – for unordered access views (UAV)
   RWBYTEADDRESSBUFFER
   RWSTRUCTUREDBUFFER
   APPENDSTRUCTUREDBUFFER
   CONSUMESTRUCTUREDBUFFER
   RWBUFFER
   RWTEXTURE1D
   RWTEXTURE1DARRAY
   RWTEXTURE2D
   RWTEXTURE2DARRAY
   RWTEXTURE3D

b – for constant buffer views (CBV)
   CBUFFER
   CONSTANTBUFFER
 ********************************************************************************/
struct TDefaultHlslIoResolver : public TDefaultIoResolverBase
{
    bool validateBinding(EShLanguage /*stage*/, const char* /*name*/, const glslang::TType& /*type*/, bool /*is_live*/) override
    {
        return true;
    }

    int resolveBinding(EShLanguage /*stage*/, const char* /*name*/, const glslang::TType& type, bool is_live) override
    {
        const int set = getLayoutSet(type);

        if (type.getQualifier().hasBinding()) {
            if (isUavType(type))
                return reserveSlot(set, baseUavBinding + type.getQualifier().layoutBinding);

            if (isSrvType(type))
                return reserveSlot(set, baseTextureBinding + type.getQualifier().layoutBinding);

            if (isSamplerType(type))
                return reserveSlot(set, baseSamplerBinding + type.getQualifier().layoutBinding);

            if (isUboType(type))
                return reserveSlot(set, baseUboBinding + type.getQualifier().layoutBinding);
        } else if (is_live && doAutoBindingMapping) {
            // find free slot, the caller did make sure it passes all vars with binding
            // first and now all are passed that do not have a binding and needs one

            if (isUavType(type))
                return getFreeSlot(set, baseUavBinding);

            if (isSrvType(type))
                return getFreeSlot(set, baseTextureBinding);

            if (isSamplerType(type))
                return getFreeSlot(set, baseSamplerBinding);

            if (isUboType(type))
                return getFreeSlot(set, baseUboBinding);
        }

        return -1;
    }

protected:
    // Return true if this is a SRV (shader resource view) type:
    static bool isSrvType(const glslang::TType& type) {
        return isTextureType(type) || type.getQualifier().storage == EvqBuffer;
    }

    // Return true if this is a UAV (unordered access view) type:
    static bool isUavType(const glslang::TType& type) {
        if (type.getQualifier().readonly)
            return false;

        return (type.getBasicType() == glslang::EbtSampler && type.getSampler().isImage()) ||
            (type.getQualifier().storage == EvqBuffer);
    }
};


// Map I/O variables to provided offsets, and make bindings for
// unbound but live variables.
//
// Returns false if the input is too malformed to do this.
bool TIoMapper::addStage(EShLanguage stage, TIntermediate &intermediate, TInfoSink &infoSink, TIoMapResolver *resolver)
{
    // Trivial return if there is nothing to do.
    if (intermediate.getShiftSamplerBinding() == 0 &&
        intermediate.getShiftTextureBinding() == 0 &&
        intermediate.getShiftImageBinding() == 0 &&
        intermediate.getShiftUboBinding() == 0 &&
        intermediate.getShiftSsboBinding() == 0 &&
        intermediate.getShiftUavBinding() == 0 &&
        intermediate.getResourceSetBinding().empty() &&
        intermediate.getAutoMapBindings() == false &&
        intermediate.getAutoMapLocations() == false &&
        resolver == nullptr)
        return true;

    if (intermediate.getNumEntryPoints() != 1 || intermediate.isRecursive())
        return false;

    TIntermNode* root = intermediate.getTreeRoot();
    if (root == nullptr)
        return false;

    // if no resolver is provided, use the default resolver with the given shifts and auto map settings
    TDefaultIoResolver defaultResolver;
    TDefaultHlslIoResolver defaultHlslResolver;

    if (resolver == nullptr) {
        TDefaultIoResolverBase* resolverBase;

        // TODO: use a passed in IO mapper for this
        if (intermediate.usingHlslIoMapping())
            resolverBase = &defaultHlslResolver;
        else
            resolverBase = &defaultResolver;

        resolverBase->baseSamplerBinding = intermediate.getShiftSamplerBinding();
        resolverBase->baseTextureBinding = intermediate.getShiftTextureBinding();
        resolverBase->baseImageBinding = intermediate.getShiftImageBinding();
        resolverBase->baseUboBinding = intermediate.getShiftUboBinding();
        resolverBase->baseSsboBinding = intermediate.getShiftSsboBinding();
        resolverBase->baseUavBinding = intermediate.getShiftUavBinding();
        resolverBase->baseResourceSetBinding = intermediate.getResourceSetBinding();
        resolverBase->doAutoBindingMapping = intermediate.getAutoMapBindings();
        resolverBase->doAutoLocationMapping = intermediate.getAutoMapLocations();

        resolver = resolverBase;
    }

    TVarLiveMap inVarMap, outVarMap, uniformVarMap;
    TVarGatherTraverser iter_binding_all(intermediate, true, inVarMap, outVarMap, uniformVarMap);
    TVarGatherTraverser iter_binding_live(intermediate, false, inVarMap, outVarMap, uniformVarMap);

    root->traverse(&iter_binding_all);
    iter_binding_live.pushFunction(intermediate.getEntryPointMangledName().c_str());

    while (!iter_binding_live.functions.empty()) {
        TIntermNode* function = iter_binding_live.functions.back();
        iter_binding_live.functions.pop_back();
        function->traverse(&iter_binding_live);
    }

    // sort entries by priority. see TVarEntryInfo::TOrderByPriority for info.
    std::sort(uniformVarMap.begin(), uniformVarMap.end(), TVarEntryInfo::TOrderByPriority());

    bool hadError = false;
    TNotifyInOutAdaptor inOutNotify(stage, *resolver);
    TNotifyUniformAdaptor uniformNotify(stage, *resolver);
    TResolverUniformAdaptor uniformResolve(stage, *resolver, infoSink, hadError, intermediate);
    TResolverInOutAdaptor inOutResolve(stage, *resolver, infoSink, hadError, intermediate);
    std::for_each(inVarMap.begin(), inVarMap.end(), inOutNotify);
    std::for_each(outVarMap.begin(), outVarMap.end(), inOutNotify);
    std::for_each(uniformVarMap.begin(), uniformVarMap.end(), uniformNotify);
    resolver->endNotifications();
    std::for_each(inVarMap.begin(), inVarMap.end(), inOutResolve);
    std::for_each(outVarMap.begin(), outVarMap.end(), inOutResolve);
    std::for_each(uniformVarMap.begin(), uniformVarMap.end(), uniformResolve);

    if (!hadError) {
        // sort by id again, so we can use lower bound to find entries
        std::sort(uniformVarMap.begin(), uniformVarMap.end(), TVarEntryInfo::TOrderById());
        TVarSetTraverser iter_iomap(intermediate, inVarMap, outVarMap, uniformVarMap);
        root->traverse(&iter_iomap);
    }

    return !hadError;
}

} // end namespace glslang
