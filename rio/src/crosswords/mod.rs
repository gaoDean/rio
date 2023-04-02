/*
    Crosswords -> Rio's grid manager

    |----------------------------------|
    |-$-bash:-echo-1-------------------|
    |-1--------------------------------|
    |----------------------------------|
    |----------------------------------|
    |----------------------------------|
    |----------------------------------|
    |----------------------------------|

*/

pub mod attr;
pub mod dimensions;
pub mod pos;
pub mod row;
pub mod square;
pub mod storage;

use std::cmp::max;
use std::cmp::min;
use std::cmp::{Ordering};
use std::mem;
use crate::performer::handler::Handler;
use attr::*;
use bitflags::bitflags;
use colors::AnsiColor;
use dimensions::Dimensions;
use pos::CharsetIndex;
use pos::{Column, Cursor, Line, Pos};
use row::Row;
use square::Square;
use std::ops::{Index, IndexMut, Range};
use std::ptr;
use storage::Storage;
use unicode_width::UnicodeWidthChar;

pub type NamedColor = colors::NamedColor;

bitflags! {
    #[derive(Debug, Clone)]
    pub struct Mode: u32 {
        const NONE                = 0;
        const SHOW_CURSOR         = 0b0000_0000_0000_0000_0001;
        const APP_CURSOR          = 0b0000_0000_0000_0000_0010;
        const APP_KEYPAD          = 0b0000_0000_0000_0000_0100;
        const MOUSE_REPORT_CLICK  = 0b0000_0000_0000_0000_1000;
        const BRACKETED_PASTE     = 0b0000_0000_0000_0001_0000;
        const SGR_MOUSE           = 0b0000_0000_0000_0010_0000;
        const MOUSE_MOTION        = 0b0000_0000_0000_0100_0000;
        const LINE_WRAP           = 0b0000_0000_0000_1000_0000;
        const LINE_FEED_NEW_LINE  = 0b0000_0000_0001_0000_0000;
        const ORIGIN              = 0b0000_0000_0010_0000_0000;
        const INSERT              = 0b0000_0000_0100_0000_0000;
        const FOCUS_IN_OUT        = 0b0000_0000_1000_0000_0000;
        const ALT_SCREEN          = 0b0000_0001_0000_0000_0000;
        const MOUSE_DRAG          = 0b0000_0010_0000_0000_0000;
        const MOUSE_MODE          = 0b0000_0010_0000_0100_1000;
        const UTF8_MOUSE          = 0b0000_0100_0000_0000_0000;
        const ALTERNATE_SCROLL    = 0b0000_1000_0000_0000_0000;
        const VI                  = 0b0001_0000_0000_0000_0000;
        const URGENCY_HINTS       = 0b0010_0000_0000_0000_0000;
        const ANY                 = u32::MAX;
    }
}

#[derive(Debug, Clone)]
struct ScrollRegion {
    start: Line,
    end: Line,
}

#[derive(Debug, Clone)]
pub struct Crosswords<U> {
    active_charset: CharsetIndex,
    cols: usize,
    cursor: Cursor<Square>,
    mode: Mode,
    rows: usize,
    scroll: usize,
    scroll_limit: usize,
    scroll_region: ScrollRegion,
    storage: Storage<Square>,
    tabs: TabStops,
    #[allow(dead_code)]
    event_proxy: U,
    window_title: Option<String>,
}

#[derive(Debug, Clone)]
struct TabStops {
    tabs: Vec<bool>,
}

/// Default tab interval, corresponding to terminfo `it` value.
const INITIAL_TABSTOPS: usize = 8;

impl TabStops {
    #[inline]
    fn new(columns: usize) -> TabStops {
        TabStops {
            tabs: (0..columns).map(|i| i % INITIAL_TABSTOPS == 0).collect(),
        }
    }

    /// Remove all tabstops.
    #[inline]
    #[allow(unused)]
    fn clear_all(&mut self) {
        unsafe {
            ptr::write_bytes(self.tabs.as_mut_ptr(), 0, self.tabs.len());
        }
    }

    /// Increase tabstop capacity.
    #[inline]
    #[allow(unused)]
    fn resize(&mut self, columns: usize) {
        let mut index = self.tabs.len();
        self.tabs.resize_with(columns, || {
            let is_tabstop = index % INITIAL_TABSTOPS == 0;
            index += 1;
            is_tabstop
        });
    }
}

impl Index<Column> for TabStops {
    type Output = bool;

    fn index(&self, index: Column) -> &bool {
        &self.tabs[index.0]
    }
}

impl IndexMut<Column> for TabStops {
    fn index_mut(&mut self, index: Column) -> &mut bool {
        self.tabs.index_mut(index.0)
    }
}

impl<U> Index<Line> for Crosswords<U> {
    type Output = Row<Square>;

    #[inline]
    fn index(&self, index: Line) -> &Row<Square> {
        &self.storage[index]
    }
}

impl<U> IndexMut<Line> for Crosswords<U> {
    #[inline]
    fn index_mut(&mut self, index: Line) -> &mut Row<Square> {
        &mut self.storage[index]
    }
}

impl<U> Index<Pos> for Crosswords<U> {
    type Output = Square;

    #[inline]
    fn index(&self, pos: Pos) -> &Square {
        &self[pos.row][pos.col]
    }
}

impl<U> IndexMut<Pos> for Crosswords<U> {
    #[inline]
    fn index_mut(&mut self, pos: Pos) -> &mut Square {
        &mut self[pos.row][pos.col]
    }
}

impl<U> Crosswords<U> {
    pub fn new(cols: usize, rows: usize, event_proxy: U) -> Crosswords<U> {
        Crosswords {
            cols,
            rows,
            storage: Storage::with_capacity(rows, cols),
            cursor: Cursor::default(),
            active_charset: CharsetIndex::default(),
            scroll: 0,
            scroll_region: ScrollRegion {
                start: pos::Line(0),
                end: pos::Line(rows.try_into().unwrap()),
            },
            event_proxy,
            window_title: std::option::Option::Some(String::from("")),
            tabs: TabStops::new(cols),
            scroll_limit: 10_000,
            mode: Mode::SHOW_CURSOR
                | Mode::LINE_WRAP
                | Mode::ALTERNATE_SCROLL
                | Mode::URGENCY_HINTS,
        }
    }

    pub fn resize(&mut self, reflow: bool, columns: usize, lines: usize)
    {
        // Use empty template cell for resetting cells due to resize.
        let template = mem::take(&mut self.cursor.template);

        match self.rows.cmp(&lines) {
            Ordering::Less => self.grow_lines(lines),
            Ordering::Greater => self.storage.shrink_lines(lines),
            Ordering::Equal => (),
        }

        match self.cols.cmp(&columns) {
            Ordering::Less => {
                // self.grow_columns(reflow, columns)
            },
            Ordering::Greater => {
                self.shrink_columns(reflow, columns);
            },
            Ordering::Equal => (),
        }

        // Restore template cell.
        self.cursor.template = template;
    }

    fn grow_lines(&mut self, target: usize)
    {
        let lines_added = target - self.rows;

        // Need to resize before updating buffer.
        self.storage.grow_visible_lines(target);
        self.rows = target;

        let history_size = self.history_size();
        let from_history = min(history_size, lines_added);

        // Move existing lines up for every line that couldn't be pulled from history.
        if from_history != lines_added {
            let delta = lines_added - from_history;
            self.scroll_up(&(Line(0)..Line(target as i32)), delta);
        }

        // Move cursor down for every line pulled from history.
        // self.saved_cursor.point.line += from_history;
        self.cursor.pos.row += from_history;

        self.scroll = self.scroll.saturating_sub(lines_added);
        self.decrease_scroll_limit(lines_added);
    }

    fn decrease_scroll_limit(&mut self, count: usize) {
        let count = min(count, self.history_size());
        if count != 0 {
            self.storage.shrink_lines(min(count, self.history_size()));
            self.scroll = min(self.scroll, self.history_size());
        }
    }

    fn shrink_columns(&mut self, reflow: bool, cols: usize) {
        self.cols = cols;

        // Remove the linewrap special case, by moving the cursor outside of the grid.
        if self.cursor.should_wrap && reflow {
            self.cursor.should_wrap = false;
            self.cursor.pos.col += 1;
        }

        let mut new_raw = Vec::with_capacity(self.storage.len());
        let mut buffered: Option<Vec<Square>> = None;

        let mut rows = self.storage.take_all();
        for (i, mut row) in rows.drain(..).enumerate().rev() {
            // Append lines left over from the previous row.
            if let Some(buffered) = buffered.take() {
                // Add a column for every cell added before the cursor, if it goes beyond the new
                // width it is then later reflown.
                let cursor_buffer_line = self.rows - self.cursor.pos.row.0 as usize - 1;
                if i == cursor_buffer_line {
                    self.cursor.pos.col += buffered.len();
                }

                row.append_front(buffered);
            }

            loop {
                // Remove all cells which require reflowing.
                let mut wrapped = match row.shrink(cols) {
                    Some(wrapped) if reflow => wrapped,
                    _ => {
                        let cursor_buffer_line = self.rows - self.cursor.pos.row.0 as usize - 1;
                        if reflow && i == cursor_buffer_line && self.cursor.pos.col > cols {
                            // If there are empty cells before the cursor, we assume it is explicit
                            // whitespace and need to wrap it like normal content.
                            Vec::new()
                        } else {
                            // Since it fits, just push the existing line without any reflow.
                            new_raw.push(row);
                            break;
                        }
                    },
                };

                // Insert spacer if a wide char would be wrapped into the last column.
                if row.len() >= cols
                    // && row[Column(cols - 1)].flags().contains(Flags::WIDE_CHAR)
                {
                    let mut spacer = Square::default();
                    // spacer.flags_mut().insert(Flags::LEADING_WIDE_CHAR_SPACER);

                    let wide_char = mem::replace(&mut row[Column(cols - 1)], spacer);
                    wrapped.insert(0, wide_char);
                }

                // Remove wide char spacer before shrinking.
                let len = wrapped.len();
                // if len > 0 && wrapped[len - 1].flags().contains(Flags::LEADING_WIDE_CHAR_SPACER) {
                if len > 0 {
                    if len == 1 {
                        // row[Column(cols - 1)].flags_mut().insert(Flags::WRAPLINE);
                        new_raw.push(row);
                        break;
                    } else {
                        // Remove the leading spacer from the end of the wrapped row.
                        // wrapped[len - 2].flags_mut().insert(Flags::WRAPLINE);
                        wrapped.truncate(len - 1);
                    }
                }

                new_raw.push(row);

                // Set line as wrapped if cells got removed.
                if let Some(cell) = new_raw.last_mut().and_then(|r| r.last_mut()) {
                    // cell.flags_mut().insert(Flags::WRAPLINE);
                }

                if wrapped
                    .last()
                    .map(|c| i >= 1)
                    // .map(|c| c.flags().contains(Flags::WRAPLINE) && i >= 1)
                    .unwrap_or(false)
                    && wrapped.len() < cols
                {
                    // Make sure previous wrap flag doesn't linger around.
                    if let Some(cell) = wrapped.last_mut() {
                        // cell.flags_mut().remove(Flags::WRAPLINE);
                    }

                    // Add removed cells to start of next row.
                    buffered = Some(wrapped);
                    break;
                } else {
                    // Reflow cursor if a line below it is deleted.
                    let cursor_buffer_line = self.rows - self.cursor.pos.row.0 as usize - 1;
                    if (i == cursor_buffer_line && self.cursor.pos.col < cols)
                        || i < cursor_buffer_line
                    {
                        self.cursor.pos.row = max(self.cursor.pos.row - 1, Line(0));
                    }

                    // Reflow the cursor if it is on this line beyond the width.
                    if i == cursor_buffer_line && self.cursor.pos.col >= cols {
                        // Since only a single new line is created, we subtract only `columns`
                        // from the cursor instead of reflowing it completely.
                        self.cursor.pos.col -= cols;
                    }

                    // Make sure new row is at least as long as new width.
                    let occ = wrapped.len();
                    if occ < cols {
                        wrapped.resize_with(cols, Square::default);
                    }
                    row = Row::from_vec(wrapped, occ);

                    if i < self.scroll {
                        // Since we added a new line, rotate up the viewport.
                        self.scroll += 1;
                    }
                }
            }
        }

        // Reverse iterator and use it as the new grid storage.
        let mut reversed: Vec<Row<Square>> = new_raw.drain(..).rev().collect();
        reversed.truncate(self.scroll_limit + self.rows);
        self.storage.replace_inner(reversed);

        // Reflow the primary cursor, or clamp it if reflow is disabled.
        if !reflow {
            self.cursor.pos.col = min(self.cursor.pos.col, Column(cols - 1));
        } else if self.cursor.pos.col == cols
            // && !self[self.cursor.pos.line][Column(cols - 1)].flags().contains(Flags::WRAPLINE)
        {
            self.cursor.should_wrap = true;
            self.cursor.pos.col -= 1;
        } else {
            // self.cursor.pos = self.cursor.pos.grid_clamp(self, Boundary::Cursor);
        }

        // Clamp the saved cursor to the grid.
        // self.saved_cursor.pos.column = min(self.saved_cursor.point.column, Column(cols - 1));
    }

    #[inline]
    pub fn wrapline(&mut self) {
        if !self.mode.contains(Mode::LINE_WRAP) {
            return;
        }

        // self.cursor_cell().flags.insert(Flags::WRAPLINE);

        if self.cursor.pos.row + 1 >= self.scroll_region.end {
            self.linefeed();
        } else {
            // self.damage_cursor();
            self.cursor.pos.row += 1;
        }

        self.cursor.pos.col = Column(0);
        self.cursor.should_wrap = false;
        // self.damage_cursor();
    }

    #[allow(dead_code)]
    pub fn update_history(&mut self, history_size: usize) {
        let current_history_size = self.history_size();
        if current_history_size > history_size {
            self.storage
                .shrink_lines(current_history_size - history_size);
        }
        self.scroll = std::cmp::min(self.scroll, history_size);
        self.scroll_limit = history_size;
    }

    #[allow(dead_code)]
    #[inline]
    pub fn cursor(&self) -> (Column, Line) {
        (self.cursor.pos.col, self.cursor.pos.row)
    }

    // pub fn scroll_display(&mut self, scroll: Scroll) {
    //     self.scroll = match scroll {
    //         Scroll::Delta(count) => {
    //             min(max((self.scroll as i32) + count, 0) as usize, self.history_size())
    //         },
    //         Scroll::PageUp => min(self.scroll + self.lines, self.history_size()),
    //         Scroll::PageDown => self.scroll.saturating_sub(self.lines),
    //         Scroll::Top => self.history_size(),
    //         Scroll::Bottom => 0,
    //     };
    // }

    pub fn scroll_up(&mut self, region: &Range<Line>, positions: usize) {
        // When rotating the entire region with fixed lines at the top, just reset everything.
        if region.end - region.start <= positions && region.start != 0 {
            for i in (region.start.0..region.end.0).map(Line::from) {
                self.storage[i].reset(&self.cursor.template);
            }

            return;
        }

        // Update display offset when not pinned to active area.
        if self.scroll != 0 {
            self.scroll = std::cmp::min(self.scroll + positions, self.scroll_limit);
        }

        // Increase scroll limit
        let count = std::cmp::min(positions, self.scroll_limit - self.history_size());
        if count != 0 {
            self.storage.initialize(count, self.cols);
        }

        // Swap the lines fixed at the top to their target positions after rotation.
        //
        // Since we've made sure that the rotation will never rotate away the entire region, we
        // know that the position of the fixed lines before the rotation must already be
        // visible.
        //
        // We need to start from the bottom, to make sure the fixed lines aren't swapped with each
        // other.
        for i in (0..region.start.0).rev().map(Line::from) {
            self.storage.swap(i, i + positions);
        }

        // Rotate the entire line buffer upward.
        self.storage.rotate(-(positions as isize));

        // Ensure all new lines are fully cleared.
        let screen_lines = self.screen_lines();
        for i in ((screen_lines - positions)..screen_lines).map(Line::from) {
            self.storage[i].reset(&self.cursor.template);
        }

        // Swap the fixed lines at the bottom back into position.
        for i in (region.end.0..(screen_lines as i32)).rev().map(Line::from) {
            self.storage.swap(i, i - positions);
        }
    }

    pub fn history_size(&self) -> usize {
        self.total_lines().saturating_sub(self.screen_lines())
    }

    #[inline]
    pub fn scroll_up_from_origin(&mut self, origin: Line, mut lines: usize) {
        // println!("Scrolling up: origin={origin}, lines={lines}");

        lines = std::cmp::min(
            lines,
            (self.scroll_region.end - self.scroll_region.start).0 as usize,
        );

        let region = origin..self.scroll_region.end;

        // Scroll selection.
        // self.selection = self.selection.take().and_then(|s| s.rotate(self, &region, lines as i32));

        self.scroll_up(&region, lines);

        // // Scroll vi mode cursor.
        // let viewport_top = Line(-(self.grid.display_offset() as i32));
        // let top = if region.start == 0 { viewport_top } else { region.start };
        // let line = &mut self.vi_mode_cursor.point.line;
        // if (top <= *line) && region.end > *line {
        // *line = cmp::max(*line - lines, top);
        // }
        // self.mark_fully_damaged();
    }

    #[allow(dead_code)]
    pub fn rows(&mut self) -> usize {
        self.storage.len()
    }

    pub fn cursor_square(&mut self) -> &mut Square {
        let pos = &self.cursor.pos;
        &mut self.storage[pos.row][pos.col]
    }

    pub fn write_at_cursor(&mut self, c: char) {
        let c = self.cursor.charsets[self.active_charset].map(c);
        let fg = self.cursor.template.fg;
        let bg = self.cursor.template.bg;
        //     let flags = self.grid.cursor.template.flags;
        //     let extra = self.grid.cursor.template.extra.clone();

        let mut cursor_square = self.cursor_square();
        cursor_square.c = c;
        cursor_square.fg = fg;
        cursor_square.bg = bg;
        // cursor_cell.flags = flags;
        // cursor_cell.extra = extra;
    }

    #[allow(dead_code)]
    pub fn visible_rows_to_string(&mut self) -> String {
        let mut text = String::from("");

        for row in self.scroll_region.start.0..self.scroll_region.end.0 {
            for column in 0..self.cols {
                let square_content = &mut self[Line(row)][Column(column)];
                text.push(square_content.c);
                for c in square_content.zerowidth().into_iter().flatten() {
                    text.push(*c);
                }

                if column == (self.cols - 1) {
                    text.push('\n');
                }
            }
        }

        text
    }

    #[inline]
    pub fn visible_rows(&mut self) -> Vec<Row<Square>> {
        let mut visible_rows = vec![];
        for row in self.scroll_region.start.0..self.scroll_region.end.0 {
            visible_rows.push(self[Line(row)].to_owned());
        }

        visible_rows
    }
}

impl<U> Handler for Crosswords<U> {
    #[inline]
    fn terminal_attribute(&mut self, attr: Attr) {
        let cursor = &mut self.cursor;
        // println!("{:?}", attr);
        match attr {
            Attr::Foreground(color) => cursor.template.fg = color,
            Attr::Background(color) => cursor.template.bg = color,
            // Attr::UnderlineColor(color) => cursor.template.set_underline_color(color),
            Attr::Reset => {
                cursor.template.fg = AnsiColor::Named(NamedColor::Foreground);
                cursor.template.bg = AnsiColor::Named(NamedColor::Background);
                // cursor.template.flags = Flags::empty();
                // cursor.template.set_underline_color(None);
            }
            // Attr::Reverse => cursor.template.flags.insert(Flags::INVERSE),
            // Attr::CancelReverse => cursor.template.flags.remove(Flags::INVERSE),
            // Attr::Bold => cursor.template.flags.insert(Flags::BOLD),
            // Attr::CancelBold => cursor.template.flags.remove(Flags::BOLD),
            // Attr::Dim => cursor.template.flags.insert(Flags::DIM),
            // Attr::CancelBoldDim => cursor.template.flags.remove(Flags::BOLD | Flags::DIM),
            // Attr::Italic => cursor.template.flags.insert(Flags::ITALIC),
            // Attr::CancelItalic => cursor.template.flags.remove(Flags::ITALIC),
            // Attr::Underline => {
            //     cursor.template.flags.remove(Flags::ALL_UNDERLINES);
            //     cursor.template.flags.insert(Flags::UNDERLINE);
            // },
            // Attr::DoubleUnderline => {
            //     cursor.template.flags.remove(Flags::ALL_UNDERLINES);
            //     cursor.template.flags.insert(Flags::DOUBLE_UNDERLINE);
            // },
            // Attr::Undercurl => {
            //     cursor.template.flags.remove(Flags::ALL_UNDERLINES);
            //     cursor.template.flags.insert(Flags::UNDERCURL);
            // },
            // Attr::DottedUnderline => {
            //     cursor.template.flags.remove(Flags::ALL_UNDERLINES);
            //     cursor.template.flags.insert(Flags::DOTTED_UNDERLINE);
            // },
            // Attr::DashedUnderline => {
            //     cursor.template.flags.remove(Flags::ALL_UNDERLINES);
            //     cursor.template.flags.insert(Flags::DASHED_UNDERLINE);
            // },
            // Attr::CancelUnderline => cursor.template.flags.remove(Flags::ALL_UNDERLINES),
            // Attr::Hidden => cursor.template.flags.insert(Flags::HIDDEN),
            // Attr::CancelHidden => cursor.template.flags.remove(Flags::HIDDEN),
            // Attr::Strike => cursor.template.flags.insert(Flags::STRIKEOUT),
            // Attr::CancelStrike => cursor.template.flags.remove(Flags::STRIKEOUT),
            _ => {
                println!("Term got unhandled attr: {:?}", attr);
            }
        }
    }

    fn set_title(&mut self, window_title: Option<String>) {
        self.window_title = window_title;

        let _title: String = match &self.window_title {
            Some(title) => title.to_string(),
            None => String::from(""),
        };
        // title
    }

    /// Move lines at the bottom toward the top.

    /// Text moves up; clear at top

    fn input(&mut self, c: char) {
        let width = match c.width() {
            Some(width) => width,
            None => return,
        };

        let row = self.cursor.pos.row;

        // Handle zero-width characters.
        if width == 0 {
            // // Get previous column.
            let mut column = self.cursor.pos.col;
            if !self.cursor.should_wrap {
                column.0 = column.saturating_sub(1);
            }

            // // Put zerowidth characters over first fullwidth character cell.
            // let row = self.cursor.pos.row;
            // if self[row][column].flags.contains(Flags::WIDE_CHAR_SPACER) {
            //     column.0 = column.saturating_sub(1);
            // }

            self[row][column].push_zerowidth(c);
            return;
        }

        if self.cursor.should_wrap {
            self.wrapline();
        }

        if width == 1 {
            self.write_at_cursor(c);
        } else if self.cursor.pos.col == self.cols {
            // Place cursor to beginning if hits the max of cols
            self.cursor.pos.row += 1;
            self.cursor.pos.col = pos::Column(0);
        }

        if self.cursor.pos.col + 1 < self.cols {
            self.cursor.pos.col += 1;
        } else {
            self.cursor.should_wrap = true;
        }
    }

    #[inline]
    fn backspace(&mut self) {
        if self.cursor.pos.col > Column(0) {
            self.cursor.should_wrap = false;
            self.cursor.pos.col -= 1;
        }
    }

    fn linefeed(&mut self) {
        let next = self.cursor.pos.row + 1;
        if next == self.scroll_region.end {
            self.scroll_up_from_origin(self.scroll_region.start, 1);
        } else if next < self.screen_lines() {
            self.cursor.pos.row += 1;
        }
    }

    #[inline]
    fn bell(&mut self) {
        println!("[unimplemented] Bell");
    }

    #[inline]
    fn substitute(&mut self) {
        println!("[unimplemented] Substitute");
    }

    #[inline]
    fn put_tab(&mut self, mut count: u16) {
        // A tab after the last column is the same as a linebreak.
        if self.cursor.should_wrap {
            self.wrapline();
            return;
        }

        while self.cursor.pos.col < self.columns() && count != 0 {
            count -= 1;

            let c = self.cursor.charsets[self.active_charset].map('\t');
            let cell = self.cursor_square();
            if cell.c == ' ' {
                cell.c = c;
            }

            loop {
                if (self.cursor.pos.col + 1) == self.columns() {
                    break;
                }

                self.cursor.pos.col += 1;

                if self.tabs[self.cursor.pos.col] {
                    break;
                }
            }
        }
    }

    // #[inline]
    // fn damage_row(&mut self, line: usize, left: usize, right: usize) {
    //     self.storage[line.into()].expand(left, right);
    // }

    fn carriage_return(&mut self) {
        let new_col = 0;
        // let row = self.cursor.pos.row.0 as usize;
        // self.damage_row(row, new_col, self.cursor.pos.col.0);
        self.cursor.pos.col = Column(new_col);
        self.cursor.should_wrap = false;
    }

    #[inline]
    fn clear_line(&mut self, mode: u16) {
        let cursor = &self.cursor;
        let _bg = cursor.template.bg;
        let pos = &cursor.pos;
        let (_left, _right) = match mode {
            // Right
            0 => {
                if self.cursor.should_wrap {
                    return;
                }
                (pos.col, Column(self.columns()))
            }
            // Left
            1 => (Column(0), pos.col + 1),
            // All
            2 => (Column(0), Column(self.columns())),
            _ => todo!(),
        };

        // self.damage.damage_line(point.line.0 as usize, left.0, right.0 - 1);
        // let row = &mut self[pos.row];
        // for cell in &mut row[left..right] {
        // *cell = bg.into();
        // }
        // let range = self.cursor.pos.row..=self.cursor.pos.row;
        // self.selection = self.selection.take().filter(|s| !s.intersects_range(range));
    }

    #[inline]
    fn text_area_size_pixels(&mut self) {
        println!("text_area_size_pixels");
        // self.event_proxy.send_event(Event::TextAreaSizeRequest(Arc::new(move |window_size| {
            // let height = window_size.num_lines * window_size.cell_height;
            // let width = window_size.num_cols * window_size.cell_width;
            // format!("\x1b[4;{height};{width}t")
        // })));
    }

    #[inline]
    fn text_area_size_chars(&mut self) {
        let text = format!("\x1b[8;{};{}t", self.screen_lines(), self.columns());
        println!("text_area_size_chars {:?}", text);
        // self.event_proxy.send_event(Event::PtyWrite(text));
    }

    // fn to_arr_u8(&mut self, line: Line) -> Row<Square> {
    //     self.storage[line]
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::VoidListener;

    #[test]
    fn scroll_up() {
        let mut cw = Crosswords::new(1, 10, VoidListener {});
        for i in 0..10 {
            cw[Line(i)][Column(0)].c = i as u8 as char;
        }

        cw.scroll_up(&(Line(0)..Line(10)), 2);

        assert_eq!(cw[Line(0)][Column(0)].c, '\u{2}');
        assert_eq!(cw[Line(0)].occ, 1);
        assert_eq!(cw[Line(1)][Column(0)].c, '\u{3}');
        assert_eq!(cw[Line(1)].occ, 1);
        assert_eq!(cw[Line(2)][Column(0)].c, '\u{4}');
        assert_eq!(cw[Line(2)].occ, 1);
        assert_eq!(cw[Line(3)][Column(0)].c, '\u{5}');
        assert_eq!(cw[Line(3)].occ, 1);
        assert_eq!(cw[Line(4)][Column(0)].c, '\u{6}');
        assert_eq!(cw[Line(4)].occ, 1);
        assert_eq!(cw[Line(5)][Column(0)].c, '\u{7}');
        assert_eq!(cw[Line(5)].occ, 1);
        assert_eq!(cw[Line(6)][Column(0)].c, '\u{8}');
        assert_eq!(cw[Line(6)].occ, 1);
        assert_eq!(cw[Line(7)][Column(0)].c, '\u{9}');
        assert_eq!(cw[Line(7)].occ, 1);
        assert_eq!(cw[Line(8)][Column(0)].c, ' '); // was 0.
        assert_eq!(cw[Line(8)].occ, 0);
        assert_eq!(cw[Line(9)][Column(0)].c, ' '); // was 1.
        assert_eq!(cw[Line(9)].occ, 0);
    }

    #[test]
    fn test_linefeed() {
        let mut cw: Crosswords<VoidListener> = Crosswords::new(1, 1, VoidListener {});
        assert_eq!(cw.rows(), 1);

        cw.linefeed();
        assert_eq!(cw.rows(), 2);
    }

    #[test]
    fn test_linefeed_moving_cursor() {
        let mut cw: Crosswords<VoidListener> = Crosswords::new(1, 3, VoidListener {});
        let (col, row) = cw.cursor();
        assert_eq!(col, 0);
        assert_eq!(row, 0);

        cw.linefeed();
        let (col, row) = cw.cursor();
        assert_eq!(col, 0);
        assert_eq!(row, 1);

        // Keep adding lines but keep cursor at max row
        for _ in 0..20 {
            cw.linefeed();
        }
        let (col, row) = cw.cursor();
        assert_eq!(col, 0);
        assert_eq!(row, 2);
        assert_eq!(cw.rows(), 22);
    }

    #[test]
    fn test_input() {
        let columns: usize = 5;
        let rows: usize = 10;
        let mut cw: Crosswords<VoidListener> =
            Crosswords::new(columns, rows, VoidListener {});
        for i in 0..4 {
            cw[Line(0)][Column(i)].c = i as u8 as char;
        }
        cw[Line(1)][Column(3)].c = 'b';

        assert_eq!(cw[Line(0)][Column(0)].c, '\u{0}');
        assert_eq!(cw[Line(0)][Column(1)].c, '\u{1}');
        assert_eq!(cw[Line(0)][Column(2)].c, '\u{2}');
        assert_eq!(cw[Line(0)][Column(3)].c, '\u{3}');
        assert_eq!(cw[Line(0)][Column(4)].c, ' ');
        assert_eq!(cw[Line(1)][Column(2)].c, ' ');
        assert_eq!(cw[Line(1)][Column(3)].c, 'b');
        assert_eq!(cw[Line(0)][Column(4)].c, ' ');
    }
}