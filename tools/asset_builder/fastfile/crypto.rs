pub const PC_KEY: [u8; 32] = [
    0x64, 0x1d, 0x8a, 0x2f, 0xe3, 0x1d, 0x3a, 0xa6, 0x36, 0x22, 0xbb, 0xc9, 0xce, 0x85, 0x87, 0x22,
    0x9d, 0x42, 0xb0, 0xf8, 0xed, 0x9b, 0x92, 0x41, 0x30, 0xbf, 0x88, 0xb6, 0x5e, 0xdc, 0x50, 0xbe,
];

pub fn salsa20(data: &mut [u8], nonce: &[u8; 8]) {
    let word = |bytes: &[u8]| u32::from_le_bytes(bytes.try_into().unwrap());
    let constants = b"expand 32-byte k";
    let mut state = [0_u32; 16];
    for i in 0..4 {
        state[i * 5] = word(&constants[i * 4..i * 4 + 4]);
        state[i + 1] = word(&PC_KEY[i * 4..i * 4 + 4]);
        state[i + 11] = word(&PC_KEY[i * 4 + 16..i * 4 + 20]);
    }
    state[6] = word(&nonce[..4]);
    state[7] = word(&nonce[4..]);
    for bytes in data.chunks_mut(64) {
        let mut x = state;
        for _ in 0..10 {
            for [a, b, c, d] in [
                [0, 4, 8, 12],
                [5, 9, 13, 1],
                [10, 14, 2, 6],
                [15, 3, 7, 11],
                [0, 1, 2, 3],
                [5, 6, 7, 4],
                [10, 11, 8, 9],
                [15, 12, 13, 14],
            ] {
                x[b] ^= x[a].wrapping_add(x[d]).rotate_left(7);
                x[c] ^= x[b].wrapping_add(x[a]).rotate_left(9);
                x[d] ^= x[c].wrapping_add(x[b]).rotate_left(13);
                x[a] ^= x[d].wrapping_add(x[c]).rotate_left(18);
            }
        }
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte ^= x[i / 4].wrapping_add(state[i / 4]).to_le_bytes()[i % 4];
        }
        state[8] = state[8].wrapping_add(1);
        if state[8] == 0 {
            state[9] = state[9].wrapping_add(1);
        }
    }
}

pub fn sha1(input: &[u8]) -> [u8; 20] {
    let mut padded = input.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&((input.len() as u64) * 8).to_be_bytes());
    let mut state = [
        0x67452301_u32,
        0xefcdab89,
        0x98badcfe,
        0x10325476,
        0xc3d2e1f0,
    ];
    for block in padded.chunks_exact(64) {
        let mut words = [0_u32; 80];
        for (out, bytes) in words[..16].iter_mut().zip(block.chunks_exact(4)) {
            *out = u32::from_be_bytes(bytes.try_into().unwrap());
        }
        for i in 16..80 {
            words[i] = (words[i - 3] ^ words[i - 8] ^ words[i - 14] ^ words[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = state;
        for (i, word) in words.into_iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5a827999_u32),
                20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let next = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = next;
        }
        for (s, v) in state.iter_mut().zip([a, b, c, d, e]) {
            *s = s.wrapping_add(v);
        }
    }
    let mut result = [0_u8; 20];
    for (out, value) in result.chunks_exact_mut(4).zip(state) {
        out.copy_from_slice(&value.to_be_bytes());
    }
    result
}

pub struct FastfileCipher {
    ivs: [u8; 16000],
    counters: [usize; 4],
    stream: usize,
}

impl FastfileCipher {
    pub fn new(zone: &str) -> Result<Self, &'static str> {
        if zone.is_empty() || zone.len() >= 32 || !zone.is_ascii() || zone.as_bytes().contains(&0) {
            return Err("invalid fastfile zone name");
        }
        let mut ivs = [0_u8; 16000];
        for (i, out) in ivs.iter_mut().enumerate() {
            *out = zone.as_bytes()[(i / 4) % zone.len()];
        }
        Ok(Self {
            ivs,
            counters: [1; 4],
            stream: 0,
        })
    }

    pub fn decrypt(&mut self, compressed: &mut [u8]) {
        let j = self.stream;
        let index = (((self.counters[j] - 1) * 4 + j) % 800) * 20;
        let nonce = self.ivs[index..index + 8].try_into().unwrap();
        salsa20(compressed, nonce);
        let hash = sha1(compressed);
        let next = ((self.counters[j] * 4 + j) % 800) * 20;
        for (out, value) in self.ivs[next..next + 20].iter_mut().zip(hash) {
            *out ^= value;
        }
        self.counters[j] += 1;
        self.stream = (j + 1) % 4;
    }
}
