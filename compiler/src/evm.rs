pub fn runtime_return_word(word: [u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + 32 + 2 + 2 + 1);
    out.push(0x7f);
    out.extend_from_slice(&word);
    out.push(0x60);
    out.push(0x00);
    out.push(0x52);
    out.push(0x60);
    out.push(0x20);
    out.push(0x60);
    out.push(0x00);
    out.push(0xf3);
    out
}

pub fn init_return_runtime(runtime: &[u8]) -> Vec<u8> {
    let mut offset = 0usize;

    for _ in 0..8 {
        let mut prefix = Vec::new();
        prefix.extend(push_usize(runtime.len()));
        prefix.extend(push_usize(offset));
        prefix.extend(push_usize(0));
        prefix.push(0x39);
        prefix.extend(push_usize(runtime.len()));
        prefix.extend(push_usize(0));
        prefix.push(0xf3);

        let new_offset = prefix.len();
        if new_offset == offset {
            let mut out = prefix;
            out.extend_from_slice(runtime);
            return out;
        }

        offset = new_offset;
    }

    let mut prefix = Vec::new();
    prefix.extend(push_usize(runtime.len()));
    prefix.extend(push_usize(offset));
    prefix.extend(push_usize(0));
    prefix.push(0x39);
    prefix.extend(push_usize(runtime.len()));
    prefix.extend(push_usize(0));
    prefix.push(0xf3);
    prefix.extend_from_slice(runtime);
    prefix
}

fn push_usize(value: usize) -> Vec<u8> {
    if value == 0 {
        return vec![0x60, 0x00];
    }

    let mut buf = [0u8; 32];
    let mut v = value;
    let mut i = 32;
    while v > 0 {
        i -= 1;
        buf[i] = (v & 0xff) as u8;
        v >>= 8;
    }

    let n = 32 - i;
    let mut out = Vec::with_capacity(1 + n);
    out.push(0x5f + (n as u8));
    out.extend_from_slice(&buf[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_runtime_return() {
        let mut word = [0u8; 32];
        word[31] = 1;
        let code = runtime_return_word(word);
        assert_eq!(code[0], 0x7f);
        assert_eq!(code[33], 0x60);
        assert_eq!(code[34], 0x00);
        assert_eq!(code[35], 0x52);
        assert_eq!(code[36], 0x60);
        assert_eq!(code[37], 0x20);
        assert_eq!(code[38], 0x60);
        assert_eq!(code[39], 0x00);
        assert_eq!(code[40], 0xf3);
        assert_eq!(code.len(), 41);
    }

    #[test]
    fn init_code_appends_runtime() {
        let mut word = [0u8; 32];
        word[31] = 1;
        let runtime = runtime_return_word(word);
        let init = init_return_runtime(&runtime);

        assert!(init.ends_with(&runtime));
        let runtime_start = init.len() - runtime.len();
        assert_eq!(init[runtime_start], 0x7f);
        assert_eq!(init[runtime_start - 1], 0xf3);
        assert!(init[..runtime_start].contains(&0x39));
    }
}
