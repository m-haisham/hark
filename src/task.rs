use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(usize);

impl TaskId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task-{}", self.0)
    }
}
