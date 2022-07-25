use crate::bits::Bits;

pub fn compress_varuint(num: u128) -> Bits {
    let mut bits = Bits::new();
    if num == 0 {
        bits.push(false);
        bits.push(false);
    } else if num == 1 {
        bits.push(true);
        bits.push(false);
    } else if num == 2 {
        bits.push(false);
        bits.push(true);
    } else {
        bits.push(true);
        bits.push(true);

        let mut tmp = num;
        let mut n_bits = 0;
        while tmp > 0 {
            tmp /= 2;
            n_bits += 1;
        }
        n_bits -= 2;

        bits.extend(&compress_varuint(n_bits));
        for i in 0..n_bits {
            bits.push(((num >> i) & 1) != 0);
        }
    }

    bits
}

pub fn compress_varint(num: i128) -> Bits {
    let mut bits = Bits::new();
    if num < 0 {
        bits.push(true);
        bits.extend(&compress_varuint((-num - 1) as u128));
    } else {
        bits.push(false);
        bits.extend(&compress_varuint(num as u128));
    }
    bits
}
