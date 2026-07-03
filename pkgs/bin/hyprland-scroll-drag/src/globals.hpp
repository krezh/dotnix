#pragma once

#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/config/values/types/FloatValue.hpp>
#include <hyprland/src/config/values/types/IntValue.hpp>

inline HANDLE PHANDLE = nullptr;

struct SGlobalState {
    struct {
        SP<Config::Values::CFloatValue> sensitivity;
        SP<Config::Values::CIntValue>   deadzone;
        SP<Config::Values::CIntValue>   workspaceSwitchThreshold;
    } config;
};

inline UP<SGlobalState> g_pGlobalState;
