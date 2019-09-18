use std::mem;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr;
use std::ptr::NonNull;

use winapi::shared::winerror;
use winapi::shared::winerror::HRESULT;
use winapi::um::unknwnbase::IUnknown;
use winapi::Interface;

#[derive(Debug, PartialEq, Eq)]
pub struct ComPtr<T>(NonNull<T>)
where
    T: Interface;

impl<T> ComPtr<T>
where
    T: Interface,
{
    pub fn as_ptr(&mut self) -> *mut T {
        self.0.as_ptr()
    }

    pub fn forget(self) -> *mut T {
        let ptr = self.0.as_ptr();
        mem::forget(self);
        ptr
    }

    pub unsafe fn create_with(f: impl FnOnce(&mut *mut T) -> HRESULT) -> Result<Self, HRESULT> {
        let mut new_ptr = ptr::null_mut();
        let hr = f(&mut new_ptr);
        if !winerror::SUCCEEDED(hr) {
            Err(hr)
        } else {
            // If the creation function didn't fail, the pointer should be valid.
            assert!(!new_ptr.is_null());
            Ok(Self(NonNull::new(new_ptr).unwrap()))
        }
    }
}

impl<T> Deref for ComPtr<T>
where
    T: Interface,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T> DerefMut for ComPtr<T>
where
    T: Interface,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.as_ptr() }
    }
}

impl<T> Drop for ComPtr<T>
where
    T: Interface,
{
    fn drop(&mut self) {
        unsafe {
            (*(self.0.as_ptr() as *mut IUnknown)).Release();
        }
    }
}

#[cfg(test)]
mod com_ptr_tests {
    use super::*;
    use winapi::ctypes::c_void;
    use winapi::shared::guiddef::REFIID;
    use winapi::shared::minwindef::ULONG;
    use winapi::um::unknwnbase::IUnknownVtbl;

    static foo_iunknown_vtbl: IUnknownVtbl = IUnknownVtbl {
        QueryInterface: Foo::iunknown_query_interface,
        AddRef: Foo::iunknown_add_ref,
        Release: Foo::iunknown_release,
    };

    #[derive(Debug, PartialEq)]
    struct Foo {
        lpVtbl: *const IUnknownVtbl,
        ref_count: u32,
    }

    impl Foo {
        unsafe extern "system" fn iunknown_query_interface(
            this: *mut IUnknown,
            riid: REFIID,
            ppvObject: *mut *mut c_void) -> HRESULT {
            unimplemented!()
        }
        
        unsafe extern "system" fn iunknown_add_ref(this: *mut IUnknown) -> ULONG {
            unimplemented!()
        }
        
        unsafe extern "system" fn iunknown_release(this: *mut IUnknown) -> ULONG {
            (&mut *(this as *mut Foo)).release()
        }

        fn release(&mut self) -> u32 {
            assert!(self.ref_count > 0);
            self.ref_count -= 1;
            self.ref_count
        }
    }
    
    impl Drop for Foo {
        fn drop(&mut self) {
            self.release();
        }
    }

    impl Interface for Foo {
        fn uuidof() -> winapi::shared::guiddef::GUID {
            unimplemented!()
        }
    }

    #[test]
    fn create_success() {
        use winapi::shared::dxgi::CreateDXGIFactory;
        use winapi::shared::dxgi::IDXGIFactory;

        unsafe {
            // Create a `foo` and forget it so `ComPtr` can release
            let mut foo = Foo {
                lpVtbl: &foo_iunknown_vtbl,
                ref_count: 1,
            };

            let foo_ptr = &mut foo as *mut _;
            std::mem::forget(foo);

            let mut com_ptr: Result<ComPtr<Foo>, HRESULT> = ComPtr::create_with(|out_ptr| {
                *out_ptr = foo_ptr;
                winerror::S_OK
            });

            assert!(com_ptr.is_ok());

            let mut com_ptr = com_ptr.unwrap();
            assert_eq!(foo_ptr, com_ptr.as_ptr());
            assert_eq!(1, (&*(com_ptr.as_ptr() as *mut Foo)).ref_count);

            // `com_ptr` is dropped, and releases `foo`.
        }
    }

    #[test]
    fn create_fail() {
        unsafe {
            let com_ptr: Result<ComPtr<Foo>, HRESULT> = ComPtr::create_with(|out_ptr| {
                winerror::E_FAIL
            });

            assert_eq!(Err(winerror::E_FAIL), com_ptr);
        }
    }

    /// Can't create a `ComPtr` with `null` and `S_OK`.
    #[test]
    #[should_panic]
    fn create_invalid() {
        unsafe {
            let _: Result<ComPtr<Foo>, HRESULT> = ComPtr::create_with(|out_ptr| {
                *out_ptr = std::ptr::null_mut();
                winerror::S_OK
            });
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
