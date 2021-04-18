pub fn encode_result(result: String) -> Vec<u8> {
    let mut bytes: Vec<u8> = vec![0; result.len() + 2 + 4 * 2];
    let ret_len = result.len() as u32;
    bytes[10..].clone_from_slice(result.as_bytes());
    bytes[6..10].clone_from_slice(&ret_len.to_be_bytes());
    bytes[0..4].clone_from_slice(&(ret_len + 6).to_be_bytes());
    bytes
}
