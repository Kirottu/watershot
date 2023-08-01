// {x},{y} {w}x{h}
#[derive(Debug, PartialEq)]
pub struct NotSlurpStyleError;

// {w}x{h}+{x}+{y}
#[derive(Debug, PartialEq)]
pub struct NotXRectSelStyleError;

#[derive(Debug, PartialEq)]
pub struct UnparseableRectError;
