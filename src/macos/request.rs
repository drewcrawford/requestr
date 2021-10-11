use foundationr::{NSMutableURLRequest, NSURL, NSURLSession, magic_string::*, autoreleasepool, NSURLSessionDataTask, NSURLSessionDownloadTask, NSString, DataTaskResult, NSError, NSURLResponse, NSData};
use objr::bindings::{StrongMutCell, ActiveAutoreleasePool, StrongLifetimeCell, StrongCell};
use crate::Error;
use super::response::{Response, Downloaded};
use blocksr::continuation::Continuation;
use std::path::{PathBuf};
use tempfile::tempdir;
use pcore::string::{IntoParameterString, ParameterString};
use pcore::release_pool::ReleasePool;
use std::future::Future;
use std::collections::HashMap;
use pcore::pstr;

pub struct Request<'a> {
    url: StrongLifetimeCell<'a, NSString>,
    headers: HashMap<ParameterString<'a>, ParameterString<'a>>,
    body: Option<Box<[u8]>>,
    method: ParameterString<'a>,
    file_name: String
}

struct DataTaskDropper(StrongMutCell<NSURLSessionDataTask>);
impl Drop for DataTaskDropper {
    fn drop(&mut self) {
        autoreleasepool(|pool| {
            self.0.cancel(pool)
        });
    }
}
struct DownloadTaskDropper(StrongMutCell<NSURLSessionDownloadTask>);
impl Drop for DownloadTaskDropper {
    fn drop(&mut self) {
        autoreleasepool(|pool| {
            self.0.cancel(pool)
        });
    }
}

impl<'a> Request<'a> {
    ///Create a new builder with the given URL.
    ///
    /// On some platforms, Error::InvalidURL may be raised int his method
    pub fn new<U: IntoParameterString<'a>>(url: U, pool: &ReleasePool) ->
    Result<Request<'a>,Error> {
        let url = url.into_nsstring(pool);
        let url_buffer = url.to_str(pool);
        let proposed_file_name = url_buffer.rsplit("/").next().ok_or_else(|| Error::InvalidURL(url.to_string()))?;
        let file_name =  if proposed_file_name.is_empty() {
            "requestsr"
        }
        else {
            proposed_file_name
        }.to_owned();
        Ok(Request {
            url: url,
            file_name,
            headers: HashMap::new(),
            body: None,
            method: pstr!("GET").into_parameter_string(pool),
        })
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
    ///Set the HTTP method.
    pub fn method<P: IntoParameterString<'a>>(mut self, method: P, pool: &ReleasePool) -> Self{
        self.method = method.into_parameter_string(pool);
        self

    }
    ///Set the HTTP body data.
    pub fn body(&mut self, body: Box<[u8]>) -> &mut Self {
        self.body = Some(body);
        self
    }

    pub fn perform(self, pool: &ReleasePool) -> impl Future<Output=Result<Response,Error>>  {
        //Need to manually implement this to avoid holding the autoreleasepool over a suspend point

        //The below is a bit tricky, but basically it boils down to getting the "nil case" inside the future, since
        //we can only return 1 future
        enum FutureInput {
            Continuation(Continuation<DataTaskDropper,DataTaskResult>),
            Error(Error)
        }
        let input = match NSURL::from_string(self.url.as_nsstring(), pool) {
            None => {
                FutureInput::Error(Error::InvalidURL(self.url.to_str(pool).to_owned()))
            }
            Some(u) => {
                let mut request = NSMutableURLRequest::from_url(&u, pool);
                request.setHTTPMethod(&self.method.into_nsstring(pool), pool);
                match self.body {
                    None => {}
                    Some(bytes) => {request.setHTTPBody(&NSData::from_boxed_bytes(bytes,pool), pool)}
                }
                for header in self.headers {
                    request.setValueForHTTPHeaderField(Some(&header.1.into_nsstring(pool)), &header.0.into_nsstring(pool), pool);
                }
                let session = NSURLSession::shared(&pool);
                let (mut continuation, completion) = Continuation::new();
                let mut task = session.dataTaskWithRequestCompletionHandler(request.as_immutable(),&pool, |result| {
                    completion.complete(result);
                });
                task.resume(&pool);
                continuation.accept(DataTaskDropper(task));
                FutureInput::Continuation(continuation)
            }
        };

        async {
            match input {
                FutureInput::Continuation(continuation) => {
                    let result = continuation.await
                        //erase the partial response
                        .map_err(|e| {
                            Error::PcoreError(pcore::error::Error::from_nserror(e.0))
                        })?;
                    Ok(Response::new(result.1, result.0))
                }
                FutureInput::Error(e) => {
                    Err(e)
                }
            }
        }

    }

    ///Downloads the request into a file.
    ///
    /// The file will be located in a temporary directory and will be deleted when the return value is dropped.
    pub fn download(self, pool: &ReleasePool) -> impl Future<Output=Result<Downloaded,Error>>{
        //Need to manually implement this to a) avoid holding the autoreleasepool over a suspend point, b) move inside closure
        //The below is a bit tricky, but basically it boils down to getting the "nil case" inside the future, since
        //we can only return 1 future
        enum FutureInput {
            Continuation(Continuation<DownloadTaskDropper, Result<Downloaded, (StrongCell<NSError>, Option<StrongCell<NSURLResponse>>)>>),
            Error(Error)
        }
        let input = match NSURL::from_string(self.url.as_nsstring(),pool) {
            None => {
                FutureInput::Error(Error::InvalidURL(self.url.to_str(pool).to_string()))
            }
            Some(url) => {
                let mut request = NSMutableURLRequest::from_url(&url,pool);
                request.setHTTPMethod(&self.method.into_nsstring(pool), pool);
                match self.body {
                    None => {}
                    Some(bytes) => {request.setHTTPBody(&NSData::from_boxed_bytes(bytes,pool), pool)}
                }
                for header in self.headers {
                    request.setValueForHTTPHeaderField(Some(&header.1.into_nsstring(pool)), &header.0.into_nsstring(pool), pool);
                }
                let session = NSURLSession::shared(&pool);
                let (mut continuation, completion) = Continuation::new();
                //need to be able to send the filename into the completion handler
                let move_filename = self.file_name.clone();
                let mut task = session.downloadTaskWithRequestCompletionHandler(request.as_immutable(),&pool, move |result| {
                    let result = result.map(|r| {
                        //I assume there's a pool when we're called back from foundation
                        let pool = unsafe{ ActiveAutoreleasePool::assume_autoreleasepool() };
                        let current_path = PathBuf::from(r.0.path(&pool).unwrap().to_str(&pool));
                        let dir = tempdir().unwrap();

                        let new_path = dir.path().join(move_filename);
                        std::fs::rename(current_path,new_path.clone()).unwrap();
                        Downloaded::new(dir,new_path)
                    });
                    completion.complete(result);
                });
                task.resume(&pool);
                continuation.accept(DownloadTaskDropper(task));
                FutureInput::Continuation(continuation)
            }

        };
        async {
            match input {
                FutureInput::Continuation(c) => {
                    let result = c.await.map_err(|e| {
                        Error::PcoreError(pcore::error::Error::from_nserror(e.0))
                    });
                    result
                }
                FutureInput::Error(e) => {Err(e)}
            }
        }
    }

}
#[cfg(test)] mod test {
    use crate::Request;
    use pcore::pstr;
    use pcore::release_pool::autoreleasepool;
    #[test] fn github() {
        autoreleasepool(|pool| {
            let r = Request::new(pstr!("https://sealedabstract.com"), pool).unwrap();
            let future = r
                .header(pstr!("Accept"),Some(pstr!("application/vnd.github.v3+json")), pool)
                .header(pstr!("Authorization"), Some(pstr!("token foobar")), pool)
                .perform(pool);
            let result = kiruna::test::test_await(future, std::time::Duration::from_secs(10));
            let response = result.unwrap();
            let data = response.check_status(pool).unwrap();
            println!("{:?}",data);
        });

    }

    #[test] fn download() {
        autoreleasepool(|pool| {
            let r = Request::new(pstr!("https://sealedabstract.com/index.html"), pool).unwrap();
            let future = r
                .download(pool);

            let result = kiruna::test::test_await(future, std::time::Duration::from_secs(10));
            let response = result.unwrap();
            println!("{:?}", response);
        });
    }
}

