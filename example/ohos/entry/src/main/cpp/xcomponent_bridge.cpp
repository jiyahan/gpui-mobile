#include <ace/xcomponent/native_interface_xcomponent.h>
#include <hilog/log.h>
#include <napi/native_api.h>
#include <window_manager/oh_display_manager.h>

#include <cstddef>
#include <cstdint>
#include <limits>
#include <mutex>

extern "C" bool gpui_ohos_register_app();
extern "C" bool gpui_ohos_surface_created(void* window, uint32_t width, uint32_t height,
                                            float scale, char* error_buffer, std::size_t error_capacity);
extern "C" void gpui_ohos_touch(uint32_t phase, int64_t device_id, int32_t native_id,
                                float x, float y);
extern "C" void gpui_ohos_surface_destroyed();
extern "C" uint32_t gpui_ohos_background_color();
extern "C" uint32_t gpui_ohos_foreground_color();
extern "C" uint32_t gpui_ohos_bottom_bar_color();

namespace {
constexpr unsigned int kLogDomain = 0xD0A0;
constexpr char kLogTag[] = "gpui-ohos";

napi_value GetBackgroundColor(napi_env env, napi_callback_info) {
    napi_value color = nullptr;
    if (napi_create_uint32(env, gpui_ohos_background_color(), &color) != napi_ok) {
        return nullptr;
    }
    return color;
}

napi_value GetForegroundColor(napi_env env, napi_callback_info) {
    napi_value color = nullptr;
    if (napi_create_uint32(env, gpui_ohos_foreground_color(), &color) != napi_ok) {
        return nullptr;
    }
    return color;
}

napi_value GetBottomBarColor(napi_env env, napi_callback_info) {
    napi_value color = nullptr;
    if (napi_create_uint32(env, gpui_ohos_bottom_bar_color(), &color) != napi_ok) {
        return nullptr;
    }
    return color;
}

void AttachSurface(OH_NativeXComponent* component, void* window) {
    uint64_t width = 0;
    uint64_t height = 0;
    if (OH_NativeXComponent_GetXComponentSize(component, window, &width, &height) !=
            OH_NATIVEXCOMPONENT_RESULT_SUCCESS ||
        width == 0 || height == 0 ||
        width > std::numeric_limits<int32_t>::max() ||
        height > std::numeric_limits<int32_t>::max()) {
        OH_LOG_Print(LOG_APP, LOG_ERROR, kLogDomain, kLogTag, "Invalid XComponent size");
        return;
    }
    char error[512] = {};
    float scale = 1.0f;
    if (OH_NativeDisplayManager_GetDefaultDisplayDensityPixels(&scale) != DISPLAY_MANAGER_OK ||
        scale <= 0.0f) {
        scale = 1.0f;
    }
    if (!gpui_ohos_surface_created(window, static_cast<uint32_t>(width),
                                   static_cast<uint32_t>(height), scale, error, sizeof(error))) {
        OH_LOG_Print(LOG_APP, LOG_ERROR, kLogDomain, kLogTag,
                     "GPUI surface creation failed: %{public}s", error);
    }
}

void OnSurfaceCreated(OH_NativeXComponent* component, void* window) {
    AttachSurface(component, window);
}

void OnSurfaceChanged(OH_NativeXComponent* component, void* window) {
    AttachSurface(component, window);
}

void OnSurfaceDestroyed(OH_NativeXComponent*, void*) {
    gpui_ohos_surface_destroyed();
}

void OnTouch(OH_NativeXComponent* component, void* window) {
    OH_NativeXComponent_TouchEvent event{};
    if (OH_NativeXComponent_GetTouchEvent(component, window, &event) ==
            OH_NATIVEXCOMPONENT_RESULT_SUCCESS) {
        gpui_ohos_touch(static_cast<uint32_t>(event.type), event.deviceId, event.id,
                        event.x, event.y);
    }
}

napi_value Init(napi_env env, napi_value exports) {
    static std::once_flag registration;
    std::call_once(registration, [] {
        if (!gpui_ohos_register_app()) {
            OH_LOG_Print(LOG_APP, LOG_ERROR, kLogDomain, kLogTag, "GPUI app registration failed");
        }
    });
    napi_property_descriptor properties[] = {
        {"getBackgroundColor", nullptr, GetBackgroundColor, nullptr, nullptr, nullptr,
         napi_default, nullptr},
        {"getForegroundColor", nullptr, GetForegroundColor, nullptr, nullptr, nullptr,
         napi_default, nullptr},
        {"getBottomBarColor", nullptr, GetBottomBarColor, nullptr, nullptr, nullptr,
         napi_default, nullptr}
    };
    if (napi_define_properties(env, exports, 3, properties) != napi_ok) {
        return exports;
    }
    napi_value nativeObject = nullptr;
    if (napi_get_named_property(env, exports, OH_NATIVE_XCOMPONENT_OBJ, &nativeObject) != napi_ok) {
        return exports;
    }
    OH_NativeXComponent* component = nullptr;
    if (napi_unwrap(env, nativeObject, reinterpret_cast<void**>(&component)) != napi_ok ||
        component == nullptr) {
        return exports;
    }
    static OH_NativeXComponent_Callback callbacks = {
        OnSurfaceCreated, OnSurfaceChanged, OnSurfaceDestroyed, OnTouch
    };
    if (OH_NativeXComponent_RegisterCallback(component, &callbacks) !=
        OH_NATIVEXCOMPONENT_RESULT_SUCCESS) {
        OH_LOG_Print(LOG_APP, LOG_ERROR, kLogDomain, kLogTag, "XComponent callback registration failed");
    }
    return exports;
}

napi_module module = { 1, 0, nullptr, Init, "entry", nullptr, {nullptr, nullptr, nullptr, nullptr} };
}

extern "C" __attribute__((constructor)) void RegisterModule() {
    napi_module_register(&module);
}
