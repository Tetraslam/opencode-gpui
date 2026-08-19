use gpui::{Pixels, Point, WrappedLine, point};

#[derive(Clone)]
pub(crate) struct EditorLayout {
    pub(super) lines: Vec<WrappedLine>,
    pub(super) starts: Vec<usize>,
    pub(super) line_height: Pixels,
    pub(super) scroll_row: usize,
}

impl EditorLayout {
    pub(super) fn new(lines: Vec<WrappedLine>, line_height: Pixels, cursor: usize) -> Self {
        let mut starts = Vec::with_capacity(lines.len());
        let mut start = 0;
        for line in &lines {
            starts.push(start);
            start += line.len() + 1;
        }
        let mut layout = Self {
            lines,
            starts,
            line_height,
            scroll_row: 0,
        };
        let cursor_row = layout.row_for_y(layout.position_for_offset(cursor).y);
        layout.scroll_row = cursor_row.saturating_sub(super::MAX_VISIBLE_LINES - 1);
        layout
    }

    pub(super) fn visual_lines(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.wrap_boundaries().len() + 1)
            .sum()
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(super) fn row_for_y(&self, y: Pixels) -> usize {
        (y.max(Pixels::ZERO) / self.line_height) as usize
    }

    pub(super) fn position_for_offset(&self, offset: usize) -> Point<Pixels> {
        let mut row = 0;
        for (line, start) in self.lines.iter().zip(&self.starts) {
            let end = start + line.len();
            if offset <= end {
                let local = line
                    .position_for_index(offset.saturating_sub(*start), self.line_height)
                    .unwrap_or_default();
                return point(local.x, local.y + self.line_height * row);
            }
            row += line.wrap_boundaries().len() + 1;
        }
        point(Pixels::ZERO, self.line_height * row.saturating_sub(1))
    }

    pub(super) fn closest_index(&self, position: Point<Pixels>) -> usize {
        let target_y = position.y + self.line_height * self.scroll_row;
        let mut row = 0;
        for (line, start) in self.lines.iter().zip(&self.starts) {
            let rows = line.wrap_boundaries().len() + 1;
            if target_y < self.line_height * (row + rows) {
                let local = point(position.x, target_y - self.line_height * row);
                let offset = line
                    .closest_index_for_position(local, self.line_height)
                    .unwrap_or_else(|index| index);
                return start + offset;
            }
            row += rows;
        }
        self.starts
            .last()
            .zip(self.lines.last())
            .map_or(0, |(start, line)| start + line.len())
    }
}
