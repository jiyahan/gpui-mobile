# OHOS full-repository migration baseline

Recorded on 2026-09-26 at `36e3b28`. Target: OpenHarmony SDK 23, arm64 phone.
This is an inventory, not a claim of OHOS feature support. At baseline, the
OHOS example built with SDK 23 and targeted SDK 23, but declared
`compatibleSdkVersion: 12`. The compatibility floor was raised to 23 in the
first code unit; see the acceptance record below.

The repo has 30 feature packages, 17 shared example routes and 14 TODO ideas.
No package has an OHOS-specific backend. Many wrappers compile through a
fallback that returns an error, `None`, or a default; that is not runtime
support. The Android example opens the shared router, the iOS entry point
opens a separate chat view, and OHOS hard-codes a two-page Counter/About probe
inside the library. CI has no OHOS job.

## Acceptance ledger rules

Use **probe verified** only for an observed scenario on the tested device,
**source present** for code without OHOS acceptance, **pending** for missing
integration, **design pending** for an idea without an accepted contract, and
**blocked** for an external dependency. Each implementation unit records its
API behavior, permission and SDK requirements, focused check, build result,
phone scenario, observed result and remaining limits. Shared API changes also
need Android/iOS regression checks. A package is complete only when each of
its supported public behaviors meets that contract; a fallback result cannot
be counted as success by default.

Use OpenHarmony APIs first, then compatible open-source alternatives that run
locally. A solution requiring an account, key, fee or proprietary SDK needs a
separate decision. If none is acceptable, mark that unit blocked and continue
independent units. Do not declare the whole migration complete with blockers.

## Platform and public API

| Surface | Current OHOS state | First acceptance |
| --- | --- | --- |
| `target_platform()`, `DEFAULT_PLATFORM`, `TargetPlatform::Ohos` | Source present | Cross-compile and assert detection without changing other targets. |
| `current_platform()` | Pending; its non-Android/iOS branch panics. | Open an app-owned root view from a host-provided native surface. |
| `gpui::Platform`, `PlatformWindow`, dispatcher and renderer | Counter/About probe and window lifecycle verified on one device; other trait callbacks remain empty. | Recreate and resize the surface, background/foreground, wake frames and release resources. |
| `set_system_chrome()`, `safe_area_insets()` | Public OHOS paths are no-op/zero; ArkUI probe measures its own inset. | Correct system bars and layout in both orientations and themes. |
| `show_keyboard()`, `show_keyboard_with_type()`, `hide_keyboard()`, `keyboard_height()`, `set_keyboard_height()` | No OHOS keyboard host. | Open, change type, reposition content and dismiss the IME. |
| `set_text_input_callback()`, `dispatch_text_input()`, `TEXT_INPUT_DIRTY` | Shared hooks exist; no OHOS input bridge. | Chinese/English composition, insertion, deletion, selection and redraw. |
| `PlatformView`, factory, registry and handle | Shared registry exists; no OHOS native view host. | Create, position, resize, hide and dispose a native view. |
| OHOS C ABI surface create/destroy, touch and system colors | App root registration and existing ABI verified on one device. | Keep the ABI working as the shared example router is connected. |

Core units, in order: SDK-23 compatibility, separating the example root
from the OHOS platform, and window lifecycle are verified; add chrome/safe-area,
back/touch and keyboard/IME behavior one logical unit at a time. Do not bundle
a GPUI or vendored-renderer upgrade with an unrelated unit.

## Package matrix (30)

API lists name public function and handle-method entry points declared in each
package's `mod.rs`; public data types and their field semantics are reviewed
in the corresponding package unit. All package rows remain **pending on OHOS**
because none has an OHOS backend. Platform-independent helpers may work and must be checked
separately. Each row is one implementation and acceptance unit; the group
order expresses dependencies.

### Basic capabilities (12)

| Package | Public API | First phone acceptance |
| --- | --- | --- |
| `package_info` | `get_package_info` | Read the installed bundle name/version. |
| `device_info` | `get_device_info` | Read model, OS and SDK without fabricated values. |
| `path_provider` | `temporary_directory`, `documents_directory`, `cache_directory`, `support_directory` | Create/read/delete a file in each returned writable location. |
| `shared_preferences` | `instance`, `get_string`, `set_string`, `get_int`, `set_int`, `get_bool`, `set_bool`, `remove`, `clear`, `contains_key` | Round-trip types and survive app restart. |
| `battery` | `battery_level`, `battery_state`, `is_battery_save_mode`, `battery_info` | Compare with device state; report unavailable fields honestly. |
| `connectivity` | `check_connectivity` | Detect Wi-Fi on/off and disconnected states. |
| `network_info` | `get_network_info` | Return network details under the required access state. |
| `vibration` | `vibrate`, `haptic_feedback`, `can_vibrate` | Feel vibration and distinguish absent hardware. |
| `clipboard` | `set_text`, `get_text`, `has_text` | Copy/paste Unicode between apps. |
| `url_launcher` | `launch_url`, `can_launch_url` | Open HTTPS externally; handle no installed handler. |
| `share` | `share_text`, `share_uri` | Show share targets and transfer text/URI. |
| `deeplink` | `set_deep_link_handler`, `get_initial_link`, `get_latest_link` | Route cold and warm `gpui://` launches. |

### Permissions and device data (11)

Implement `permission_handler` first. Each protected service must test both
permission denial and the approved grant path; do not add unrelated HAP
permissions.

| Package | Public API | First phone acceptance |
| --- | --- | --- |
| `permission_handler` | `check_permission`, `request_permission`, `request_permissions`, `service_status`, `open_app_settings`, `should_show_request_rationale`; `PermissionStatus::{is_granted,is_denied,is_permanently_denied,is_restricted,is_limited}` | Grant/deny camera, microphone and location; map OS-specific variants explicitly. |
| `file_selector` | `open_file`, `open_files`, `get_save_path`, `get_directory_path` | Pick/cancel, save and reopen a document. |
| `image_picker` | `pick_image`, `pick_multi_image`, `pick_video` | Gallery and camera sources, including cancel. |
| `camera` | `available_cameras`, `create_camera`, `start_preview`, `stop_preview`, `preview_platform_view_handle`, `take_picture`, `start_video_recording`, `stop_video_recording`, `set_flash_mode`, `set_focus_mode`, `set_exposure_mode`, `get_min_zoom`, `get_max_zoom`, `set_zoom`, `set_camera`, `dispose`; `CameraHandle::from_id`, `ios_get_session` | Preview, capture and release after rotation/navigation; keep `ios_get_session` platform-specific. |
| `microphone` | `is_available`, `start_recording`, `stop_recording`, `is_recording`, `pause_recording`, `resume_recording`, `get_amplitude` | Record, pause/resume, play back and release the mic. |
| `location` | `is_location_service_enabled`, `get_current_position`, `get_last_known_position`, `distance_between`, `bearing_between` | Authorized position and denied/disabled outcomes; check pure calculations separately. |
| `sensors` | `available_sensors`, `accelerometer`, `gyroscope`, `magnetometer`, `barometer` | Observe movement; report absent sensors. |
| `notifications` | `initialize`, `show`, `cancel`, `cancel_all`; conversion helper `as_i32` | Show/cancel a local notification with granted/denied permission. |
| `contacts` | `get_contacts`, `search_contacts`, `get_contact` | Read/search one authorized test contact. |
| `calendar` | `get_calendars`, `get_events`, `create_event`, `delete_event` | Create/read/delete a test event. |
| `local_auth` | `is_device_supported`, `can_authenticate`, `get_available_biometrics`, `authenticate` | Complete/cancel authentication and reject unsupported hardware. |

### Native views and media (5)

These depend on an OHOS platform-view host. Existing Android/iOS `maps` and
`webview` code also contains TODO calls, so each unit reviews all three
platform contracts rather than assuming source-platform completeness.

| Package | Public API | First phone acceptance |
| --- | --- | --- |
| `webview` | `load_url`, `load_html`, `evaluate_javascript`, `go_back`, `reload`, `stop_loading`, `dismiss`, `platform_view_handle` | Embed, navigate, run JS, resize and dispose. |
| `maps` | `MapView::new`, `platform_view_handle`, `set_center`, `set_zoom`, `set_map_type`, `add_marker`, `remove_marker`, `clear_markers`, `dispose` | Pan/zoom a suitable map source, update a marker, dispose; decide data/service terms separately. |
| `video_player` | `VideoPlayer::new`, `set_url`, `set_file_path`, `play`, `pause`, `seek`, `set_volume`, `set_speed`, `set_looping`, `position`, `duration`, `video_size`, `is_playing`, `show_surface`, `hide_surface`, `platform_view_handle`, `dispose`; `ios_get_player` | Play/pause/seek a local video, rotate and release the surface. |
| `audio` | `AudioPlayer::new`, `set_url`, `set_file_path`, `play`, `pause`, `stop`, `seek`, `set_volume`, `set_speed`, `set_loop_mode`, `position`, `duration`, `state`, `is_playing` | Play/pause/seek local audio; release after navigation. |
| `media_session` | `set_action_handler`, `set_seek_handler`, `init`, `set_metadata`, `set_playback_state`, `release` | Show metadata and respond to available system controls; inspect Android-only baseline. |

### External ecosystem (2)

| Package | Public API | First phone acceptance |
| --- | --- | --- |
| `maps_launcher` | `open_coordinates`, `open_query`, `open_directions`, `is_available` | Open an installed handler or truthfully report none. |
| `in_app_review` | `is_available`, `request_review`, `open_store_listing` | Research a compatible distribution/review service or local alternative; Google/Apple flows do not count. |

## Shared example routes (17)

Counter and About are **probe verified** only through a separate OHOS router;
none of these shared routes has OHOS end-to-end acceptance. The iOS entry point
currently starts an independent chat view, which matters for shared changes.

| Route | Dependency / first acceptance |
| --- | --- |
| Home | Cards and navigation open the intended route. |
| Counter | Count survives tabs, backgrounding and rotation. |
| Settings | Theme/name persist across navigation. |
| About | Scroll to footer and return without losing state. |
| AppleGlass | Render controls and handle touch in both orientations. |
| Material | Render and activate component examples. |
| Form | Keyboard types, composition, deletion and focus. |
| Animations | Animate, leave and return without stuck frames. |
| Shaders | Render without a blank GPU surface. |
| PackagesDemo | Exercise each migrated package and show honest denied/unavailable states. |
| WebViewBrowser | Needs `webview` and native-view composition. |
| Swiper | Swipe items without confusing system-edge gestures. |
| Feed | Scroll/open content with its required assets and network setup. |
| Chat | Keyboard, message entry and microphone flow. |
| AudioPlayer | Needs `audio` and `media_session`. |
| VideoPlayer | Needs `video_player`, native view and `media_session`. |
| Markdown | Render text/images and scroll through the shared route. |

## TODO.md design queue (14)

All are **design pending**. Each needs purpose, API, dependencies, feasible
OpenHarmony implementation and acceptance scenario, followed by a separate
implement/drop decision. Fold overlap with the existing `deeplink` and `maps`
packages into those units instead of building duplicates.

| Idea/task | First design question |
| --- | --- |
| Public API usability | Which consumer task fails with today's API? |
| Assistive technologies | How do labels/focus reach GPUI and native views? |
| Deeplinking | Does the package cold/warm contract cover the intended use? |
| Built-in routing/navigation | What need remains after the example router and deep links? |
| Maps | Does the package cover the need with acceptable data terms? |
| In-app purchases | What product/store and transaction/restore contract is required? |
| Asset management | Which assets cannot use existing app packaging? |
| Google Fonts | What loading/licensing need is not met by bundled/system fonts? |
| GPUI Component View System | Check upstream availability before any upgrade or rewrite. |
| GIF viewer example | Define source, playback and lifecycle behavior. |
| Image gallery example | Define selection, paging and memory behavior. |
| Image loading example | Define local/remote loading and failures. |
| Text input feature parity | Compare the referenced input example and current IME tests. |
| Texture-based platform-view composition | Require a measured hybrid-composition limit first. |

## Device evidence and next gate

The probe README records Counter, About, emoji, immersive layout, rotation
and state checks on one physical phone. On 2026-09-26 the signed arm64 HAP was
retested in landscape at 2760×1256: a held physical finger plus HDC About
tap did not switch tabs; after release, a fresh About tap worked, and return
to Counter preserved count 1. Local screenshots were inspected but remain
ignored diagnostic files. This closes only that single-device scenario.

On 2026-09-26, the SDK-23 compatibility unit passed `example/ohos/build.ps1`
and signed `assembleHap`. The installed bundle reported
`apiCompatibleVersion: 23` and `apiTargetVersion: 23` on device
`2MH0224411027452`. The app launched in portrait; tapping +1 changed the
counter from 0 to 1, About opened, and returning to Counter preserved 1.
Screenshots are ignored local diagnostics. This verifies the SDK floor and
basic probe interaction on one arm64 phone, not the remaining migration.

On 2026-09-26, the OHOS root-view unit moved the Counter/About probe router
to `example/ohos/rust/`. The example registers its root before XComponent
surface creation. The Rust static library now links into `libentry.so`, whose
dynamic symbol table retains the existing `gpui_ohos_*` entry points. A clean
signed HAP contains `libentry.so` and no old `libgpui_mobile.so`. On the same
arm64 device, installation and launch succeeded; portrait +1, About and return
preserved count 1. Rotating to 2760×1256 kept count 1, About switching worked,
and a scrolled Counter +1 changed the milestone from 1/10 to 2/10. Returning
to 1256×2760 showed count 2. Root-crate tests passed 46/46. These checks do
not establish surface-recreation state preservation or shared-route support.

On 2026-09-26, the window-lifecycle unit passed format checking, 46/46 root
tests, the OHOS Rust release build, and signed `assembleHap`. On device
`2MH0224411027452`, Home → reopen fired background/foreground callbacks and
preserved Counter 1. A temporary test build removed and reinserted XComponent
once within the same process; hilog recorded surface destruction at 12:59:44
and creation at 12:59:45. Counter still showed 1 after reconstruction; About
→ Counter and +1 then produced 2. The temporary mount toggle was removed
before the final HAP build. The final signed HAP was installed again: at
2760×1256, Counter could scroll and both tabs worked; the count reached 51
and remained 51 after returning to 1256×2760 and after Home → reopen. This
covers one arm64 phone and one forced surface recreation; it does not prove
process-death restoration.

**Next gate:** accept the window-lifecycle unit, then handle system chrome and
safe-area behavior as the next core unit.
