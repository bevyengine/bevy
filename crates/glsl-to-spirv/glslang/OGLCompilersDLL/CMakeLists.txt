set(SOURCES InitializeDll.cpp InitializeDll.h)

add_library(OGLCompiler STATIC ${SOURCES})
set_property(TARGET OGLCompiler PROPERTY FOLDER glslang POSITION_INDEPENDENT_CODE ON)

if(WIN32)
    source_group("Source" FILES ${SOURCES})
endif(WIN32)

install(TARGETS OGLCompiler
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
