cmake_minimum_required(VERSION 3.16)
project(gpui_ohos_probe)

add_library(gpui_mobile_ohos_example STATIC IMPORTED)
set_target_properties(gpui_mobile_ohos_example PROPERTIES IMPORTED_LOCATION
  "${CMAKE_CURRENT_SOURCE_DIR}/../../../libs/arm64-v8a/libgpui_mobile_ohos_example.a")

add_library(entry SHARED xcomponent_bridge.cpp)
target_link_libraries(entry PUBLIC gpui_mobile_ohos_example ace_ndk.z ace_napi.z hilog_ndk.z native_display_manager time_service_ndk)
