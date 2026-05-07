use libubox::{BlobBuf, BlobmsgBuf, BlobmsgParser, BlobmsgType};

#[test]
fn blob_buf_root_starts_empty() {
    let b = BlobBuf::new().expect("init");
    let root = b.root();
    assert_eq!(root.id(), 0);
    assert_eq!(root.len(), 0);
    assert!(root.iter().next().is_none());
}

#[test]
fn blob_buf_add_string_and_iter() {
    let mut b = BlobBuf::new().expect("init");
    b.add_string(7, c"hello").expect("add_string");
    let root = b.root();
    let mut it = root.iter();
    let child = it.next().expect("child");
    assert_eq!(child.id(), 7);
    assert_eq!(child.data(), b"hello\0");
    assert!(it.next().is_none());
}

#[test]
fn blob_buf_add_primitives_roundtrip() {
    let mut b = BlobBuf::new().expect("init");
    b.add_u8(1, 0xAB).unwrap();
    b.add_u16(2, 0x1234).unwrap();
    b.add_u32(3, 0xDEADBEEF).unwrap();
    b.add_u64(4, 0x0123456789ABCDEF).unwrap();
    b.add_i32(5, -1).unwrap();

    let root = b.root();
    let kids: Vec<_> = root.iter().collect();
    assert_eq!(kids.len(), 5);
    assert_eq!(kids[0].id(), 1);
    assert_eq!(kids[0].data(), &[0xAB]);
    assert_eq!(kids[1].id(), 2);
    assert_eq!(kids[1].data(), &0x1234u16.to_be_bytes());
    assert_eq!(kids[2].id(), 3);
    assert_eq!(kids[2].data(), &0xDEADBEEFu32.to_be_bytes());
    assert_eq!(kids[3].id(), 4);
    assert_eq!(kids[3].data(), &0x0123456789ABCDEFu64.to_be_bytes());
    assert_eq!(kids[4].id(), 5);
    assert_eq!(kids[4].data(), &(-1i32 as u32).to_be_bytes());
}

#[test]
fn blob_buf_nested() {
    let mut b = BlobBuf::new().expect("init");
    b.nest(9, |inner| {
        inner.add_u32(1, 42)?;
        inner.add_u32(2, 100)?;
        Ok(())
    })
    .expect("nest");

    let root = b.root();
    let nested = root.iter().next().expect("nested attr");
    assert_eq!(nested.id(), 9);
    let inner_kids: Vec<_> = nested.iter().collect();
    assert_eq!(inner_kids.len(), 2);
    assert_eq!(inner_kids[0].id(), 1);
    assert_eq!(inner_kids[0].data(), &42u32.to_be_bytes());
    assert_eq!(inner_kids[1].id(), 2);
    assert_eq!(inner_kids[1].data(), &100u32.to_be_bytes());
}

#[test]
fn blobmsg_basic_roundtrip() {
    let mut b = BlobmsgBuf::new().expect("init");
    b.add_string(c"name", c"foo").unwrap();
    b.add_u32(c"count", 42).unwrap();
    b.add_bool(c"flag", true).unwrap();

    let root = b.root();
    assert_eq!(root.type_id(), Some(BlobmsgType::Table));
    let kids: Vec<_> = root.iter().collect();
    assert_eq!(kids.len(), 3);

    let by_name: std::collections::HashMap<_, _> = kids
        .iter()
        .map(|k| (k.name().to_str().unwrap().to_owned(), *k))
        .collect();

    assert_eq!(
        by_name["name"].as_str().unwrap().to_str().unwrap(),
        "foo"
    );
    assert_eq!(by_name["count"].as_u32(), Some(42));
    assert_eq!(by_name["flag"].as_bool(), Some(true));
}

#[test]
fn blobmsg_all_primitives_roundtrip() {
    let mut b = BlobmsgBuf::new().expect("init");
    b.add_u8(c"a", 0xAB).unwrap();
    b.add_u16(c"b", 0x1234).unwrap();
    b.add_u32(c"c", 0xDEADBEEF).unwrap();
    b.add_u64(c"d", 0x0123456789ABCDEF).unwrap();
    b.add_i32(c"e", -7).unwrap();
    b.add_double(c"f", 1.234567890123456).unwrap();
    b.add_string(c"g", c"hello").unwrap();
    b.add_bool(c"h", false).unwrap();

    let root = b.root();
    let by_name: std::collections::HashMap<_, _> = root
        .iter()
        .map(|k| (k.name().to_str().unwrap().to_owned(), k))
        .collect();

    assert_eq!(by_name["a"].as_u8(), Some(0xAB));
    assert_eq!(by_name["b"].as_u16(), Some(0x1234));
    assert_eq!(by_name["c"].as_u32(), Some(0xDEADBEEF));
    assert_eq!(by_name["d"].as_u64(), Some(0x0123456789ABCDEF));
    assert_eq!(by_name["e"].as_i32(), Some(-7));
    assert!((by_name["f"].as_double().unwrap() - 1.234567890123456).abs() < 1e-12);
    assert_eq!(
        by_name["g"].as_str().unwrap().to_str().unwrap(),
        "hello"
    );
    assert_eq!(by_name["h"].as_bool(), Some(false));
}

#[test]
fn blobmsg_nested_array_and_table() {
    let mut b = BlobmsgBuf::new().expect("init");
    b.add_string(c"name", c"foo").unwrap();
    b.add_u32(c"count", 42).unwrap();
    b.add_array(c"items", |arr| {
        arr.add_string(c"", c"a")?;
        arr.add_string(c"", c"b")?;
        Ok(())
    })
    .unwrap();
    b.add_table(c"nested", |t| {
        t.add_u32(c"inner", 7)?;
        Ok(())
    })
    .unwrap();

    let root = b.root();
    let kids: Vec<_> = root.iter().collect();
    assert_eq!(kids.len(), 4);

    let items = kids
        .iter()
        .find(|k| k.name().to_str() == Ok("items"))
        .unwrap()
        .as_array()
        .unwrap();
    let item_strs: Vec<String> = items
        .map(|i| i.as_str().unwrap().to_str().unwrap().to_owned())
        .collect();
    assert_eq!(item_strs, vec!["a", "b"]);

    let nested = kids
        .iter()
        .find(|k| k.name().to_str() == Ok("nested"))
        .unwrap()
        .as_table()
        .unwrap()
        .next()
        .unwrap();
    assert_eq!(nested.name().to_str().unwrap(), "inner");
    assert_eq!(nested.as_u32(), Some(7));
}

#[test]
fn blobmsg_parser_indexes_by_name() {
    let mut b = BlobmsgBuf::new().expect("init");
    b.add_string(c"name", c"foo").unwrap();
    b.add_u32(c"count", 42).unwrap();

    let root = b.root();

    let mut parser = BlobmsgParser::new();
    parser
        .field(c"name", BlobmsgType::String)
        .field(c"count", BlobmsgType::Int32)
        .field(c"missing", BlobmsgType::Int32);
    let out = parser.parse(root).expect("parse");

    assert_eq!(out.len(), 3);
    assert_eq!(
        out[0].expect("name present").as_str().unwrap().to_str().unwrap(),
        "foo"
    );
    assert_eq!(out[1].expect("count present").as_u32(), Some(42));
    assert!(out[2].is_none());
}

#[test]
fn blobmsg_parser_is_case_sensitive() {
    let mut b = BlobmsgBuf::new().expect("init");
    b.add_u32(c"Count", 1).unwrap();

    let root = b.root();
    let mut parser = BlobmsgParser::new();
    parser.field(c"count", BlobmsgType::Int32); // lowercase, doesn't match
    let out = parser.parse(root).expect("parse");
    assert!(out[0].is_none());
}

#[cfg(feature = "json")]
#[test]
fn blobmsg_json_roundtrip() {
    let mut b = BlobmsgBuf::new().expect("init");
    b.add_string(c"name", c"foo").unwrap();
    b.add_u32(c"count", 42).unwrap();
    b.add_array(c"items", |arr| {
        arr.add_string(c"", c"a")?;
        arr.add_string(c"", c"b")?;
        Ok(())
    })
    .unwrap();

    // Don't assert on exact JSON whitespace — libubox's formatter is
    // version-dependent. The round-trip parse below is the real check.
    let json = b.format_json();
    let mut b2 = BlobmsgBuf::new().expect("init");
    let json_c = std::ffi::CString::new(json).unwrap();
    b2.add_json_str(&json_c).expect("add_json_str");

    let root = b2.root();
    let by_name: std::collections::HashMap<_, _> = root
        .iter()
        .map(|k| (k.name().to_str().unwrap().to_owned(), k))
        .collect();
    assert_eq!(
        by_name["name"].as_str().unwrap().to_str().unwrap(),
        "foo"
    );
    assert_eq!(by_name["count"].as_u32(), Some(42));
}
