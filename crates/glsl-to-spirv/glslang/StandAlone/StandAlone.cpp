//
// Copyright (C) 2002-2005  3Dlabs Inc. Ltd.
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

// this only applies to the standalone wrapper, not the front end in general
#ifndef _CRT_SECURE_NO_WARNINGS
#define _CRT_SECURE_NO_WARNINGS
#endif

#include "ResourceLimits.h"
#include "Worklist.h"
#include "DirStackFileIncluder.h"
#include "./../glslang/Include/ShHandle.h"
#include "./../glslang/Include/revision.h"
#include "./../glslang/Public/ShaderLang.h"
#include "../SPIRV/GlslangToSpv.h"
#include "../SPIRV/GLSL.std.450.h"
#include "../SPIRV/doc.h"
#include "../SPIRV/disassemble.h"

#include <cstring>
#include <cstdlib>
#include <cctype>
#include <cmath>
#include <array>
#include <memory>
#include <thread>

#include "../glslang/OSDependent/osinclude.h"

extern "C" {
    SH_IMPORT_EXPORT void ShOutputHtml();
}

// Command-line options
enum TOptions {
    EOptionNone                 = 0,
    EOptionIntermediate         = (1 <<  0),
    EOptionSuppressInfolog      = (1 <<  1),
    EOptionMemoryLeakMode       = (1 <<  2),
    EOptionRelaxedErrors        = (1 <<  3),
    EOptionGiveWarnings         = (1 <<  4),
    EOptionLinkProgram          = (1 <<  5),
    EOptionMultiThreaded        = (1 <<  6),
    EOptionDumpConfig           = (1 <<  7),
    EOptionDumpReflection       = (1 <<  8),
    EOptionSuppressWarnings     = (1 <<  9),
    EOptionDumpVersions         = (1 << 10),
    EOptionSpv                  = (1 << 11),
    EOptionHumanReadableSpv     = (1 << 12),
    EOptionVulkanRules          = (1 << 13),
    EOptionDefaultDesktop       = (1 << 14),
    EOptionOutputPreprocessed   = (1 << 15),
    EOptionOutputHexadecimal    = (1 << 16),
    EOptionReadHlsl             = (1 << 17),
    EOptionCascadingErrors      = (1 << 18),
    EOptionAutoMapBindings      = (1 << 19),
    EOptionFlattenUniformArrays = (1 << 20),
    EOptionNoStorageFormat      = (1 << 21),
    EOptionKeepUncalled         = (1 << 22),
    EOptionHlslOffsets          = (1 << 23),
    EOptionHlslIoMapping        = (1 << 24),
    EOptionAutoMapLocations     = (1 << 25),
    EOptionDebug                = (1 << 26),
};

//
// Return codes from main/exit().
//
enum TFailCode {
    ESuccess = 0,
    EFailUsage,
    EFailCompile,
    EFailLink,
    EFailCompilerCreate,
    EFailThreadCreate,
    EFailLinkerCreate
};

//
// Forward declarations.
//
EShLanguage FindLanguage(const std::string& name, bool parseSuffix=true);
void CompileFile(const char* fileName, ShHandle);
void usage();
char* ReadFileData(const char* fileName);
void FreeFileData(char* data);
void InfoLogMsg(const char* msg, const char* name, const int num);

// Globally track if any compile or link failure.
bool CompileFailed = false;
bool LinkFailed = false;

TBuiltInResource Resources;
std::string ConfigFile;

//
// Parse either a .conf file provided by the user or the default from glslang::DefaultTBuiltInResource
//
void ProcessConfigFile()
{
    if (ConfigFile.size() == 0)
        Resources = glslang::DefaultTBuiltInResource;
    else {
        char* configString = ReadFileData(ConfigFile.c_str());
        glslang::DecodeResourceLimits(&Resources,  configString);
        FreeFileData(configString);
    }
}

int Options = 0;
const char* ExecutableName = nullptr;
const char* binaryFileName = nullptr;
const char* entryPointName = nullptr;
const char* sourceEntryPointName = nullptr;
const char* shaderStageName = nullptr;
const char* variableName = nullptr;
std::vector<std::string> IncludeDirectoryList;

std::array<unsigned int, EShLangCount> baseSamplerBinding;
std::array<unsigned int, EShLangCount> baseTextureBinding;
std::array<unsigned int, EShLangCount> baseImageBinding;
std::array<unsigned int, EShLangCount> baseUboBinding;
std::array<unsigned int, EShLangCount> baseSsboBinding;
std::array<unsigned int, EShLangCount> baseUavBinding;
std::array<std::vector<std::string>, EShLangCount> baseResourceSetBinding;


// Add things like "#define ..." to a preamble to use in the beginning of the shader.
class TPreamble {
public:
    TPreamble() { }

    bool isSet() const { return text.size() > 0; }
    const char* get() const { return text.c_str(); }

    // #define...
    void addDef(std::string def)
    {
        text.append("#define ");
        fixLine(def);

        // The first "=" needs to turn into a space
        int equal = def.find_first_of("=");
        if (equal != def.npos)
            def[equal] = ' ';

        text.append(def);
        text.append("\n");
    }

    // #undef...
    void addUndef(std::string undef)
    {
        text.append("#undef ");
        fixLine(undef);
        text.append(undef);
        text.append("\n");
    }

protected:
    void fixLine(std::string& line)
    {
        // Can't go past a newline in the line
        int end = line.find_first_of("\n");
        if (end != line.npos)
            line = line.substr(0, end);
    }

    std::string text;  // contents of preamble
};

TPreamble UserPreamble;

//
// Create the default name for saving a binary if -o is not provided.
//
const char* GetBinaryName(EShLanguage stage)
{
    const char* name;
    if (binaryFileName == nullptr) {
        switch (stage) {
        case EShLangVertex:          name = "vert.spv";    break;
        case EShLangTessControl:     name = "tesc.spv";    break;
        case EShLangTessEvaluation:  name = "tese.spv";    break;
        case EShLangGeometry:        name = "geom.spv";    break;
        case EShLangFragment:        name = "frag.spv";    break;
        case EShLangCompute:         name = "comp.spv";    break;
        default:                     name = "unknown";     break;
        }
    } else
        name = binaryFileName;

    return name;
}

//
// *.conf => this is a config file that can set limits/resources
//
bool SetConfigFile(const std::string& name)
{
    if (name.size() < 5)
        return false;

    if (name.compare(name.size() - 5, 5, ".conf") == 0) {
        ConfigFile = name;
        return true;
    }

    return false;
}

//
// Give error and exit with failure code.
//
void Error(const char* message)
{
    printf("%s: Error %s (use -h for usage)\n", ExecutableName, message);
    exit(EFailUsage);
}

//
// Process an optional binding base of the form:
//   --argname [stage] base
// Where stage is one of the forms accepted by FindLanguage, and base is an integer
//
void ProcessBindingBase(int& argc, char**& argv, std::array<unsigned int, EShLangCount>& base)
{
    if (argc < 2)
        usage();

    if (!isdigit(argv[1][0])) {
        if (argc < 3) // this form needs one more argument
            usage();

        // Parse form: --argname stage base
        const EShLanguage lang = FindLanguage(argv[1], false);
        base[lang] = atoi(argv[2]);
        argc-= 2;
        argv+= 2;
    } else {
        // Parse form: --argname base
        for (int lang=0; lang<EShLangCount; ++lang)
            base[lang] = atoi(argv[1]);

        argc--;
        argv++;
    }
}

void ProcessResourceSetBindingBase(int& argc, char**& argv, std::array<std::vector<std::string>, EShLangCount>& base)
{
    if (argc < 2)
        usage();

    if (!isdigit(argv[1][0])) {
        if (argc < 5) // this form needs one more argument
            usage();

        // Parse form: --argname stage base
        const EShLanguage lang = FindLanguage(argv[1], false);

        base[lang].push_back(argv[2]);
        base[lang].push_back(argv[3]);
        base[lang].push_back(argv[4]);
        argc-= 4;
        argv+= 4;
        while(argv[1] != NULL) {
            if(argv[1][0] != '-') {
                base[lang].push_back(argv[1]);
                base[lang].push_back(argv[2]);
                base[lang].push_back(argv[3]);
                argc-= 3;
                argv+= 3;
            }
            else {
                break;
            }
        }
    } else {
        // Parse form: --argname base
        for (int lang=0; lang<EShLangCount; ++lang)
            base[lang].push_back(argv[1]);

        argc--;
        argv++;
    }
}

//
// Do all command-line argument parsing.  This includes building up the work-items
// to be processed later, and saving all the command-line options.
//
// Does not return (it exits) if command-line is fatally flawed.
//
void ProcessArguments(std::vector<std::unique_ptr<glslang::TWorkItem>>& workItems, int argc, char* argv[])
{
    baseSamplerBinding.fill(0);
    baseTextureBinding.fill(0);
    baseImageBinding.fill(0);
    baseUboBinding.fill(0);
    baseSsboBinding.fill(0);
    baseUavBinding.fill(0);

    ExecutableName = argv[0];
    workItems.reserve(argc);

    const auto getStringOperand = [&](const char* desc) {
        if (argv[0][2] == 0) {
            printf("%s must immediately follow option (no spaces)\n", desc);
            exit(EFailUsage);
        }
        return argv[0] + 2;
    };

    argc--;
    argv++;
    for (; argc >= 1; argc--, argv++) {
        if (argv[0][0] == '-') {
            switch (argv[0][1]) {
            case '-':
                {
                    std::string lowerword(argv[0]+2);
                    std::transform(lowerword.begin(), lowerword.end(), lowerword.begin(), ::tolower);

                    // handle --word style options
                    if (lowerword == "auto-map-bindings" ||  // synonyms
                               lowerword == "auto-map-binding"  ||
                               lowerword == "amb") {
                        Options |= EOptionAutoMapBindings;
                    } else if (lowerword == "auto-map-locations" || // synonyms
                               lowerword == "aml") {
                        Options |= EOptionAutoMapLocations;
                    } else if (lowerword == "flatten-uniform-arrays" || // synonyms
                               lowerword == "flatten-uniform-array"  ||
                               lowerword == "fua") {
                        Options |= EOptionFlattenUniformArrays;
                    } else if (lowerword == "hlsl-offsets") {
                        Options |= EOptionHlslOffsets;
                    } else if (lowerword == "hlsl-iomap" ||
                               lowerword == "hlsl-iomapper" ||
                               lowerword == "hlsl-iomapping") {
                        Options |= EOptionHlslIoMapping;
                    } else if (lowerword == "keep-uncalled" || // synonyms
                               lowerword == "ku") {
                        Options |= EOptionKeepUncalled;
                    } else if (lowerword == "no-storage-format" || // synonyms
                               lowerword == "nsf") {
                        Options |= EOptionNoStorageFormat;
                    } else if (lowerword == "resource-set-bindings" ||  // synonyms
                               lowerword == "resource-set-binding"  ||
                               lowerword == "rsb") {
                        ProcessResourceSetBindingBase(argc, argv, baseResourceSetBinding);
                    } else if (lowerword == "shift-image-bindings" ||  // synonyms
                               lowerword == "shift-image-binding"  ||
                               lowerword == "sib") {
                        ProcessBindingBase(argc, argv, baseImageBinding);
                    } else if (lowerword == "shift-sampler-bindings" || // synonyms
                        lowerword == "shift-sampler-binding"  ||
                        lowerword == "ssb") {
                        ProcessBindingBase(argc, argv, baseSamplerBinding);
                    } else if (lowerword == "shift-uav-bindings" ||  // synonyms
                               lowerword == "shift-uav-binding"  ||
                               lowerword == "suavb") {
                        ProcessBindingBase(argc, argv, baseUavBinding);
                    } else if (lowerword == "shift-texture-bindings" ||  // synonyms
                               lowerword == "shift-texture-binding"  ||
                               lowerword == "stb") {
                        ProcessBindingBase(argc, argv, baseTextureBinding);
                    } else if (lowerword == "shift-ubo-bindings" ||  // synonyms
                               lowerword == "shift-ubo-binding"  ||
                               lowerword == "shift-cbuffer-bindings" ||
                               lowerword == "shift-cbuffer-binding"  ||
                               lowerword == "sub" ||
                               lowerword == "scb") {
                        ProcessBindingBase(argc, argv, baseUboBinding);
                    } else if (lowerword == "shift-ssbo-bindings" ||  // synonyms
                               lowerword == "shift-ssbo-binding"  ||
                               lowerword == "sbb") {
                        ProcessBindingBase(argc, argv, baseSsboBinding);
                    } else if (lowerword == "source-entrypoint" || // synonyms
                               lowerword == "sep") {
                        sourceEntryPointName = argv[1];
                        if (argc > 0) {
                            argc--;
                            argv++;
                        } else
                            Error("no <entry-point> provided for --source-entrypoint");
                        break;
                    } else if (lowerword == "variable-name" || // synonyms
                        lowerword == "vn") {
                        Options |= EOptionOutputHexadecimal;
                        variableName = argv[1];
                        if (argc > 0) {
                            argc--;
                            argv++;
                        } else
                            Error("no <C-variable-name> provided for --variable-name");
                        break;
                    } else {
                        usage();
                    }
                }
                break;
            case 'C':
                Options |= EOptionCascadingErrors;
                break;
            case 'D':
                if (argv[0][2] == 0)
                    Options |= EOptionReadHlsl;
                else
                    UserPreamble.addDef(getStringOperand("-D<macro> macro name"));
                break;
            case 'E':
                Options |= EOptionOutputPreprocessed;
                break;
            case 'G':
                Options |= EOptionSpv;
                Options |= EOptionLinkProgram;
                // undo a -H default to Vulkan
                Options &= ~EOptionVulkanRules;
                break;
            case 'H':
                Options |= EOptionHumanReadableSpv;
                if ((Options & EOptionSpv) == 0) {
                    // default to Vulkan
                    Options |= EOptionSpv;
                    Options |= EOptionVulkanRules;
                    Options |= EOptionLinkProgram;
                }
                break;
            case 'I':
                IncludeDirectoryList.push_back(getStringOperand("-I<dir> include path"));
                break;
            case 'S':
                shaderStageName = argv[1];
                if (argc > 0) {
                    argc--;
                    argv++;
                } else
                    Error("no <stage> specified for -S");
                break;
            case 'U':
                UserPreamble.addUndef(getStringOperand("-U<macro>: macro name"));
                break;
            case 'V':
                Options |= EOptionSpv;
                Options |= EOptionVulkanRules;
                Options |= EOptionLinkProgram;
                break;
            case 'c':
                Options |= EOptionDumpConfig;
                break;
            case 'd':
                Options |= EOptionDefaultDesktop;
                break;
            case 'e':
                // HLSL todo: entry point handle needs much more sophistication.
                // This is okay for one compilation unit with one entry point.
                entryPointName = argv[1];
                if (argc > 0) {
                    argc--;
                    argv++;
                } else
                    Error("no <entry-point> provided for -e");
                break;
            case 'g':
                Options |= EOptionDebug;
                break;
            case 'h':
                usage();
                break;
            case 'i':
                Options |= EOptionIntermediate;
                break;
            case 'l':
                Options |= EOptionLinkProgram;
                break;
            case 'm':
                Options |= EOptionMemoryLeakMode;
                break;
            case 'o':
                binaryFileName = argv[1];
                if (argc > 0) {
                    argc--;
                    argv++;
                } else
                    Error("no <file> provided for -o");
                break;
            case 'q':
                Options |= EOptionDumpReflection;
                break;
            case 'r':
                Options |= EOptionRelaxedErrors;
                break;
            case 's':
                Options |= EOptionSuppressInfolog;
                break;
            case 't':
                Options |= EOptionMultiThreaded;
                break;
            case 'v':
                Options |= EOptionDumpVersions;
                break;
            case 'w':
                Options |= EOptionSuppressWarnings;
                break;
            case 'x':
                Options |= EOptionOutputHexadecimal;
                break;
            default:
                usage();
                break;
            }
        } else {
            std::string name(argv[0]);
            if (! SetConfigFile(name)) {
                workItems.push_back(std::unique_ptr<glslang::TWorkItem>(new glslang::TWorkItem(name)));
            }
        }
    }

    // Make sure that -E is not specified alongside linking (which includes SPV generation)
    if ((Options & EOptionOutputPreprocessed) && (Options & EOptionLinkProgram))
        Error("can't use -E when linking is selected");

    // -o or -x makes no sense if there is no target binary
    if (binaryFileName && (Options & EOptionSpv) == 0)
        Error("no binary generation requested (e.g., -V)");

    if ((Options & EOptionFlattenUniformArrays) != 0 &&
        (Options & EOptionReadHlsl) == 0)
        Error("uniform array flattening only valid when compiling HLSL source.");
}

//
// Translate the meaningful subset of command-line options to parser-behavior options.
//
void SetMessageOptions(EShMessages& messages)
{
    if (Options & EOptionRelaxedErrors)
        messages = (EShMessages)(messages | EShMsgRelaxedErrors);
    if (Options & EOptionIntermediate)
        messages = (EShMessages)(messages | EShMsgAST);
    if (Options & EOptionSuppressWarnings)
        messages = (EShMessages)(messages | EShMsgSuppressWarnings);
    if (Options & EOptionSpv)
        messages = (EShMessages)(messages | EShMsgSpvRules);
    if (Options & EOptionVulkanRules)
        messages = (EShMessages)(messages | EShMsgVulkanRules);
    if (Options & EOptionOutputPreprocessed)
        messages = (EShMessages)(messages | EShMsgOnlyPreprocessor);
    if (Options & EOptionReadHlsl)
        messages = (EShMessages)(messages | EShMsgReadHlsl);
    if (Options & EOptionCascadingErrors)
        messages = (EShMessages)(messages | EShMsgCascadingErrors);
    if (Options & EOptionKeepUncalled)
        messages = (EShMessages)(messages | EShMsgKeepUncalled);
    if (Options & EOptionHlslOffsets)
        messages = (EShMessages)(messages | EShMsgHlslOffsets);
    if (Options & EOptionDebug)
        messages = (EShMessages)(messages | EShMsgDebugInfo);
}

//
// Thread entry point, for non-linking asynchronous mode.
//
void CompileShaders(glslang::TWorklist& worklist)
{
    glslang::TWorkItem* workItem;
    while (worklist.remove(workItem)) {
        ShHandle compiler = ShConstructCompiler(FindLanguage(workItem->name), Options);
        if (compiler == 0)
            return;

        CompileFile(workItem->name.c_str(), compiler);

        if (! (Options & EOptionSuppressInfolog))
            workItem->results = ShGetInfoLog(compiler);

        ShDestruct(compiler);
    }
}

// Outputs the given string, but only if it is non-null and non-empty.
// This prevents erroneous newlines from appearing.
void PutsIfNonEmpty(const char* str)
{
    if (str && str[0]) {
        puts(str);
    }
}

// Outputs the given string to stderr, but only if it is non-null and non-empty.
// This prevents erroneous newlines from appearing.
void StderrIfNonEmpty(const char* str)
{
    if (str && str[0])
        fprintf(stderr, "%s\n", str);
}

// Simple bundling of what makes a compilation unit for ease in passing around,
// and separation of handling file IO versus API (programmatic) compilation.
struct ShaderCompUnit {
    EShLanguage stage;
    static const int maxCount = 1;
    int count;                          // live number of strings/names
    const char* text[maxCount];         // memory owned/managed externally
    std::string fileName[maxCount];     // hold's the memory, but...
    const char* fileNameList[maxCount]; // downstream interface wants pointers

    ShaderCompUnit(EShLanguage stage) : stage(stage), count(0) { }

    ShaderCompUnit(const ShaderCompUnit& rhs)
    {
        stage = rhs.stage;
        count = rhs.count;
        for (int i = 0; i < count; ++i) {
            fileName[i] = rhs.fileName[i];
            text[i] = rhs.text[i];
            fileNameList[i] = rhs.fileName[i].c_str();
        }
    }

    void addString(std::string& ifileName, const char* itext)
    {
        assert(count < maxCount);
        fileName[count] = ifileName;
        text[count] = itext;
        fileNameList[count] = fileName[count].c_str();
        ++count;
    }
};

//
// For linking mode: Will independently parse each compilation unit, but then put them
// in the same program and link them together, making at most one linked module per
// pipeline stage.
//
// Uses the new C++ interface instead of the old handle-based interface.
//

void CompileAndLinkShaderUnits(std::vector<ShaderCompUnit> compUnits)
{
    // keep track of what to free
    std::list<glslang::TShader*> shaders;

    EShMessages messages = EShMsgDefault;
    SetMessageOptions(messages);

    //
    // Per-shader processing...
    //

    glslang::TProgram& program = *new glslang::TProgram;
    for (auto it = compUnits.cbegin(); it != compUnits.cend(); ++it) {
        const auto &compUnit = *it;
        glslang::TShader* shader = new glslang::TShader(compUnit.stage);
        shader->setStringsWithLengthsAndNames(compUnit.text, NULL, compUnit.fileNameList, compUnit.count);
        if (entryPointName) // HLSL todo: this needs to be tracked per compUnits
            shader->setEntryPoint(entryPointName);
        if (sourceEntryPointName)
            shader->setSourceEntryPoint(sourceEntryPointName);
        if (UserPreamble.isSet())
            shader->setPreamble(UserPreamble.get());

        shader->setShiftSamplerBinding(baseSamplerBinding[compUnit.stage]);
        shader->setShiftTextureBinding(baseTextureBinding[compUnit.stage]);
        shader->setShiftImageBinding(baseImageBinding[compUnit.stage]);
        shader->setShiftUboBinding(baseUboBinding[compUnit.stage]);
        shader->setShiftSsboBinding(baseSsboBinding[compUnit.stage]);
        shader->setShiftUavBinding(baseUavBinding[compUnit.stage]);
        shader->setFlattenUniformArrays((Options & EOptionFlattenUniformArrays) != 0);
        shader->setNoStorageFormat((Options & EOptionNoStorageFormat) != 0);
        shader->setResourceSetBinding(baseResourceSetBinding[compUnit.stage]);

        if (Options & EOptionHlslIoMapping)
            shader->setHlslIoMapping(true);

        if (Options & EOptionAutoMapBindings)
            shader->setAutoMapBindings(true);

        if (Options & EOptionAutoMapLocations)
            shader->setAutoMapLocations(true);

        shaders.push_back(shader);

        const int defaultVersion = Options & EOptionDefaultDesktop? 110: 100;

        DirStackFileIncluder includer;
        std::for_each(IncludeDirectoryList.rbegin(), IncludeDirectoryList.rend(), [&includer](const std::string& dir) {
            includer.pushExternalLocalDirectory(dir); });
        if (Options & EOptionOutputPreprocessed) {
            std::string str;
            if (shader->preprocess(&Resources, defaultVersion, ENoProfile, false, false,
                                   messages, &str, includer)) {
                PutsIfNonEmpty(str.c_str());
            } else {
                CompileFailed = true;
            }
            StderrIfNonEmpty(shader->getInfoLog());
            StderrIfNonEmpty(shader->getInfoDebugLog());
            continue;
        }
        if (! shader->parse(&Resources, defaultVersion, false, messages, includer))
            CompileFailed = true;

        program.addShader(shader);

        if (! (Options & EOptionSuppressInfolog) &&
            ! (Options & EOptionMemoryLeakMode)) {
            PutsIfNonEmpty(compUnit.fileName[0].c_str());
            PutsIfNonEmpty(shader->getInfoLog());
            PutsIfNonEmpty(shader->getInfoDebugLog());
        }
    }

    //
    // Program-level processing...
    //

    // Link
    if (! (Options & EOptionOutputPreprocessed) && ! program.link(messages))
        LinkFailed = true;

    // Map IO
    if (Options & EOptionSpv) {
        if (!program.mapIO())
            LinkFailed = true;
    }

    // Report
    if (! (Options & EOptionSuppressInfolog) &&
        ! (Options & EOptionMemoryLeakMode)) {
        PutsIfNonEmpty(program.getInfoLog());
        PutsIfNonEmpty(program.getInfoDebugLog());
    }

    // Reflect
    if (Options & EOptionDumpReflection) {
        program.buildReflection();
        program.dumpReflection();
    }

    // Dump SPIR-V
    if (Options & EOptionSpv) {
        if (CompileFailed || LinkFailed)
            printf("SPIR-V is not generated for failed compile or link\n");
        else {
            for (int stage = 0; stage < EShLangCount; ++stage) {
                if (program.getIntermediate((EShLanguage)stage)) {
                    std::vector<unsigned int> spirv;
                    std::string warningsErrors;
                    spv::SpvBuildLogger logger;
                    glslang::SpvOptions spvOptions;
                    if (Options & EOptionDebug)
                        spvOptions.generateDebugInfo = true;
                    glslang::GlslangToSpv(*program.getIntermediate((EShLanguage)stage), spirv, &logger, &spvOptions);

                    // Dump the spv to a file or stdout, etc., but only if not doing
                    // memory/perf testing, as it's not internal to programmatic use.
                    if (! (Options & EOptionMemoryLeakMode)) {
                        printf("%s", logger.getAllMessages().c_str());
                        if (Options & EOptionOutputHexadecimal) {
                            glslang::OutputSpvHex(spirv, GetBinaryName((EShLanguage)stage), variableName);
                        } else {
                            glslang::OutputSpvBin(spirv, GetBinaryName((EShLanguage)stage));
                        }
                        if (Options & EOptionHumanReadableSpv) {
                            spv::Disassemble(std::cout, spirv);
                        }
                    }
                }
            }
        }
    }

    // Free everything up, program has to go before the shaders
    // because it might have merged stuff from the shaders, and
    // the stuff from the shaders has to have its destructors called
    // before the pools holding the memory in the shaders is freed.
    delete &program;
    while (shaders.size() > 0) {
        delete shaders.back();
        shaders.pop_back();
    }
}

//
// Do file IO part of compile and link, handing off the pure
// API/programmatic mode to CompileAndLinkShaderUnits(), which can
// be put in a loop for testing memory footprint and performance.
//
// This is just for linking mode: meaning all the shaders will be put into the
// the same program linked together.
//
// This means there are a limited number of work items (not multi-threading mode)
// and that the point is testing at the linking level. Hence, to enable
// performance and memory testing, the actual compile/link can be put in
// a loop, independent of processing the work items and file IO.
//
void CompileAndLinkShaderFiles(glslang::TWorklist& Worklist)
{
    std::vector<ShaderCompUnit> compUnits;

    // Transfer all the work items from to a simple list of
    // of compilation units.  (We don't care about the thread
    // work-item distribution properties in this path, which
    // is okay due to the limited number of shaders, know since
    // they are all getting linked together.)
    glslang::TWorkItem* workItem;
    while (Worklist.remove(workItem)) {
        ShaderCompUnit compUnit(FindLanguage(workItem->name));
        char* fileText = ReadFileData(workItem->name.c_str());
        if (fileText == nullptr)
            usage();
        compUnit.addString(workItem->name, fileText);
        compUnits.push_back(compUnit);
    }

    // Actual call to programmatic processing of compile and link,
    // in a loop for testing memory and performance.  This part contains
    // all the perf/memory that a programmatic consumer will care about.
    for (int i = 0; i < ((Options & EOptionMemoryLeakMode) ? 100 : 1); ++i) {
        for (int j = 0; j < ((Options & EOptionMemoryLeakMode) ? 100 : 1); ++j)
           CompileAndLinkShaderUnits(compUnits);

        if (Options & EOptionMemoryLeakMode)
            glslang::OS_DumpMemoryCounters();
    }

    // free memory from ReadFileData, which got stored in a const char*
    // as the first string above
    for (auto it = compUnits.begin(); it != compUnits.end(); ++it)
        FreeFileData(const_cast<char*>(it->text[0]));
}

int C_DECL main(int argc, char* argv[])
{
    // array of unique places to leave the shader names and infologs for the asynchronous compiles
    std::vector<std::unique_ptr<glslang::TWorkItem>> workItems;
    ProcessArguments(workItems, argc, argv);

    glslang::TWorklist workList;
    std::for_each(workItems.begin(), workItems.end(), [&workList](std::unique_ptr<glslang::TWorkItem>& item) {
        assert(item);
        workList.add(item.get());
    });

    if (Options & EOptionDumpConfig) {
        printf("%s", glslang::GetDefaultTBuiltInResourceString().c_str());
        if (workList.empty())
            return ESuccess;
    }

    if (Options & EOptionDumpVersions) {
        printf("Glslang Version: %s %s\n", GLSLANG_REVISION, GLSLANG_DATE);
        printf("ESSL Version: %s\n", glslang::GetEsslVersionString());
        printf("GLSL Version: %s\n", glslang::GetGlslVersionString());
        std::string spirvVersion;
        glslang::GetSpirvVersion(spirvVersion);
        printf("SPIR-V Version %s\n", spirvVersion.c_str());
        printf("GLSL.std.450 Version %d, Revision %d\n", GLSLstd450Version, GLSLstd450Revision);
        printf("Khronos Tool ID %d\n", glslang::GetKhronosToolId());
        printf("GL_KHR_vulkan_glsl version %d\n", 100);
        printf("ARB_GL_gl_spirv version %d\n", 100);
        if (workList.empty())
            return ESuccess;
    }

    if (workList.empty()) {
        usage();
    }

    ProcessConfigFile();

    //
    // Two modes:
    // 1) linking all arguments together, single-threaded, new C++ interface
    // 2) independent arguments, can be tackled by multiple asynchronous threads, for testing thread safety, using the old handle interface
    //
    if (Options & EOptionLinkProgram ||
        Options & EOptionOutputPreprocessed) {
        glslang::InitializeProcess();
        CompileAndLinkShaderFiles(workList);
        glslang::FinalizeProcess();
    } else {
        ShInitialize();

        bool printShaderNames = workList.size() > 1;

        if (Options & EOptionMultiThreaded)
        {
            std::array<std::thread, 16> threads;
            for (unsigned int t = 0; t < threads.size(); ++t)
            {
                threads[t] = std::thread(CompileShaders, std::ref(workList));
                if (threads[t].get_id() == std::thread::id())
                {
                    printf("Failed to create thread\n");
                    return EFailThreadCreate;
                }
            }

            std::for_each(threads.begin(), threads.end(), [](std::thread& t) { t.join(); });
        } else
            CompileShaders(workList);

        // Print out all the resulting infologs
        for (size_t w = 0; w < workItems.size(); ++w) {
            if (workItems[w]) {
                if (printShaderNames || workItems[w]->results.size() > 0)
                    PutsIfNonEmpty(workItems[w]->name.c_str());
                PutsIfNonEmpty(workItems[w]->results.c_str());
            }
        }

        ShFinalize();
    }

    if (CompileFailed)
        return EFailCompile;
    if (LinkFailed)
        return EFailLink;

    return 0;
}

//
//   Deduce the language from the filename.  Files must end in one of the
//   following extensions:
//
//   .vert = vertex
//   .tesc = tessellation control
//   .tese = tessellation evaluation
//   .geom = geometry
//   .frag = fragment
//   .comp = compute
//
EShLanguage FindLanguage(const std::string& name, bool parseSuffix)
{
    size_t ext = 0;
    std::string suffix;

    if (shaderStageName)
        suffix = shaderStageName;
    else {
        // Search for a suffix on a filename: e.g, "myfile.frag".  If given
        // the suffix directly, we skip looking for the '.'
        if (parseSuffix) {
            ext = name.rfind('.');
            if (ext == std::string::npos) {
                usage();
                return EShLangVertex;
            }
            ++ext;
        }
        suffix = name.substr(ext, std::string::npos);
    }

    if (suffix == "vert")
        return EShLangVertex;
    else if (suffix == "tesc")
        return EShLangTessControl;
    else if (suffix == "tese")
        return EShLangTessEvaluation;
    else if (suffix == "geom")
        return EShLangGeometry;
    else if (suffix == "frag")
        return EShLangFragment;
    else if (suffix == "comp")
        return EShLangCompute;

    usage();
    return EShLangVertex;
}

//
// Read a file's data into a string, and compile it using the old interface ShCompile,
// for non-linkable results.
//
void CompileFile(const char* fileName, ShHandle compiler)
{
    int ret = 0;
    char* shaderString = ReadFileData(fileName);

    // move to length-based strings, rather than null-terminated strings
    int* lengths = new int[1];
    lengths[0] = (int)strlen(shaderString);

    EShMessages messages = EShMsgDefault;
    SetMessageOptions(messages);

    if (UserPreamble.isSet())
        Error("-D and -U options require -l (linking)\n");

    for (int i = 0; i < ((Options & EOptionMemoryLeakMode) ? 100 : 1); ++i) {
        for (int j = 0; j < ((Options & EOptionMemoryLeakMode) ? 100 : 1); ++j) {
            // ret = ShCompile(compiler, shaderStrings, NumShaderStrings, lengths, EShOptNone, &Resources, Options, (Options & EOptionDefaultDesktop) ? 110 : 100, false, messages);
            ret = ShCompile(compiler, &shaderString, 1, nullptr, EShOptNone, &Resources, Options, (Options & EOptionDefaultDesktop) ? 110 : 100, false, messages);
            // const char* multi[12] = { "# ve", "rsion", " 300 e", "s", "\n#err",
            //                         "or should be l", "ine 1", "string 5\n", "float glo", "bal",
            //                         ";\n#error should be line 2\n void main() {", "global = 2.3;}" };
            // const char* multi[7] = { "/", "/", "\\", "\n", "\n", "#", "version 300 es" };
            // ret = ShCompile(compiler, multi, 7, nullptr, EShOptNone, &Resources, Options, (Options & EOptionDefaultDesktop) ? 110 : 100, false, messages);
        }

        if (Options & EOptionMemoryLeakMode)
            glslang::OS_DumpMemoryCounters();
    }

    delete [] lengths;
    FreeFileData(shaderString);

    if (ret == 0)
        CompileFailed = true;
}

//
//   print usage to stdout
//
void usage()
{
    printf("Usage: glslangValidator [option]... [file]...\n"
           "\n"
           "'file' can end in .<stage> for auto-stage classification, where <stage> is:\n"
           "    .conf   to provide a config file that replaces the default configuration\n"
           "            (see -c option below for generating a template)\n"
           "    .vert   for a vertex shader\n"
           "    .tesc   for a tessellation control shader\n"
           "    .tese   for a tessellation evaluation shader\n"
           "    .geom   for a geometry shader\n"
           "    .frag   for a fragment shader\n"
           "    .comp   for a compute shader\n"
           "\n"
           "Options:\n"
           "  -C          cascading errors; risk crash from accumulation of error recoveries\n"
           "  -D          input is HLSL\n"
           "  -D<macro=def>\n"
           "  -D<macro>   define a pre-processor macro\n"
           "  -E          print pre-processed GLSL; cannot be used with -l;\n"
           "              errors will appear on stderr.\n"
           "  -G          create SPIR-V binary, under OpenGL semantics; turns on -l;\n"
           "              default file name is <stage>.spv (-o overrides this)\n"
           "  -H          print human readable form of SPIR-V; turns on -V\n"
           "  -I<dir>     add dir to the include search path; includer's directory\n"
           "              is searched first, followed by left-to-right order of -I\n"
           "  -S <stage>  uses specified stage rather than parsing the file extension\n"
           "              choices for <stage> are vert, tesc, tese, geom, frag, or comp\n"
           "  -U<macro>   undefine a pre-precossor macro\n"
           "  -V          create SPIR-V binary, under Vulkan semantics; turns on -l;\n"
           "              default file name is <stage>.spv (-o overrides this)\n"
           "  -c          configuration dump;\n"
           "              creates the default configuration file (redirect to a .conf file)\n"
           "  -d          default to desktop (#version 110) when there is no shader #version\n"
           "              (default is ES version 100)\n"
           "  -e          specify entry-point name\n"
           "  -g          generate debug information\n"
           "  -h          print this usage message\n"
           "  -i          intermediate tree (glslang AST) is printed out\n"
           "  -l          link all input files together to form a single module\n"
           "  -m          memory leak mode\n"
           "  -o  <file>  save binary to <file>, requires a binary option (e.g., -V)\n"
           "  -q          dump reflection query database\n"
           "  -r          relaxed semantic error-checking mode\n"
           "  -s          silent mode\n"
           "  -t          multi-threaded mode\n"
           "  -v          print version strings\n"
           "  -w          suppress warnings (except as required by #extension : warn)\n"
           "  -x          save binary output as text-based 32-bit hexadecimal numbers\n"
           "  --auto-map-bindings                  automatically bind uniform variables\n"
           "                                       without explicit bindings.\n"
           "  --amb                                synonym for --auto-map-bindings\n"
           "  --auto-map-locations                 automatically locate input/output lacking\n"
           "                                       'location'\n (fragile, not cross stage)\n"
           "  --aml                                synonym for --auto-map-locations\n"
           "  --flatten-uniform-arrays             flatten uniform texture/sampler arrays to\n"
           "                                       scalars\n"
           "  --fua                                synonym for --flatten-uniform-arrays\n"
           "\n"
           "  --hlsl-offsets                       Allow block offsets to follow HLSL rules\n"
           "                                       Works independently of source language\n"
           "  --hlsl-iomap                         Perform IO mapping in HLSL register space\n"
           "  --keep-uncalled                      don't eliminate uncalled functions\n"
           "  --ku                                 synonym for --keep-uncalled\n"
           "  --no-storage-format                  use Unknown image format\n"
           "  --nsf                                synonym for --no-storage-format\n"
           "  --resource-set-binding [stage] num   descriptor set and binding for resources\n"
           "  --rsb [stage] type set binding       synonym for --resource-set-binding\n"
           "  --shift-image-binding [stage] num    base binding number for images (uav)\n"
           "  --sib [stage] num                    synonym for --shift-image-binding\n"
           "  --shift-sampler-binding [stage] num  base binding number for samplers\n"
           "  --ssb [stage] num                    synonym for --shift-sampler-binding\n"
           "  --shift-ssbo-binding [stage] num     base binding number for SSBOs\n"
           "  --sbb [stage] num                    synonym for --shift-ssbo-binding\n"
           "  --shift-texture-binding [stage] num  base binding number for textures\n"
           "  --stb [stage] num                    synonym for --shift-texture-binding\n"
           "  --shift-uav-binding [stage] num      base binding number for UAVs\n"
           "  --suavb [stage] num                  synonym for --shift-uav-binding\n"
           "  --shift-UBO-binding [stage] num      base binding number for UBOs\n"
           "  --shift-cbuffer-binding [stage] num  synonym for --shift-UBO-binding\n"
           "  --sub [stage] num                    synonym for --shift-UBO-binding\n"
           "  --source-entrypoint name             the given shader source function is\n"
           "                                       renamed to be the entry point given in -e\n"
           "  --sep                                synonym for --source-entrypoint\n"
           "  --variable-name <name>               Creates a C header file that contains a\n"
           "                                       uint32_t array named <name>\n"
           "                                       initialized with the shader binary code.\n"
           "  --vn <name>                          synonym for --variable-name <name>\n"
           );

    exit(EFailUsage);
}

#if !defined _MSC_VER && !defined MINGW_HAS_SECURE_API

#include <errno.h>

int fopen_s(
   FILE** pFile,
   const char* filename,
   const char* mode
)
{
   if (!pFile || !filename || !mode) {
      return EINVAL;
   }

   FILE* f = fopen(filename, mode);
   if (! f) {
      if (errno != 0) {
         return errno;
      } else {
         return ENOENT;
      }
   }
   *pFile = f;

   return 0;
}

#endif

//
//   Malloc a string of sufficient size and read a string into it.
//
char* ReadFileData(const char* fileName)
{
    FILE *in = nullptr;
    int errorCode = fopen_s(&in, fileName, "r");
    if (errorCode || in == nullptr)
        Error("unable to open input file");

    int count = 0;
    while (fgetc(in) != EOF)
        count++;

    fseek(in, 0, SEEK_SET);

    char* return_data = (char*)malloc(count + 1);  // freed in FreeFileData()
    if ((int)fread(return_data, 1, count, in) != count) {
        free(return_data);
        Error("can't read input file");
    }

    return_data[count] = '\0';
    fclose(in);

    return return_data;
}

void FreeFileData(char* data)
{
    free(data);
}

void InfoLogMsg(const char* msg, const char* name, const int num)
{
    if (num >= 0 )
        printf("#### %s %s %d INFO LOG ####\n", msg, name, num);
    else
        printf("#### %s %s INFO LOG ####\n", msg, name);
}
