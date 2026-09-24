# OHOS GPUI view probe

ArkUI XComponent supplies an `OHNativeWindow`. A minimal OHOS `gpui::Platform`
opens a GPUI window and renders the existing Counter and About screens from
`example/src/screens/` through `gpui-pre-wgpu`. A two-tab probe router keeps
the Counter state while switching screens; the Android/iOS multi-screen router
is not yet connected.

## Build

1. Install the `aarch64-unknown-linux-ohos` Rust target and an OHOS native SDK.
2. Set `OHOS_SDK_HOME` to the SDK's `native` directory.
3. Run `./build.ps1` from this directory. The script compiles the Rust library
   and copies `libgpui_mobile.so` into `entry/libs/arm64-v8a/`.
4. Copy `build-profile.example.json5` to `build-profile.json5`, then open this
   directory in DevEco Studio and build the `entry` HAP. Configure local
   signing in DevEco Studio to install it on a device. The local
   `build-profile.json5` is ignored by Git; keep signing material outside
   this repository.

## Device acceptance

- Launch: the Counter screen shows zero, action buttons, and a milestone bar.
- Touch: `+1`, `+5`, and `Reset to zero` update the count from 0 to 1 to 6 to 0;
  `+10` then `-` update it from 0 to 10 to 9 and cross the first milestone.
- About: the tab opens the existing About layout, which scrolls to its footer;
  returning to Counter preserves the count.
- Rotate, leave and return: the surface resizes or recreates without a crash
  or blank screen.

Launch, those five Counter actions, About tab switching, scrolling to the
footer, and Counter state preservation were checked on one physical OHOS
device. About's emoji glyphs were also checked on that device after loading
its color emoji font and selecting an RGBA color atlas. About's existing
Android/iOS-only descriptions remain inaccurate on OHOS.
The window now uses immersive layout with the measured status-bar inset at the
top. GPUI draws the bottom navigation through the gesture area, without an
extra safe-area strip. Both tabs respond to taps, and the system bottom-swipe
gesture still opens the recent-apps view on the tested device.
The `+50` and `+100` actions and rotation still need device validation.
