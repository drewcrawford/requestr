use crate::{Error};
use std::future::Future;
use crate::windows::response::{Response, Downloaded};
use std::collections::{HashMap};
use std::mem::MaybeUninit;

use pcore::string::{IntoParameterString, ParameterString, U16ZErasedLength};
use pcore::release_pool::ReleasePool;
use pcore::pstr;
use requestr_winbindings::Windows::Foundation::IAsyncOperationWithProgress;
use requestr_winbindings::Windows::Web::Http::{HttpResponseMessage,HttpProgress};
use crate::windows::bufferbridge::WinBuffer;

pub struct Request<'a> {
    url: ParameterString<'a>,
    headers: HashMap<ParameterString<'a>,ParameterString<'a>>,
    body: Option<WinBuffer>,
    method: ParameterString<'a>,
}




impl<'a> Request<'a> {
    ///Create a new builder with the given URL.
    ///
    /// # Errors
    /// On Windows, this will not fail.
    // - todo: We could potentially optimize this by writing our options into a rust-like struct
    // and eliding a bunch of intermediate autoreleasepools into one big fn
    pub fn new<U: IntoParameterString<'a>>(url: U, pool: &ReleasePool) ->
    Result<Request<'a>, Error> {
        Ok(
            Request{
                url: url.into_parameter_string(pool),
                headers: HashMap::new(),
                method: pstr!("GET").into_parameter_string(pool),
                body: None
            }
        )

    }
    ///Set (or unset) a header field
    pub fn header<K: IntoParameterString<'a>,V:IntoParameterString<'a>>(mut self, key: K,value: Option<V>, pool: &ReleasePool) -> Self {
        match value {
            Some(v) => {self.headers.insert(key.into_parameter_string(pool), v.into_parameter_string(pool));}
            None => {
                self.headers.remove(&key.into_parameter_string(pool));
            }
        }
        self
    }
    ///Set the HTTP method.
    pub fn method<P: IntoParameterString<'a>>(mut self, method: P, pool: &ReleasePool) -> Self{
        self.method = method.into_parameter_string(pool);
        self
    }
    ///Set the HTTP body data.
    pub fn body(mut self, body: Box<[u8]>) -> Self {
        self.body = Some(WinBuffer(body));
        self
    }

    pub fn perform(self, _release_pool: &ReleasePool) -> impl Future<Output=Result<Response,Error>> + 'a {
            let deferred_request = DeferredRequest::new(self);
            async {
                let new_request = deferred_request.perform().unwrap();
                let r = new_request.await?;
                Ok(Response::new(r))
            }
        }

    ///Downloads the request into a file.
    ///
    /// The file will be located in a temporary directory and will be deleted when the return value is dropped.
    pub async fn download(self, _pool: &ReleasePool) -> Result<Downloaded,Error>{
        use requestr_winbindings::Windows::Storage::{StorageFile,FileAccessMode};
        use requestr_winbindings::Windows::Win32::Storage::FileSystem::{GetTempPathW,GetTempFileNameW};
        use requestr_winbindings::Windows::Win32::Foundation::{MAX_PATH,PWSTR};
        use requestr_winbindings::Windows::Storage::Streams::IOutputStream;
        use windows::Interface;
        let deferred_request = DeferredRequest::new(self);
        let response = deferred_request.perform()?.await?;
        let status = response.StatusCode().unwrap().0;
        if status >299 || status < 200 {
            return Err(Error::StatusCode(status as u16));
        }
        let content_stream = response.Content().unwrap();

        //get temporary directory
        let mut buf: MaybeUninit<[u16; MAX_PATH as usize +1]> = MaybeUninit::uninit();
        let r = unsafe {
            let lpbuffer = PWSTR(buf.assume_init_mut().as_mut_ptr());
            GetTempPathW(buf.assume_init().len() as u32,lpbuffer)
        };
        if r == 0 {
            return Err(Error::PcoreError(pcore::error::Error::win32_last()))
        }
        //get temporary path
        let mut filepath: MaybeUninit<[u16; MAX_PATH as usize +1]> = MaybeUninit::uninit();
        let r= unsafe {
            let pathbuf = PWSTR(buf.assume_init_mut().as_mut_ptr());
            let prefixstring: PWSTR = std::mem::transmute(pcore::pstr!("drs").into_unsafe_const_pwzstr());
            GetTempFileNameW(pathbuf,prefixstring, 0, PWSTR(filepath.assume_init_mut().as_mut_ptr()))
        };
        if r == 0 {
            return Err(Error::PcoreError(pcore::error::Error::win32_last()))
        }
        let tempfile_str = unsafe{U16ZErasedLength::with_u16_z_unknown_length(filepath.assume_init_mut()).find_length().to_owned()};
        let mut header = MaybeUninit::uninit();
        let winfile = StorageFile::GetFileFromPathAsync(&unsafe{tempfile_str.into_hstring_trampoline(header.assume_init_mut())}).unwrap().await?;
        let opened_file = winfile.OpenAsync(FileAccessMode::ReadWrite)?.await?;
        let output_stream = opened_file.GetOutputStreamAt(0).unwrap();
        let output_stream: IOutputStream = output_stream.cast().unwrap();
        content_stream.WriteToStreamAsync(output_stream)?.await?;
        println!("wrote to tempfile_str {:?}",tempfile_str);
        Ok(Downloaded(tempfile_str))
    }

}
///This is a request that is not yet made.  We move the builder type into this.
struct DeferredRequest<'a> {
    url: ParameterString<'a>,
    headers: HashMap<ParameterString<'a>,ParameterString<'a>>,
    body: Option<WinBuffer>,
    method: ParameterString<'a>,
}
impl<'a> DeferredRequest<'a> {
    fn new(request: Request<'a>) -> Self {
        DeferredRequest {
            url: request.url,
            headers: request.headers,
            body: request.body,
            method: request.method,
        }
    }
    fn perform(self) -> Result<IAsyncOperationWithProgress<HttpResponseMessage, HttpProgress>,Error> {
        use requestr_winbindings::Windows::Web::Http::{HttpClient,HttpRequestMessage,HttpMethod};
        use requestr_winbindings::Windows::Foundation::Uri;
        let client = HttpClient::new().unwrap();
        let headers = client.DefaultRequestHeaders().unwrap();
        let mut str_header = MaybeUninit::uninit();
        let useragent = unsafe{pstr!("drewcrawford/requestr 0.1 (rust)").into_hstring_trampoline(&mut str_header)};
        headers.UserAgent().unwrap().ParseAdd(&useragent).unwrap();
        for header in self.headers {
            unsafe {
                let mut key_header = MaybeUninit::uninit();
                let mut value_header = MaybeUninit::uninit();
                let key_hstr = header.0.into_hstring_trampoline(&mut key_header);
                let val_hstr = header.1.into_hstring_trampoline(&mut value_header);
                headers.Append(&key_hstr,&val_hstr).unwrap();
            }
        }

        let mut str_header = MaybeUninit::uninit();
        let uri = Uri::CreateUri(&unsafe{self.url.into_hstring_trampoline(&mut str_header)})?;
        let request_message = HttpRequestMessage::new().unwrap();
        let mut str_header = MaybeUninit::uninit();

        let http_method = HttpMethod::Create(unsafe{&self.method.into_hstring_trampoline(&mut str_header)}).unwrap();
        request_message.SetMethod(http_method).unwrap();
        request_message.SetRequestUri(uri).unwrap();
        match self.body {
            None => {}
            Some(body) => {
                request_message.SetContent(body.as_http_buffer()).unwrap();
            }
        }
        let response = client.SendRequestAsync(request_message)?;
        Ok(response)
    }
}
#[cfg(test)] mod test {
    use crate::Request;
    use pcore::release_pool::autoreleasepool;

    use pcore::pstr;
    #[test] fn github() {
        autoreleasepool(|pool| {
            let r = Request::new(pstr!("https://sealedabstract.com"),pool).unwrap();
            let future = r
                .header(pstr!("Accept"),Some(pstr!("application/vnd.github.v3+json")),pool)
                .header(pstr!("Authorization"), Some(pstr!("token foobar")),pool)
                .perform(pool);
            let result = kiruna::test::test_await(future, std::time::Duration::from_secs(5));
            let response = result.unwrap();
            let data = response.check_status(pool).unwrap();
            println!("{:?}",data);
        });
    }

    #[test] fn download() {
        autoreleasepool(|pool| {
            let r = Request::new(pstr!("https://sealedabstract.com/index.html"),pool).unwrap();
            let future = r
                .download(pool);

            let result = kiruna::test::test_await(future, std::time::Duration::from_secs(10));
            let response = result.unwrap();
            println!("{:?}", response);
        });
    }
}

