/// Fixed-capacity circular buffer for sparkline history.
#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    buf: Vec<T>,
    cap: usize,
    head: usize, // next write position
    len: usize,
}

impl<T: Clone + Default> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: vec![T::default(); capacity],
            cap: capacity,
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, val: T) {
        self.buf[self.head] = val;
        self.head = (self.head + 1) % self.cap;
        if self.len < self.cap {
            self.len += 1;
        }
    }

    /// Return data oldest→newest.
    pub fn as_vec(&self) -> Vec<T> {
        if self.len < self.cap {
            // haven't wrapped yet
            self.buf[..self.len].to_vec()
        } else {
            let mut out = Vec::with_capacity(self.cap);
            out.extend_from_slice(&self.buf[self.head..]);
            out.extend_from_slice(&self.buf[..self.head]);
            out
        }
    }

}
