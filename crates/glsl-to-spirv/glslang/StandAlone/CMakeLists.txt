add_library(glslang-default-resource-limits
            ${CMAKE_CURRENT_SOURCE_DIR}/ResourceLimits.cpp)
set_property(TARGET glslang-default-resource-limits PROPERTY FOLDER glslang POSITION_INDEPENDENT_CODE ON)

target_include_directories(glslang-default-resource-limits
                           PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}
                           PUBLIC ${PROJECT_SOURCE_DIR})

set(SOURCES StandAlone.cpp DirStackFileIncluder.h)
set(REMAPPER_SOURCES spirv-remap.cpp)

add_executable(glslangValidator ${SOURCES})
add_executable(spirv-remap ${REMAPPER_SOURCES})
set_property(TARGET glslangValidator PROPERTY FOLDER tools)
set_property(TARGET spirv-remap PROPERTY FOLDER tools)
glslang_set_link_args(glslangValidator)
glslang_set_link_args(spirv-remap)

set(LIBRARIES
    glslang
    SPIRV
    SPVRemapper
    glslang-default-resource-limits)

if(WIN32)
    set(LIBRARIES ${LIBRARIES} psapi)
elseif(UNIX)
    if(NOT ANDROID)
        set(LIBRARIES ${LIBRARIES} pthread)
    endif()
endif(WIN32)

target_link_libraries(glslangValidator ${LIBRARIES})
target_link_libraries(spirv-remap ${LIBRARIES})

if(WIN32)
    source_group("Source" FILES ${SOURCES})
endif(WIN32)

install(TARGETS glslangValidator
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})

install(TARGETS spirv-remap
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
