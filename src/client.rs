use objr::bindings::{autoreleasepool, ActiveAutoreleasePool, AutoreleasePool};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ActiveClient {
    ///don't allow anyone else to construct this
    /// !Send !Sync
    _marker: PhantomData<*const ()>
}
const POOL: ActiveAutoreleasePool = unsafe{ ActiveAutoreleasePool::assume_autoreleasepool() };
impl ActiveClient {
    pub(crate) fn active_pool(&self) -> &ActiveAutoreleasePool {
         &POOL
    }
}
///This models some global state involved in a group of requests.
pub fn with_client<F: FnOnce(&ActiveClient) -> R, R>(f: F) -> R {
    autoreleasepool(|_pool| {
        let active_client = ActiveClient {
            _marker: PhantomData::default()
        };
        f(&active_client)
    })

}

//Basically we need to assign a lifetime to the closure
//because higher-ranked trait bounds don't work with unnamed types
pub trait WithClientAsyncClosure<'a,Argument,MoreArgs,Return> {
    type Fut: std::future::Future<Output=Return> + 'a;
    fn call(self, arg: &'a Argument,more_args: MoreArgs) -> Self::Fut;
}
//Note that this blanket impl does not work for closures, unfortunately
//see https://github.com/rust-lang/rust/issues/70263
impl<'a,Argument,MoreArgs,Return,Fu,F> WithClientAsyncClosure<'a,Argument,MoreArgs,Return> for F
    where
        F: FnOnce(&'a Argument,MoreArgs) -> Fu,
        Fu: std::future::Future<Output=Return> + 'a,
        Argument: 'a
{
    type Fut = Fu;
    fn call(self, rt: &'a Argument, args: MoreArgs) -> Fu {
        self(rt,args)
    }
}

///Get scoped access for an async fn.
///
/// Unfortunately, this does not work with closures, see https://github.com/rust-lang/rust/issues/70263
///
/// Pass a bare fn in instead.
pub async fn with_client_async<C,MoreArgs,R>(c: C,args:MoreArgs) -> R
    where
            C: for<'a> WithClientAsyncClosure<'a,ActiveClient,MoreArgs,R>,
{
    let _a = unsafe{ AutoreleasePool::new() };
    let active_client = ActiveClient {
        _marker: PhantomData::default()
    };
    c.call(&active_client,args).await
}


#[test] fn test_async_client() {
    use objr::bindings::objc_nsstring;
    use crate::Request;

    async fn async_thunk(client: &ActiveClient,_more:()) {

        let _r = Request::new(objc_nsstring!("https://www.sealedabstract.com"), client);
    }

    let _ = with_client_async(async_thunk,());
}