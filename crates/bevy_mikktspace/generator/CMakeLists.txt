cmake_minimum_required(VERSION 3.7...3.26)

if(${CMAKE_VERSION} VERSION_LESS 3.12)
    cmake_policy(VERSION ${CMAKE_MAJOR_VERSION}.${CMAKE_MINOR_VERSION})
endif()

project(
    MtGenerator VERSION 1.0
    DESCRIPTION "Mikktspace Test Data Generator"
    LANGUAGES C)

add_executable(mtg_generator src/generator.c src/mikktspace.c src/obj_loader.c)
set_property(TARGET mtg_generator PROPERTY C_STANDARD 99)

# Because in C, math is optional! Yes, your high school teacher lied to you!
target_link_libraries(mtg_generator m)
