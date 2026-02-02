cmake_minimum_required(VERSION 3.27)

project(hypr-ungrab
    DESCRIPTION "Release pointer grab/constraints with a keybind"
    VERSION 1.0.0
)

set(CMAKE_CXX_STANDARD 23)

file(GLOB_RECURSE SRC "*.cpp")

add_library(hypr-ungrab SHARED ${SRC})

find_package(PkgConfig REQUIRED)
pkg_check_modules(deps REQUIRED IMPORTED_TARGET
    hyprland
)
target_link_libraries(hypr-ungrab PRIVATE PkgConfig::deps)

install(TARGETS hypr-ungrab)
