use libubox_sys::*;
use std::mem::MaybeUninit;

#[test]
fn uloop_init_and_done() {
    assert_eq!(unsafe { uloop_init() }, 0);
    unsafe {
        uloop_done();
    }
}

#[test]
fn blob_buf_lifecycle() {
    let mut buf: MaybeUninit<blob_buf> = MaybeUninit::zeroed();
    assert_eq!(unsafe { blob_buf_init(buf.as_mut_ptr(), 0) }, 0);
    unsafe {
        blob_buf_free(buf.as_mut_ptr());
    }
}

#[cfg(feature = "json")]
#[test]
fn blobmsg_json_symbol_present() {
    // `blobmsg_format_json` itself is `static inline` and so isn't emitted
    // by bindgen; check that the underlying extern is linked.
    let _f = blobmsg_format_json_with_cb as *const () as usize;
}
