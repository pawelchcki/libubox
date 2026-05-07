use libubox_sys as sys;
use std::marker::PhantomData;

const HDR_SIZE: usize = 4;

#[inline]
pub(crate) fn pad4(n: usize) -> usize {
    n.next_multiple_of(4)
}

/// Borrowed view of a `blob_attr` payload. The header layout (7-bit id,
/// 1-bit extended flag, 24-bit length, all big-endian) is fixed and
/// re-implemented here in Rust — the C inline accessors (`blob_id`,
/// `blob_len`, ...) are not exported from libubox.
#[derive(Copy, Clone)]
pub struct BlobAttr<'a> {
    ptr: *const sys::blob_attr,
    _marker: PhantomData<&'a ()>,
}

impl<'a> BlobAttr<'a> {
    /// # Safety
    /// `ptr` must point to a valid, well-formed `blob_attr` whose declared
    /// size fits in the underlying allocation, and that allocation must
    /// outlive `'a`.
    pub unsafe fn from_raw(ptr: *const sys::blob_attr) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const sys::blob_attr {
        self.ptr
    }

    /// Decode the big-endian header word. The struct is `repr(C, packed)`,
    /// so `read_unaligned` tolerates any alignment the allocator chose.
    #[inline]
    fn id_len(&self) -> u32 {
        unsafe { u32::from_be(std::ptr::addr_of!((*self.ptr).id_len).read_unaligned()) }
    }

    #[inline]
    pub fn id(&self) -> u32 {
        (self.id_len() & sys::BLOB_ATTR_ID_MASK) >> sys::BLOB_ATTR_ID_SHIFT
    }

    #[inline]
    pub fn is_extended(&self) -> bool {
        self.id_len() & sys::BLOB_ATTR_EXTENDED != 0
    }

    /// Total on-wire size including the 4-byte header (no padding).
    #[inline]
    pub fn raw_len(&self) -> usize {
        (self.id_len() & sys::BLOB_ATTR_LEN_MASK) as usize
    }

    /// Padded total size including header (4-byte aligned).
    #[inline]
    pub fn pad_len(&self) -> usize {
        pad4(self.raw_len())
    }

    /// Length of `data()`.
    #[inline]
    pub fn len(&self) -> usize {
        self.raw_len().saturating_sub(HDR_SIZE)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Payload bytes (after the 4-byte attr header). For blobmsg-extended
    /// attrs this still includes the inline `blobmsg_hdr` (name length +
    /// name); use `BlobmsgAttr` for typed access.
    pub fn data(&self) -> &'a [u8] {
        let len = self.len();
        unsafe {
            let data_ptr = std::ptr::addr_of!((*self.ptr).data) as *const u8;
            std::slice::from_raw_parts(data_ptr, len)
        }
    }

    /// Iterate `self.data()` as a sequence of child `blob_attr`s. Stops
    /// (yields `None`) on a malformed child rather than panicking.
    pub fn iter(&self) -> BlobIter<'a> {
        let data = self.data();
        BlobIter {
            cur: data.as_ptr() as *const sys::blob_attr,
            rem: data.len(),
            _marker: PhantomData,
        }
    }
}

pub struct BlobIter<'a> {
    cur: *const sys::blob_attr,
    rem: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> BlobIter<'a> {
    pub(crate) fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            cur: bytes.as_ptr() as *const sys::blob_attr,
            rem: bytes.len(),
            _marker: PhantomData,
        }
    }
}

impl<'a> Iterator for BlobIter<'a> {
    type Item = BlobAttr<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rem < HDR_SIZE {
            return None;
        }
        // SAFETY: cur points into a slice of `rem` bytes (verified above).
        let attr = unsafe { BlobAttr::from_raw(self.cur) };
        let pad = attr.pad_len();
        if pad < HDR_SIZE || pad > self.rem {
            return None;
        }
        // SAFETY: pad <= rem so the advanced pointer stays within bounds.
        self.cur = unsafe { (self.cur as *const u8).add(pad) as *const sys::blob_attr };
        self.rem -= pad;
        Some(attr)
    }
}
