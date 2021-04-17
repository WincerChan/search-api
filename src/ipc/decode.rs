use std::{convert::TryInto, io::Read, os::unix::net::UnixStream};

static LENGTH_SIZE: usize = 8;

fn extract_length(stream: &mut UnixStream) -> usize {
    let mut len_buf = vec![0; LENGTH_SIZE];
    stream.read(&mut len_buf).expect("err get length");
    usize::from_be_bytes(len_buf.try_into().expect("error to int"))
}

fn extract_string(offset: usize, raw_params: &Vec<u8>) -> (String, usize) {
    let offset = offset + 3; // 3 = 1(delimeter) + 2(type)
    let len_buf = &raw_params[offset..offset + LENGTH_SIZE];
    let length = usize::from_be_bytes(len_buf.try_into().expect("err to int"));
    let param_buf = &raw_params[LENGTH_SIZE + offset..length + LENGTH_SIZE + offset];
    (
        std::str::from_utf8(param_buf)
            .expect("err to get param")
            .to_owned(),
        (length + LENGTH_SIZE + offset) + 1,
    )
}

fn extract_integer(offset: usize, raw_params: &Vec<u8>) -> (i64, usize) {
    let offset = offset + 3;
    let param_buf = &raw_params[LENGTH_SIZE + offset..2 * LENGTH_SIZE + offset];
    (
        i64::from_be_bytes(param_buf.try_into().expect("err to ")),
        (2 * LENGTH_SIZE + offset),
    )
}

fn extract_string_list(offset: usize, raw_params: &Vec<u8>) -> (Vec<String>, usize) {
    let offset = offset + 3;
    let mut strings = Vec::with_capacity(raw_params[offset - 1] as usize);
    let len_buf = &raw_params[offset..offset + LENGTH_SIZE];
    let length = usize::from_be_bytes(len_buf.try_into().expect("err to int"));
    let param_buf = &raw_params[LENGTH_SIZE + offset..length + LENGTH_SIZE + offset];
    let mut sub_offset = 0;
    while sub_offset < param_buf.len() {
        let r = extract_string(sub_offset, &param_buf.to_vec());
        strings.push(r.0);
        sub_offset += r.1;
    }
    (strings, offset + sub_offset + LENGTH_SIZE)
}

fn extract_integer_list(offset: usize, raw_params: &Vec<u8>) -> (Vec<i64>, usize) {
    let offset = offset + 3;
    let mut integers = Vec::new();
    let len_buf = &raw_params[offset..offset + LENGTH_SIZE];
    let length = usize::from_be_bytes(len_buf.try_into().expect("err to int"));
    let param_buf = &raw_params[LENGTH_SIZE + offset..length + LENGTH_SIZE + offset];
    let mut sub_offset = 0;
    while sub_offset < param_buf.len() {
        let r = extract_integer(sub_offset, &param_buf.to_vec());
        integers.push(r.0);
        sub_offset = r.1;
    }
    println!("len integer defail{:?}", integers);
    println!("new offset {:?}", offset + sub_offset);
    (integers, offset + sub_offset + LENGTH_SIZE)
}

pub fn extract_params(
    stream: &mut UnixStream,
) -> Result<(Vec<i64>, Vec<i64>, Vec<String>, Vec<String>), ()> {
    let length = extract_length(stream);
    if 0 == length {
        return Err(());
    }
    let mut raw_params = vec![0; length];
    stream.read(&mut raw_params).expect("err get params");
    println!("{:?}", raw_params);
    let offset = 0;
    let (pages, offset) = extract_integer_list(offset, &raw_params);
    let (range, offset) = extract_integer_list(offset, &raw_params);
    let (terms, offset) = extract_string_list(offset, &raw_params);
    let (q, _) = extract_string_list(offset, &raw_params);
    Ok((pages, range, terms, q))
}
