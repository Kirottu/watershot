/// Get the handle positions for altering the selection
#[macro_export]
macro_rules! handles {
    ($extents:expr) => {
        &[
            // Corners
            (
                $extents.start_x,
                $extents.start_y,
                SelectionModifier::TopLeft,
            ),
            (
                $extents.end_x,
                $extents.start_y,
                SelectionModifier::TopRight,
            ),
            (
                $extents.end_x,
                $extents.end_y,
                SelectionModifier::BottomRight,
            ),
            (
                $extents.start_x,
                $extents.end_y,
                SelectionModifier::BottomLeft,
            ),
            // Edges
            (
                $extents.start_x + ($extents.end_x - $extents.start_x) / 2,
                $extents.start_y,
                SelectionModifier::Top,
            ),
            (
                $extents.end_x,
                $extents.start_y + ($extents.end_y - $extents.start_y) / 2,
                SelectionModifier::Right,
            ),
            (
                $extents.start_x + ($extents.end_x - $extents.start_x) / 2,
                $extents.end_y,
                SelectionModifier::Bottom,
            ),
            (
                $extents.start_x,
                $extents.start_y + ($extents.end_y - $extents.start_y) / 2,
                SelectionModifier::Left,
            ),
        ]
    };
}
