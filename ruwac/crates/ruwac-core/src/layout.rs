//! Persistent binary split tree. Geometry is shared by every client.
use serde_json::{Value, json};
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Layout {
    Leaf(String),
    Split {
        id: String,
        vertical: bool,
        ratio: u16,
        first: Box<Layout>,
        second: Box<Layout>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub rows: u16,
    pub cols: u16,
}
impl Layout {
    pub fn split(&mut self, pane: &str, new: String, vertical: bool) -> bool {
        match self {
            Self::Leaf(id) if id == pane => {
                *self = Self::Split {
                    id: format!("split_{new}"),
                    vertical,
                    ratio: 500,
                    first: Box::new(Self::Leaf(id.clone())),
                    second: Box::new(Self::Leaf(new)),
                };
                true
            }
            Self::Leaf(_) => false,
            Self::Split { first, second, .. } => {
                first.split(pane, new.clone(), vertical) || second.split(pane, new, vertical)
            }
        }
    }
    pub fn remove(self, pane: &str) -> Option<Self> {
        match self {
            Self::Leaf(id) => {
                if id == pane {
                    None
                } else {
                    Some(Self::Leaf(id))
                }
            }
            Self::Split {
                id,
                vertical,
                ratio,
                first,
                second,
            } => match (first.remove(pane), second.remove(pane)) {
                (Some(a), Some(b)) => Some(Self::Split {
                    id,
                    vertical,
                    ratio,
                    first: Box::new(a),
                    second: Box::new(b),
                }),
                (a, b) => a.or(b),
            },
        }
    }
    pub fn resize(&mut self, split: &str, value: u16) -> bool {
        match self {
            Self::Leaf(_) => false,
            Self::Split {
                id,
                ratio,
                first,
                second,
                ..
            } => {
                if id == split {
                    *ratio = value;
                    true
                } else {
                    first.resize(split, value) || second.resize(split, value)
                }
            }
        }
    }
    pub fn rects(&self, r: Rect, out: &mut Vec<(String, Rect)>) {
        match self {
            Self::Leaf(id) => out.push((id.clone(), r)),
            Self::Split {
                vertical,
                ratio,
                first,
                second,
                ..
            } => {
                let size = if *vertical { r.cols } else { r.rows };
                let available = size.max(2);
                let gap = u16::from(available >= 3);
                let content = available - gap;
                let n = ((content as u32 * *ratio as u32) / 1000)
                    .clamp(1, content.saturating_sub(1) as u32) as u16;
                let mut a = r;
                let mut b = r;
                if *vertical {
                    a.cols = n;
                    b.x += n + gap;
                    b.cols = content - n;
                } else {
                    a.rows = n;
                    b.y += n + gap;
                    b.rows = content - n;
                }
                first.rects(a, out);
                second.rects(b, out);
            }
        }
    }
    pub fn json(&self) -> Value {
        match self {
            Self::Leaf(p) => json!({"pane":p}),
            Self::Split {
                id,
                vertical,
                ratio,
                first,
                second,
            } => {
                json!({"id":id,"orientation":if *vertical{"vertical"}else{"horizontal"},"ratio":ratio,"first":first.json(),"second":second.json()})
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_divider_cells_are_reserved_without_overlapping_panes() {
        let mut layout = Layout::Leaf("a".into());
        assert!(layout.split("a", "b".into(), true));
        let mut rects = Vec::new();
        layout.rects(
            Rect {
                x: 0,
                y: 0,
                rows: 23,
                cols: 80,
            },
            &mut rects,
        );
        let a = rects[0].1;
        let b = rects[1].1;
        assert_eq!(a.cols + b.cols + 1, 80);
        assert_eq!(b.x, a.cols + 1);
        assert_eq!(a.rows, 23);
        assert_eq!(b.rows, 23);
        let collapsed = layout.remove("b").unwrap();
        rects.clear();
        collapsed.rects(
            Rect {
                x: 0,
                y: 0,
                rows: 23,
                cols: 80,
            },
            &mut rects,
        );
        assert_eq!(rects[0].1.cols, 80);
    }
    #[test]
    fn nested_layout_resize_remove() {
        let mut l = Layout::Leaf("a".into());
        assert!(l.split("a", "b".into(), true));
        assert!(l.split("b", "c".into(), false));
        let mut r = Vec::new();
        l.rects(
            Rect {
                x: 0,
                y: 0,
                rows: 24,
                cols: 80,
            },
            &mut r,
        );
        assert_eq!(r.len(), 3);
        assert!(r[1].1.x > 0);
        assert!(r[2].1.y > 0);
        assert!(l.resize("split_b", 700));
        let mut resized = Vec::new();
        l.rects(
            Rect {
                x: 0,
                y: 0,
                rows: 24,
                cols: 80,
            },
            &mut resized,
        );
        assert!(resized[0].1.cols > r[0].1.cols);
        let l = l.remove("b").unwrap();
        assert_eq!(l.json()["second"]["pane"], "c");
    }
}
