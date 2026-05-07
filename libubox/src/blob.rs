use libubox_sys as sys;
use std::ffi::CStr;
use std::fmt;
use std::os::raw::{c_int, c_void};

pub mod attr;

pub use attr::{BlobAttr, BlobIter};

#[derive(Debug)]
pub enum BlobError {
    /// `blob_buf_init` failed (libubox returns `-ENOMEM` from its initial
    /// `blob_add` allocation).
    InitFailed(i32),
    /// A `blob_put_*` / `blob_nest_start` could not grow the buffer. The
    /// `BlobBuf` is left in an undefined state — drop it.
    Oom,
    /// `blobmsg_parse` rejected the buffer.
    ParseFailed(i32),
    /// `blobmsg_add_json_from_string` rejected the input.
    #[cfg(feature = "json")]
    JsonParse,
}

impl fmt::Display for BlobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlobError::InitFailed(rc) => write!(f, "blob_buf_init failed: rc={rc}"),
            BlobError::Oom => f.write_str("blob_buf grow failed (out of memory)"),
            BlobError::ParseFailed(rc) => write!(f, "blobmsg_parse failed: rc={rc}"),
            #[cfg(feature = "json")]
            BlobError::JsonParse => f.write_str("blobmsg_add_json_from_string rejected input"),
        }
    }
}

impl std::error::Error for BlobError {}

/// Owned `blob_buf`. Calls `blob_buf_free` on drop. Use
/// [`as_mut_ptr`](Self::as_mut_ptr) to drive the `libubox_sys::blob_*` /
/// `blobmsg_*` FFI directly when you need behavior beyond the safe builders.
pub struct BlobBuf {
    inner: Box<sys::blob_buf>,
}

// SAFETY: `blob_buf`'s raw pointers (`head: *mut blob_attr`, `buf: *mut c_void`)
// make it `!Send` by default. `BlobBuf` uniquely owns both allocations and
// libubox does not stash thread-local references to either, so transferring
// ownership across threads is sound.
unsafe impl Send for BlobBuf {}

macro_rules! impl_add_int {
    ($name:ident, $t:ty) => {
        pub fn $name(&mut self, id: u32, value: $t) -> Result<(), BlobError> {
            self.add_bytes(id, &value.to_be_bytes())
        }
    };
}

impl BlobBuf {
    pub fn new() -> Result<Self, BlobError> {
        Self::with_root_id(0)
    }

    pub(crate) fn with_root_id(id: c_int) -> Result<Self, BlobError> {
        // SAFETY: `blob_buf` is repr(C) over function/data pointers — all-zero
        // is a valid bit pattern, and blob_buf_init expects exactly that.
        let mut inner: Box<sys::blob_buf> = Box::new(unsafe { std::mem::zeroed() });
        let rc = unsafe { sys::blob_buf_init(&mut *inner, id) };
        if rc != 0 {
            return Err(BlobError::InitFailed(rc));
        }
        Ok(Self { inner })
    }

    /// Raw pointer to the underlying `blob_buf`, for use with `libubox_sys`
    /// FFI calls. Stable for the lifetime of `self`; do not free.
    pub fn as_mut_ptr(&mut self) -> *mut sys::blob_buf {
        &mut *self.inner
    }

    pub fn as_ptr(&self) -> *const sys::blob_buf {
        &*self.inner
    }

    pub fn add_string(&mut self, id: u32, s: &CStr) -> Result<(), BlobError> {
        self.add_bytes(id, s.to_bytes_with_nul())
    }

    pub fn add_bytes(&mut self, id: u32, data: &[u8]) -> Result<(), BlobError> {
        // Any `*blob_attr` previously returned by libubox is invalidated by
        // grow-induced reallocs; we hand none out, so this stays sound.
        let attr = unsafe {
            sys::blob_put(
                self.as_mut_ptr(),
                id as c_int,
                data.as_ptr() as *const c_void,
                data.len() as u32,
            )
        };
        if attr.is_null() {
            Err(BlobError::Oom)
        } else {
            Ok(())
        }
    }

    impl_add_int!(add_u8, u8);
    impl_add_int!(add_u16, u16);
    impl_add_int!(add_u32, u32);
    impl_add_int!(add_u64, u64);
    impl_add_int!(add_i8, i8);
    impl_add_int!(add_i16, i16);
    impl_add_int!(add_i32, i32);
    impl_add_int!(add_i64, i64);

    /// Open a nested container of `id` and call `f` to populate it. The C
    /// cookie is an offset into `buf`, so it survives any reallocs `f`
    /// triggers via further `blob_put` / `blob_nest_start` calls.
    pub fn nest<F, R>(&mut self, id: u32, f: F) -> Result<R, BlobError>
    where
        F: FnOnce(&mut BlobBuf) -> Result<R, BlobError>,
    {
        let cookie = unsafe { sys::blob_nest_start(self.as_mut_ptr(), id as c_int) };
        if cookie.is_null() {
            return Err(BlobError::Oom);
        }
        let result = f(self);
        unsafe { sys::blob_nest_end(self.as_mut_ptr(), cookie) };
        result
    }

    /// Borrowed view of the buffer's root attribute.
    pub fn root(&self) -> BlobAttr<'_> {
        // SAFETY: blob_buf_init sets `head` to a valid attr; libubox only
        // nulls it in blob_buf_free (Drop), so it's live for `&self`.
        unsafe { BlobAttr::from_raw(self.inner.head) }
    }
}

impl Drop for BlobBuf {
    fn drop(&mut self) {
        // SAFETY: matched with successful blob_buf_init in new().
        unsafe {
            sys::blob_buf_free(&mut *self.inner);
        }
    }
}
