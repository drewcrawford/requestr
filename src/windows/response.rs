use std::fmt::{Debug, Formatter};
use pcore::release_pool::ReleasePool;
use pcore::string::IntoParameterString;
use windows::Storage::Streams::IBuffer;

#[derive(Debug)]
pub struct Downloaded(pub(crate) OwnedString);
impl Downloaded {
    pub fn copy_path(&self) -> PathBuf {
        PathBuf::from_str(&self.0.to_string()).unwrap()
    }
}


impl Drop for Downloaded {
    fn drop(&mut self) {
        use windows::Win32::Storage::FileSystem::DeleteFileW;
        use windows::Win32::Foundation::PWSTR;
        unsafe {
            let pwstr: PWSTR = std::mem::transmute(self.0.into_unsafe_const_pwzstr());
            let r = DeleteFileW(pwstr);
            if r.0 == 0 {
                panic!("{:?}",pcore::error::Error::win32_last())
            }
        }

    }
}
use windows::Web::Http::HttpResponseMessage;
use windows::Win32::System::WinRT::IBufferByteAccess;
use pcore::string::{OwnedString};
use std::path::{PathBuf};
use std::str::FromStr;
use winfuture::AsyncFuture;

pub struct Response {
    response: HttpResponseMessage,
    data: Option<Data>,
}
///An opaque data type, may wrap a platform-specific buffer
pub struct Data(IBufferByteAccess);
//IBufferByteAccess does not implement Debug
impl Debug for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Data({:?})",unsafe{self.0.Buffer()}))
    }
}

impl Data {
    pub fn as_slice(&self) -> &[u8] {
        use windows::core::Interface;
        let len = self.0.cast::<IBuffer>().unwrap().Length().unwrap() as usize;
        unsafe { std::slice::from_raw_parts(self.0.Buffer().unwrap(), len)}
    }
}

impl Response {
    pub(crate) fn new(response: HttpResponseMessage) -> Self {
        Self {
            response: response,
            data: None
        }
    }
    pub async fn data(&mut self) -> &Data {
        let m = &mut self.data;
        match m {
            None => {
                let content = self.response.Content().unwrap();
                let buffers = content.ReadAsBufferAsync().unwrap();
                let b = AsyncFuture::new(buffers).await.unwrap();
                use windows::core::Interface;
                let byte_access: IBufferByteAccess = b.cast().unwrap();
                *m = Some(Data(byte_access));
                m.as_ref().unwrap()
            }
            Some(data) => {
                data
            }
        }
    }
    /**
    Converts to a result that models success or error based on http status codes.

    If HTTP code suggests 'success', returns Ok(()).
    Otherwise, returns Err(statusCode).*/
    pub fn check_status(&self,_release_pool: &ReleasePool) -> Result<(), u16> {
        let status = self.response.StatusCode().unwrap().0;
        if status >299 || status < 200 {
            Err(status as u16)
        }
        else {
            Ok(())
        }

    }

}