cmake_minimum_required(VERSION 3.27)

project(hyprland-scroll-drag
    DESCRIPTION "SUPER + middle-mouse drag pan/switch for Hyprland's scrolling layout"
    VERSION 0.1
)

set(CMAKE_CXX_STANDARD 23)

file(GLOB_RECURSE SRC "src/*.cpp")

add_library(hyprland-scroll-drag SHARED ${SRC})

find_package(PkgConfig REQUIRED)
pkg_check_modules(deps REQUIRED IMPORTED_TARGET
    hyprland
    libdrm
    libinput
    libudev
    pixman-1
    wayland-server
    xkbcommon
)
target_link_libraries(hyprland-scroll-drag PRIVATE rt PkgConfig::deps)

install(TARGETS hyprland-scroll-drag)
