use foundationr::{NSData, autoreleasepool};
use objr::bindings::{StrongCell};
use std::convert::TryInto;
use std::path::{PathBuf, Path};

///An opaque data type, may wrap a platform-specific buffer
#[derive(Debug)]
pub struct Data {
    nsdata: StrongCell<NSData>,
}
impl Data {
    pub fn as_slice(&self) -> &[u8] {
        autoreleasepool(|pool| {
            self.nsdata.as_slice(pool)
        })
    }
}
pub struct Response{
    response: StrongCell<foundationr::NSURLResponse>,
    data: Data,
}
impl Response {
    pub fn new(response: StrongCell<foundationr::NSURLResponse>, data: StrongCell<foundationr::NSData>) -> Response {
        Response {
            response,
            data: Data{nsdata: data},
        }
    }
    fn data(&self) -> &Data {
        &self.data
    }
    ///Converts to a result that models success or error based on http status codes.
    ///
    /// If HTTP code suggests 'success', returns Ok(data).
    /// Otherwise, returns Err(statusCode,data).
    pub fn check_status(&self) -> Result<&Data, (u16, &Data)> {
        autoreleasepool(|pool| {
            let code = self.response.statusCode(pool);
            if code >= 200 && code <= 299 {
                Ok(self.data())
            }
            else {
                Err((code.try_into().unwrap(),self.data()))
            }
        })

    }

}

#[derive(Debug)]
pub struct Downloaded(tempfile::TempDir,PathBuf);
impl Downloaded {
    pub fn as_path(&self) -> &Path {
        &self.1
    }
    pub(crate) fn new(dir: tempfile::TempDir, path_buf: PathBuf) -> Self {
        Self(dir,path_buf)
    }

}
