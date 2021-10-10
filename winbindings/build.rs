fn main() {
    windows::build! {
         Windows::Web::Http::{HttpClient,
            HttpRequestMessage,
            HttpMethod,
            HttpResponseMessage,
            IHttpContent,
            HttpBufferContent,

                Headers::{HttpRequestHeaderCollection,HttpProductInfoHeaderValueCollection}
        },
        Windows::Foundation::Uri,
        Windows::Win32::System::WinRT::IBufferByteAccess,
        Windows::Storage::{StorageFile,FileAccessMode},
        Windows::Storage::Streams::{IRandomAccessStream,IBuffer},
        Windows::Win32::Storage::FileSystem::{GetTempPathW,GetTempFileNameW,DeleteFileW},
        Windows::Win32::Foundation::MAX_PATH,
    }
}