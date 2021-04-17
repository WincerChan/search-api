pub fn encode_result(result: String) -> Vec<u8> {
    let mut bytes: Vec<u8> = vec![0; result.len() + 1 + 8];
    let ret_len = result.len() as u32;
    // let mut bytes: Vec<u8> = Vec::with_capacity(result.len() + 1 + 8);
    bytes[9..].clone_from_slice(result.as_bytes());
    bytes[5..9].clone_from_slice(&ret_len.to_be_bytes());
    bytes[4] = 0;
    bytes[0..4].clone_from_slice(&(ret_len + 5).to_be_bytes());
    bytes
}
