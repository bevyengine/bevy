//
// Copyright (C) 2016 Google, Inc.
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
//    Neither the name of Google Inc. nor the names of its
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

#ifndef GLSLANG_GTESTS_TEST_FIXTURE_H
#define GLSLANG_GTESTS_TEST_FIXTURE_H

#include <cstdint>
#include <fstream>
#include <sstream>
#include <streambuf>
#include <tuple>
#include <string>

#include <gtest/gtest.h>

#include "SPIRV/GlslangToSpv.h"
#include "SPIRV/disassemble.h"
#include "SPIRV/doc.h"
#include "SPIRV/SPVRemapper.h"
#include "StandAlone/ResourceLimits.h"
#include "glslang/Public/ShaderLang.h"

#include "Initializer.h"
#include "Settings.h"

namespace glslangtest {

// This function is used to provide custom test name suffixes based on the
// shader source file names. Otherwise, the test name suffixes will just be
// numbers, which are not quite obvious.
std::string FileNameAsCustomTestSuffix(
    const ::testing::TestParamInfo<std::string>& info);

enum class Source {
  GLSL,
  HLSL,
};

// Enum for shader compilation semantics.
enum class Semantics {
    OpenGL,
    Vulkan,
};

// Enum for compilation target.
enum class Target {
    AST,
    Spv,
    BothASTAndSpv,
};

EShLanguage GetShaderStage(const std::string& stage);

EShMessages DeriveOptions(Source, Semantics, Target);

// Reads the content of the file at the given |path|. On success, returns true
// and the contents; otherwise, returns false and an empty string.
std::pair<bool, std::string> ReadFile(const std::string& path);
std::pair<bool, std::vector<std::uint32_t> > ReadSpvBinaryFile(const std::string& path);

// Writes the given |contents| into the file at the given |path|. Returns true
// on successful output.
bool WriteFile(const std::string& path, const std::string& contents);

// Returns the suffix of the given |name|.
std::string GetSuffix(const std::string& name);

// Base class for glslang integration tests. It contains many handy utility-like
// methods such as reading shader source files, compiling into AST/SPIR-V, and
// comparing with expected outputs.
//
// To write value-Parameterized tests:
//   using ValueParamTest = GlslangTest<::testing::TestWithParam<std::string>>;
// To use as normal fixture:
//   using FixtureTest = GlslangTest<::testing::Test>;
template <typename GT>
class GlslangTest : public GT {
public:
    GlslangTest()
        : defaultVersion(100),
          defaultProfile(ENoProfile),
          forceVersionProfile(false),
          isForwardCompatible(false) {}

    // Tries to load the contents from the file at the given |path|. On success,
    // writes the contents into |contents|. On failure, errors out.
    void tryLoadFile(const std::string& path, const std::string& tag,
                     std::string* contents)
    {
        bool fileReadOk;
        std::tie(fileReadOk, *contents) = ReadFile(path);
        ASSERT_TRUE(fileReadOk) << "Cannot open " << tag << " file: " << path;
    }

    // Tries to load the contents from the file at the given |path|. On success,
    // writes the contents into |contents|. On failure, errors out.
    void tryLoadSpvFile(const std::string& path, const std::string& tag,
                        std::vector<uint32_t>& contents)
    {
        bool fileReadOk;
        std::tie(fileReadOk, contents) = ReadSpvBinaryFile(path);
        ASSERT_TRUE(fileReadOk) << "Cannot open " << tag << " file: " << path;
    }

    // Checks the equality of |expected| and |real|. If they are not equal,
    // write |real| to the given file named as |fname| if update mode is on.
    void checkEqAndUpdateIfRequested(const std::string& expected,
                                     const std::string& real,
                                     const std::string& fname)
    {
        // In order to output the message we want under proper circumstances,
        // we need the following operator<< stuff.
        EXPECT_EQ(expected, real)
            << (GlobalTestSettings.updateMode
                    ? ("Mismatch found and update mode turned on - "
                       "flushing expected result output.")
                    : "");

        // Update the expected output file if requested.
        // It looks weird to duplicate the comparison between expected_output
        // and stream.str(). However, if creating a variable for the comparison
        // result, we cannot have pretty print of the string diff in the above.
        if (GlobalTestSettings.updateMode && expected != real) {
            EXPECT_TRUE(WriteFile(fname, real)) << "Flushing failed";
        }
    }

    struct ShaderResult {
        std::string shaderName;
        std::string output;
        std::string error;
    };

    // A struct for holding all the information returned by glslang compilation
    // and linking.
    struct GlslangResult {
        std::vector<ShaderResult> shaderResults;
        std::string linkingOutput;
        std::string linkingError;
        std::string spirvWarningsErrors;
        std::string spirv;  // Optional SPIR-V disassembly text.
    };

    // Compiles and the given source |code| of the given shader |stage| into
    // the target under the semantics conveyed via |controls|. Returns true
    // and modifies |shader| on success.
    bool compile(glslang::TShader* shader, const std::string& code,
                 const std::string& entryPointName, EShMessages controls,
                 const TBuiltInResource* resources=nullptr)
    {
        const char* shaderStrings = code.data();
        const int shaderLengths = static_cast<int>(code.size());

        shader->setStringsWithLengths(&shaderStrings, &shaderLengths, 1);
        if (!entryPointName.empty()) shader->setEntryPoint(entryPointName.c_str());
        return shader->parse(
                (resources ? resources : &glslang::DefaultTBuiltInResource),
                defaultVersion, isForwardCompatible, controls);
    }

    // Compiles and links the given source |code| of the given shader
    // |stage| into the target under the semantics specified via |controls|.
    // Returns a GlslangResult instance containing all the information generated
    // during the process. If the target includes SPIR-V, also disassembles
    // the result and returns disassembly text.
    GlslangResult compileAndLink(
            const std::string shaderName, const std::string& code,
            const std::string& entryPointName, EShMessages controls,
            bool flattenUniformArrays = false,
            EShTextureSamplerTransformMode texSampTransMode = EShTexSampTransKeep)
    {
        const EShLanguage kind = GetShaderStage(GetSuffix(shaderName));

        glslang::TShader shader(kind);
        shader.setAutoMapLocations(true);
        shader.setTextureSamplerTransformMode(texSampTransMode);
        shader.setFlattenUniformArrays(flattenUniformArrays);

        bool success = compile(&shader, code, entryPointName, controls);

        glslang::TProgram program;
        program.addShader(&shader);
        success &= program.link(controls);

        spv::SpvBuildLogger logger;

        if (success && (controls & EShMsgSpvRules)) {
            std::vector<uint32_t> spirv_binary;
            glslang::GlslangToSpv(*program.getIntermediate(kind),
                                  spirv_binary, &logger);

            std::ostringstream disassembly_stream;
            spv::Parameterize();
            spv::Disassemble(disassembly_stream, spirv_binary);
            return {{{shaderName, shader.getInfoLog(), shader.getInfoDebugLog()},},
                    program.getInfoLog(), program.getInfoDebugLog(),
                    logger.getAllMessages(), disassembly_stream.str()};
        } else {
            return {{{shaderName, shader.getInfoLog(), shader.getInfoDebugLog()},},
                    program.getInfoLog(), program.getInfoDebugLog(), "", ""};
        }
    }

    // Compiles and links the given source |code| of the given shader
    // |stage| into the target under the semantics specified via |controls|.
    // Returns a GlslangResult instance containing all the information generated
    // during the process. If the target includes SPIR-V, also disassembles
    // the result and returns disassembly text.
    GlslangResult compileLinkIoMap(
            const std::string shaderName, const std::string& code,
            const std::string& entryPointName, EShMessages controls,
            int baseSamplerBinding,
            int baseTextureBinding,
            int baseImageBinding,
            int baseUboBinding,
            int baseSsboBinding,
            bool autoMapBindings,
            bool flattenUniformArrays)
    {
        const EShLanguage kind = GetShaderStage(GetSuffix(shaderName));

        glslang::TShader shader(kind);
        shader.setShiftSamplerBinding(baseSamplerBinding);
        shader.setShiftTextureBinding(baseTextureBinding);
        shader.setShiftImageBinding(baseImageBinding);
        shader.setShiftUboBinding(baseUboBinding);
        shader.setShiftSsboBinding(baseSsboBinding);
        shader.setAutoMapBindings(autoMapBindings);
        shader.setAutoMapLocations(true);
        shader.setFlattenUniformArrays(flattenUniformArrays);

        bool success = compile(&shader, code, entryPointName, controls);

        glslang::TProgram program;
        program.addShader(&shader);
        
        success &= program.link(controls);
        success &= program.mapIO();

        spv::SpvBuildLogger logger;

        if (success && (controls & EShMsgSpvRules)) {
            std::vector<uint32_t> spirv_binary;
            glslang::GlslangToSpv(*program.getIntermediate(kind),
                                  spirv_binary, &logger);

            std::ostringstream disassembly_stream;
            spv::Parameterize();
            spv::Disassemble(disassembly_stream, spirv_binary);
            return {{{shaderName, shader.getInfoLog(), shader.getInfoDebugLog()},},
                    program.getInfoLog(), program.getInfoDebugLog(),
                    logger.getAllMessages(), disassembly_stream.str()};
        } else {
            return {{{shaderName, shader.getInfoLog(), shader.getInfoDebugLog()},},
                    program.getInfoLog(), program.getInfoDebugLog(), "", ""};
        }
    }

    // This is like compileAndLink but with remapping of the SPV binary
    // through spirvbin_t::remap().  While technically this could be merged
    // with compileAndLink() above (with the remap step optionally being a no-op)
    // it is given separately here for ease of future extraction.
    GlslangResult compileLinkRemap(
            const std::string shaderName, const std::string& code,
            const std::string& entryPointName, EShMessages controls,
            const unsigned int remapOptions = spv::spirvbin_t::NONE)
    {
        const EShLanguage kind = GetShaderStage(GetSuffix(shaderName));

        glslang::TShader shader(kind);
        shader.setAutoMapLocations(true);

        bool success = compile(&shader, code, entryPointName, controls);

        glslang::TProgram program;
        program.addShader(&shader);
        success &= program.link(controls);

        spv::SpvBuildLogger logger;

        if (success && (controls & EShMsgSpvRules)) {
            std::vector<uint32_t> spirv_binary;
            glslang::GlslangToSpv(*program.getIntermediate(kind),
                                  spirv_binary, &logger);

            spv::spirvbin_t(0 /*verbosity*/).remap(spirv_binary, remapOptions);

            std::ostringstream disassembly_stream;
            spv::Parameterize();
            spv::Disassemble(disassembly_stream, spirv_binary);
            return {{{shaderName, shader.getInfoLog(), shader.getInfoDebugLog()},},
                    program.getInfoLog(), program.getInfoDebugLog(),
                    logger.getAllMessages(), disassembly_stream.str()};
        } else {
            return {{{shaderName, shader.getInfoLog(), shader.getInfoDebugLog()},},
                    program.getInfoLog(), program.getInfoDebugLog(), "", ""};
        }
    }

    // remap the binary in 'code' with the options in remapOptions
    GlslangResult remap(
            const std::string shaderName, const std::vector<uint32_t>& code,
            EShMessages controls,
            const unsigned int remapOptions = spv::spirvbin_t::NONE)
    {
        if ((controls & EShMsgSpvRules)) {
            std::vector<uint32_t> spirv_binary(code); // scratch copy

            spv::spirvbin_t(0 /*verbosity*/).remap(spirv_binary, remapOptions);
            
            std::ostringstream disassembly_stream;
            spv::Parameterize();
            spv::Disassemble(disassembly_stream, spirv_binary);

            return {{{shaderName, "", ""},},
                    "", "",
                        "", disassembly_stream.str()};
        } else {
            return {{{shaderName, "", ""},}, "", "", "", ""};
        }
    }

    void outputResultToStream(std::ostringstream* stream,
                              const GlslangResult& result,
                              EShMessages controls)
    {
        const auto outputIfNotEmpty = [&stream](const std::string& str) {
            if (!str.empty()) *stream << str << "\n";
        };

        for (const auto& shaderResult : result.shaderResults) {
            *stream << shaderResult.shaderName << "\n";
            outputIfNotEmpty(shaderResult.output);
            outputIfNotEmpty(shaderResult.error);
        }
        outputIfNotEmpty(result.linkingOutput);
        outputIfNotEmpty(result.linkingError);
        *stream << result.spirvWarningsErrors;

        if (controls & EShMsgSpvRules) {
            *stream
                << (result.spirv.empty()
                        ? "SPIR-V is not generated for failed compile or link\n"
                        : result.spirv);
        }
    }

    void loadFileCompileAndCheck(const std::string& testDir,
                                 const std::string& testName,
                                 Source source,
                                 Semantics semantics,
                                 Target target,
                                 const std::string& entryPointName="")
    {
        const std::string inputFname = testDir + "/" + testName;
        const std::string expectedOutputFname =
            testDir + "/baseResults/" + testName + ".out";
        std::string input, expectedOutput;

        tryLoadFile(inputFname, "input", &input);
        tryLoadFile(expectedOutputFname, "expected output", &expectedOutput);

        const EShMessages controls = DeriveOptions(source, semantics, target);
        GlslangResult result = compileAndLink(testName, input, entryPointName, controls);

        // Generate the hybrid output in the way of glslangValidator.
        std::ostringstream stream;
        outputResultToStream(&stream, result, controls);

        checkEqAndUpdateIfRequested(expectedOutput, stream.str(),
                                    expectedOutputFname);
    }

    void loadFileCompileFlattenUniformsAndCheck(const std::string& testDir,
                                                const std::string& testName,
                                                Source source,
                                                Semantics semantics,
                                                Target target,
                                                const std::string& entryPointName="")
    {
        const std::string inputFname = testDir + "/" + testName;
        const std::string expectedOutputFname =
            testDir + "/baseResults/" + testName + ".out";
        std::string input, expectedOutput;

        tryLoadFile(inputFname, "input", &input);
        tryLoadFile(expectedOutputFname, "expected output", &expectedOutput);

        const EShMessages controls = DeriveOptions(source, semantics, target);
        GlslangResult result = compileAndLink(testName, input, entryPointName, controls, true);

        // Generate the hybrid output in the way of glslangValidator.
        std::ostringstream stream;
        outputResultToStream(&stream, result, controls);

        checkEqAndUpdateIfRequested(expectedOutput, stream.str(),
                                    expectedOutputFname);
    }

    void loadFileCompileIoMapAndCheck(const std::string& testDir,
                                      const std::string& testName,
                                      Source source,
                                      Semantics semantics,
                                      Target target,
                                      const std::string& entryPointName,
                                      int baseSamplerBinding,
                                      int baseTextureBinding,
                                      int baseImageBinding,
                                      int baseUboBinding,
                                      int baseSsboBinding,
                                      bool autoMapBindings,
                                      bool flattenUniformArrays)
    {
        const std::string inputFname = testDir + "/" + testName;
        const std::string expectedOutputFname =
            testDir + "/baseResults/" + testName + ".out";
        std::string input, expectedOutput;

        tryLoadFile(inputFname, "input", &input);
        tryLoadFile(expectedOutputFname, "expected output", &expectedOutput);

        const EShMessages controls = DeriveOptions(source, semantics, target);
        GlslangResult result = compileLinkIoMap(testName, input, entryPointName, controls,
                                                baseSamplerBinding, baseTextureBinding, baseImageBinding,
                                                baseUboBinding, baseSsboBinding,
                                                autoMapBindings,
                                                flattenUniformArrays);

        // Generate the hybrid output in the way of glslangValidator.
        std::ostringstream stream;
        outputResultToStream(&stream, result, controls);

        checkEqAndUpdateIfRequested(expectedOutput, stream.str(),
                                    expectedOutputFname);
    }

    void loadFileCompileRemapAndCheck(const std::string& testDir,
                                      const std::string& testName,
                                      Source source,
                                      Semantics semantics,
                                      Target target,
                                      const std::string& entryPointName="",
                                      const unsigned int remapOptions = spv::spirvbin_t::NONE)
    {
        const std::string inputFname = testDir + "/" + testName;
        const std::string expectedOutputFname =
            testDir + "/baseResults/" + testName + ".out";
        std::string input, expectedOutput;

        tryLoadFile(inputFname, "input", &input);
        tryLoadFile(expectedOutputFname, "expected output", &expectedOutput);

        const EShMessages controls = DeriveOptions(source, semantics, target);
        GlslangResult result = compileLinkRemap(testName, input, entryPointName, controls, remapOptions);

        // Generate the hybrid output in the way of glslangValidator.
        std::ostringstream stream;
        outputResultToStream(&stream, result, controls);

        checkEqAndUpdateIfRequested(expectedOutput, stream.str(),
                                    expectedOutputFname);
    }

    void loadFileRemapAndCheck(const std::string& testDir,
                               const std::string& testName,
                               Source source,
                               Semantics semantics,
                               Target target,
                               const unsigned int remapOptions = spv::spirvbin_t::NONE)
    {
        const std::string inputFname = testDir + "/" + testName;
        const std::string expectedOutputFname =
            testDir + "/baseResults/" + testName + ".out";
        std::vector<std::uint32_t> input;
        std::string expectedOutput;

        tryLoadSpvFile(inputFname, "input", input);
        tryLoadFile(expectedOutputFname, "expected output", &expectedOutput);

        const EShMessages controls = DeriveOptions(source, semantics, target);
        GlslangResult result = remap(testName, input, controls, remapOptions);

        // Generate the hybrid output in the way of glslangValidator.
        std::ostringstream stream;
        outputResultToStream(&stream, result, controls);

        checkEqAndUpdateIfRequested(expectedOutput, stream.str(),
                                    expectedOutputFname);
    }

    // Preprocesses the given |source| code. On success, returns true, the
    // preprocessed shader, and warning messages. Otherwise, returns false, an
    // empty string, and error messages.
    std::tuple<bool, std::string, std::string> preprocess(
        const std::string& source)
    {
        const char* shaderStrings = source.data();
        const int shaderLengths = static_cast<int>(source.size());

        glslang::TShader shader(EShLangVertex);
        shader.setStringsWithLengths(&shaderStrings, &shaderLengths, 1);
        std::string ppShader;
        glslang::TShader::ForbidIncluder includer;
        const bool success = shader.preprocess(
            &glslang::DefaultTBuiltInResource, defaultVersion, defaultProfile,
            forceVersionProfile, isForwardCompatible, (EShMessages)(EShMsgOnlyPreprocessor | EShMsgCascadingErrors),
            &ppShader, includer);

        std::string log = shader.getInfoLog();
        log += shader.getInfoDebugLog();
        if (success) {
            return std::make_tuple(true, ppShader, log);
        } else {
            return std::make_tuple(false, "", log);
        }
    }

    void loadFilePreprocessAndCheck(const std::string& testDir,
                                    const std::string& testName)
    {
        const std::string inputFname = testDir + "/" + testName;
        const std::string expectedOutputFname =
            testDir + "/baseResults/" + testName + ".out";
        const std::string expectedErrorFname =
            testDir + "/baseResults/" + testName + ".err";
        std::string input, expectedOutput, expectedError;

        tryLoadFile(inputFname, "input", &input);
        tryLoadFile(expectedOutputFname, "expected output", &expectedOutput);
        tryLoadFile(expectedErrorFname, "expected error", &expectedError);

        bool ppOk;
        std::string output, error;
        std::tie(ppOk, output, error) = preprocess(input);
        if (!output.empty()) output += '\n';
        if (!error.empty()) error += '\n';

        checkEqAndUpdateIfRequested(expectedOutput, output,
                                    expectedOutputFname);
        checkEqAndUpdateIfRequested(expectedError, error,
                                    expectedErrorFname);
    }

    void loadCompileUpgradeTextureToSampledTextureAndDropSamplersAndCheck(const std::string& testDir,
                                                                          const std::string& testName,
                                                                          Source source,
                                                                          Semantics semantics,
                                                                          Target target,
                                                                          const std::string& entryPointName = "")
    {
        const std::string inputFname = testDir + "/" + testName;
        const std::string expectedOutputFname = testDir + "/baseResults/" + testName + ".out";
        std::string input, expectedOutput;

        tryLoadFile(inputFname, "input", &input);
        tryLoadFile(expectedOutputFname, "expected output", &expectedOutput);

        const EShMessages controls = DeriveOptions(source, semantics, target);
        GlslangResult result = compileAndLink(testName, input, entryPointName, controls, false, EShTexSampTransUpgradeTextureRemoveSampler);

        // Generate the hybrid output in the way of glslangValidator.
        std::ostringstream stream;
        outputResultToStream(&stream, result, controls);

        checkEqAndUpdateIfRequested(expectedOutput, stream.str(),
                                    expectedOutputFname);
    }

private:
    const int defaultVersion;
    const EProfile defaultProfile;
    const bool forceVersionProfile;
    const bool isForwardCompatible;
};

}  // namespace glslangtest

#endif  // GLSLANG_GTESTS_TEST_FIXTURE_H
