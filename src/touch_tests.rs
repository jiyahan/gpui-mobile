use std::{cell::Cell, rc::Rc};

use gpui::{
    div, point, px, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext, TouchEvent, TouchId, TouchPhase, Window,
};

struct Buttons(Rc<Cell<u32>>);

impl Render for Buttons {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let one = self.0.clone();
        let five = self.0.clone();
        div()
            .size_full()
            .flex()
            .child(
                div()
                    .id("one")
                    .w(px(100.))
                    .h(px(100.))
                    .on_click(move |_, _, _| one.set(one.get() + 1)),
            )
            .child(
                div()
                    .id("five")
                    .w(px(100.))
                    .h(px(100.))
                    .on_click(move |_, _, _| five.set(five.get() + 5)),
            )
    }
}

fn touch(id: u64, phase: TouchPhase, x: f32, y: f32) -> TouchEvent {
    TouchEvent {
        id: TouchId(id),
        phase,
        position: point(px(x), px(y)),
        predicted_position: None,
        force: None,
    }
}

#[gpui::test]
fn two_contacts_activate_separate_buttons(cx: &mut TestAppContext) {
    let score = Rc::new(Cell::new(0));
    let (_, cx) = cx.add_window_view({
        let score = score.clone();
        move |_, _| Buttons(score)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));

    for (first_up, second_up) in [(1, 2), (2, 1)] {
        cx.simulate_event(touch(1, TouchPhase::Started, 50., 50.));
        cx.simulate_event(touch(2, TouchPhase::Started, 150., 50.));
        for id in [first_up, second_up] {
            cx.simulate_event(touch(
                id,
                TouchPhase::Ended,
                if id == 1 { 50. } else { 150. },
                50.,
            ));
        }
        assert_eq!(score.replace(0), 6);
    }

    cx.simulate_event(touch(3, TouchPhase::Started, 300., 300.));
    cx.simulate_event(touch(4, TouchPhase::Started, 150., 50.));
    cx.simulate_event(touch(4, TouchPhase::Ended, 150., 50.));
    cx.simulate_event(touch(3, TouchPhase::Ended, 300., 300.));
    assert_eq!(score.replace(0), 5);

    cx.simulate_event(touch(5, TouchPhase::Started, 50., 50.));
    cx.simulate_event(touch(6, TouchPhase::Started, 150., 50.));
    cx.simulate_event(touch(5, TouchPhase::Cancelled, 50., 50.));
    cx.simulate_event(touch(6, TouchPhase::Ended, 150., 50.));
    assert_eq!(score.replace(0), 5);

    cx.simulate_event(touch(7, TouchPhase::Started, 50., 50.));
    cx.simulate_event(touch(7, TouchPhase::Moved, 50., 100.));
    cx.simulate_event(touch(8, TouchPhase::Started, 150., 50.));
    cx.simulate_event(touch(8, TouchPhase::Ended, 150., 50.));
    cx.simulate_event(touch(7, TouchPhase::Cancelled, 50., 100.));
    assert_eq!(score.replace(0), 5);

    cx.simulate_event(touch(9, TouchPhase::Started, 50., 50.));
    cx.simulate_event(touch(10, TouchPhase::Started, 150., 50.));
    cx.simulate_event(touch(10, TouchPhase::Moved, 150., 100.));
    cx.simulate_event(touch(10, TouchPhase::Ended, 150., 100.));
    cx.simulate_event(touch(9, TouchPhase::Ended, 50., 50.));
    assert_eq!(score.get(), 1);
}
