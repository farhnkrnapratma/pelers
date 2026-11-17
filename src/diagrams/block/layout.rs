use crate::errors::{PelersError, PelersErrorCode};
use crate::util::jsnum::js_max;

const MAX_SPAN: f64 = 1000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub children: Vec<usize>,
    pub columns: f64,
    pub span: f64,
    pub space: bool,
    pub head: f64,
    pub nw: f64,
    pub nh: f64,
    pub col: f64,
    pub row: f64,
    pub cols: f64,
    pub heights: Vec<f64>,
    pub row_gaps: Vec<f64>,
    pub col_gaps: Vec<f64>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Cell {
    pub fn new(nw: f64, nh: f64, span: f64) -> Self {
        Self {
            children: Vec::new(),
            columns: -1.0,
            span,
            space: false,
            head: 0.0,
            nw,
            nh,
            col: 0.0,
            row: 0.0,
            cols: 0.0,
            heights: Vec::new(),
            row_gaps: Vec::new(),
            col_gaps: Vec::new(),
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }
    }
}

pub fn calculate_block_position(columns: f64, position: f64) -> Result<(f64, f64), PelersError> {
    if columns == 0.0 || columns.fract() != 0.0 || columns.is_nan() {
        let mut error = PelersError::new(
            "Columns must be an integer !== 0.",
            PelersErrorCode::Semantic,
        );
        error.diagram_type = Some("block".to_owned());
        return Err(error);
    }
    if position < 0.0 || position.fract() != 0.0 || position.is_nan() {
        let mut error = PelersError::new(
            format!("Position must be a non-negative integer.{position}"),
            PelersErrorCode::Semantic,
        );
        error.diagram_type = Some("block".to_owned());
        return Err(error);
    }
    if columns < 0.0 {
        return Ok((position, 0.0));
    }
    Ok((position % columns, (position / columns).floor()))
}

fn total(gaps: &[f64]) -> f64 {
    gaps.iter().sum()
}

fn measure(cells: &mut [Cell], at: usize, gap: f64, inset: f64) -> Result<(), PelersError> {
    let columns = cells[at].columns;
    let children = cells[at].children.clone();
    let mut heights: Vec<f64> = Vec::new();
    let mut position = 0.0;
    let mut cols: f64 = 0.0;
    let mut cw: f64 = 0.0;
    let mut tallest: f64 = 0.0;
    for &child in &children {
        let (px, py) = calculate_block_position(columns, position)?;
        let mut span = if cells[child].span >= 1.0 {
            js_min_f(cells[child].span, MAX_SPAN)
        } else {
            1.0
        };
        if columns > 0.0 && span > columns - px {
            span = columns - px;
        }
        cells[child].span = span;
        cells[child].col = px;
        cells[child].row = py;
        position += span;
        if px + span > cols {
            cols = px + span;
        }
        let row = py as usize;
        if row == heights.len() {
            heights.push(0.0);
        }
        if cells[child].children.is_empty() {
            heights[row] = -1.0;
        } else if heights[row] >= 0.0 && cells[child].nh > heights[row] {
            heights[row] = cells[child].nh;
        }
        if cells[child].space {
            continue;
        }
        let width = (cells[child].nw - gap * (span - 1.0)) / span;
        if width > cw {
            cw = width;
        }
        if cells[child].nh > tallest {
            tallest = cells[child].nh;
        }
    }
    let mut height = (heights.len() as f64 - 1.0) * gap + total(&cells[at].row_gaps);
    for h in heights.iter_mut() {
        if *h < 0.0 {
            *h = tallest;
        }
        height += *h;
    }
    let (cols_gaps, nw, head, inset_value) = (
        total(&cells[at].col_gaps),
        cells[at].nw,
        cells[at].head,
        inset,
    );
    cells[at].cols = cols;
    cells[at].heights = heights;
    cells[at].nw = js_max(
        nw,
        cols * cw + (cols - 1.0) * gap + cols_gaps + 2.0 * inset_value,
    );
    cells[at].nh = height + 2.0 * inset_value + head;
    Ok(())
}

fn js_min_f(a: f64, b: f64) -> f64 {
    crate::util::jsnum::js_min(a, b)
}

fn arrange(cells: &mut [Cell], at: usize, gap: f64, inset: f64) {
    let (cols, heights, row_gaps, col_gaps, bx, by, bw, bh, head) = {
        let b = &cells[at];
        (
            b.cols,
            b.heights.clone(),
            b.row_gaps.clone(),
            b.col_gaps.clone(),
            b.x,
            b.y,
            b.w,
            b.h,
            b.head,
        )
    };
    let rows = heights.len() as f64;
    let cw = (bw - 2.0 * inset - (cols - 1.0) * gap - total(&col_gaps)) / cols;
    let room = bh - 2.0 * inset - head - (rows - 1.0) * gap - total(&row_gaps);
    let mut offsets: Vec<f64> = Vec::new();
    if !col_gaps.is_empty() {
        let mut sum = 0.0;
        for col in 0..(cols as usize) {
            offsets.push(sum);
            sum += col_gaps.get(col).copied().unwrap_or(0.0);
        }
    }
    let needed: f64 = heights.iter().sum();
    let scale = if needed > 0.0 { room / needed } else { 0.0 };
    let share = if needed > 0.0 { 0.0 } else { room / rows };
    let left = bx - bw / 2.0 + inset;
    let mut top = by - bh / 2.0 + inset + head;
    let mut row = 0usize;
    let children = cells[at].children.clone();
    for child in children {
        let child_row = cells[child].row as usize;
        while row < child_row {
            top += heights[row] * scale + share + gap + row_gaps.get(row).copied().unwrap_or(0.0);
            row += 1;
        }
        let child_col = cells[child].col as usize;
        let span = cells[child].span;
        let before = offsets.get(child_col).copied().unwrap_or(0.0);
        let last = offsets
            .get((cells[child].col + span - 1.0) as usize)
            .copied()
            .unwrap_or(0.0);
        cells[child].w = cw * span + gap * (span - 1.0) + last - before;
        cells[child].h = heights[row] * scale + share;
        cells[child].x = left + cells[child].col * (cw + gap) + before + cells[child].w / 2.0;
        cells[child].y = top + cells[child].h / 2.0;
    }
}

pub fn layout(cells: &mut [Cell], root: usize, gap: f64) -> Result<(f64, f64), PelersError> {
    let mut order = vec![root];
    let mut i = 0;
    while i < order.len() {
        let children = cells[order[i]].children.clone();
        order.extend(children);
        i += 1;
    }
    for i in (0..order.len()).rev() {
        let at = order[i];
        if !cells[at].children.is_empty() {
            measure(cells, at, gap, if i == 0 { 0.0 } else { gap })?;
        }
    }
    cells[root].w = cells[root].nw;
    cells[root].h = cells[root].nh;
    cells[root].x = cells[root].w / 2.0;
    cells[root].y = cells[root].h / 2.0;
    for (i, &at) in order.iter().enumerate() {
        if !cells[at].children.is_empty() {
            arrange(cells, at, gap, if i == 0 { 0.0 } else { gap });
        }
    }
    Ok((cells[root].w, cells[root].h))
}
