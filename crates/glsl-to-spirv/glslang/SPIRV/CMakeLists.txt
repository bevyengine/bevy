set(SOURCES
    GlslangToSpv.cpp
    InReadableOrder.cpp
    Logger.cpp
    SpvBuilder.cpp
    doc.cpp
    disassemble.cpp)

set(SPVREMAP_SOURCES
    SPVRemapper.cpp
    doc.cpp)

set(HEADERS
    bitutils.h
    spirv.hpp
    GLSL.std.450.h
    GLSL.ext.KHR.h
    GlslangToSpv.h
    hex_float.h
    Logger.h
    SpvBuilder.h
    spvIR.h
    doc.h
    disassemble.h)

set(SPVREMAP_HEADERS
    SPVRemapper.h
    doc.h)

if(ENABLE_AMD_EXTENSIONS)
    list(APPEND
         HEADERS
         GLSL.ext.AMD.h)
endif(ENABLE_AMD_EXTENSIONS)

if(ENABLE_NV_EXTENSIONS)
    list(APPEND
         HEADERS
         GLSL.ext.NV.h)
endif(ENABLE_NV_EXTENSIONS)

add_library(SPIRV STATIC ${SOURCES} ${HEADERS})
set_property(TARGET SPIRV PROPERTY FOLDER glslang POSITION_INDEPENDENT_CODE ON)
target_link_libraries(SPIRV glslang)

add_library(SPVRemapper STATIC ${SPVREMAP_SOURCES} ${SPVREMAP_HEADERS})
set_property(TARGET SPVRemapper PROPERTY FOLDER glslang POSITION_INDEPENDENT_CODE ON)

if(WIN32)
    source_group("Source" FILES ${SOURCES} ${HEADERS})
    source_group("Source" FILES ${SPVREMAP_SOURCES} ${SPVREMAP_HEADERS})
endif(WIN32)

install(TARGETS SPIRV SPVRemapper
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})

install(FILES ${HEADERS} ${SPVREMAP_HEADERS} DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/SPIRV/)
