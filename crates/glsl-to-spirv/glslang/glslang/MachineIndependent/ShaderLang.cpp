//
// Copyright (C) 2002-2005  3Dlabs Inc. Ltd.
// Copyright (C) 2013-2016 LunarG, Inc.
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
// Implement the top-level of interface to the compiler/linker,
// as defined in ShaderLang.h
// This is the platform independent interface between an OGL driver
// and the shading language compiler/linker.
//
#include <cstring>
#include <iostream>
#include <sstream>
#include <memory>
#include "SymbolTable.h"
#include "ParseHelper.h"
#include "Scan.h"
#include "ScanContext.h"

#ifdef ENABLE_HLSL
#include "../../hlsl/hlslParseHelper.h"
#include "../../hlsl/hlslParseables.h"
#include "../../hlsl/hlslScanContext.h"
#endif

#include "../Include/ShHandle.h"
#include "../../OGLCompilersDLL/InitializeDll.h"

#include "preprocessor/PpContext.h"

#define SH_EXPORTING
#include "../Public/ShaderLang.h"
#include "reflection.h"
#include "iomapper.h"
#include "Initialize.h"

namespace { // anonymous namespace for file-local functions and symbols

using namespace glslang;

// Create a language specific version of parseables.
TBuiltInParseables* CreateBuiltInParseables(TInfoSink& infoSink, EShSource source)
{
    switch (source) {
    case EShSourceGlsl: return new TBuiltIns();              // GLSL builtIns
#ifdef ENABLE_HLSL
    case EShSourceHlsl: return new TBuiltInParseablesHlsl(); // HLSL intrinsics
#endif

    default:
        infoSink.info.message(EPrefixInternalError, "Unable to determine source language");
        return nullptr;
    }
}

// Create a language specific version of a parse context.
TParseContextBase* CreateParseContext(TSymbolTable& symbolTable, TIntermediate& intermediate,
                                      int version, EProfile profile, EShSource source,
                                      EShLanguage language, TInfoSink& infoSink,
                                      SpvVersion spvVersion, bool forwardCompatible, EShMessages messages,
                                      bool parsingBuiltIns, const std::string sourceEntryPointName = "")
{
#ifndef ENABLE_HLSL
    (void)sourceEntryPointName; // Unused argument.
#endif

    switch (source) {
    case EShSourceGlsl:
        intermediate.setEntryPointName("main");
        return new TParseContext(symbolTable, intermediate, parsingBuiltIns, version, profile, spvVersion,
                                 language, infoSink, forwardCompatible, messages);

#ifdef ENABLE_HLSL
    case EShSourceHlsl:
        return new HlslParseContext(symbolTable, intermediate, parsingBuiltIns, version, profile, spvVersion,
                                    language, infoSink, sourceEntryPointName.c_str(), forwardCompatible, messages);
#endif
    default:
        infoSink.info.message(EPrefixInternalError, "Unable to determine source language");
        return nullptr;
    }
}

// Local mapping functions for making arrays of symbol tables....

const int VersionCount = 15;  // index range in MapVersionToIndex

int MapVersionToIndex(int version)
{
    int index = 0;

    switch (version) {
    case 100: index =  0; break;
    case 110: index =  1; break;
    case 120: index =  2; break;
    case 130: index =  3; break;
    case 140: index =  4; break;
    case 150: index =  5; break;
    case 300: index =  6; break;
    case 330: index =  7; break;
    case 400: index =  8; break;
    case 410: index =  9; break;
    case 420: index = 10; break;
    case 430: index = 11; break;
    case 440: index = 12; break;
    case 310: index = 13; break;
    case 450: index = 14; break;
    default:              break;
    }

    assert(index < VersionCount);

    return index;
}

const int SpvVersionCount = 3;  // index range in MapSpvVersionToIndex

int MapSpvVersionToIndex(const SpvVersion& spvVersion)
{
    int index = 0;

    if (spvVersion.openGl > 0)
        index = 1;
    else if (spvVersion.vulkan > 0)
        index = 2;

    assert(index < SpvVersionCount);

    return index;
}

const int ProfileCount = 4;   // index range in MapProfileToIndex

int MapProfileToIndex(EProfile profile)
{
    int index = 0;

    switch (profile) {
    case ENoProfile:            index = 0; break;
    case ECoreProfile:          index = 1; break;
    case ECompatibilityProfile: index = 2; break;
    case EEsProfile:            index = 3; break;
    default:                               break;
    }

    assert(index < ProfileCount);

    return index;
}

const int SourceCount = 2;

int MapSourceToIndex(EShSource source)
{
    int index = 0;

    switch (source) {
    case EShSourceGlsl: index = 0; break;
    case EShSourceHlsl: index = 1; break;
    default:                       break;
    }

    assert(index < SourceCount);

    return index;
}

// only one of these needed for non-ES; ES needs 2 for different precision defaults of built-ins
enum EPrecisionClass {
    EPcGeneral,
    EPcFragment,
    EPcCount
};

// A process-global symbol table per version per profile for built-ins common
// to multiple stages (languages), and a process-global symbol table per version
// per profile per stage for built-ins unique to each stage.  They will be sparsely
// populated, so they will only be generated as needed.
//
// Each has a different set of built-ins, and we want to preserve that from
// compile to compile.
//
TSymbolTable* CommonSymbolTable[VersionCount][SpvVersionCount][ProfileCount][SourceCount][EPcCount] = {};
TSymbolTable* SharedSymbolTables[VersionCount][SpvVersionCount][ProfileCount][SourceCount][EShLangCount] = {};

TPoolAllocator* PerProcessGPA = 0;

//
// Parse and add to the given symbol table the content of the given shader string.
//
bool InitializeSymbolTable(const TString& builtIns, int version, EProfile profile, const SpvVersion& spvVersion, EShLanguage language,
                           EShSource source, TInfoSink& infoSink, TSymbolTable& symbolTable)
{
    TIntermediate intermediate(language, version, profile);

    intermediate.setSource(source);

    std::unique_ptr<TParseContextBase> parseContext(CreateParseContext(symbolTable, intermediate, version, profile, source,
                                                                       language, infoSink, spvVersion, true, EShMsgDefault,
                                                                       true));

    TShader::ForbidIncluder includer;
    TPpContext ppContext(*parseContext, "", includer);
    TScanContext scanContext(*parseContext);
    parseContext->setScanContext(&scanContext);
    parseContext->setPpContext(&ppContext);

    //
    // Push the symbol table to give it an initial scope.  This
    // push should not have a corresponding pop, so that built-ins
    // are preserved, and the test for an empty table fails.
    //

    symbolTable.push();

    const char* builtInShaders[2];
    size_t builtInLengths[2];
    builtInShaders[0] = builtIns.c_str();
    builtInLengths[0] = builtIns.size();

    if (builtInLengths[0] == 0)
        return true;

    TInputScanner input(1, builtInShaders, builtInLengths);
    if (! parseContext->parseShaderStrings(ppContext, input) != 0) {
        infoSink.info.message(EPrefixInternalError, "Unable to parse built-ins");
        printf("Unable to parse built-ins\n%s\n", infoSink.info.c_str());
        printf("%s\n", builtInShaders[0]);

        return false;
    }

    return true;
}

int CommonIndex(EProfile profile, EShLanguage language)
{
    return (profile == EEsProfile && language == EShLangFragment) ? EPcFragment : EPcGeneral;
}

//
// To initialize per-stage shared tables, with the common table already complete.
//
void InitializeStageSymbolTable(TBuiltInParseables& builtInParseables, int version, EProfile profile, const SpvVersion& spvVersion,
                                EShLanguage language, EShSource source, TInfoSink& infoSink, TSymbolTable** commonTable,
                                TSymbolTable** symbolTables)
{
    (*symbolTables[language]).adoptLevels(*commonTable[CommonIndex(profile, language)]);
    InitializeSymbolTable(builtInParseables.getStageString(language), version, profile, spvVersion, language, source,
                          infoSink, *symbolTables[language]);
    builtInParseables.identifyBuiltIns(version, profile, spvVersion, language, *symbolTables[language]);
    if (profile == EEsProfile && version >= 300)
        (*symbolTables[language]).setNoBuiltInRedeclarations();
    if (version == 110)
        (*symbolTables[language]).setSeparateNameSpaces();
}

//
// Initialize the full set of shareable symbol tables;
// The common (cross-stage) and those shareable per-stage.
//
bool InitializeSymbolTables(TInfoSink& infoSink, TSymbolTable** commonTable,  TSymbolTable** symbolTables, int version, EProfile profile, const SpvVersion& spvVersion, EShSource source)
{
    std::unique_ptr<TBuiltInParseables> builtInParseables(CreateBuiltInParseables(infoSink, source));

    if (builtInParseables == nullptr)
        return false;

    builtInParseables->initialize(version, profile, spvVersion);

    // do the common tables
    InitializeSymbolTable(builtInParseables->getCommonString(), version, profile, spvVersion, EShLangVertex, source,
                          infoSink, *commonTable[EPcGeneral]);
    if (profile == EEsProfile)
        InitializeSymbolTable(builtInParseables->getCommonString(), version, profile, spvVersion, EShLangFragment, source,
                              infoSink, *commonTable[EPcFragment]);

    // do the per-stage tables

    // always have vertex and fragment
    InitializeStageSymbolTable(*builtInParseables, version, profile, spvVersion, EShLangVertex, source,
                               infoSink, commonTable, symbolTables);
    InitializeStageSymbolTable(*builtInParseables, version, profile, spvVersion, EShLangFragment, source,
                               infoSink, commonTable, symbolTables);

    // check for tessellation
    if ((profile != EEsProfile && version >= 150) ||
        (profile == EEsProfile && version >= 310)) {
        InitializeStageSymbolTable(*builtInParseables, version, profile, spvVersion, EShLangTessControl, source,
                                   infoSink, commonTable, symbolTables);
        InitializeStageSymbolTable(*builtInParseables, version, profile, spvVersion, EShLangTessEvaluation, source,
                                   infoSink, commonTable, symbolTables);
    }

    // check for geometry
    if ((profile != EEsProfile && version >= 150) ||
        (profile == EEsProfile && version >= 310))
        InitializeStageSymbolTable(*builtInParseables, version, profile, spvVersion, EShLangGeometry, source,
                                   infoSink, commonTable, symbolTables);

    // check for compute
    if ((profile != EEsProfile && version >= 420) ||
        (profile == EEsProfile && version >= 310))
        InitializeStageSymbolTable(*builtInParseables, version, profile, spvVersion, EShLangCompute, source,
                                   infoSink, commonTable, symbolTables);

    return true;
}

bool AddContextSpecificSymbols(const TBuiltInResource* resources, TInfoSink& infoSink, TSymbolTable& symbolTable, int version,
                               EProfile profile, const SpvVersion& spvVersion, EShLanguage language, EShSource source)
{
    std::unique_ptr<TBuiltInParseables> builtInParseables(CreateBuiltInParseables(infoSink, source));

    if (builtInParseables == nullptr)
        return false;

    builtInParseables->initialize(*resources, version, profile, spvVersion, language);
    InitializeSymbolTable(builtInParseables->getCommonString(), version, profile, spvVersion, language, source, infoSink, symbolTable);
    builtInParseables->identifyBuiltIns(version, profile, spvVersion, language, symbolTable, *resources);

    return true;
}

//
// To do this on the fly, we want to leave the current state of our thread's
// pool allocator intact, so:
//  - Switch to a new pool for parsing the built-ins
//  - Do the parsing, which builds the symbol table, using the new pool
//  - Switch to the process-global pool to save a copy the resulting symbol table
//  - Free up the new pool used to parse the built-ins
//  - Switch back to the original thread's pool
//
// This only gets done the first time any thread needs a particular symbol table
// (lazy evaluation).
//
void SetupBuiltinSymbolTable(int version, EProfile profile, const SpvVersion& spvVersion, EShSource source)
{
    TInfoSink infoSink;

    // Make sure only one thread tries to do this at a time
    glslang::GetGlobalLock();

    // See if it's already been done for this version/profile combination
    int versionIndex = MapVersionToIndex(version);
    int spvVersionIndex = MapSpvVersionToIndex(spvVersion);
    int profileIndex = MapProfileToIndex(profile);
    int sourceIndex = MapSourceToIndex(source);
    if (CommonSymbolTable[versionIndex][spvVersionIndex][profileIndex][sourceIndex][EPcGeneral]) {
        glslang::ReleaseGlobalLock();

        return;
    }

    // Switch to a new pool
    TPoolAllocator& previousAllocator = GetThreadPoolAllocator();
    TPoolAllocator* builtInPoolAllocator = new TPoolAllocator();
    SetThreadPoolAllocator(*builtInPoolAllocator);

    // Dynamically allocate the local symbol tables so we can control when they are deallocated WRT when the pool is popped.
    TSymbolTable* commonTable[EPcCount];
    TSymbolTable* stageTables[EShLangCount];
    for (int precClass = 0; precClass < EPcCount; ++precClass)
        commonTable[precClass] = new TSymbolTable;
    for (int stage = 0; stage < EShLangCount; ++stage)
        stageTables[stage] = new TSymbolTable;

    // Generate the local symbol tables using the new pool
    InitializeSymbolTables(infoSink, commonTable, stageTables, version, profile, spvVersion, source);

    // Switch to the process-global pool
    SetThreadPoolAllocator(*PerProcessGPA);

    // Copy the local symbol tables from the new pool to the global tables using the process-global pool
    for (int precClass = 0; precClass < EPcCount; ++precClass) {
        if (! commonTable[precClass]->isEmpty()) {
            CommonSymbolTable[versionIndex][spvVersionIndex][profileIndex][sourceIndex][precClass] = new TSymbolTable;
            CommonSymbolTable[versionIndex][spvVersionIndex][profileIndex][sourceIndex][precClass]->copyTable(*commonTable[precClass]);
            CommonSymbolTable[versionIndex][spvVersionIndex][profileIndex][sourceIndex][precClass]->readOnly();
        }
    }
    for (int stage = 0; stage < EShLangCount; ++stage) {
        if (! stageTables[stage]->isEmpty()) {
            SharedSymbolTables[versionIndex][spvVersionIndex][profileIndex][sourceIndex][stage] = new TSymbolTable;
            SharedSymbolTables[versionIndex][spvVersionIndex][profileIndex][sourceIndex][stage]->adoptLevels(*CommonSymbolTable
                              [versionIndex][spvVersionIndex][profileIndex][sourceIndex][CommonIndex(profile, (EShLanguage)stage)]);
            SharedSymbolTables[versionIndex][spvVersionIndex][profileIndex][sourceIndex][stage]->copyTable(*stageTables[stage]);
            SharedSymbolTables[versionIndex][spvVersionIndex][profileIndex][sourceIndex][stage]->readOnly();
        }
    }

    // Clean up the local tables before deleting the pool they used.
    for (int precClass = 0; precClass < EPcCount; ++precClass)
        delete commonTable[precClass];
    for (int stage = 0; stage < EShLangCount; ++stage)
        delete stageTables[stage];

    delete builtInPoolAllocator;
    SetThreadPoolAllocator(previousAllocator);

    glslang::ReleaseGlobalLock();
}

// Return true if the shader was correctly specified for version/profile/stage.
bool DeduceVersionProfile(TInfoSink& infoSink, EShLanguage stage, bool versionNotFirst, int defaultVersion,
                          EShSource source, int& version, EProfile& profile, const SpvVersion& spvVersion)
{
    const int FirstProfileVersion = 150;
    bool correct = true;

    if (source == EShSourceHlsl) {
        version = 500;          // shader model; currently a characteristic of glslang, not the input
        profile = ECoreProfile; // allow doubles in prototype parsing
        return correct;
    }

    // Get a good version...
    if (version == 0) {
        version = defaultVersion;
        // infoSink.info.message(EPrefixWarning, "#version: statement missing; use #version on first line of shader");
    }

    // Get a good profile...
    if (profile == ENoProfile) {
        if (version == 300 || version == 310) {
            correct = false;
            infoSink.info.message(EPrefixError, "#version: versions 300 and 310 require specifying the 'es' profile");
            profile = EEsProfile;
        } else if (version == 100)
            profile = EEsProfile;
        else if (version >= FirstProfileVersion)
            profile = ECoreProfile;
        else
            profile = ENoProfile;
    } else {
        // a profile was provided...
        if (version < 150) {
            correct = false;
            infoSink.info.message(EPrefixError, "#version: versions before 150 do not allow a profile token");
            if (version == 100)
                profile = EEsProfile;
            else
                profile = ENoProfile;
        } else if (version == 300 || version == 310) {
            if (profile != EEsProfile) {
                correct = false;
                infoSink.info.message(EPrefixError, "#version: versions 300 and 310 support only the es profile");
            }
            profile = EEsProfile;
        } else {
            if (profile == EEsProfile) {
                correct = false;
                infoSink.info.message(EPrefixError, "#version: only version 300 and 310 support the es profile");
                if (version >= FirstProfileVersion)
                    profile = ECoreProfile;
                else
                    profile = ENoProfile;
            }
            // else: typical desktop case... e.g., "#version 410 core"
        }
    }

    // Correct for stage type...
    switch (stage) {
    case EShLangGeometry:
        if ((profile == EEsProfile && version < 310) ||
            (profile != EEsProfile && version < 150)) {
            correct = false;
            infoSink.info.message(EPrefixError, "#version: geometry shaders require es profile with version 310 or non-es profile with version 150 or above");
            version = (profile == EEsProfile) ? 310 : 150;
            if (profile == EEsProfile || profile == ENoProfile)
                profile = ECoreProfile;
        }
        break;
    case EShLangTessControl:
    case EShLangTessEvaluation:
        if ((profile == EEsProfile && version < 310) ||
            (profile != EEsProfile && version < 150)) {
            correct = false;
            infoSink.info.message(EPrefixError, "#version: tessellation shaders require es profile with version 310 or non-es profile with version 150 or above");
            version = (profile == EEsProfile) ? 310 : 400; // 150 supports the extension, correction is to 400 which does not
            if (profile == EEsProfile || profile == ENoProfile)
                profile = ECoreProfile;
        }
        break;
    case EShLangCompute:
        if ((profile == EEsProfile && version < 310) ||
            (profile != EEsProfile && version < 420)) {
            correct = false;
            infoSink.info.message(EPrefixError, "#version: compute shaders require es profile with version 310 or above, or non-es profile with version 420 or above");
            version = profile == EEsProfile ? 310 : 420;
        }
        break;
    default:
        break;
    }

    if (profile == EEsProfile && version >= 300 && versionNotFirst) {
        correct = false;
        infoSink.info.message(EPrefixError, "#version: statement must appear first in es-profile shader; before comments or newlines");
    }

    // Check for SPIR-V compatibility
    if (spvVersion.spv != 0) {
        switch (profile) {
        case  EEsProfile:
            if (spvVersion.vulkan >= 100 && version < 310) {
                correct = false;
                infoSink.info.message(EPrefixError, "#version: ES shaders for Vulkan SPIR-V require version 310 or higher");
                version = 310;
            }
            if (spvVersion.openGl >= 100) {
                correct = false;
                infoSink.info.message(EPrefixError, "#version: ES shaders for OpenGL SPIR-V are not supported");
                version = 310;
            }
            break;
        case ECompatibilityProfile:
            infoSink.info.message(EPrefixError, "#version: compilation for SPIR-V does not support the compatibility profile");
            break;
        default:
            if (spvVersion.vulkan >= 100 && version < 140) {
                correct = false;
                infoSink.info.message(EPrefixError, "#version: Desktop shaders for Vulkan SPIR-V require version 140 or higher");
                version = 140;
            }
            if (spvVersion.openGl >= 100 && version < 330) {
                correct = false;
                infoSink.info.message(EPrefixError, "#version: Desktop shaders for OpenGL SPIR-V require version 330 or higher");
                version = 330;
            }
            break;
        }
    }

    return correct;
}

// This is the common setup and cleanup code for PreprocessDeferred and
// CompileDeferred.
// It takes any callable with a signature of
//  bool (TParseContextBase& parseContext, TPpContext& ppContext,
//                  TInputScanner& input, bool versionWillBeError,
//                  TSymbolTable& , TIntermediate& ,
//                  EShOptimizationLevel , EShMessages );
// Which returns false if a failure was detected and true otherwise.
//
template<typename ProcessingContext>
bool ProcessDeferred(
    TCompiler* compiler,
    const char* const shaderStrings[],
    const int numStrings,
    const int* inputLengths,
    const char* const stringNames[],
    const char* customPreamble,
    const EShOptimizationLevel optLevel,
    const TBuiltInResource* resources,
    int defaultVersion,  // use 100 for ES environment, 110 for desktop; this is the GLSL version, not SPIR-V or Vulkan
    EProfile defaultProfile,
    // set version/profile to defaultVersion/defaultProfile regardless of the #version
    // directive in the source code
    bool forceDefaultVersionAndProfile,
    bool forwardCompatible,     // give errors for use of deprecated features
    EShMessages messages,       // warnings/errors/AST; things to print out
    TIntermediate& intermediate, // returned tree, etc.
    ProcessingContext& processingContext,
    bool requireNonempty,
    TShader::Includer& includer,
    const std::string sourceEntryPointName = ""
    )
{
    if (! InitThread())
        return false;

    // This must be undone (.pop()) by the caller, after it finishes consuming the created tree.
    GetThreadPoolAllocator().push();

    if (numStrings == 0)
        return true;

    // Move to length-based strings, rather than null-terminated strings.
    // Also, add strings to include the preamble and to ensure the shader is not null,
    // which lets the grammar accept what was a null (post preprocessing) shader.
    //
    // Shader will look like
    //   string 0:                system preamble
    //   string 1:                custom preamble
    //   string 2...numStrings+1: user's shader
    //   string numStrings+2:     "int;"
    const int numPre = 2;
    const int numPost = requireNonempty? 1 : 0;
    const int numTotal = numPre + numStrings + numPost;
    size_t* lengths = new size_t[numTotal];
    const char** strings = new const char*[numTotal];
    const char** names = new const char*[numTotal];
    for (int s = 0; s < numStrings; ++s) {
        strings[s + numPre] = shaderStrings[s];
        if (inputLengths == nullptr || inputLengths[s] < 0)
            lengths[s + numPre] = strlen(shaderStrings[s]);
        else
            lengths[s + numPre] = inputLengths[s];
    }
    if (stringNames != nullptr) {
        for (int s = 0; s < numStrings; ++s)
            names[s + numPre] = stringNames[s];
    } else {
        for (int s = 0; s < numStrings; ++s)
            names[s + numPre] = nullptr;
    }

    // First, without using the preprocessor or parser, find the #version, so we know what
    // symbol tables, processing rules, etc. to set up.  This does not need the extra strings
    // outlined above, just the user shader, after the system and user preambles.
    glslang::TInputScanner userInput(numStrings, &strings[numPre], &lengths[numPre]);
    int version = 0;
    EProfile profile = ENoProfile;
    bool versionNotFirstToken = false;
    bool versionNotFirst = (messages & EShMsgReadHlsl) ?
                                true :
                                userInput.scanVersion(version, profile, versionNotFirstToken);
    bool versionNotFound = version == 0;
    if (forceDefaultVersionAndProfile && (messages & EShMsgReadHlsl) == 0) {
        if (! (messages & EShMsgSuppressWarnings) && ! versionNotFound &&
            (version != defaultVersion || profile != defaultProfile)) {
            compiler->infoSink.info << "Warning, (version, profile) forced to be ("
                                    << defaultVersion << ", " << ProfileName(defaultProfile)
                                    << "), while in source code it is ("
                                    << version << ", " << ProfileName(profile) << ")\n";
        }

        if (versionNotFound) {
            versionNotFirstToken = false;
            versionNotFirst = false;
            versionNotFound = false;
        }
        version = defaultVersion;
        profile = defaultProfile;
    }
    SpvVersion spvVersion;
    if (messages & EShMsgSpvRules)
        spvVersion.spv = 0x00010000;    // TODO: eventually have this come from the outside
    EShSource source = (messages & EShMsgReadHlsl) ? EShSourceHlsl : EShSourceGlsl;
    if (messages & EShMsgVulkanRules)
        spvVersion.vulkan = 100;     // TODO: eventually have this come from the outside
    else if (spvVersion.spv != 0)
        spvVersion.openGl = 100;     // TODO: eventually have this come from the outside
    bool goodVersion = DeduceVersionProfile(compiler->infoSink, compiler->getLanguage(),
                                            versionNotFirst, defaultVersion, source, version, profile, spvVersion);
    bool versionWillBeError = (versionNotFound || (profile == EEsProfile && version >= 300 && versionNotFirst));
    bool warnVersionNotFirst = false;
    if (! versionWillBeError && versionNotFirstToken) {
        if (messages & EShMsgRelaxedErrors)
            warnVersionNotFirst = true;
        else
            versionWillBeError = true;
    }

    intermediate.setSource(source);
    intermediate.setVersion(version);
    intermediate.setProfile(profile);
    intermediate.setSpv(spvVersion);
    if (spvVersion.vulkan >= 100)
        intermediate.setOriginUpperLeft();
    if ((messages & EShMsgHlslOffsets) || (messages & EShMsgReadHlsl))
        intermediate.setHlslOffsets();
    if (messages & EShMsgDebugInfo) {
        intermediate.setSourceFile(names[numPre]);
        for (int s = 0; s < numStrings; ++s)
            intermediate.addSourceText(strings[numPre + s]);
    }
    SetupBuiltinSymbolTable(version, profile, spvVersion, source);

    TSymbolTable* cachedTable = SharedSymbolTables[MapVersionToIndex(version)]
                                                  [MapSpvVersionToIndex(spvVersion)]
                                                  [MapProfileToIndex(profile)]
                                                  [MapSourceToIndex(source)]
                                                  [compiler->getLanguage()];

    // Dynamically allocate the symbol table so we can control when it is deallocated WRT the pool.
    TSymbolTable* symbolTableMemory = new TSymbolTable;
    TSymbolTable& symbolTable = *symbolTableMemory;
    if (cachedTable)
        symbolTable.adoptLevels(*cachedTable);

    // Add built-in symbols that are potentially context dependent;
    // they get popped again further down.
    if (! AddContextSpecificSymbols(resources, compiler->infoSink, symbolTable, version, profile, spvVersion,
                                    compiler->getLanguage(), source))
        return false;

    //
    // Now we can process the full shader under proper symbols and rules.
    //

    TParseContextBase* parseContext = CreateParseContext(symbolTable, intermediate, version, profile, source,
                                                         compiler->getLanguage(), compiler->infoSink,
                                                         spvVersion, forwardCompatible, messages, false, sourceEntryPointName);

    TPpContext ppContext(*parseContext, names[numPre] ? names[numPre] : "", includer);

    // only GLSL (bison triggered, really) needs an externally set scan context
    glslang::TScanContext scanContext(*parseContext);
    if ((messages & EShMsgReadHlsl) == 0)
        parseContext->setScanContext(&scanContext);

    parseContext->setPpContext(&ppContext);
    parseContext->setLimits(*resources);
    if (! goodVersion)
        parseContext->addError();
    if (warnVersionNotFirst) {
        TSourceLoc loc;
        loc.init();
        parseContext->warn(loc, "Illegal to have non-comment, non-whitespace tokens before #version", "#version", "");
    }

    parseContext->initializeExtensionBehavior();

    // Fill in the strings as outlined above.
    std::string preamble;
    parseContext->getPreamble(preamble);
    strings[0] = preamble.c_str();
    lengths[0] = strlen(strings[0]);
    names[0] = nullptr;
    strings[1] = customPreamble;
    lengths[1] = strlen(strings[1]);
    names[1] = nullptr;
    assert(2 == numPre);
    if (requireNonempty) {
        const int postIndex = numStrings + numPre;
        strings[postIndex] = "\n int;";
        lengths[postIndex] = strlen(strings[numStrings + numPre]);
        names[postIndex] = nullptr;
    }
    TInputScanner fullInput(numStrings + numPre + numPost, strings, lengths, names, numPre, numPost);

    // Push a new symbol allocation scope that will get used for the shader's globals.
    symbolTable.push();

    bool success = processingContext(*parseContext, ppContext, fullInput,
                                     versionWillBeError, symbolTable,
                                     intermediate, optLevel, messages);

    // Clean up the symbol table. The AST is self-sufficient now.
    delete symbolTableMemory;

    delete parseContext;
    delete [] lengths;
    delete [] strings;
    delete [] names;

    return success;
}

// Responsible for keeping track of the most recent source string and line in
// the preprocessor and outputting newlines appropriately if the source string
// or line changes.
class SourceLineSynchronizer {
public:
    SourceLineSynchronizer(const std::function<int()>& lastSourceIndex,
                           std::stringstream* output)
      : getLastSourceIndex(lastSourceIndex), output(output), lastSource(-1), lastLine(0) {}
//    SourceLineSynchronizer(const SourceLineSynchronizer&) = delete;
//    SourceLineSynchronizer& operator=(const SourceLineSynchronizer&) = delete;

    // Sets the internally tracked source string index to that of the most
    // recently read token. If we switched to a new source string, returns
    // true and inserts a newline. Otherwise, returns false and outputs nothing.
    bool syncToMostRecentString() {
        if (getLastSourceIndex() != lastSource) {
            // After switching to a new source string, we need to reset lastLine
            // because line number resets every time a new source string is
            // used. We also need to output a newline to separate the output
            // from the previous source string (if there is one).
            if (lastSource != -1 || lastLine != 0)
                *output << std::endl;
            lastSource = getLastSourceIndex();
            lastLine = -1;
            return true;
        }
        return false;
    }

    // Calls syncToMostRecentString() and then sets the internally tracked line
    // number to tokenLine. If we switched to a new line, returns true and inserts
    // newlines appropriately. Otherwise, returns false and outputs nothing.
    bool syncToLine(int tokenLine) {
        syncToMostRecentString();
        const bool newLineStarted = lastLine < tokenLine;
        for (; lastLine < tokenLine; ++lastLine) {
            if (lastLine > 0) *output << std::endl;
        }
        return newLineStarted;
    }

    // Sets the internally tracked line number to newLineNum.
    void setLineNum(int newLineNum) { lastLine = newLineNum; }

private:
    SourceLineSynchronizer& operator=(const SourceLineSynchronizer&);

    // A function for getting the index of the last valid source string we've
    // read tokens from.
    const std::function<int()> getLastSourceIndex;
    // output stream for newlines.
    std::stringstream* output;
    // lastSource is the source string index (starting from 0) of the last token
    // processed. It is tracked in order for newlines to be inserted when a new
    // source string starts. -1 means we haven't started processing any source
    // string.
    int lastSource;
    // lastLine is the line number (starting from 1) of the last token processed.
    // It is tracked in order for newlines to be inserted when a token appears
    // on a new line. 0 means we haven't started processing any line in the
    // current source string.
    int lastLine;
};

// DoPreprocessing is a valid ProcessingContext template argument,
// which only performs the preprocessing step of compilation.
// It places the result in the "string" argument to its constructor.
struct DoPreprocessing {
    explicit DoPreprocessing(std::string* string): outputString(string) {}
    bool operator()(TParseContextBase& parseContext, TPpContext& ppContext,
                    TInputScanner& input, bool versionWillBeError,
                    TSymbolTable&, TIntermediate&,
                    EShOptimizationLevel, EShMessages)
    {
        // This is a list of tokens that do not require a space before or after.
        static const std::string unNeededSpaceTokens = ";()[]";
        static const std::string noSpaceBeforeTokens = ",";
        glslang::TPpToken ppToken;

        parseContext.setScanner(&input);
        ppContext.setInput(input, versionWillBeError);

        std::stringstream outputStream;
        SourceLineSynchronizer lineSync(
            std::bind(&TInputScanner::getLastValidSourceIndex, &input), &outputStream);

        parseContext.setExtensionCallback([&lineSync, &outputStream](
            int line, const char* extension, const char* behavior) {
                lineSync.syncToLine(line);
                outputStream << "#extension " << extension << " : " << behavior;
        });

        parseContext.setLineCallback([&lineSync, &outputStream, &parseContext](
            int curLineNum, int newLineNum, bool hasSource, int sourceNum, const char* sourceName) {
            // SourceNum is the number of the source-string that is being parsed.
            lineSync.syncToLine(curLineNum);
            outputStream << "#line " << newLineNum;
            if (hasSource) {
                outputStream << " ";
                if (sourceName != nullptr) {
                    outputStream << "\"" << sourceName << "\"";
                } else {
                    outputStream << sourceNum;
                }
            }
            if (parseContext.lineDirectiveShouldSetNextLine()) {
                // newLineNum is the new line number for the line following the #line
                // directive. So the new line number for the current line is
                newLineNum -= 1;
            }
            outputStream << std::endl;
            // And we are at the next line of the #line directive now.
            lineSync.setLineNum(newLineNum + 1);
        });

        parseContext.setVersionCallback(
            [&lineSync, &outputStream](int line, int version, const char* str) {
                lineSync.syncToLine(line);
                outputStream << "#version " << version;
                if (str) {
                    outputStream << " " << str;
                }
            });

        parseContext.setPragmaCallback([&lineSync, &outputStream](
            int line, const glslang::TVector<glslang::TString>& ops) {
                lineSync.syncToLine(line);
                outputStream << "#pragma ";
                for(size_t i = 0; i < ops.size(); ++i) {
                    outputStream << ops[i];
                }
        });

        parseContext.setErrorCallback([&lineSync, &outputStream](
            int line, const char* errorMessage) {
                lineSync.syncToLine(line);
                outputStream << "#error " << errorMessage;
        });

        int lastToken = EndOfInput; // lastToken records the last token processed.
        do {
            int token = ppContext.tokenize(ppToken);
            if (token == EndOfInput)
                break;

            bool isNewString = lineSync.syncToMostRecentString();
            bool isNewLine = lineSync.syncToLine(ppToken.loc.line);

            if (isNewLine) {
                // Don't emit whitespace onto empty lines.
                // Copy any whitespace characters at the start of a line
                // from the input to the output.
                outputStream << std::string(ppToken.loc.column - 1, ' ');
            }

            // Output a space in between tokens, but not at the start of a line,
            // and also not around special tokens. This helps with readability
            // and consistency.
            if (!isNewString && !isNewLine && lastToken != EndOfInput &&
                (unNeededSpaceTokens.find((char)token) == std::string::npos) &&
                (unNeededSpaceTokens.find((char)lastToken) == std::string::npos) &&
                (noSpaceBeforeTokens.find((char)token) == std::string::npos)) {
                outputStream << " ";
            }
            lastToken = token;
            outputStream << ppToken.name;
        } while (true);
        outputStream << std::endl;
        *outputString = outputStream.str();

        bool success = true;
        if (parseContext.getNumErrors() > 0) {
            success = false;
            parseContext.infoSink.info.prefix(EPrefixError);
            parseContext.infoSink.info << parseContext.getNumErrors() << " compilation errors.  No code generated.\n\n";
        }
        return success;
    }
    std::string* outputString;
};

// DoFullParse is a valid ProcessingConext template argument for fully
// parsing the shader.  It populates the "intermediate" with the AST.
struct DoFullParse{
  bool operator()(TParseContextBase& parseContext, TPpContext& ppContext,
                  TInputScanner& fullInput, bool versionWillBeError,
                  TSymbolTable&, TIntermediate& intermediate,
                  EShOptimizationLevel optLevel, EShMessages messages)
    {
        bool success = true;
        // Parse the full shader.
        if (! parseContext.parseShaderStrings(ppContext, fullInput, versionWillBeError))
            success = false;

        if (success && intermediate.getTreeRoot()) {
            if (optLevel == EShOptNoGeneration)
                parseContext.infoSink.info.message(EPrefixNone, "No errors.  No code generation or linking was requested.");
            else
                success = intermediate.postProcess(intermediate.getTreeRoot(), parseContext.getLanguage());
        } else if (! success) {
            parseContext.infoSink.info.prefix(EPrefixError);
            parseContext.infoSink.info << parseContext.getNumErrors() << " compilation errors.  No code generated.\n\n";
        }

        if (messages & EShMsgAST)
            intermediate.output(parseContext.infoSink, true);

        return success;
    }
};

// Take a single compilation unit, and run the preprocessor on it.
// Return: True if there were no issues found in preprocessing,
//         False if during preprocessing any unknown version, pragmas or
//         extensions were found.
bool PreprocessDeferred(
    TCompiler* compiler,
    const char* const shaderStrings[],
    const int numStrings,
    const int* inputLengths,
    const char* const stringNames[],
    const char* preamble,
    const EShOptimizationLevel optLevel,
    const TBuiltInResource* resources,
    int defaultVersion,         // use 100 for ES environment, 110 for desktop
    EProfile defaultProfile,
    bool forceDefaultVersionAndProfile,
    bool forwardCompatible,     // give errors for use of deprecated features
    EShMessages messages,       // warnings/errors/AST; things to print out
    TShader::Includer& includer,
    TIntermediate& intermediate, // returned tree, etc.
    std::string* outputString)
{
    DoPreprocessing parser(outputString);
    return ProcessDeferred(compiler, shaderStrings, numStrings, inputLengths, stringNames,
                           preamble, optLevel, resources, defaultVersion,
                           defaultProfile, forceDefaultVersionAndProfile,
                           forwardCompatible, messages, intermediate, parser,
                           false, includer);
}

//
// do a partial compile on the given strings for a single compilation unit
// for a potential deferred link into a single stage (and deferred full compile of that
// stage through machine-dependent compilation).
//
// all preprocessing, parsing, semantic checks, etc. for a single compilation unit
// are done here.
//
// return:  the tree and other information is filled into the intermediate argument,
//          and true is returned by the function for success.
//
bool CompileDeferred(
    TCompiler* compiler,
    const char* const shaderStrings[],
    const int numStrings,
    const int* inputLengths,
    const char* const stringNames[],
    const char* preamble,
    const EShOptimizationLevel optLevel,
    const TBuiltInResource* resources,
    int defaultVersion,         // use 100 for ES environment, 110 for desktop
    EProfile defaultProfile,
    bool forceDefaultVersionAndProfile,
    bool forwardCompatible,     // give errors for use of deprecated features
    EShMessages messages,       // warnings/errors/AST; things to print out
    TIntermediate& intermediate,// returned tree, etc.
    TShader::Includer& includer,
    const std::string sourceEntryPointName = "")
{
    DoFullParse parser;
    return ProcessDeferred(compiler, shaderStrings, numStrings, inputLengths, stringNames,
                           preamble, optLevel, resources, defaultVersion,
                           defaultProfile, forceDefaultVersionAndProfile,
                           forwardCompatible, messages, intermediate, parser,
                           true, includer, sourceEntryPointName);
}

} // end anonymous namespace for local functions

//
// ShInitialize() should be called exactly once per process, not per thread.
//
int ShInitialize()
{
    glslang::InitGlobalLock();

    if (! InitProcess())
        return 0;

    if (! PerProcessGPA)
        PerProcessGPA = new TPoolAllocator();

    glslang::TScanContext::fillInKeywordMap();
#ifdef ENABLE_HLSL
    glslang::HlslScanContext::fillInKeywordMap();
#endif

    return 1;
}

//
// Driver calls these to create and destroy compiler/linker
// objects.
//

ShHandle ShConstructCompiler(const EShLanguage language, int debugOptions)
{
    if (!InitThread())
        return 0;

    TShHandleBase* base = static_cast<TShHandleBase*>(ConstructCompiler(language, debugOptions));

    return reinterpret_cast<void*>(base);
}

ShHandle ShConstructLinker(const EShExecutable executable, int debugOptions)
{
    if (!InitThread())
        return 0;

    TShHandleBase* base = static_cast<TShHandleBase*>(ConstructLinker(executable, debugOptions));

    return reinterpret_cast<void*>(base);
}

ShHandle ShConstructUniformMap()
{
    if (!InitThread())
        return 0;

    TShHandleBase* base = static_cast<TShHandleBase*>(ConstructUniformMap());

    return reinterpret_cast<void*>(base);
}

void ShDestruct(ShHandle handle)
{
    if (handle == 0)
        return;

    TShHandleBase* base = static_cast<TShHandleBase*>(handle);

    if (base->getAsCompiler())
        DeleteCompiler(base->getAsCompiler());
    else if (base->getAsLinker())
        DeleteLinker(base->getAsLinker());
    else if (base->getAsUniformMap())
        DeleteUniformMap(base->getAsUniformMap());
}

//
// Cleanup symbol tables
//
int __fastcall ShFinalize()
{
    for (int version = 0; version < VersionCount; ++version) {
        for (int spvVersion = 0; spvVersion < SpvVersionCount; ++spvVersion) {
            for (int p = 0; p < ProfileCount; ++p) {
                for (int source = 0; source < SourceCount; ++source) {
                    for (int stage = 0; stage < EShLangCount; ++stage) {
                        delete SharedSymbolTables[version][spvVersion][p][source][stage];
                        SharedSymbolTables[version][spvVersion][p][source][stage] = 0;
                    }
                }
            }
        }
    }

    for (int version = 0; version < VersionCount; ++version) {
        for (int spvVersion = 0; spvVersion < SpvVersionCount; ++spvVersion) {
            for (int p = 0; p < ProfileCount; ++p) {
                for (int source = 0; source < SourceCount; ++source) {
                    for (int pc = 0; pc < EPcCount; ++pc) {
                        delete CommonSymbolTable[version][spvVersion][p][source][pc];
                        CommonSymbolTable[version][spvVersion][p][source][pc] = 0;
                    }
                }
            }
        }
    }

    if (PerProcessGPA) {
        PerProcessGPA->popAll();
        delete PerProcessGPA;
        PerProcessGPA = 0;
    }

    glslang::TScanContext::deleteKeywordMap();
#ifdef ENABLE_HLSL
    glslang::HlslScanContext::deleteKeywordMap();
#endif

    return 1;
}

//
// Do a full compile on the given strings for a single compilation unit
// forming a complete stage.  The result of the machine dependent compilation
// is left in the provided compile object.
//
// Return:  The return value is really boolean, indicating
// success (1) or failure (0).
//
int ShCompile(
    const ShHandle handle,
    const char* const shaderStrings[],
    const int numStrings,
    const int* inputLengths,
    const EShOptimizationLevel optLevel,
    const TBuiltInResource* resources,
    int /*debugOptions*/,
    int defaultVersion,        // use 100 for ES environment, 110 for desktop
    bool forwardCompatible,    // give errors for use of deprecated features
    EShMessages messages       // warnings/errors/AST; things to print out
    )
{
    // Map the generic handle to the C++ object
    if (handle == 0)
        return 0;

    TShHandleBase* base = reinterpret_cast<TShHandleBase*>(handle);
    TCompiler* compiler = base->getAsCompiler();
    if (compiler == 0)
        return 0;

    compiler->infoSink.info.erase();
    compiler->infoSink.debug.erase();

    TIntermediate intermediate(compiler->getLanguage());
    TShader::ForbidIncluder includer;
    bool success = CompileDeferred(compiler, shaderStrings, numStrings, inputLengths, nullptr,
                                   "", optLevel, resources, defaultVersion, ENoProfile, false,
                                   forwardCompatible, messages, intermediate, includer);

    //
    // Call the machine dependent compiler
    //
    if (success && intermediate.getTreeRoot() && optLevel != EShOptNoGeneration)
        success = compiler->compile(intermediate.getTreeRoot(), intermediate.getVersion(), intermediate.getProfile());

    intermediate.removeTree();

    // Throw away all the temporary memory used by the compilation process.
    // The push was done in the CompileDeferred() call above.
    GetThreadPoolAllocator().pop();

    return success ? 1 : 0;
}

//
// Link the given compile objects.
//
// Return:  The return value of is really boolean, indicating
// success or failure.
//
int ShLinkExt(
    const ShHandle linkHandle,
    const ShHandle compHandles[],
    const int numHandles)
{
    if (linkHandle == 0 || numHandles == 0)
        return 0;

    THandleList cObjects;

    for (int i = 0; i < numHandles; ++i) {
        if (compHandles[i] == 0)
            return 0;
        TShHandleBase* base = reinterpret_cast<TShHandleBase*>(compHandles[i]);
        if (base->getAsLinker()) {
            cObjects.push_back(base->getAsLinker());
        }
        if (base->getAsCompiler())
            cObjects.push_back(base->getAsCompiler());

        if (cObjects[i] == 0)
            return 0;
    }

    TShHandleBase* base = reinterpret_cast<TShHandleBase*>(linkHandle);
    TLinker* linker = static_cast<TLinker*>(base->getAsLinker());

    if (linker == 0)
        return 0;

    linker->infoSink.info.erase();

    for (int i = 0; i < numHandles; ++i) {
        if (cObjects[i]->getAsCompiler()) {
            if (! cObjects[i]->getAsCompiler()->linkable()) {
                linker->infoSink.info.message(EPrefixError, "Not all shaders have valid object code.");
                return 0;
            }
        }
    }

    bool ret = linker->link(cObjects);

    return ret ? 1 : 0;
}

//
// ShSetEncrpytionMethod is a place-holder for specifying
// how source code is encrypted.
//
void ShSetEncryptionMethod(ShHandle handle)
{
    if (handle == 0)
        return;
}

//
// Return any compiler/linker/uniformmap log of messages for the application.
//
const char* ShGetInfoLog(const ShHandle handle)
{
    if (!InitThread())
        return 0;

    if (handle == 0)
        return 0;

    TShHandleBase* base = static_cast<TShHandleBase*>(handle);
    TInfoSink* infoSink;

    if (base->getAsCompiler())
        infoSink = &(base->getAsCompiler()->getInfoSink());
    else if (base->getAsLinker())
        infoSink = &(base->getAsLinker()->getInfoSink());
    else
        return 0;

    infoSink->info << infoSink->debug.c_str();
    return infoSink->info.c_str();
}

//
// Return the resulting binary code from the link process.  Structure
// is machine dependent.
//
const void* ShGetExecutable(const ShHandle handle)
{
    if (!InitThread())
        return 0;

    if (handle == 0)
        return 0;

    TShHandleBase* base = reinterpret_cast<TShHandleBase*>(handle);

    TLinker* linker = static_cast<TLinker*>(base->getAsLinker());
    if (linker == 0)
        return 0;

    return linker->getObjectCode();
}

//
// Let the linker know where the application said it's attributes are bound.
// The linker does not use these values, they are remapped by the ICD or
// hardware.  It just needs them to know what's aliased.
//
// Return:  The return value of is really boolean, indicating
// success or failure.
//
int ShSetVirtualAttributeBindings(const ShHandle handle, const ShBindingTable* table)
{
    if (!InitThread())
        return 0;

    if (handle == 0)
        return 0;

    TShHandleBase* base = reinterpret_cast<TShHandleBase*>(handle);
    TLinker* linker = static_cast<TLinker*>(base->getAsLinker());

    if (linker == 0)
        return 0;

    linker->setAppAttributeBindings(table);

    return 1;
}

//
// Let the linker know where the predefined attributes have to live.
//
int ShSetFixedAttributeBindings(const ShHandle handle, const ShBindingTable* table)
{
    if (!InitThread())
        return 0;

    if (handle == 0)
        return 0;

    TShHandleBase* base = reinterpret_cast<TShHandleBase*>(handle);
    TLinker* linker = static_cast<TLinker*>(base->getAsLinker());

    if (linker == 0)
        return 0;

    linker->setFixedAttributeBindings(table);
    return 1;
}

//
// Some attribute locations are off-limits to the linker...
//
int ShExcludeAttributes(const ShHandle handle, int *attributes, int count)
{
    if (!InitThread())
        return 0;

    if (handle == 0)
        return 0;

    TShHandleBase* base = reinterpret_cast<TShHandleBase*>(handle);
    TLinker* linker = static_cast<TLinker*>(base->getAsLinker());
    if (linker == 0)
        return 0;

    linker->setExcludedAttributes(attributes, count);

    return 1;
}

//
// Return the index for OpenGL to use for knowing where a uniform lives.
//
// Return:  The return value of is really boolean, indicating
// success or failure.
//
int ShGetUniformLocation(const ShHandle handle, const char* name)
{
    if (!InitThread())
        return 0;

    if (handle == 0)
        return -1;

    TShHandleBase* base = reinterpret_cast<TShHandleBase*>(handle);
    TUniformMap* uniformMap= base->getAsUniformMap();
    if (uniformMap == 0)
        return -1;

    return uniformMap->getLocation(name);
}

////////////////////////////////////////////////////////////////////////////////////////////
//
// Deferred-Lowering C++ Interface
// -----------------------------------
//
// Below is a new alternate C++ interface that might potentially replace the above
// opaque handle-based interface.
//
// See more detailed comment in ShaderLang.h
//

namespace glslang {

#include "../Include/revision.h"

const char* GetEsslVersionString()
{
    return "OpenGL ES GLSL 3.00 glslang LunarG Khronos." GLSLANG_REVISION " " GLSLANG_DATE;
}

const char* GetGlslVersionString()
{
    return "4.20 glslang LunarG Khronos." GLSLANG_REVISION " " GLSLANG_DATE;
}

int GetKhronosToolId()
{
    return 8;
}

bool InitializeProcess()
{
    return ShInitialize() != 0;
}

void FinalizeProcess()
{
    ShFinalize();
}

class TDeferredCompiler : public TCompiler {
public:
    TDeferredCompiler(EShLanguage s, TInfoSink& i) : TCompiler(s, i) { }
    virtual bool compile(TIntermNode*, int = 0, EProfile = ENoProfile) { return true; }
};

TShader::TShader(EShLanguage s)
    : pool(0), stage(s), lengths(nullptr), stringNames(nullptr), preamble("")
{
    infoSink = new TInfoSink;
    compiler = new TDeferredCompiler(stage, *infoSink);
    intermediate = new TIntermediate(s);
}

TShader::~TShader()
{
    delete infoSink;
    delete compiler;
    delete intermediate;
    delete pool;
}

void TShader::setStrings(const char* const* s, int n)
{
    strings = s;
    numStrings = n;
    lengths = nullptr;
}

void TShader::setStringsWithLengths(const char* const* s, const int* l, int n)
{
    strings = s;
    numStrings = n;
    lengths = l;
}

void TShader::setStringsWithLengthsAndNames(
    const char* const* s, const int* l, const char* const* names, int n)
{
    strings = s;
    numStrings = n;
    lengths = l;
    stringNames = names;
}

void TShader::setEntryPoint(const char* entryPoint)
{
    intermediate->setEntryPointName(entryPoint);
}

void TShader::setSourceEntryPoint(const char* name)
{
    sourceEntryPointName = name;
}

// Set binding base for sampler types
void TShader::setShiftSamplerBinding(unsigned int base) { intermediate->setShiftSamplerBinding(base); }
// Set binding base for texture types (SRV)
void TShader::setShiftTextureBinding(unsigned int base) { intermediate->setShiftTextureBinding(base); }
// Set binding base for image types
void TShader::setShiftImageBinding(unsigned int base)   { intermediate->setShiftImageBinding(base); }
// Set binding base for uniform buffer objects (CBV)
void TShader::setShiftUboBinding(unsigned int base)     { intermediate->setShiftUboBinding(base); }
// Synonym for setShiftUboBinding, to match HLSL language.
void TShader::setShiftCbufferBinding(unsigned int base) { intermediate->setShiftUboBinding(base); }
// Set binding base for UAV (unordered access view)
void TShader::setShiftUavBinding(unsigned int base)     { intermediate->setShiftUavBinding(base); }
// Set binding base for SSBOs
void TShader::setShiftSsboBinding(unsigned int base)    { intermediate->setShiftSsboBinding(base); }
// Enables binding automapping using TIoMapper
void TShader::setAutoMapBindings(bool map)              { intermediate->setAutoMapBindings(map); }
// Fragile: currently within one stage: simple auto-assignment of location
void TShader::setAutoMapLocations(bool map)              { intermediate->setAutoMapLocations(map); }
// See comment above TDefaultHlslIoMapper in iomapper.cpp:
void TShader::setHlslIoMapping(bool hlslIoMap)          { intermediate->setHlslIoMapping(hlslIoMap); }
void TShader::setFlattenUniformArrays(bool flatten)     { intermediate->setFlattenUniformArrays(flatten); }
void TShader::setNoStorageFormat(bool useUnknownFormat) { intermediate->setNoStorageFormat(useUnknownFormat); }
void TShader::setResourceSetBinding(const std::vector<std::string>& base)   { intermediate->setResourceSetBinding(base); }
void TShader::setTextureSamplerTransformMode(EShTextureSamplerTransformMode mode) { intermediate->setTextureSamplerTransformMode(mode); }

//
// Turn the shader strings into a parse tree in the TIntermediate.
//
// Returns true for success.
//
bool TShader::parse(const TBuiltInResource* builtInResources, int defaultVersion, EProfile defaultProfile, bool forceDefaultVersionAndProfile,
                    bool forwardCompatible, EShMessages messages, Includer& includer)
{
    if (! InitThread())
        return false;

    pool = new TPoolAllocator();
    SetThreadPoolAllocator(*pool);
    if (! preamble)
        preamble = "";

    return CompileDeferred(compiler, strings, numStrings, lengths, stringNames,
                           preamble, EShOptNone, builtInResources, defaultVersion,
                           defaultProfile, forceDefaultVersionAndProfile,
                           forwardCompatible, messages, *intermediate, includer, sourceEntryPointName);
}

// Fill in a string with the result of preprocessing ShaderStrings
// Returns true if all extensions, pragmas and version strings were valid.
bool TShader::preprocess(const TBuiltInResource* builtInResources,
                         int defaultVersion, EProfile defaultProfile,
                         bool forceDefaultVersionAndProfile,
                         bool forwardCompatible, EShMessages message,
                         std::string* output_string,
                         Includer& includer)
{
    if (! InitThread())
        return false;

    pool = new TPoolAllocator();
    SetThreadPoolAllocator(*pool);
    if (! preamble)
        preamble = "";

    return PreprocessDeferred(compiler, strings, numStrings, lengths, stringNames, preamble,
                              EShOptNone, builtInResources, defaultVersion,
                              defaultProfile, forceDefaultVersionAndProfile,
                              forwardCompatible, message, includer, *intermediate, output_string);
}

const char* TShader::getInfoLog()
{
    return infoSink->info.c_str();
}

const char* TShader::getInfoDebugLog()
{
    return infoSink->debug.c_str();
}

TProgram::TProgram() : pool(0), reflection(0), ioMapper(nullptr), linked(false)
{
    infoSink = new TInfoSink;
    for (int s = 0; s < EShLangCount; ++s) {
        intermediate[s] = 0;
        newedIntermediate[s] = false;
    }
}

TProgram::~TProgram()
{
    delete ioMapper;
    delete infoSink;
    delete reflection;

    for (int s = 0; s < EShLangCount; ++s)
        if (newedIntermediate[s])
            delete intermediate[s];

    delete pool;
}

//
// Merge the compilation units within each stage into a single TIntermediate.
// All starting compilation units need to be the result of calling TShader::parse().
//
// Return true for success.
//
bool TProgram::link(EShMessages messages)
{
    if (linked)
        return false;
    linked = true;

    bool error = false;

    pool = new TPoolAllocator();
    SetThreadPoolAllocator(*pool);

    for (int s = 0; s < EShLangCount; ++s) {
        if (! linkStage((EShLanguage)s, messages))
            error = true;
    }

    // TODO: Link: cross-stage error checking

    return ! error;
}

//
// Merge the compilation units within the given stage into a single TIntermediate.
//
// Return true for success.
//
bool TProgram::linkStage(EShLanguage stage, EShMessages messages)
{
    if (stages[stage].size() == 0)
        return true;

    int numEsShaders = 0, numNonEsShaders = 0;
    for (auto it = stages[stage].begin(); it != stages[stage].end(); ++it) {
        if ((*it)->intermediate->getProfile() == EEsProfile) {
            numEsShaders++;
        } else {
            numNonEsShaders++;
        }
    }

    if (numEsShaders > 0 && numNonEsShaders > 0) {
        infoSink->info.message(EPrefixError, "Cannot mix ES profile with non-ES profile shaders");
        return false;
    } else if (numEsShaders > 1) {
        infoSink->info.message(EPrefixError, "Cannot attach multiple ES shaders of the same type to a single program");
        return false;
    }

    //
    // Be efficient for the common single compilation unit per stage case,
    // reusing it's TIntermediate instead of merging into a new one.
    //
    TIntermediate *firstIntermediate = stages[stage].front()->intermediate;
    if (stages[stage].size() == 1)
        intermediate[stage] = firstIntermediate;
    else {
        intermediate[stage] = new TIntermediate(stage,
                                                firstIntermediate->getVersion(),
                                                firstIntermediate->getProfile());


        // The new TIntermediate must use the same origin as the original TIntermediates.
        // Otherwise linking will fail due to different coordinate systems.
        if (firstIntermediate->getOriginUpperLeft()) {
            intermediate[stage]->setOriginUpperLeft();
        }
        intermediate[stage]->setSpv(firstIntermediate->getSpv());

        newedIntermediate[stage] = true;
    }

    if (messages & EShMsgAST)
        infoSink->info << "\nLinked " << StageName(stage) << " stage:\n\n";

    if (stages[stage].size() > 1) {
        std::list<TShader*>::const_iterator it;
        for (it = stages[stage].begin(); it != stages[stage].end(); ++it)
            intermediate[stage]->merge(*infoSink, *(*it)->intermediate);
    }

    intermediate[stage]->finalCheck(*infoSink, (messages & EShMsgKeepUncalled) != 0);

    if (messages & EShMsgAST)
        intermediate[stage]->output(*infoSink, true);

    return intermediate[stage]->getNumErrors() == 0;
}

const char* TProgram::getInfoLog()
{
    return infoSink->info.c_str();
}

const char* TProgram::getInfoDebugLog()
{
    return infoSink->debug.c_str();
}

//
// Reflection implementation.
//

bool TProgram::buildReflection()
{
    if (! linked || reflection)
        return false;

    reflection = new TReflection;

    for (int s = 0; s < EShLangCount; ++s) {
        if (intermediate[s]) {
            if (! reflection->addStage((EShLanguage)s, *intermediate[s]))
                return false;
        }
    }

    return true;
}

int TProgram::getNumLiveUniformVariables() const             { return reflection->getNumUniforms(); }
int TProgram::getNumLiveUniformBlocks() const                { return reflection->getNumUniformBlocks(); }
const char* TProgram::getUniformName(int index) const        { return reflection->getUniform(index).name.c_str(); }
const char* TProgram::getUniformBlockName(int index) const   { return reflection->getUniformBlock(index).name.c_str(); }
int TProgram::getUniformBlockSize(int index) const           { return reflection->getUniformBlock(index).size; }
int TProgram::getUniformIndex(const char* name) const        { return reflection->getIndex(name); }
int TProgram::getUniformBlockIndex(int index) const          { return reflection->getUniform(index).index; }
int TProgram::getUniformBlockCounterIndex(int index) const   { return reflection->getUniformBlock(index).counterIndex; }
int TProgram::getUniformType(int index) const                { return reflection->getUniform(index).glDefineType; }
int TProgram::getUniformBufferOffset(int index) const        { return reflection->getUniform(index).offset; }
int TProgram::getUniformArraySize(int index) const           { return reflection->getUniform(index).size; }
int TProgram::getNumLiveAttributes() const                   { return reflection->getNumAttributes(); }
const char* TProgram::getAttributeName(int index) const      { return reflection->getAttribute(index).name.c_str(); }
int TProgram::getAttributeType(int index) const              { return reflection->getAttribute(index).glDefineType; }
const TType* TProgram::getAttributeTType(int index) const    { return reflection->getAttribute(index).getType(); }
const TType* TProgram::getUniformTType(int index) const      { return reflection->getUniform(index).getType(); }
const TType* TProgram::getUniformBlockTType(int index) const { return reflection->getUniformBlock(index).getType(); }
unsigned TProgram::getLocalSize(int dim) const               { return reflection->getLocalSize(dim); }

void TProgram::dumpReflection()                      { reflection->dump(); }

//
// I/O mapping implementation.
//
bool TProgram::mapIO(TIoMapResolver* resolver)
{
    if (! linked || ioMapper)
        return false;

    ioMapper = new TIoMapper;

    for (int s = 0; s < EShLangCount; ++s) {
        if (intermediate[s]) {
            if (! ioMapper->addStage((EShLanguage)s, *intermediate[s], *infoSink, resolver))
                return false;
        }
    }

    return true;
}

} // end namespace glslang
