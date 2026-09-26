//! Hosts the example app's existing Counter and About screens on OHOS.
//!
//! The screen sources are shared directly so their layout and handlers are the
//! same code used by the Android/iOS example. This test router only holds the
//! state these two screens need.

use crate::components::material::NavigationBarBuilder;
use gpui::{
    div, rgb, Context, Font, FontFallbacks, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window,
};

#[rustfmt::skip]
#[path = "../../example/src/screens/about.rs"]
mod about;

#[rustfmt::skip]
#[path = "../../example/src/screens/counter.rs"]
mod counter;

pub(super) const BASE: u32 = 0x121318;
pub(super) const DEFAULT_DARK_MODE: bool = true;
const SURFACE0: u32 = 0x1E1F25;
const SURFACE1: u32 = 0x282A2F;
pub(super) const TEXT: u32 = 0xE2E2E9;
const SUBTEXT: u32 = 0xC4C6D0;
const BLUE: u32 = 0x4285F4;
const GREEN: u32 = 0x34A853;
const RED: u32 = 0xEA4335;
const MAUVE: u32 = 0xA142F4;
const YELLOW: u32 = 0xFBBC04;
const PEACH: u32 = 0xFA7B17;
const MANTLE: u32 = 0x0D0E13;
const LIGHT_TEXT: u32 = 0x1A1B20;
const LIGHT_SUBTEXT: u32 = 0x44474F;
const LIGHT_CARD_BG: u32 = 0xEDEDF4;
const LIGHT_DIVIDER: u32 = 0xC4C6D0;
const TEAL: u32 = 0x24C1E0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Counter,
    About,
}

pub(super) struct Router {
    pub(super) tap_count: u32,
    pub(super) dark_mode: bool,
    page: Page,
}

impl Router {
    pub(super) fn new() -> Self {
        Self {
            tap_count: 0,
            dark_mode: DEFAULT_DARK_MODE,
            page: Page::Counter,
        }
    }
}

impl Render for Router {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(BASE))
            .font(Font {
                fallbacks: Some(FontFallbacks::from_fonts(vec!["HMOS Color Emoji".into()])),
                ..Font::default()
            })
            .text_color(rgb(TEXT))
            .child(match self.page {
                Page::Counter => div()
                    .id("counter-scroll-container")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(counter::render(self, cx))
                    .into_any_element(),
                Page::About => div()
                    .id("about-scroll-container")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(about::render(self))
                    .into_any_element(),
            })
            .child(
                NavigationBarBuilder::new(self.dark_mode)
                    .item(
                        "1",
                        "Counter",
                        self.page == Page::Counter,
                        cx.listener(|this, _, _, cx| {
                            this.page = Page::Counter;
                            cx.notify();
                        }),
                    )
                    .item(
                        "i",
                        "About",
                        self.page == Page::About,
                        cx.listener(|this, _, _, cx| {
                            this.page = Page::About;
                            cx.notify();
                        }),
                    )
                    .build(),
            )
    }
}
