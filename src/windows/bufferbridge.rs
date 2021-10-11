use requestr_winbindings::*;
/*
> generally the implement macro expects that you include use bindings::* and then everything should work as expected. I'd like to not require this specific use path but that's beyond what Rust can manage at the moment.
https://github.com/microsoft/windows-rs/issues/81#issuecomment-903175223
 */
use requestr_winbindings::Windows::Win32::System::WinRT::IBufferByteAccess;
use requestr_winbindings::Windows::Web::Http::{IHttpContent,HttpBufferContent};
use requestr_winbindings::Windows::Storage::Streams::IBuffer;
use ::windows::*;
#[implement(Windows::Win32::System::WinRT::IBufferByteAccess,Windows::Storage::Streams::IBuffer)]
pub struct WinBuffer(pub Box<[u8]>);
#[allow(non_snake_case)]
impl WinBuffer {
    fn Buffer(&mut self) -> Result<*mut u8> {
        Ok(self.0.as_mut_ptr())
    }
    fn Capacity(&self) -> Result<u32> {
        Ok(self.0.len() as u32)
    }
    fn Length(&self) -> Result<u32> {
        Ok(self.0.len() as u32)
    }
    fn SetLength(&self,_value:u32) -> Result<u32> {
        panic!("Read only");
    }
    pub fn as_http_buffer(self) -> IHttpContent {
        let a: IBufferByteAccess = self.into();
        let as_buffer: IBuffer = a.cast().unwrap();
        let buffer_content = HttpBufferContent::CreateFromBuffer(as_buffer);
        buffer_content.unwrap().cast().unwrap()

    }
}
