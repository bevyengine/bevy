set(SOURCES
    hlslAttributes.cpp
    hlslParseHelper.cpp
    hlslScanContext.cpp
    hlslOpMap.cpp
    hlslTokenStream.cpp
    hlslGrammar.cpp
    hlslParseables.cpp)

set(HEADERS
    hlslAttributes.h
    hlslParseHelper.h
    hlslTokens.h
    hlslScanContext.h
    hlslOpMap.h
    hlslTokenStream.h
    hlslGrammar.h
    hlslParseables.h)

add_library(HLSL STATIC ${SOURCES} ${HEADERS})
set_property(TARGET HLSL PROPERTY FOLDER hlsl POSITION_INDEPENDENT_CODE ON)

if(WIN32)
    source_group("Source" FILES ${SOURCES} ${HEADERS})
endif(WIN32)

install(TARGETS HLSL
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
