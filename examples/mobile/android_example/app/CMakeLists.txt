# This is part of a hack to convince gradle to insert libc++_shared.so
cmake_minimum_required(VERSION 3.4.1)
project(cppshared_dummy)
set (SRC_DIR ./src/main/cpp)
add_library (dummy SHARED ${SRC_DIR}/dummy.cpp)
