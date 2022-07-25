#[derive(Clone, Debug)]
pub struct Bits {
    bits: Vec<bool>,
}

impl Bits {
    pub fn new() -> Self {
        Self {
            bits: Vec::new(),
        }
    }

    pub fn push(&mut self, x: bool) {
        self.bits.push(x);
    }

    pub fn extend(&mut self, rhs: &Bits) {
        for x in &rhs.bits {
            self.push(*x);
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = Vec::new();
        for chunk in self.bits.chunks(8) {
            let mut byte = 0u8;
            for b in chunk {
                byte <<= 1;
                byte |= *b as u8;
            }
            res.push(byte);
        }
        res
    }

    pub fn len(&self) -> usize {
        self.bits.len()
    }
}

