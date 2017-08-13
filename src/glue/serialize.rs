use byteorder::{LittleEndian, WriteBytesExt};

pub fn std_map<'a, T, K, V>(hm: T, len: usize) -> Vec<u8> where
    T: Iterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>
{
    let mut ret = Vec::new();

    write_obj_type(&mut ret, "map");

    ret.write_u32::<LittleEndian>(len as u32).unwrap();

    for (k, v) in hm {
        write_str(&mut ret, k.as_ref());
        write_str(&mut ret, v.as_ref());
    }

    ret
}

#[allow(dead_code)]
pub fn std_array<'a, T, V>(v: T, len: usize) -> Vec<u8> where
    T: Iterator<Item = V>,
    V: AsRef<str>
{
    let mut ret = Vec::new();

    write_obj_type(&mut ret, "array");

    ret.write_u32::<LittleEndian>(len as u32).unwrap();

    for item in v {
        write_str(&mut ret, item.as_ref());
    }

    ret
}

fn write_obj_type(b: &mut Vec<u8>, t: &str) {
    b.write_u16::<LittleEndian>(t.len() as u16).unwrap();
    b.extend_from_slice(t.as_bytes());
}

fn write_str(b: &mut Vec<u8>, s: &str) {
    b.write_u32::<LittleEndian>(s.len() as u32).unwrap();
    b.extend_from_slice(s.as_bytes());
    b.push(0);
}
