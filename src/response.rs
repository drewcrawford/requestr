use foundationr::{NSData};
use objr::bindings::{StrongCell};
use std::convert::TryInto;
use crate::client::ActiveClient;

///An opaque data type, may wrap a platform-specific buffer
#[derive(Debug)]
pub struct Data<'a> {
    nsdata: StrongCell<NSData>,
    client: &'a ActiveClient
}
impl<'a> Data<'a> {
    pub fn as_slice(&self) -> &[u8] {
        self.nsdata.as_slice(&self.client.active_pool())
    }
}
pub struct Response<'a>{
    response: StrongCell<foundationr::NSURLResponse>,
    data: Data<'a>,
}
impl<'a> Response<'a> {
    pub fn new(response: StrongCell<foundationr::NSURLResponse>, data: StrongCell<foundationr::NSData>, client: &'a ActiveClient) -> Response<'a> {
        Response {
            response,
            data: Data{nsdata: data,client},
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
        let code = self.response.statusCode(&self.data.client.active_pool());
        if code >= 200 && code <= 299 {
            Ok(self.data())
        }
        else {
            Err((code.try_into().unwrap(),self.data()))
        }
    }

}