use super::DescribesWindow;

pub trait DescribesWindowClone {
    fn clone_box(&self) -> Box<dyn DescribesWindow>;
}

impl<T> DescribesWindowClone for T
where
    T: 'static + DescribesWindow + Clone,
{
    fn clone_box(&self) -> Box<dyn DescribesWindow> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn DescribesWindow> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
