mod helpers;

#[test]
fn deckpam() {
    helpers::fixture("deckpam");
}

#[test]
fn ri() {
    helpers::fixture("ri");
}

#[test]
fn ris() {
    helpers::fixture("ris");
}

#[test]
fn vb() {
    struct State {
        vb: usize,
    }

    impl shpool_vt100::Callbacks for State {
        fn visual_bell(&mut self, _: &mut shpool_vt100::Screen) {
            self.vb += 1;
        }
    }

    let mut parser = shpool_vt100::Parser::default();
    let mut state = State { vb: 0 };
    assert_eq!(state.vb, 0);

    let screen = parser.screen().clone();
    parser.process_cb(b"\x1bg", &mut state);
    assert_eq!(state.vb, 1);
    assert_eq!(parser.screen().contents_diff(&screen), b"");

    let screen = parser.screen().clone();
    parser.process_cb(b"\x1bg", &mut state);
    assert_eq!(state.vb, 2);
    assert_eq!(parser.screen().contents_diff(&screen), b"");

    let screen = parser.screen().clone();
    parser.process_cb(b"\x1bg\x1bg\x1bg", &mut state);
    assert_eq!(state.vb, 5);
    assert_eq!(parser.screen().contents_diff(&screen), b"");

    let screen = parser.screen().clone();
    parser.process_cb(b"foo", &mut state);
    assert_eq!(state.vb, 5);
    assert_eq!(parser.screen().contents_diff(&screen), b"foo");

    let screen = parser.screen().clone();
    parser.process_cb(b"ba\x1bgr", &mut state);
    assert_eq!(state.vb, 6);
    assert_eq!(parser.screen().contents_diff(&screen), b"bar");
}

#[test]
fn decsc() {
    helpers::fixture("decsc");
}

#[test]
fn restore_out_of_bounds_pos() {
    let mut parser = shpool_vt100::Parser::new(5, 10, 20);

    // move cursor to the bottom right
    parser.process(b"\n\n\n\n\n\n\nxxxxxxxx");

    // save cursor
    parser.process(b"\x1b7");

    parser.screen_mut().set_size(2, 2);

    // restore cursor
    parser.process(b"\x1b8");
}

#[test]
fn restore_keeps_a_pending_wrap() {
    // Filling a row leaves the cursor past its last column, waiting to wrap.
    // Saving and restoring the cursor there must keep it waiting, so that
    // the next character starts the next row rather than overwriting the
    // last character of this one.
    let mut parser = shpool_vt100::Parser::new(5, 10, 0);
    parser.process(b"0123456789");
    assert_eq!(parser.screen().cursor_position(), (0, 10));

    parser.process(b"\x1b7\x1b8");
    assert_eq!(parser.screen().cursor_position(), (0, 10));

    parser.process(b"X");
    let rows: Vec<String> = parser.screen().rows(0, 10).take(2).collect();
    assert_eq!(rows, vec!["0123456789", "X"]);
}

#[test]
fn restore_after_shrinking_lands_on_the_screen() {
    // A cursor saved further right than the screen is wide after a resize
    // comes back on the screen, in its last column.
    let mut parser = shpool_vt100::Parser::new(5, 10, 0);
    parser.process(b"\x1b[1;9H\x1b7");
    parser.screen_mut().set_size(5, 4);
    parser.process(b"\x1b8");
    assert_eq!(parser.screen().cursor_position(), (0, 3));
}
