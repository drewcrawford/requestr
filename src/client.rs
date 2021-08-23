use objr::bindings::{autoreleasepool, ActiveAutoreleasePool};
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