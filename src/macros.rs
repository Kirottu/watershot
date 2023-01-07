/// Get the handle positions for altering the selection
#[macro_export]
macro_rules! handles {
    ($extents:expr) => {
        &[
            // Corners
            (
                $extents.start_x,
                $extents.start_y,
                $crate::types::SelectionModifier::TopLeft,
            ),
            (
                $extents.end_x,
                $extents.start_y,
                $crate::types::SelectionModifier::TopRight,
            ),
            (
                $extents.end_x,
                $extents.end_y,
                $crate::types::SelectionModifier::BottomRight,
            ),
            (
                $extents.start_x,
                $extents.end_y,
                $crate::types::SelectionModifier::BottomLeft,
            ),
            // Edges
            (
                $extents.start_x + ($extents.end_x - $extents.start_x) / 2,
                $extents.start_y,
                $crate::types::SelectionModifier::Top,
            ),
            (
                $extents.end_x,
                $extents.start_y + ($extents.end_y - $extents.start_y) / 2,
                $crate::types::SelectionModifier::Right,
            ),
            (
                $extents.start_x + ($extents.end_x - $extents.start_x) / 2,
                $extents.end_y,
                $crate::types::SelectionModifier::Bottom,
            ),
            (
                $extents.start_x,
                $extents.start_y + ($extents.end_y - $extents.start_y) / 2,
                $crate::types::SelectionModifier::Left,
            ),
        ]
    };
}
