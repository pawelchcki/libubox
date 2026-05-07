//! Typed blobmsg builder and parser. The newtype `BlobmsgBuf` wraps a
//! `BlobBuf` whose root attribute id is `BLOBMSG_TYPE_TABLE`, so consumers
//! that parse the buffer see it as a top-level table.

use libubox_sys as sys;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::os::raw::{c_int, c_void};

use crate::blob::attr::{pad4, BlobIter};
use crate::blob::{BlobAttr, BlobBuf, BlobError};

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlobmsgType {
    Unspec = sys::blobmsg_type_BLOBMSG_TYPE_UNSPEC,
    Array = sys::blobmsg_type_BLOBMSG_TYPE_ARRAY,
    Table = sys::blobmsg_type_BLOBMSG_TYPE_TABLE,
    String = sys::blobmsg_type_BLOBMSG_TYPE_STRING,
    Int64 = sys::blobmsg_type_BLOBMSG_TYPE_INT64,
    Int32 = sys::blobmsg_type_BLOBMSG_TYPE_INT32,
    Int16 = sys::blobmsg_type_BLOBMSG_TYPE_INT16,
    /// Also used for `Bool`.
    Int8 = sys::blobmsg_type_BLOBMSG_TYPE_INT8,
    Double = sys::blobmsg_type_BLOBMSG_TYPE_DOUBLE,
}

impl BlobmsgType {
    pub fn from_raw(v: u32) -> Option<Self> {
        match v {
            sys::blobmsg_type_BLOBMSG_TYPE_UNSPEC => Some(Self::Unspec),
            sys::blobmsg_type_BLOBMSG_TYPE_ARRAY => Some(Self::Array),
            sys::blobmsg_type_BLOBMSG_TYPE_TABLE => Some(Self::Table),
            sys::blobmsg_type_BLOBMSG_TYPE_STRING => Some(Self::String),
            sys::blobmsg_type_BLOBMSG_TYPE_INT64 => Some(Self::Int64),
            sys::blobmsg_type_BLOBMSG_TYPE_INT32 => Some(Self::Int32),
            sys::blobmsg_type_BLOBMSG_TYPE_INT16 => Some(Self::Int16),
            sys::blobmsg_type_BLOBMSG_TYPE_INT8 => Some(Self::Int8),
            sys::blobmsg_type_BLOBMSG_TYPE_DOUBLE => Some(Self::Double),
            _ => None,
        }
    }
}

pub struct BlobmsgBuf(BlobBuf);

macro_rules! impl_add_be {
    ($name:ident, $t:ty, $ty_const:expr) => {
        pub fn $name(&mut self, name: &CStr, value: $t) -> Result<(), BlobError> {
            self.add_field($ty_const, name, &value.to_be_bytes())
        }
    };
}

impl BlobmsgBuf {
    pub fn new() -> Result<Self, BlobError> {
        BlobBuf::with_root_id(sys::blobmsg_type_BLOBMSG_TYPE_TABLE as c_int).map(Self)
    }

    pub fn as_mut_ptr(&mut self) -> *mut sys::blob_buf {
        self.0.as_mut_ptr()
    }

    pub fn as_ptr(&self) -> *const sys::blob_buf {
        self.0.as_ptr()
    }

    pub fn into_inner(self) -> BlobBuf {
        self.0
    }

    fn add_field(
        &mut self,
        ty: BlobmsgType,
        name: &CStr,
        data: &[u8],
    ) -> Result<(), BlobError> {
        let rc = unsafe {
            sys::blobmsg_add_field(
                self.as_mut_ptr(),
                ty as c_int,
                name.as_ptr(),
                data.as_ptr() as *const c_void,
                data.len() as u32,
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(BlobError::Oom)
        }
    }

    pub fn add_string(&mut self, name: &CStr, value: &CStr) -> Result<(), BlobError> {
        self.add_field(BlobmsgType::String, name, value.to_bytes_with_nul())
    }

    impl_add_be!(add_u8, u8, BlobmsgType::Int8);
    impl_add_be!(add_u16, u16, BlobmsgType::Int16);
    impl_add_be!(add_u32, u32, BlobmsgType::Int32);
    impl_add_be!(add_u64, u64, BlobmsgType::Int64);
    impl_add_be!(add_i8, i8, BlobmsgType::Int8);
    impl_add_be!(add_i16, i16, BlobmsgType::Int16);
    impl_add_be!(add_i32, i32, BlobmsgType::Int32);
    impl_add_be!(add_i64, i64, BlobmsgType::Int64);
    impl_add_be!(add_double, f64, BlobmsgType::Double);

    pub fn add_bool(&mut self, name: &CStr, value: bool) -> Result<(), BlobError> {
        self.add_u8(name, u8::from(value))
    }

    fn open_nested<F>(&mut self, name: &CStr, array: bool, f: F) -> Result<(), BlobError>
    where
        F: FnOnce(&mut BlobmsgBuf) -> Result<(), BlobError>,
    {
        // The cookie is an offset, not a pointer, so it survives reallocs
        // triggered inside `f` (mirrors blob_nest_start).
        let cookie =
            unsafe { sys::blobmsg_open_nested(self.as_mut_ptr(), name.as_ptr(), array) };
        if cookie.is_null() {
            return Err(BlobError::Oom);
        }
        let result = f(self);
        unsafe { sys::blob_nest_end(self.as_mut_ptr(), cookie) };
        result
    }

    pub fn add_array<F>(&mut self, name: &CStr, f: F) -> Result<(), BlobError>
    where
        F: FnOnce(&mut BlobmsgBuf) -> Result<(), BlobError>,
    {
        self.open_nested(name, true, f)
    }

    pub fn add_table<F>(&mut self, name: &CStr, f: F) -> Result<(), BlobError>
    where
        F: FnOnce(&mut BlobmsgBuf) -> Result<(), BlobError>,
    {
        self.open_nested(name, false, f)
    }

    /// Borrowed view of the buffer's root table.
    pub fn root(&self) -> BlobmsgAttr<'_> {
        BlobmsgAttr::from_blob(self.0.root())
    }

    /// Build the JSON representation of this buffer's root.
    #[cfg(feature = "json")]
    pub fn format_json(&self) -> String {
        self.root().format_json()
    }

    /// Append fields from a JSON object to this buffer.
    #[cfg(feature = "json")]
    pub fn add_json_str(&mut self, json: &CStr) -> Result<(), BlobError> {
        let ok =
            unsafe { sys::blobmsg_add_json_from_string(self.as_mut_ptr(), json.as_ptr()) };
        if ok {
            Ok(())
        } else {
            Err(BlobError::JsonParse)
        }
    }
}

/// Typed view of a blobmsg-extended `blob_attr`. Use [`BlobmsgBuf::root`]
/// for the root table, or iterate child fields via [`BlobmsgAttr::iter`].
#[derive(Copy, Clone)]
pub struct BlobmsgAttr<'a> {
    inner: BlobAttr<'a>,
}

macro_rules! impl_as_be {
    ($name:ident, $t:ty, $expected_ty:expr) => {
        pub fn $name(&self) -> Option<$t> {
            if self.type_id()? != $expected_ty {
                return None;
            }
            const N: usize = std::mem::size_of::<$t>();
            let arr: [u8; N] = self.payload().get(..N)?.try_into().ok()?;
            Some(<$t>::from_be_bytes(arr))
        }
    };
}

impl<'a> BlobmsgAttr<'a> {
    pub(crate) fn from_blob(inner: BlobAttr<'a>) -> Self {
        Self { inner }
    }

    pub fn raw(&self) -> BlobAttr<'a> {
        self.inner
    }

    pub fn type_id(&self) -> Option<BlobmsgType> {
        BlobmsgType::from_raw(self.inner.id())
    }

    /// Field name. For non-extended attrs (notably the root table set up by
    /// `blob_buf_init(BLOBMSG_TYPE_TABLE)`) this is an empty CStr.
    pub fn name(&self) -> &'a CStr {
        if !self.inner.is_extended() {
            return c"";
        }
        let data = self.inner.data();
        if data.len() < 2 {
            return c"";
        }
        let namelen = u16::from_be_bytes([data[0], data[1]]) as usize;
        let end = 2 + namelen + 1;
        if end > data.len() || data[2 + namelen] != 0 {
            return c"";
        }
        unsafe { CStr::from_bytes_with_nul_unchecked(&data[2..end]) }
    }

    /// Bytes after the inline blobmsg header. For non-extended attrs this is
    /// just `self.raw().data()`.
    pub fn payload(&self) -> &'a [u8] {
        let data = self.inner.data();
        if !self.inner.is_extended() || data.len() < 2 {
            return data;
        }
        let namelen = u16::from_be_bytes([data[0], data[1]]) as usize;
        // blobmsg_hdrlen: pad4(sizeof(hdr) + namelen + 1) where sizeof(hdr) = 2
        let hdr_len = pad4(2 + namelen + 1);
        if hdr_len >= data.len() {
            return &[];
        }
        &data[hdr_len..]
    }

    pub fn as_str(&self) -> Option<&'a CStr> {
        if self.type_id()? != BlobmsgType::String {
            return None;
        }
        let p = self.payload();
        if p.last() != Some(&0) {
            return None;
        }
        Some(unsafe { CStr::from_bytes_with_nul_unchecked(p) })
    }

    impl_as_be!(as_u8, u8, BlobmsgType::Int8);
    impl_as_be!(as_u16, u16, BlobmsgType::Int16);
    impl_as_be!(as_u32, u32, BlobmsgType::Int32);
    impl_as_be!(as_u64, u64, BlobmsgType::Int64);
    impl_as_be!(as_double, f64, BlobmsgType::Double);

    pub fn as_bool(&self) -> Option<bool> {
        Some(self.as_u8()? != 0)
    }

    pub fn as_i8(&self) -> Option<i8> {
        self.as_u8().map(|v| v as i8)
    }
    pub fn as_i16(&self) -> Option<i16> {
        self.as_u16().map(|v| v as i16)
    }
    pub fn as_i32(&self) -> Option<i32> {
        self.as_u32().map(|v| v as i32)
    }
    pub fn as_i64(&self) -> Option<i64> {
        self.as_u64().map(|v| v as i64)
    }

    fn iter_if(&self, expected: BlobmsgType) -> Option<BlobmsgIter<'a>> {
        (self.type_id()? == expected).then(|| self.iter())
    }

    /// The root table built by `blob_buf_init` has type Table but isn't
    /// extended; `iter()` works for either case.
    pub fn as_table(&self) -> Option<BlobmsgIter<'a>> {
        self.iter_if(BlobmsgType::Table)
    }

    pub fn as_array(&self) -> Option<BlobmsgIter<'a>> {
        self.iter_if(BlobmsgType::Array)
    }

    /// Iterate child fields. Children of an extended attr are themselves
    /// blobmsg-extended; for the (non-extended) root they are too.
    pub fn iter(&self) -> BlobmsgIter<'a> {
        BlobmsgIter {
            inner: BlobIter::from_bytes(self.payload()),
        }
    }

    /// Build the JSON representation of this attribute.
    #[cfg(feature = "json")]
    pub fn format_json(&self) -> String {
        // libubox returns a malloc'd C string we must free ourselves.
        let s = unsafe {
            sys::blobmsg_format_json_with_cb(
                self.inner.as_ptr() as *mut _,
                true,
                None,
                std::ptr::null_mut(),
                0,
            )
        };
        if s.is_null() {
            return String::new();
        }
        let cstr = unsafe { CStr::from_ptr(s) };
        let result = cstr.to_string_lossy().into_owned();
        unsafe { libc::free(s as *mut _) };
        result
    }
}

/// Iterator over blobmsg child attrs.
pub struct BlobmsgIter<'a> {
    inner: BlobIter<'a>,
}

impl<'a> Iterator for BlobmsgIter<'a> {
    type Item = BlobmsgAttr<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(BlobmsgAttr::from_blob)
    }
}

/// Builder for a fixed-shape parser. Insertion order is preserved in the
/// output of [`parse`](Self::parse), so callers can pattern-match on the
/// returned slice.
pub struct BlobmsgParser<'p> {
    policies: Vec<sys::blobmsg_policy>,
    _marker: PhantomData<&'p CStr>,
}

impl<'p> BlobmsgParser<'p> {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn field(&mut self, name: &'p CStr, ty: BlobmsgType) -> &mut Self {
        self.policies.push(sys::blobmsg_policy {
            name: name.as_ptr(),
            type_: ty as sys::blobmsg_type,
        });
        self
    }

    pub fn parse<'a>(
        &self,
        src: BlobmsgAttr<'a>,
    ) -> Result<Vec<Option<BlobmsgAttr<'a>>>, BlobError> {
        let payload = src.payload();
        let mut tb: Vec<*mut sys::blob_attr> =
            vec![std::ptr::null_mut(); self.policies.len()];
        let rc = unsafe {
            sys::blobmsg_parse(
                self.policies.as_ptr(),
                self.policies.len() as c_int,
                tb.as_mut_ptr(),
                payload.as_ptr() as *mut c_void,
                payload.len() as u32,
            )
        };
        if rc != 0 {
            return Err(BlobError::ParseFailed(rc));
        }
        Ok(tb
            .into_iter()
            .map(|p| {
                if p.is_null() {
                    None
                } else {
                    // SAFETY: blobmsg_parse only writes pointers into `src`'s
                    // bounds (validated by it). They live as long as 'a.
                    Some(unsafe { BlobmsgAttr::from_blob(BlobAttr::from_raw(p)) })
                }
            })
            .collect())
    }
}

impl<'p> Default for BlobmsgParser<'p> {
    fn default() -> Self {
        Self::new()
    }
}
