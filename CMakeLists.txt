cmake_minimum_required(VERSION 3.24)
project(Newton VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# 6.9 is the floor, not a preference: earlier Qt links the AGL framework that
# Xcode 26 removed, so macOS builds fail at link time (QTBUG-137687).
find_package(Qt6 6.9 REQUIRED COMPONENTS Core Gui Qml Quick Test)
qt_standard_project_setup(REQUIRES 6.9)

if(MSVC)
    add_compile_options(/W4 /permissive-)
else()
    add_compile_options(-Wall -Wextra -Wpedantic)
endif()

enable_testing()
add_subdirectory(src)
add_subdirectory(tests)
