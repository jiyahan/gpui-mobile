#include <ace/xcomponent/native_interface_xcomponent.h>
#include <hilog/log.h>
#include <napi/native_api.h>
#include <window_manager/oh_display_manager.h>

#include <cmath>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <memory>
#include <mutex>

extern "C" bool gpui_ohos_register_app();
extern "C" bool gpui_ohos_surface_created(void* window, uint32_t width, uint32_t height,
                                            float scale, char* error_buffer, std::size_t error_capacity);
extern "C" void gpui_ohos_touch(uint32_t phase, int64_t device_id, int32_t native_id,
                                float x, float y);
extern "C" bool gpui_ohos_back();
extern "C" void gpui_ohos_surface_destroyed();
extern "C" void gpui_ohos_set_foreground(bool foreground);
extern "C" void gpui_ohos_set_safe_area(float top, float bottom, float left, float right, float scale);
extern "C" uint32_t gpui_ohos_background_color();
extern "C" uint32_t gpui_ohos_foreground_color();
extern "C" uint32_t gpui_ohos_bottom_bar_color();

namespace {
constexpr unsigned int kLogDomain = 0xD0A0;
constexpr char kLogTag[] = "gpui-ohos";

struct ChromeStyle {
    bool hasStatusColor;
    uint32_t statusColor;
    bool hasNavigationColor;
    uint32_t navigationColor;
    bool lightContent;
};

std::mutex chromeMutex;
napi_threadsafe_function chromeCallback = nullptr;
ChromeStyle lastChrome{};
bool hasChrome = false;

void CallChrome(napi_env env, napi_value callback, void*, void* data) {
    std::unique_ptr<ChromeStyle> style(static_cast<ChromeStyle*>(data));
    if (env == nullptr || callback == nullptr) {
        return;
    }
    napi_value args[3] = {};
    if ((style->hasStatusColor ? napi_create_uint32(env, style->statusColor, &args[0])
                               : napi_get_null(env, &args[0])) != napi_ok ||
        (style->hasNavigationColor ? napi_create_uint32(env, style->navigationColor, &args[1])
                                   : napi_get_null(env, &args[1])) != napi_ok ||
        napi_get_boolean(env, style->lightContent, &args[2]) != napi_ok) {
        return;
    }
    napi_value receiver = nullptr;
    napi_get_undefined(env, &receiver);
    if (napi_call_function(env, receiver, callback, 3, args, nullptr) != napi_ok) {
        OH_LOG_Print(LOG_APP, LOG_ERROR, kLogDomain, kLogTag, "System chrome callback failed");
    }
}

void QueueChromeLocked(const ChromeStyle& style) {
    if (chromeCallback == nullptr) {
        return;
    }
    auto* pending = new ChromeStyle(style);
    if (napi_call_threadsafe_function(chromeCallback, pending, napi_tsfn_nonblocking) != napi_ok) {
        delete pending;
        OH_LOG_Print(LOG_APP, LOG_ERROR, kLogDomain, kLogTag, "Could not queue system chrome");
    }
}

float DisplayScale() {
    float scale = 1.0f;
    if (OH_NativeDisplayManager_GetDefaultDisplayDensityPixels(&scale) != DISPLAY_MANAGER_OK ||
        !std::isfinite(scale) || scale <= 0.0f) {
        return 1.0f;
    }
    return scale;
}

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

napi_value SetForeground(napi_env env, napi_callback_info info) {
    std::size_t argc = 1;
    napi_value args[1] = {};
    bool foreground = false;
    if (napi_get_cb_info(env, info, &argc, args, nullptr, nullptr) != napi_ok ||
        argc != 1 || napi_get_value_bool(env, args[0], &foreground) != napi_ok) {
        napi_throw_type_error(env, nullptr, "setForeground expects a boolean");
        return nullptr;
    }
    gpui_ohos_set_foreground(foreground);
    OH_LOG_Print(LOG_APP, LOG_INFO, kLogDomain, kLogTag,
                 "UIAbility %{public}s", foreground ? "foreground" : "background");
    napi_value result = nullptr;
    return napi_get_undefined(env, &result) == napi_ok ? result : nullptr;
}

napi_value DispatchBack(napi_env env, napi_callback_info) {
    napi_value handled = nullptr;
    if (napi_get_boolean(env, gpui_ohos_back(), &handled) != napi_ok) {
        return nullptr;
    }
    return handled;
}

napi_value SetSafeArea(napi_env env, napi_callback_info info) {
    std::size_t argc = 4;
    napi_value args[4] = {};
    double insets[4] = {};
    if (napi_get_cb_info(env, info, &argc, args, nullptr, nullptr) != napi_ok || argc != 4) {
        napi_throw_type_error(env, nullptr, "setSafeArea expects four pixel values");
        return nullptr;
    }
    for (int i = 0; i < 4; ++i) {
        if (napi_get_value_double(env, args[i], &insets[i]) != napi_ok ||
            !std::isfinite(insets[i]) || insets[i] < 0.0) {
            napi_throw_type_error(env, nullptr, "safe area values must be nonnegative numbers");
            return nullptr;
        }
    }
    gpui_ohos_set_safe_area(static_cast<float>(insets[0]), static_cast<float>(insets[1]),
                            static_cast<float>(insets[2]), static_cast<float>(insets[3]), DisplayScale());
    OH_LOG_Print(LOG_APP, LOG_INFO, kLogDomain, kLogTag,
                 "Safe area px top=%{public}d bottom=%{public}d left=%{public}d right=%{public}d",
                 static_cast<int>(insets[0]), static_cast<int>(insets[1]),
                 static_cast<int>(insets[2]), static_cast<int>(insets[3]));
    napi_value result = nullptr;
    return napi_get_undefined(env, &result) == napi_ok ? result : nullptr;
}

napi_value SetChromeCallback(napi_env env, napi_callback_info info) {
    std::size_t argc = 1;
    napi_value args[1] = {};
    napi_valuetype type = napi_undefined;
    if (napi_get_cb_info(env, info, &argc, args, nullptr, nullptr) != napi_ok ||
        argc != 1 || napi_typeof(env, args[0], &type) != napi_ok ||
        (type != napi_function && type != napi_null)) {
        napi_throw_type_error(env, nullptr, "setChromeCallback expects a function or null");
        return nullptr;
    }
    napi_threadsafe_function next = nullptr;
    if (type == napi_function) {
        napi_value name = nullptr;
        if (napi_create_string_utf8(env, "gpui-system-chrome", NAPI_AUTO_LENGTH, &name) != napi_ok ||
            napi_create_threadsafe_function(env, args[0], nullptr, name, 0, 1,
                                            nullptr, nullptr, nullptr, CallChrome, &next) != napi_ok) {
            napi_throw_error(env, nullptr, "Could not register system chrome callback");
            return nullptr;
        }
        napi_unref_threadsafe_function(env, next);
    }
    {
        std::lock_guard<std::mutex> lock(chromeMutex);
        if (chromeCallback != nullptr) {
            napi_release_threadsafe_function(chromeCallback, napi_tsfn_abort);
        }
        chromeCallback = next;
        if (hasChrome) {
            QueueChromeLocked(lastChrome);
        }
    }
    napi_value result = nullptr;
    return napi_get_undefined(env, &result) == napi_ok ? result : nullptr;
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
    float scale = DisplayScale();
    if (!gpui_ohos_surface_created(window, static_cast<uint32_t>(width),
                                   static_cast<uint32_t>(height), scale, error, sizeof(error))) {
        OH_LOG_Print(LOG_APP, LOG_ERROR, kLogDomain, kLogTag,
                     "GPUI surface creation failed: %{public}s", error);
    }
}

void OnSurfaceCreated(OH_NativeXComponent* component, void* window) {
    OH_LOG_Print(LOG_APP, LOG_INFO, kLogDomain, kLogTag, "XComponent surface created");
    AttachSurface(component, window);
}

void OnSurfaceChanged(OH_NativeXComponent* component, void* window) {
    AttachSurface(component, window);
}

void OnSurfaceDestroyed(OH_NativeXComponent*, void*) {
    gpui_ohos_surface_destroyed();
    OH_LOG_Print(LOG_APP, LOG_INFO, kLogDomain, kLogTag, "XComponent surface destroyed");
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
         napi_default, nullptr},
        {"setForeground", nullptr, SetForeground, nullptr, nullptr, nullptr,
         napi_default, nullptr},
        {"dispatchBack", nullptr, DispatchBack, nullptr, nullptr, nullptr,
         napi_default, nullptr},
        {"setSafeArea", nullptr, SetSafeArea, nullptr, nullptr, nullptr,
         napi_default, nullptr},
        {"setChromeCallback", nullptr, SetChromeCallback, nullptr, nullptr, nullptr,
         napi_default, nullptr}
    };
    if (napi_define_properties(env, exports, 7, properties) != napi_ok) {
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

extern "C" void gpui_ohos_apply_system_chrome(bool has_status_color, uint32_t status_color,
                                                bool has_navigation_color, uint32_t navigation_color,
                                                bool light_content) {
    ChromeStyle style{has_status_color, status_color, has_navigation_color, navigation_color, light_content};
    std::lock_guard<std::mutex> lock(chromeMutex);
    if (hasChrome && lastChrome.hasStatusColor == style.hasStatusColor &&
        lastChrome.statusColor == style.statusColor &&
        lastChrome.hasNavigationColor == style.hasNavigationColor &&
        lastChrome.navigationColor == style.navigationColor &&
        lastChrome.lightContent == style.lightContent) {
        return;
    }
    lastChrome = style;
    hasChrome = true;
    QueueChromeLocked(style);
    OH_LOG_Print(LOG_APP, LOG_INFO, kLogDomain, kLogTag,
                 "System chrome updated, light content=%{public}d", light_content);
}

extern "C" __attribute__((constructor)) void RegisterModule() {
    napi_module_register(&module);
}
