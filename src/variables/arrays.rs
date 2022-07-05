//! Iterators to get array items.

use crate::ffi::variables as ffi;
use std::convert::TryFrom;
use std::os::raw::c_char;

/// Iterator to get items in an indexed array.
pub(super) struct ArrayItemsIterator<'a> {
    array: &'a ffi::Array,
    elem: *const ffi::ArrayElement,
}

impl ArrayItemsIterator<'_> {
    pub(super) unsafe fn new(array: &ffi::Array) -> ArrayItemsIterator {
        ArrayItemsIterator {
            array,
            elem: (*array.head).next,
        }
    }
}

impl Iterator for ArrayItemsIterator<'_> {
    type Item = *const c_char;

    fn size_hint(&self) -> (usize, Option<usize>) {
        match usize::try_from(self.array.num_elements) {
            Ok(n) => (n, Some(n)),
            Err(_) => (0, None),
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.elem == self.array.head {
            return None;
        }

        let current = unsafe { &(*self.elem) };
        let value = current.value;
        self.elem = current.next;
        Some(value)
    }
}

/// Iterator to get items in an associative array.
pub(super) struct AssocItemsIterator<'a> {
    table: &'a ffi::HashTable,
    num_bucket: isize,
    current_bucket_item: Option<*const ffi::BucketContents>,
}

impl AssocItemsIterator<'_> {
    pub(super) unsafe fn new(table: &ffi::HashTable) -> AssocItemsIterator {
        AssocItemsIterator {
            table,
            num_bucket: 0,
            current_bucket_item: None,
        }
    }
}

impl Iterator for AssocItemsIterator<'_> {
    type Item = (*const c_char, *const c_char);

    fn next(&mut self) -> Option<Self::Item> {
        while self.num_bucket < self.table.nbuckets as isize {
            let bucket = self
                .current_bucket_item
                .take()
                .unwrap_or_else(|| unsafe { *self.table.bucket_array.offset(self.num_bucket) });

            if !bucket.is_null() {
                unsafe {
                    let bucket = &*bucket;
                    let item = ((*bucket).key, (*bucket).data);
                    self.current_bucket_item = Some((*bucket).next);
                    return Some(item);
                }
            }

            self.num_bucket += 1;
        }

        None
    }
}
