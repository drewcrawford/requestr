use foundationr::{NSMutableURLRequest, NSURL, NSData, NSURLSession, magic_string::*, autoreleasepool};
use objr::bindings::{StrongMutCell, objc_nsstring, AutoreleasePool};
use super::Error;
use crate::response::Response;
use foundationr::magic_string::MagicString;
use blocksr::continuation::Continuation;

pub struct Request {
    request: StrongMutCell<NSMutableURLRequest>,
}


impl Request {
    ///Create a new builder with the given URL.
    ///
    /// # Errors
    /// May error if the URL is invalid
    // - todo: We could potentially optimize this by writing our options into a rust-like struct
    // and eliding a bunch of intermediate autoreleasepools into one big fn
    pub fn new<M: MagicString>(url: M) ->
                                     Result<Request, Error> {
        let request = autoreleasepool(|pool| {
            let url_i = url.clone().as_intermediate_string(pool);
            let nsurl = NSURL::from_string(url_i.as_nsstring(), pool).ok_or(Error::InvalidURL(url.to_owned()))?;
            Ok(NSMutableURLRequest::from_url(&nsurl,pool))
        })?;
        Ok(Request {
            request
        })

    }
    ///Set (or unset) a header field
    pub fn header<K: MagicString,V:MagicString>(mut self, key: K,value: Option<V>) -> Self {
        autoreleasepool(|pool| {
            let key = key.as_intermediate_string(pool);
            let key = key.as_nsstring();
            let value = value.map(|v| v.as_intermediate_string(pool));
            let value = match &value {
                None => {None}
                Some(v) => {Some(v.as_nsstring())}
            };
            self.request.setValueForHTTPHeaderField(value, key, pool);
            self
        })
    }
    ///Set the HTTP method.
    pub fn method<M: MagicString>(mut self, method: M) -> Self{
        autoreleasepool(|pool| {
            let method = method.as_intermediate_string(pool);
            let method = method.as_nsstring();
            self.request.setHTTPMethod(method, pool);
            self
        })

    }
    ///Set the HTTP body data.
    pub fn body(mut self, body: Box<[u8]>) -> Self {
        autoreleasepool(|pool| {
            let data = NSData::from_boxed_bytes(body,pool);
            //this property is declared copy, so I believe we copy this into the request
            self.request.setHTTPBody(&data, pool);
            self
        })

    }
    pub async fn perform(self) -> Result<Response,Error> {
        //Need to manually implement this to avoid holding the autoreleasepool over a suspend point
        let continuation = {
            let pool = unsafe{ AutoreleasePool::new() };
            let session = NSURLSession::shared(&pool);
            let (mut continuation, completion) = Continuation::new();
            let mut task = session.dataTaskWithRequestCompletionHandler(self.request.as_immutable(),&pool, |result| {
                completion.complete(result);
            });
            task.resume(&pool);
            continuation.accept(task);
            continuation
        };
        let result = continuation.await
            //erase the partial response
            .map_err(|e| {
                autoreleasepool(|pool| {
                    Error::with_nserror(&e.0,&pool)
                })
            })?;
        Ok(Response::new(result.1, result.0))
    }

}

#[test] fn github() {
    let r = Request::new(objc_nsstring!("https://sealedabstract.com")).unwrap();
    let future = r
        .header(objc_nsstring!("Accept"),Some(objc_nsstring!("application/vnd.github.v3+json")))
        .header(objc_nsstring!("Authorization"), Some(objc_nsstring!("token foobar")))
        .perform();
    let result = kiruna::test::test_await(future, std::time::Duration::from_secs(10));
    let response = result.unwrap();
    let data = response.check_status().unwrap();
    println!("{:?}",data);

}