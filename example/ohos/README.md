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
On 2026-09-26, `+50` then `+100` produced 150 on the device. Ten cycles of
`+1` → About → scroll to the footer → immediate Counter switch ended at 10,
with no missed switch. Sending the app to the home screen and reopening it
preserved that count. With automatic rotation enabled, the window changed to
2760×1256 in landscape and back to 1256×2760 in portrait without a blank
screen or lost count. In landscape, however, the Counter content pushes the
bottom tab bar off-screen. Wrapping Counter content in a scroll container kept
the bottom bar visible in a later landscape device check. On the final package,
Counter → About → Counter worked in landscape and preserved count 1.

Overlapping a held finger with an HDC tap exposed a touch-ID collision: both
sources reported device 0, touch 0. The bridge now cancels the old contact on
a duplicate start. In a portrait device retest, the overlapping tap did not
switch tabs, but after the finger was released, a new About tap succeeded
without restarting the app. The same scenario was retested on the signed HAP
in landscape at 2760×1256: the HDC tap did not switch tabs while a physical
finger was held; after release, an About tap succeeded, and returning to
Counter preserved count 1. This was one device run, not a wider input test.
