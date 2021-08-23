use foundationr::{NSMutableURLRequest, NSURL, NSData, NSURLSession,magic_string::*};
use objr::bindings::{StrongMutCell, objc_nsstring};
use super::Error;
use crate::response::Response;
use foundationr::magic_string::MagicString;
use crate::client::{ActiveClient, with_client};

pub struct Request<'a> {
    request: StrongMutCell<NSMutableURLRequest>,
    client: &'a ActiveClient
}


impl<'a> Request<'a> {
    ///Create a new builder with the given URL.
    ///
    /// # Errors
    /// May error if the URL is invalid
    pub fn new<M: MagicString>(url: M, client: &ActiveClient) ->
                                     Result<Request, Error> {

        let url_i = url.clone().as_intermediate_string(client.active_pool());
        let nsurl = NSURL::from_string(url_i.as_nsstring(), client.active_pool()).ok_or(Error::InvalidURL(url.to_owned()))?;
        let request = NSMutableURLRequest::from_url(&nsurl,client.active_pool());
        Ok(Request {
            client,
            request
        })

    }
    ///Set (or unset) a header field
    pub fn header<K: MagicString,V:MagicString>(mut self, key: K,value: Option<V>) -> Self {
        let key = key.as_intermediate_string(&self.client.active_pool());
        let key = key.as_nsstring();
        let value = value.map(|v| v.as_intermediate_string(&self.client.active_pool()));
        let value = match &value {
            None => {None}
            Some(v) => {Some(v.as_nsstring())}
        };
        self.request.setValueForHTTPHeaderField(value, key, &self.client.active_pool());
        self
    }
    ///Set the HTTP method.
    pub fn method<M: MagicString>(mut self, method: M) -> Self{
        let method = method.as_intermediate_string(&self.client.active_pool());
        let method = method.as_nsstring();
        self.request.setHTTPMethod(method, &self.client.active_pool());
        self
    }
    ///Set the HTTP body data.
    pub fn body(mut self, body: Box<[u8]>) -> Self {
        let data = NSData::from_boxed_bytes(body,&self.client.active_pool());
        //this property is declared copy, so I believe we copy this into the request
        self.request.setHTTPBody(&data, &self.client.active_pool());
        self
    }
    pub async fn perform(self) -> Result<Response<'a>,Error> {
        let session = NSURLSession::shared(&self.client.active_pool());
        let result = session.dataTaskWithRequest(self.request.as_immutable(),&self.client.active_pool()).await
            //erase the partial response
            .map_err(|e| {
                Error::with_nserror(&e.0,&self.client.active_pool())
            })?;
        Ok(Response::new(result.1, result.0, self.client))
    }

}

#[test] fn github() {
    with_client(|c| {
        let r = Request::new(objc_nsstring!("https://sealedabstract.com"), c).unwrap();
        let future = r
            .header(objc_nsstring!("Accept"),Some(objc_nsstring!("application/vnd.github.v3+json")))
            .header(objc_nsstring!("Authorization"), Some(objc_nsstring!("token foobar")))
            .perform();
        let result = kiruna::test::test_await(future, std::time::Duration::from_secs(10));
        let response = result.unwrap();
        let data = response.check_status().unwrap();
        println!("{:?}",data);
    })

}