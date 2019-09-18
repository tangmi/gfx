use std::mem;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr;
use std::ptr::NonNull;

use winapi::shared::winerror;
use winapi::shared::winerror::HRESULT;
use winapi::um::unknwnbase::IUnknown;
use winapi::Interface;

#[derive(Debug)]
pub struct ComPtr<T>(NonNull<T>);

impl<T> ComPtr<T>
where
    T: Interface,
{
    pub unsafe fn from_raw(ptr: *mut T) -> ComPtr<T> {
        ComPtr(NonNull::new(ptr).unwrap())
    }

    pub fn as_ptr(&mut self) -> *mut T {
        self.0.as_ptr()
    }

    pub fn forget(self) -> *mut T {
        let ptr = self.0.as_ptr();
        mem::forget(self);
        ptr
    }

    pub unsafe fn create_with(f: impl FnOnce(*mut T) -> HRESULT) -> Result<Self, HRESULT> {
        let new_ptr = ptr::null_mut();
        let hr = f(new_ptr);
        if !winerror::SUCCEEDED(hr) {
            Err(hr)
        } else {
            // If the creation function didn't fail, the pointer should be valid.
            assert!(!new_ptr.is_null());
            Ok(Self::from_raw(new_ptr))
        }
    }
}

impl<T> Deref for ComPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T> DerefMut for ComPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.as_ptr() }
    }
}

impl<T> Drop for ComPtr<T> {
    fn drop(&mut self) {
        unsafe {
            (*(self.0.as_ptr() as *mut IUnknown)).Release();
        }
    }
}

#[macro_export]
macro_rules! try_log {
    ($message: literal, $com_ptr: expr) => {
        match $com_ptr {
            Ok(com_ptr) => com_ptr,
            Err(hr) => {
                error!("\"{}\" failed, error {:x}", $message, hr);
                return;
            },
        }
    };
}
