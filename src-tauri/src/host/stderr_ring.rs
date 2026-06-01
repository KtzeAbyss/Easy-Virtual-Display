use std::collections::VecDeque;

/// Bounded FIFO buffer keeping the last `capacity` non-empty stderr lines.
/// Mirrors `SidecarState.lastStderrLines` in `src/main/sidecar-listener.ts`,
/// which keeps the trailing 50 lines for inclusion in `native_host_unavailable` errors.
#[derive(Debug, Clone)]
pub struct StderrRing {
    capacity: usize,
    lines: VecDeque<String>,
}

impl StderrRing {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "StderrRing capacity must be positive");
        Self {
            capacity,
            lines: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, line: String) {
        if line.trim().is_empty() {
            return;
        }
        if self.lines.len() == self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    #[cfg(test)]
    pub fn snapshot(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }

    pub fn joined(&self) -> String {
        self.lines
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_oldest_when_capacity_exceeded() {
        let mut ring = StderrRing::new(3);
        ring.push("a".into());
        ring.push("b".into());
        ring.push("c".into());
        ring.push("d".into());
        assert_eq!(ring.snapshot(), vec!["b", "c", "d"]);
    }

    #[test]
    fn keeps_exactly_last_n_when_filled() {
        let mut ring = StderrRing::new(50);
        for i in 0..200 {
            ring.push(format!("line {i}"));
        }
        let snap = ring.snapshot();
        assert_eq!(snap.len(), 50);
        assert_eq!(snap.first().unwrap(), "line 150");
        assert_eq!(snap.last().unwrap(), "line 199");
    }

    #[test]
    fn ignores_blank_lines() {
        let mut ring = StderrRing::new(3);
        ring.push("".into());
        ring.push("   ".into());
        ring.push("real".into());
        assert_eq!(ring.snapshot(), vec!["real"]);
    }

    #[test]
    fn joined_uses_newline_separator() {
        let mut ring = StderrRing::new(3);
        ring.push("a".into());
        ring.push("b".into());
        assert_eq!(ring.joined(), "a\nb");
    }

    #[test]
    fn clear_resets_lines() {
        let mut ring = StderrRing::new(3);
        ring.push("x".into());
        ring.clear();
        assert!(ring.is_empty());
    }
}
