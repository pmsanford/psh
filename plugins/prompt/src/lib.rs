use bytes::Buf;
use plugin_protos::bytes::BytesMut;
use plugin_protos::plugin::Prompt;
use plugin_protos::Message;

const MEM_LEN: usize = 4_096;
static mut WRITE_BUF: [u8; MEM_LEN] = [0u8; MEM_LEN];

#[no_mangle]
pub extern "C" fn get_prompt() -> *const u8 {
    let prompt = Prompt {
        segment: "path".into(),
    };

    unsafe {
        let mut buf = BytesMut::new();

        prompt.encode_length_delimited(&mut buf).unwrap();
        buf.copy_to_slice(&mut WRITE_BUF[..buf.len()]);

        WRITE_BUF.as_ptr()
    }
}
