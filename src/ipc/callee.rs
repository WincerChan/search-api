use std::{convert::TryInto, io::Read, os::unix::net::UnixStream};

static LENGTH_SIZE: usize = 4;

fn extract_length(stream: &mut UnixStream) -> usize {
    let mut len_buf = vec![0; LENGTH_SIZE];
    stream.read(&mut len_buf).expect("err get length");
    usize::from_be_bytes(len_buf.try_into().expect("error to int"))
}

fn extract_string(offset: usize, raw_params: &Vec<u8>) -> (String, usize) {
    let len_buf = &raw_params[offset..offset + LENGTH_SIZE];
    let length = usize::from_be_bytes(len_buf.try_into().expect("err to int"));
    let param_buf = &raw_params[LENGTH_SIZE + offset..length + LENGTH_SIZE + offset];
    (
        String::from_utf8(param_buf.to_owned()).expect("err to get param"),
        length + LENGTH_SIZE + offset,
    )
}

fn extract_integer(offset: usize, raw_params: Vec<u8>) -> i32 {
    let len_buf = &raw_params[..4];
    let length = usize::from_be_bytes(len_buf.try_into().expect("err to int"));
    let param_buf = &raw_params[4..length + 4];
    i32::from_be_bytes(param_buf.try_into().expect("err to "))
}

fn extract_params(stream: &mut UnixStream) -> (String, String, String, String) {
    let length = extract_length(stream);
    let mut raw_params = vec![0; length];
    stream.read(&mut raw_params).expect("err get params");
    let offset = 1;
    let (pages, offset) = extract_string(offset, &raw_params);
    let (terms, offset) = extract_string(offset + 1, &raw_params);
    let (q, offset) = extract_string(offset + 1, &raw_params);
    let (range, _) = extract_string(offset + 1, &raw_params);
    (pages, terms, q, range)
}
