use foundationr::{NSMutableURLRequest, NSURL, NSData, NSURLSession, magic_string::*, autoreleasepool, NSURLSessionDataTask, NSURLSessionDownloadTask};
use objr::bindings::{StrongMutCell, ActiveAutoreleasePool};
use crate::Error;
use super::response::{Response, Downloaded};
use blocksr::continuation::Continuation;
use std::path::{PathBuf};
use tempfile::tempdir;
use pcore::string::{Pstr};
use pcore::release_pool::ReleasePool;
use std::future::Future;

pub struct Request {
    request: StrongMutCell<NSMutableURLRequest>,
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

impl Request {
    ///Create a new builder with the given URL.
    ///
    /// # Errors
    /// May error if the URL is invalid
    // - todo: We could potentially optimize this by writing our options into a rust-like struct
    // and eliding a bunch of intermediate autoreleasepools into one big fn
    pub fn new(url: &Pstr, pool: &ReleasePool) ->
                                     Result<Request, Error> {
        let nsurl = NSURL::from_string(url.as_platform_str(), pool).ok_or_else(|| Error::InvalidURL(url.to_string(pool)))?;
        let request = NSMutableURLRequest::from_url(&nsurl,pool);
        let url_buffer = url.to_string(pool);
        let proposed_file_name = url_buffer.rsplit("/").next().ok_or_else(|| Error::InvalidURL(url.to_string(pool)))?;
        let file_name =  if proposed_file_name.is_empty() {
            "requestsr"
        }
        else {
            proposed_file_name
        }.to_owned();
        Ok(Request {
            request,
            file_name
        })

    }
    ///Set (or unset) a header field
    pub fn header(&mut self, key: &Pstr,value: Option<&Pstr>, pool: &ReleasePool) -> &mut Self {
        let value = value.map(|v| v.as_platform_str());
        self.request.setValueForHTTPHeaderField(value, key.as_platform_str(), pool);
        self
    }
    ///Set the HTTP method.
    pub fn method<M: MagicString>(&mut self, method: M) -> &mut Self{
        autoreleasepool(|pool| {
            let method = method.as_intermediate_string(pool);
            let method = method.as_nsstring();
            self.request.setHTTPMethod(method, pool);
            self
        })

    }
    ///Set the HTTP body data.
    pub fn body(&mut self, body: Box<[u8]>) -> &mut Self {
        autoreleasepool(|pool| {
            let data = NSData::from_boxed_bytes(body,pool);
            //this property is declared copy, so I believe we copy this into the request
            self.request.setHTTPBody(&data, pool);
            self
        })

    }

    pub fn perform<'a>(&mut self, pool: &'a ReleasePool) -> impl Future<Output=Result<Response,Error>> + 'a {
        //Need to manually implement this to avoid holding the autoreleasepool over a suspend point
        let continuation = {
            let session = NSURLSession::shared(&pool);
            let (mut continuation, completion) = Continuation::new();
            let mut task = session.dataTaskWithRequestCompletionHandler(self.request.as_immutable(),&pool, |result| {
                completion.complete(result);
            });
            task.resume(&pool);
            continuation.accept(DataTaskDropper(task));
            continuation
        };
        async {
            let result = continuation.await
                //erase the partial response
                .map_err(|e| {
                    Error::with_perror(e.0.into())
                })?;
            Ok(Response::new(result.1, result.0))
        }

    }

    ///Downloads the request into a file.
    ///
    /// The file will be located in a temporary directory and will be deleted when the return value is dropped.
    pub async fn download(&mut self, pool: &ReleasePool) -> Result<Downloaded,Error>{
        //Need to manually implement this to a) avoid holding the autoreleasepool over a suspend point, b) move inside closure
        let continuation = {
            let session = NSURLSession::shared(&pool);
            let (mut continuation, completion) = Continuation::new();
            //need to be able to send the filename into the completion handler
            let move_filename = self.file_name.clone();
            let mut task = session.downloadTaskWithRequestCompletionHandler(self.request.as_immutable(),&pool, move |result| {
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
            continuation
        };
        let result = continuation.await
            .map_err(|e| {
                Error::with_perror(e.0.into())
            })?;
        Ok(result)

    }

}
#[cfg(test)] mod test {
    use crate::Request;
    use pcore::pstr;
    use pcore::release_pool::autoreleasepool;
    #[test] fn github() {
        autoreleasepool(|pool| {
            let mut r = Request::new(pstr!("https://sealedabstract.com"), pool).unwrap();
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
            let mut r = Request::new(pstr!("https://sealedabstract.com/index.html"), pool).unwrap();
            let future = r
                .download(pool);

            let result = kiruna::test::test_await(future, std::time::Duration::from_secs(10));
            let response = result.unwrap();
            println!("{:?}", response);
        });
    }
}

