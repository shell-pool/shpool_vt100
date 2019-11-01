use std::convert::TryInto as _;

const DEFAULT_MULTI_PARAMS: &[i64] = &[0];

const OUTPUT_DEFAULT: u16 = 0x0000;
const OUTPUT_AUDIBLE_BELL: u16 = 0x0001;
const OUTPUT_VISUAL_BELL: u16 = 0x0002;

const STATE_DEFAULT: u16 = 0x0000;
const STATE_HIDE_CURSOR: u16 = 0x0001;
const STATE_APPLICATION_CURSOR: u16 = 0x0002;
const STATE_KEYPAD_APPLICATION_MODE: u16 = 0x0004;
const STATE_BRACKETED_PASTE: u16 = 0x0008;
const STATE_MOUSE_REPORTING_BUTTON_MOTION: u16 = 0x0010;
const STATE_MOUSE_REPORTING_SGR_MODE: u16 = 0x0020;
const STATE_MOUSE_REPORTING_PRESS: u16 = 0x0040;
const STATE_MOUSE_REPORTING_PRESS_RELEASE: u16 = 0x0080;

struct State {
    grid: crate::grid::Grid,
    alternate_grid: Option<crate::grid::Grid>,

    attrs: crate::attrs::Attrs,

    title: String,
    icon_name: String,

    outputs: u16,
    state: u16,
}

impl State {
    fn new(rows: u16, cols: u16) -> Self {
        let size = crate::grid::Size { rows, cols };
        Self {
            grid: crate::grid::Grid::new(size),
            alternate_grid: None,

            attrs: crate::attrs::Attrs::default(),

            title: String::default(),
            icon_name: String::default(),

            outputs: OUTPUT_DEFAULT,
            state: STATE_DEFAULT,
        }
    }

    fn grid(&self) -> &crate::grid::Grid {
        if let Some(grid) = &self.alternate_grid {
            grid
        } else {
            &self.grid
        }
    }

    fn grid_mut(&mut self) -> &mut crate::grid::Grid {
        if let Some(grid) = &mut self.alternate_grid {
            grid
        } else {
            &mut self.grid
        }
    }

    fn row(&self, pos: crate::grid::Pos) -> Option<&crate::row::Row> {
        self.grid().row(pos)
    }

    fn cell(&self, pos: crate::grid::Pos) -> Option<&crate::cell::Cell> {
        self.grid().cell(pos)
    }

    fn cell_mut(
        &mut self,
        pos: crate::grid::Pos,
    ) -> Option<&mut crate::cell::Cell> {
        self.grid_mut().cell_mut(pos)
    }

    fn current_cell_mut(&mut self) -> Option<&mut crate::cell::Cell> {
        self.grid_mut().current_cell_mut()
    }

    fn enter_alternate_grid(&mut self) {
        if self.alternate_grid.is_none() {
            self.alternate_grid =
                Some(crate::grid::Grid::new(*self.grid.size()));
        }
    }

    fn exit_alternate_grid(&mut self) {
        self.alternate_grid = None;
    }

    fn set_output(&mut self, output: u16) {
        self.outputs |= output;
    }

    fn clear_output(&mut self, output: u16) {
        self.outputs &= !output;
    }

    fn check_output(&mut self, output: u16) -> bool {
        let ret = (self.outputs & output) != 0;
        self.clear_output(output);
        ret
    }

    fn set_state(&mut self, state: u16) {
        self.state |= state;
    }

    fn clear_state(&mut self, state: u16) {
        self.state &= !state;
    }

    fn state(&self, state: u16) -> bool {
        (self.state & state) != 0
    }
}

impl State {
    fn text(&mut self, c: char) {
        let pos = *self.grid().pos();
        if pos.col > 0 {
            let prev_cell = self
                .cell_mut(crate::grid::Pos {
                    row: pos.row,
                    col: pos.col - 1,
                })
                .unwrap();
            if prev_cell.is_wide() {
                prev_cell.reset();
            }
        }

        let width = crate::unicode::char_width(c);
        let attrs = self.attrs;
        self.grid_mut().col_wrap(width as u16);
        if let Some(cell) = self.current_cell_mut() {
            if width == 0 {
                if pos.col > 0 {
                    let prev_cell = self
                        .cell_mut(crate::grid::Pos {
                            row: pos.row,
                            col: pos.col - 1,
                        })
                        .unwrap();
                    prev_cell.append(c);
                } else if pos.row > 0 {
                    let prev_row = self
                        .row(crate::grid::Pos {
                            row: pos.row - 1,
                            col: 0,
                        })
                        .unwrap();
                    if prev_row.wrapped() {
                        let prev_cell = self
                            .cell_mut(crate::grid::Pos {
                                row: pos.row - 1,
                                col: self.grid().size().cols - 1,
                            })
                            .unwrap();
                        prev_cell.append(c);
                    }
                }
            } else {
                cell.set(c.to_string(), attrs);
                self.grid_mut().col_inc(width as u16);
            }
        } else {
            panic!("couldn't find current cell")
        }
    }

    // control codes

    fn bel(&mut self) {
        self.set_output(OUTPUT_AUDIBLE_BELL);
    }

    fn bs(&mut self) {
        // XXX is this correct? is backwards wrapping a thing?
        self.grid_mut().col_dec(1);
    }

    fn tab(&mut self) {
        self.grid_mut().col_tab();
    }

    fn lf(&mut self) {
        self.grid_mut().row_inc_scroll(1);
    }

    fn vt(&mut self) {
        self.lf();
    }

    fn ff(&mut self) {
        self.lf();
    }

    fn cr(&mut self) {
        self.grid_mut().col_set(0);
    }

    // escape codes

    // ESC 7
    fn decsc(&mut self) {
        self.grid_mut().save_pos();
    }

    // ESC 8
    fn decrc(&mut self) {
        self.grid_mut().restore_pos();
    }

    // ESC =
    fn deckpam(&mut self) {
        self.set_state(STATE_KEYPAD_APPLICATION_MODE);
    }

    // ESC >
    fn deckpnm(&mut self) {
        self.clear_state(STATE_KEYPAD_APPLICATION_MODE);
    }

    // ESC M
    fn ri(&mut self) {
        self.grid_mut().row_dec_scroll(1);
    }

    // ESC c
    fn ris(&mut self) {
        self.grid = crate::grid::Grid::new(*self.grid().size());
        self.alternate_grid = None;
        self.attrs = crate::attrs::Attrs::default();
        self.state = STATE_DEFAULT;
    }

    // ESC g
    fn vb(&mut self) {
        self.set_output(OUTPUT_VISUAL_BELL);
    }

    // csi codes

    // CSI @
    fn ich(&mut self, count: u16) {
        let pos = *self.grid().pos();
        self.grid_mut().insert_cells(pos, count);
    }

    // CSI A
    fn cuu(&mut self, offset: u16) {
        self.grid_mut().row_dec_clamp(offset);
    }

    // CSI B
    fn cud(&mut self, offset: u16) {
        self.grid_mut().row_inc_clamp(offset);
    }

    // CSI C
    fn cuf(&mut self, offset: u16) {
        self.grid_mut().col_inc_clamp(offset);
    }

    // CSI D
    fn cub(&mut self, offset: u16) {
        self.grid_mut().col_dec(offset);
    }

    // CSI G
    fn cha(&mut self, col: u16) {
        self.grid_mut().col_set(col - 1);
    }

    // CSI H
    fn cup(&mut self, (row, col): (u16, u16)) {
        self.grid_mut().set_pos(crate::grid::Pos {
            row: row - 1,
            col: col - 1,
        });
    }

    // CSI J
    fn ed(&mut self, mode: u16) {
        let pos = *self.grid().pos();
        match mode {
            0 => self.grid_mut().erase_all_forward(pos),
            1 => self.grid_mut().erase_all_backward(pos),
            2 => self.grid_mut().erase_all(),
            _ => {}
        }
    }

    // CSI ? J
    fn decsed(&mut self, mode: u16) {
        self.ed(mode);
    }

    // CSI K
    fn el(&mut self, mode: u16) {
        let pos = *self.grid().pos();
        match mode {
            0 => self.grid_mut().erase_row_forward(pos),
            1 => self.grid_mut().erase_row_backward(pos),
            2 => self.grid_mut().erase_row(pos),
            _ => {}
        }
    }

    // CSI ? K
    fn decsel(&mut self, mode: u16) {
        self.el(mode);
    }

    // CSI L
    fn il(&mut self, count: u16) {
        let pos = *self.grid().pos();
        self.grid_mut().insert_lines(pos, count);
    }

    // CSI M
    fn dl(&mut self, count: u16) {
        let pos = *self.grid().pos();
        self.grid_mut().delete_lines(pos, count);
    }

    // CSI P
    fn dch(&mut self, count: u16) {
        let pos = *self.grid().pos();
        self.grid_mut().delete_cells(pos, count);
    }

    // CSI S
    fn su(&mut self, count: u16) {
        self.grid_mut().scroll_up(count);
    }

    // CSI T
    fn sd(&mut self, count: u16) {
        self.grid_mut().scroll_down(count);
    }

    // CSI X
    fn ech(&mut self, count: u16) {
        let pos = *self.grid().pos();
        self.grid_mut().erase_cells(pos, count);
    }

    // CSI d
    fn vpa(&mut self, row: u16) {
        self.grid_mut().row_set(row - 1);
    }

    // CSI h
    fn sm(&mut self, _params: &[i64]) {
        // nothing, i think?
    }

    // CSI ? h
    fn decset(&mut self, params: &[i64]) {
        for param in params {
            match param {
                1 => self.set_state(STATE_APPLICATION_CURSOR),
                9 => self.set_state(STATE_MOUSE_REPORTING_PRESS),
                25 => self.clear_state(STATE_HIDE_CURSOR),
                1000 => self.set_state(STATE_MOUSE_REPORTING_PRESS_RELEASE),
                1002 => self.set_state(STATE_MOUSE_REPORTING_BUTTON_MOTION),
                1006 => self.set_state(STATE_MOUSE_REPORTING_SGR_MODE),
                1049 => self.enter_alternate_grid(),
                2004 => self.set_state(STATE_BRACKETED_PASTE),
                _ => {}
            }
        }
    }

    // CSI l
    fn rm(&mut self, _params: &[i64]) {
        // nothing, i think?
    }

    // CSI ? l
    fn decrst(&mut self, params: &[i64]) {
        for param in params {
            match param {
                1 => self.clear_state(STATE_APPLICATION_CURSOR),
                9 => self.clear_state(STATE_MOUSE_REPORTING_PRESS),
                25 => self.set_state(STATE_HIDE_CURSOR),
                1000 => self.clear_state(STATE_MOUSE_REPORTING_PRESS_RELEASE),
                1002 => self.clear_state(STATE_MOUSE_REPORTING_BUTTON_MOTION),
                1006 => self.clear_state(STATE_MOUSE_REPORTING_SGR_MODE),
                1049 => self.exit_alternate_grid(),
                2004 => self.clear_state(STATE_BRACKETED_PASTE),
                _ => {}
            }
        }
    }

    // CSI m
    fn sgr(&mut self, params: &[i64]) {
        // XXX need to handle incorrect numbers of parameters for some of the
        // fancier options
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.attrs = crate::attrs::Attrs::default(),
                1 => self.attrs.bold = true,
                3 => self.attrs.italic = true,
                4 => self.attrs.underline = true,
                7 => self.attrs.inverse = true,
                22 => self.attrs.bold = false,
                23 => self.attrs.italic = false,
                24 => self.attrs.underline = false,
                27 => self.attrs.inverse = false,
                n if n >= 30 && n <= 37 => {
                    self.attrs.fgcolor =
                        crate::color::Color::Idx((n as u8) - 30);
                }
                38 => {
                    i += 1;
                    if i >= params.len() {
                        unimplemented!()
                    }
                    match params[i] {
                        2 => {
                            i += 3;
                            if i >= params.len() {
                                unimplemented!()
                            }
                            self.attrs.fgcolor = crate::color::Color::Rgb(
                                params[i - 2] as u8,
                                params[i - 1] as u8,
                                params[i] as u8,
                            );
                        }
                        5 => {
                            i += 1;
                            if i >= params.len() {
                                unimplemented!()
                            }
                            self.attrs.fgcolor =
                                crate::color::Color::Idx(params[i] as u8);
                        }
                        _ => {}
                    }
                }
                39 => {
                    self.attrs.fgcolor = crate::color::Color::Default;
                }
                n if n >= 40 && n <= 47 => {
                    self.attrs.bgcolor =
                        crate::color::Color::Idx((n as u8) - 40);
                }
                48 => {
                    i += 1;
                    if i >= params.len() {
                        unimplemented!()
                    }
                    match params[i] {
                        2 => {
                            i += 3;
                            if i >= params.len() {
                                unimplemented!()
                            }
                            self.attrs.bgcolor = crate::color::Color::Rgb(
                                params[i - 2] as u8,
                                params[i - 1] as u8,
                                params[i] as u8,
                            );
                        }
                        5 => {
                            i += 1;
                            if i >= params.len() {
                                unimplemented!()
                            }
                            self.attrs.bgcolor =
                                crate::color::Color::Idx(params[i] as u8);
                        }
                        _ => {}
                    }
                }
                49 => {
                    self.attrs.bgcolor = crate::color::Color::Default;
                }
                n if n >= 90 && n <= 97 => {
                    self.attrs.fgcolor =
                        crate::color::Color::Idx(n as u8 - 82);
                }
                n if n >= 100 && n <= 107 => {
                    self.attrs.bgcolor =
                        crate::color::Color::Idx(n as u8 - 92);
                }
                _ => {}
            }
            i += 1;
        }
    }

    // CSI r
    fn csr(&mut self, (top, bottom, left, right): (u16, u16, u16, u16)) {
        self.grid_mut().set_scroll_region(
            top - 1,
            bottom - 1,
            left - 1,
            right - 1,
        );
    }

    // osc codes

    fn osc0(&mut self, s: &[u8]) {
        self.osc1(s);
        self.osc2(s);
    }

    fn osc1(&mut self, s: &[u8]) {
        if let Ok(s) = std::str::from_utf8(s) {
            self.icon_name = s.to_string();
        }
    }

    fn osc2(&mut self, s: &[u8]) {
        if let Ok(s) = std::str::from_utf8(s) {
            self.title = s.to_string();
        }
    }
}

impl vte::Perform for State {
    fn print(&mut self, c: char) {
        self.text(c)
    }

    fn execute(&mut self, b: u8) {
        match b {
            7 => self.bel(),
            8 => self.bs(),
            9 => self.tab(),
            10 => self.lf(),
            11 => self.vt(),
            12 => self.ff(),
            13 => self.cr(),
            _ => {}
        }
    }

    fn esc_dispatch(
        &mut self,
        _params: &[i64],
        intermediates: &[u8],
        _ignore: bool,
        b: u8,
    ) {
        match intermediates.get(0) {
            None => match b {
                b'7' => self.decsc(),
                b'8' => self.decrc(),
                b'=' => self.deckpam(),
                b'>' => self.deckpnm(),
                b'M' => self.ri(),
                b'c' => self.ris(),
                b'g' => self.vb(),
                _ => {}
            },
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &[i64],
        intermediates: &[u8],
        _ignore: bool,
        c: char,
    ) {
        match intermediates.get(0) {
            None => match c {
                '@' => self.ich(canonicalize_params_1(params, 1)),
                'A' => self.cuu(canonicalize_params_1(params, 1)),
                'B' => self.cud(canonicalize_params_1(params, 1)),
                'C' => self.cuf(canonicalize_params_1(params, 1)),
                'D' => self.cub(canonicalize_params_1(params, 1)),
                'G' => self.cha(canonicalize_params_1(params, 1)),
                'H' => self.cup(canonicalize_params_2(params, 1, 1)),
                'J' => self.ed(canonicalize_params_1(params, 0)),
                'K' => self.el(canonicalize_params_1(params, 0)),
                'L' => self.il(canonicalize_params_1(params, 1)),
                'M' => self.dl(canonicalize_params_1(params, 1)),
                'P' => self.dch(canonicalize_params_1(params, 1)),
                'S' => self.su(canonicalize_params_1(params, 1)),
                'T' => self.sd(canonicalize_params_1(params, 1)),
                'X' => self.ech(canonicalize_params_1(params, 1)),
                'd' => self.vpa(canonicalize_params_1(params, 1)),
                'h' => self.sm(canonicalize_params_multi(params)),
                'l' => self.rm(canonicalize_params_multi(params)),
                'm' => self.sgr(canonicalize_params_multi(params)),
                'r' => self.csr(canonicalize_params_csr(
                    params,
                    *self.grid().size(),
                )),
                _ => {}
            },
            Some(b'?') => match c {
                'J' => self.decsed(canonicalize_params_1(params, 0)),
                'K' => self.decsel(canonicalize_params_1(params, 0)),
                'h' => self.decset(canonicalize_params_multi(params)),
                'l' => self.decrst(canonicalize_params_multi(params)),
                _ => {}
            },
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        match (params.get(0), params.get(1)) {
            (Some(&b"0"), Some(s)) => self.osc0(s),
            (Some(&b"1"), Some(s)) => self.osc1(s),
            (Some(&b"2"), Some(s)) => self.osc2(s),
            _ => {}
        }
    }

    // don't care
    fn hook(&mut self, _: &[i64], _: &[u8], _: bool) {}
    fn put(&mut self, _b: u8) {}
    fn unhook(&mut self) {}
}

pub struct Screen {
    parser: vte::Parser,
    state: State,
}

impl Screen {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: vte::Parser::new(),
            state: State::new(rows, cols),
        }
    }

    pub fn rows(&self) -> u16 {
        self.state.grid().size().rows
    }

    pub fn cols(&self) -> u16 {
        self.state.grid().size().cols
    }

    pub fn set_window_size(&mut self, rows: u16, cols: u16) {
        self.state
            .grid_mut()
            .set_size(crate::grid::Size { rows, cols });
    }

    pub fn process(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.parser.advance(&mut self.state, *byte);
        }
    }

    pub fn cell(&self, row: u16, col: u16) -> Option<&crate::cell::Cell> {
        self.state.cell(crate::grid::Pos { row, col })
    }

    pub fn window_contents(
        &self,
        row_start: u16,
        col_start: u16,
        row_end: u16,
        col_end: u16,
    ) -> String {
        self.state
            .grid()
            .window_contents(row_start, col_start, row_end, col_end)
    }

    pub fn window_contents_formatted(
        &self,
        row_start: u16,
        col_start: u16,
        row_end: u16,
        col_end: u16,
    ) -> String {
        self.state
            .grid()
            .window_contents_formatted(row_start, col_start, row_end, col_end)
    }

    pub fn cursor_position(&self) -> (u16, u16) {
        (self.state.grid().pos().row, self.state.grid().pos().col)
    }

    pub fn fgcolor(&self) -> crate::color::Color {
        self.state.attrs.fgcolor
    }

    pub fn bgcolor(&self) -> crate::color::Color {
        self.state.attrs.bgcolor
    }

    pub fn bold(&self) -> bool {
        self.state.attrs.bold
    }

    pub fn italic(&self) -> bool {
        self.state.attrs.italic
    }

    pub fn underline(&self) -> bool {
        self.state.attrs.underline
    }

    pub fn inverse(&self) -> bool {
        self.state.attrs.inverse
    }

    pub fn title(&self) -> &str {
        &self.state.title
    }

    pub fn icon_name(&self) -> &str {
        &self.state.icon_name
    }

    pub fn hide_cursor(&self) -> bool {
        self.state.state(STATE_HIDE_CURSOR)
    }

    pub fn alternate_buffer_active(&self) -> bool {
        self.state.alternate_grid.is_some()
    }

    pub fn application_cursor(&self) -> bool {
        self.state.state(STATE_APPLICATION_CURSOR)
    }

    pub fn application_keypad(&self) -> bool {
        self.state.state(STATE_KEYPAD_APPLICATION_MODE)
    }

    pub fn bracketed_paste(&self) -> bool {
        self.state.state(STATE_BRACKETED_PASTE)
    }

    pub fn mouse_reporting_button_motion(&self) -> bool {
        self.state.state(STATE_MOUSE_REPORTING_BUTTON_MOTION)
    }

    pub fn mouse_reporting_sgr_mode(&self) -> bool {
        self.state.state(STATE_MOUSE_REPORTING_SGR_MODE)
    }

    pub fn mouse_reporting_press(&self) -> bool {
        self.state.state(STATE_MOUSE_REPORTING_PRESS)
    }

    pub fn mouse_reporting_press_release(&self) -> bool {
        self.state.state(STATE_MOUSE_REPORTING_PRESS_RELEASE)
    }

    pub fn check_audible_bell(&mut self) -> bool {
        self.state.check_output(OUTPUT_AUDIBLE_BELL)
    }

    pub fn check_visual_bell(&mut self) -> bool {
        self.state.check_output(OUTPUT_VISUAL_BELL)
    }
}

fn canonicalize_params_1(params: &[i64], default: u16) -> u16 {
    let first = params.get(0).copied().unwrap_or(0);
    if first == 0 {
        default
    } else {
        i64_to_u16(first)
    }
}

fn canonicalize_params_2(
    params: &[i64],
    default1: u16,
    default2: u16,
) -> (u16, u16) {
    let first = params.get(0).copied().unwrap_or(0);
    let first = if first == 0 {
        default1
    } else {
        i64_to_u16(first)
    };

    let second = params.get(1).copied().unwrap_or(0);
    let second = if second == 0 {
        default2
    } else {
        i64_to_u16(second)
    };

    (first, second)
}

fn canonicalize_params_multi(params: &[i64]) -> &[i64] {
    if params.is_empty() {
        DEFAULT_MULTI_PARAMS
    } else {
        params
    }
}

fn canonicalize_params_csr(
    params: &[i64],
    size: crate::grid::Size,
) -> (u16, u16, u16, u16) {
    let top = params.get(0).copied().unwrap_or(0);
    let top = if top == 0 { 1 } else { i64_to_u16(top) };

    let bottom = params.get(1).copied().unwrap_or(0);
    let bottom = if bottom == 0 {
        size.rows
    } else {
        i64_to_u16(bottom)
    };

    let left = params.get(2).copied().unwrap_or(0);
    let left = if left == 0 { 1 } else { i64_to_u16(left) };

    let right = params.get(3).copied().unwrap_or(0);
    let right = if right == 0 {
        size.cols
    } else {
        i64_to_u16(right)
    };

    (top, bottom, left, right)
}

fn i64_to_u16(i: i64) -> u16 {
    if i < 0 {
        0
    } else if i > i64::from(u16::max_value()) {
        u16::max_value()
    } else {
        i.try_into().unwrap()
    }
}
