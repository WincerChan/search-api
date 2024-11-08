use std::{convert::TryInto, io::Read};

static LENGTH_SIZE: usize = 4;
static INTEGER_SIZE: usize = 8;
static DATA_OFFSET: usize = 1; // 3 = 1 (delimeter) + 1 (basic or compound) + 1 (contains element type)

fn extract_length<T: Read>(stream: &mut T) -> usize {
    let mut len_buf = vec![0; LENGTH_SIZE];
    stream.read(&mut len_buf).expect("err get length");
    u32::from_be_bytes(len_buf.try_into().expect("error to int")) as usize
}

fn extract_string(raw_params: &[u8]) -> (String, &[u8]) {
    let offset = DATA_OFFSET;
    let len_buf = &raw_params[offset..offset + LENGTH_SIZE];
    let length = u32::from_be_bytes(len_buf.try_into().expect("err to int")) as usize;
    let param_buf = &raw_params[LENGTH_SIZE + offset..length + LENGTH_SIZE + offset];
    (
        std::str::from_utf8(param_buf)
            .expect("err to get param")
            .to_owned(),
        &raw_params[length + LENGTH_SIZE + offset..],
    )
}

fn extract_integer(raw_params: &[u8]) -> (i64, &[u8]) {
    let offset = DATA_OFFSET;
    let param_buf = &raw_params[LENGTH_SIZE + offset..INTEGER_SIZE + LENGTH_SIZE + offset];
    (
        i64::from_be_bytes(param_buf.try_into().expect("err to ")),
        &raw_params[INTEGER_SIZE + LENGTH_SIZE + offset..],
    )
}

fn extract_string_list(raw_params: &[u8]) -> (Vec<String>, &[u8]) {
    let offset = DATA_OFFSET;
    let mut strings = Vec::new();
    let len_buf = &raw_params[offset..offset + LENGTH_SIZE];
    let length = u32::from_be_bytes(len_buf.try_into().expect("err to int")) as usize;
    let mut param_buf = &raw_params[LENGTH_SIZE + offset..length + LENGTH_SIZE + offset];
    while param_buf.len() > 0 {
        let r = extract_string(&param_buf);
        strings.push(r.0);
        param_buf = r.1;
    }
    (strings, &raw_params[length + LENGTH_SIZE + offset..])
}

fn extract_integer_list(raw_params: &[u8]) -> (Vec<i64>, &[u8]) {
    let offset = DATA_OFFSET;
    let mut integers = Vec::new();
    let len_buf = &raw_params[offset..offset + LENGTH_SIZE];
    let length = u32::from_be_bytes(len_buf.try_into().expect("err to int")) as usize;
    let mut param_buf = &raw_params[LENGTH_SIZE + offset..length + LENGTH_SIZE + offset];
    while param_buf.len() > 0 {
        let r = extract_integer(&param_buf);
        integers.push(r.0);
        param_buf = r.1;
    }
    (integers, &raw_params[length + LENGTH_SIZE + offset..])
}

pub fn extract_params<T: Read>(
    stream: &mut T,
) -> Result<(Vec<i64>, Vec<i64>, Vec<String>, Vec<String>), ()> {
    let length = extract_length(stream);
    if 0 == length {
        return Err(());
    }
    let mut raw_params = vec![0; length];
    stream.read(&mut raw_params).expect("err get params");
    let raw_params = &raw_params[0..];
    let (pages, raw_params) = extract_integer_list(raw_params);
    let (range, raw_params) = extract_integer_list(raw_params);
    let (terms, raw_params) = extract_string_list(raw_params);
    let (q, _) = extract_string_list(raw_params);
    Ok((
        pages,
        range,
        terms,
        q.into_iter().filter(|x| !x.is_empty()).collect(),
    ))
}
