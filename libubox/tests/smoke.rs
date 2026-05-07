use libubox::{BlobBuf, Uloop, UloopError};

#[test]
fn uloop_round_trip_and_guard() {
    let u = Uloop::new().expect("first uloop");

    match Uloop::new() {
        Err(UloopError::AlreadyInitialized) => {}
        Err(other) => panic!("second concurrent Uloop returned wrong error: {other:?}"),
        Ok(_) => panic!("second concurrent Uloop succeeded; guard is broken"),
    }

    drop(u);
    let _u2 = Uloop::new().expect("uloop after drop");
}

#[test]
fn blob_buf_drops() {
    let _b = BlobBuf::new().expect("blob_buf_init");
}
