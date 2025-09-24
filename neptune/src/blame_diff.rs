use diffs::myers;

/// Diff operation types for blame
#[derive(Debug, Clone, PartialEq)]
pub enum DiffOperation {
    Insert { line: usize, content: String },
    Delete { line: usize },
    Equal { old_line: usize, new_line: usize },
}

/// Collector for Myers diff operations
#[derive(Debug)]
pub struct EfficientDiffCollector<'a> {
    pub operations: Vec<DiffOperation>,
    pub new_lines: &'a [String],
}

impl<'a> EfficientDiffCollector<'a> {
    pub fn new(_old_lines: &'a [String], new_lines: &'a [String]) -> Self {
        Self {
            operations: Vec::new(),
            new_lines,
        }
    }

    pub fn into_operations(self) -> Vec<DiffOperation> {
        self.operations
    }
}

impl<'a> diffs::Diff for EfficientDiffCollector<'a> {
    type Error = ();

    fn equal(&mut self, old: usize, new: usize, len: usize) -> Result<(), Self::Error> {
        for i in 0..len {
            self.operations.push(DiffOperation::Equal {
                old_line: old + i + 1, // 转成 1-based
                new_line: new + i + 1,
            });
        }
        Ok(())
    }

    fn insert(&mut self, _old: usize, new: usize, len: usize) -> Result<(), Self::Error> {
        for i in 0..len {
            let idx = new + i;
            let content = if idx < self.new_lines.len() {
                self.new_lines[idx].clone()
            } else {
                String::new()
            };
            self.operations.push(DiffOperation::Insert {
                line: new + i + 1,
                content,
            });
        }
        Ok(())
    }

    fn delete(&mut self, old: usize, _new: usize, len: usize) -> Result<(), Self::Error> {
        for i in 0..len {
            self.operations.push(DiffOperation::Delete {
                line: old + i + 1,
            });
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Public API: compute diff operations using Myers algorithm
pub fn compute_diff(old_lines: &[String], new_lines: &[String]) -> Vec<DiffOperation> {
    let mut collector = EfficientDiffCollector::new(old_lines, new_lines);

    match myers::diff(
        &mut collector,
        old_lines,
        0,
        old_lines.len(),
        new_lines,
        0,
        new_lines.len(),
    ) {
        Ok(_) => collector.into_operations(),
        Err(_) => Vec::new(),
    }
}

