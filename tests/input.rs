//! T106 proofs: event queue, per-frame edges, tap/drag/swipe, keys, wheel.

use defender_of_the_crown::engine::input::*;

#[test]
fn queue_drains_once_per_frame() {
    let mut inp = Input::new();
    inp.push(Raw::Move { x: 10, y: 20 });
    inp.push(Raw::Move { x: 30, y: 40 });
    inp.begin_frame(1);
    assert_eq!(inp.mouse(), (30, 40, false), "last event wins in a frame");
    inp.begin_frame(2);
    assert_eq!(
        inp.prev_mouse(),
        (30, 40),
        "prev holds frame-start position"
    );
}

#[test]
fn press_release_edges_fire_once() {
    let mut inp = Input::new();
    inp.push(Raw::Down { x: 5, y: 5 });
    inp.begin_frame(1);
    assert!(inp.pressed() && !inp.released());
    inp.begin_frame(2);
    assert!(!inp.pressed(), "down edge must not repeat next frame");
    inp.push(Raw::Up { x: 5, y: 5 });
    inp.begin_frame(3);
    assert!(inp.released() && !inp.pressed());
    inp.begin_frame(4);
    assert!(!inp.released());
}

#[test]
fn quick_press_release_is_tap() {
    let mut inp = Input::new();
    inp.push(Raw::Down { x: 100, y: 60 });
    inp.begin_frame(10);
    inp.push(Raw::Up { x: 103, y: 62 }); // within slop
    inp.begin_frame(11);
    assert_eq!(
        inp.gestures(),
        &[Gesture::Tap {
            x: 100,
            y: 60,
            frame: 10
        }]
    );
}

#[test]
fn slow_press_release_is_not_a_tap() {
    let mut inp = Input::new();
    inp.push(Raw::Down { x: 100, y: 60 });
    inp.begin_frame(10);
    inp.push(Raw::Up { x: 100, y: 60 });
    inp.begin_frame(10 + TAP_FRAMES + 1); // held too long
    assert!(matches!(inp.gestures()[0], Gesture::DragEnd { .. }));
}

#[test]
fn drag_reports_origin_and_travel_and_ends() {
    let mut inp = Input::new();
    inp.push(Raw::Down { x: 10, y: 10 });
    inp.push(Raw::Move { x: 40, y: 12 });
    inp.begin_frame(1);
    assert_eq!(inp.dragging(), Some((10, 10)));
    assert_eq!(inp.drag_travel(), 30);
    inp.push(Raw::Up { x: 40, y: 12 }); // 30 px < SWIPE_MIN -> drag
    inp.begin_frame(2);
    assert_eq!(
        inp.gestures(),
        &[Gesture::DragEnd {
            x0: 10,
            y0: 10,
            x1: 40,
            y1: 12
        }]
    );
    assert_eq!(inp.dragging(), None, "stroke cleared after release");
}

#[test]
fn fast_horizontal_release_is_swipe_e() {
    let mut inp = Input::new();
    inp.push(Raw::Down { x: 10, y: 100 });
    inp.begin_frame(0);
    inp.push(Raw::Move { x: 90, y: 102 });
    inp.push(Raw::Up { x: 90, y: 102 });
    inp.begin_frame(2); // 80 px in 2 frames = 40 px/f
    assert_eq!(inp.gestures(), &[Gesture::Swipe { dir: Dir::E }]);
}

#[test]
fn slow_long_drag_is_not_swipe() {
    let mut inp = Input::new();
    inp.push(Raw::Down { x: 10, y: 100 });
    inp.begin_frame(0);
    inp.push(Raw::Up { x: 200, y: 100 });
    inp.begin_frame(100); // 190 px over 100 frames = 1 px/f
    assert!(matches!(inp.gestures()[0], Gesture::DragEnd { .. }));
}

#[test]
fn bucket_eights() {
    assert_eq!(bucket(50, 0), Dir::E);
    assert_eq!(bucket(-50, 0), Dir::W);
    assert_eq!(bucket(0, -50), Dir::N);
    assert_eq!(bucket(0, 50), Dir::S);
    assert_eq!(bucket(50, -50), Dir::NE);
    assert_eq!(bucket(50, 50), Dir::SE);
    assert_eq!(bucket(-50, 50), Dir::SW);
    assert_eq!(bucket(-50, -50), Dir::NW);
    // 22.5-degree boundaries: slope < tan(22.5) stays cardinal
    assert_eq!(bucket(100, -30), Dir::E);
    assert_eq!(bucket(100, -60), Dir::NE);
    assert_eq!(bucket(30, -100), Dir::N);
}

#[test]
fn key_edges_repeat_suppressed_and_text() {
    let mut inp = Input::new();
    inp.push(Raw::Key {
        code: b'a' as u32,
        pressed: true,
    });
    inp.push(Raw::Key {
        code: b'a' as u32,
        pressed: true,
    }); // OS repeat
    inp.begin_frame(1);
    assert!(inp.key_pressed(b'a' as u32) && inp.key_down(b'a' as u32));
    assert_eq!(inp.take_text(), "aa", "repeat feeds text buffer");
    inp.begin_frame(2);
    assert!(
        !inp.key_pressed(b'a' as u32),
        "repeat must not re-fire edge"
    );
    inp.push(Raw::Key {
        code: b'a' as u32,
        pressed: false,
    });
    inp.begin_frame(3);
    assert!(inp.key_released(b'a' as u32) && !inp.key_down(b'a' as u32));
}

#[test]
fn space_key_codes_both_paths() {
    let mut inp = Input::new();
    inp.push(Raw::Key {
        code: KEY_SPACE,
        pressed: true,
    });
    inp.begin_frame(1);
    assert!(inp.key_pressed(KEY_SPACE));
    assert_eq!(inp.take_text(), " ", "KEY_SPACE yields a space char");
}

#[test]
fn wheel_accumulates_within_frame_and_resets() {
    let mut inp = Input::new();
    inp.push(Raw::Wheel { dy: -3 });
    inp.push(Raw::Wheel { dy: 1 });
    inp.begin_frame(1);
    assert_eq!(inp.wheel(), -2);
    inp.begin_frame(2);
    assert_eq!(inp.wheel(), 0);
}

#[test]
fn first_gesture_sets_unlock_flag() {
    let mut inp = Input::new();
    assert!(!inp.user_gesture);
    inp.push(Raw::Move { x: 1, y: 1 });
    inp.begin_frame(1);
    assert!(!inp.user_gesture, "mere mouse move must not unlock audio");
    inp.push(Raw::Down { x: 1, y: 1 });
    inp.begin_frame(2);
    assert!(inp.user_gesture);
}
