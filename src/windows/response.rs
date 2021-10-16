use pcore::release_pool::ReleasePool;
use pcore::string::IntoParameterString;
use requestr_winbindings::Windows::Storage::Streams::IBuffer;
use windows::Interface;

#[derive(Debug)]
pub struct Downloaded(pub(crate) OwnedString);
impl Downloaded {
    pub fn copy_path(&self) -> PathBuf {
        PathBuf::from_str(&self.0.to_string()).unwrap()
    }
}


impl Drop for Downloaded {
    fn drop(&mut self) {
        use requestr_winbindings::Windows::Win32::Storage::FileSystem::DeleteFileW;
        use requestr_winbindings::Windows::Win32::Foundation::PWSTR;
        unsafe {
            let pwstr: PWSTR = std::mem::transmute(self.0.into_unsafe_const_pwzstr());
            let r = DeleteFileW(pwstr);
            if r.0 == 0 {
                panic!("{:?}",pcore::error::Error::win32_last())
            }
        }

    }
}
use requestr_winbindings::Windows::Web::Http::HttpResponseMessage;
use requestr_winbindings::Windows::Win32::System::WinRT::IBufferByteAccess;
use pcore::string::{OwnedString};
use std::path::{PathBuf};
use std::str::FromStr;

pub struct Response {
    response: HttpResponseMessage,
    data: Option<Data>,
}
///An opaque data type, may wrap a platform-specific buffer
#[derive(Debug)]
pub struct Data(IBufferByteAccess);

impl Data {
    pub fn as_slice(&self) -> &[u8] {
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
                let b = buffers.await.unwrap();
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