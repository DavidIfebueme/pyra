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
}
