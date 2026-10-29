//! Source-private guest bootstrap via the existing HTTP bridge.
//! Session values stay inside this WASM operation and the Host's private store.
use super::*;

const PUBKEY: &[u8] = b"MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAmFCg289dTws27v8GtqIffkP4zgFR+MYIuUIeVO5AGiBV0rfpRh5gg7i8RrT12E9j6XwKoe3xJz1khDnPc65P5f7CJcNJ9A8bj7Al5K4jYGxz+4Q+n0YzSllXPit/Vz/iW5jFdlP6CTIgUVwvIoGEL2sS4cqqqSpCDKHSeiXh9CtMsktc6YyrSN+8mQbBvoSSew18r/vC07iQiaYkClcs7jIPq9tuilL//2uR9kWn5jsp8zHKVjmXuLtHDhM9lObZGCVJwdlN2KDKTh276u/pzQ1s5u8z/ARtK26N8e5w8mNlGcHcHfwyhjfEQurvrnkqYH37+12U3jGk5YNHGyOPcwIDAQAB";
static mut REQUEST: [u8;8192] = [0;8192];
static mut RESPONSE: [u8;8192] = [0;8192];
static mut VALUE: [u8;4096] = [0;4096];
static mut BODY: [u8;4096] = [0;4096];
static mut LEASE: [u8;256] = [0;256];
static mut LEASE_LEN: usize = 0;
static mut VERSION: [u8;256] = [0;256];
static mut VERSION_LEN: usize = 0;
static mut IMEI: [u8;15] = [0;15];
static mut FIRST: [u8;24] = [0;24];
static mut FIRST_LEN: usize = 0;
static mut GTS: [u8;32] = [0;32];
static mut GTS_LEN: usize = 0;
static mut REQUEST_ID: [u8;36] = [0;36];
static mut ACTIVE: bool = false;

fn req() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(REQUEST) } }
fn response() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(RESPONSE) } }
fn value() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(VALUE) } }
fn body() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(BODY) } }
fn prefix(op: &[u8], n: &mut usize) -> bool { write_bytes(req(),n,br#"{"type":"private_session","version":1,"op":""#) && write_bytes(req(),n,op) && write_bytes(req(),n,b"\"") }
fn rpc(n: usize) -> Result<&'static [u8], FetchError> {
 let r=unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(REQUEST) as *const u8,n) };
 let len=host::http_request(r,response()).map_err(|_|FetchError::Network)?;
 let out=unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(RESPONSE) as *const u8,len) };
 if json_object_member(out,b"ok")!=Some(b"true".as_slice()) { return Err(FetchError::Network) } Ok(out)
}
fn decode<'a>(raw: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], FetchError> { koma_source_sdk::plain_text::decode_json_content(raw,out).map_err(|_|FetchError::Network) }
fn save(dst: &mut [u8], input: &[u8]) -> Result<usize, FetchError> { if input.is_empty() || input.len()>dst.len(){return Err(FetchError::Network)} dst[..input.len()].copy_from_slice(input);Ok(input.len()) }
fn set_clock(out: &[u8]) -> Result<(), FetchError> {
 let g=extract_json_string(out,b"gts").ok_or(FetchError::Network)?;
 if g.len()!=19 || !g.iter().all(|x| x.is_ascii_digit() || matches!(x,b'-'|b':'|b'+')) {return Err(FetchError::Network)}
 unsafe { GTS_LEN=save(&mut *core::ptr::addr_of_mut!(GTS),g)?; } Ok(())
}
fn set_version(out: &[u8]) -> Result<(),FetchError> { let raw=extract_json_string(out,b"versionTag").ok_or(FetchError::Network)?; let n=decode(raw,value())?.len(); unsafe { VERSION_LEN=save(&mut *core::ptr::addr_of_mut!(VERSION),&VALUE[..n])?; } Ok(()) }
pub fn enabled(r: &[u8])->bool { json_object_member(r,b"hostFeatures").map(|v|v.split(|c|*c==b',').any(|x|x.trim_ascii()==br#""source_private_session_v1""# || x.trim_ascii()==br#"["source_private_session_v1"]"# || x.trim_ascii()==br#"["source_private_session_v1""# || x.trim_ascii()==br#""source_private_session_v1"]"#)).unwrap_or(false) }
pub fn active()->bool { unsafe { ACTIVE } }
pub fn imei()->&'static [u8] { unsafe { &IMEI } }
pub fn first()->&'static [u8] { unsafe { &FIRST[..FIRST_LEN] } }
pub fn gts()->&'static [u8] { unsafe { &GTS[..GTS_LEN] } }
pub fn request_id()->&'static [u8] { unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(REQUEST_ID) as *const u8,36) } }
pub fn clock()->Result<(),FetchError> {
 let mut n=0;if !prefix(b"clock",&mut n)||!write_bytes(req(),&mut n,b"}"){return Err(FetchError::Network)}
 let out=rpc(n)?;set_clock(out)?;
 let random=extract_json_string(out,b"randomHex").ok_or(FetchError::Network)?;
 if random.len()!=32 || !random.iter().all(u8::is_ascii_hexdigit) { return Err(FetchError::Network) }
 let mut id=[0u8;36];let mut j=0;for (i,dst) in id.iter_mut().enumerate(){if [8,13,18,23].contains(&i){*dst=b'-'}else{*dst=random[j].to_ascii_lowercase();j+=1}}
 id[14]=b'4';let v=random[16];let v=if v.is_ascii_digit(){v-b'0'}else{v.to_ascii_lowercase()-b'a'+10};id[19]=b"89ab"[(v&3) as usize];
 unsafe { (*core::ptr::addr_of_mut!(REQUEST_ID)).copy_from_slice(&id) } Ok(())
}
fn string_field<'a>(doc:&[u8],key:&[u8],out:&'a mut [u8])->Result<&'a [u8],FetchError>{let raw=extract_json_string(doc,key).ok_or(FetchError::Network)?;decode(raw,out)}
fn load_ready(out:&[u8])->Result<(),FetchError>{
 set_clock(out)?;set_version(out)?; let n={let raw=extract_json_string(out,b"value").ok_or(FetchError::Network)?;decode(raw,value())?.len()};
 let doc=unsafe{core::slice::from_raw_parts(core::ptr::addr_of!(VALUE) as *const u8,n)};
 let mut a=[0u8;2048];let imei=string_field(doc,b"imei",&mut a)?;if imei.len()!=15||!imei.iter().all(u8::is_ascii_digit){return Err(FetchError::Network)} unsafe{IMEI.copy_from_slice(imei)}
 let mut first=[0u8;24];let f=string_field(doc,b"firstUsedTime",&mut first)?;unsafe{FIRST_LEN=save(&mut *core::ptr::addr_of_mut!(FIRST),f)?}
 let mut uid=[0u8;32];let u=string_field(doc,b"userId",&mut uid)?;let mut token=[0u8;2048];let t=string_field(doc,b"token",&mut token)?;if !set_auth(u,t){return Err(FetchError::Network)} unsafe{ACTIVE=true} Ok(())
}
fn lease()->&'static [u8] {unsafe{&LEASE[..LEASE_LEN]}}
fn abort(){let mut n=0;if prefix(b"abort",&mut n)&&write_bytes(req(),&mut n,br#","lease":""#)&&append_json_escaped(req(),&mut n,lease())&&write_bytes(req(),&mut n,b"\"}"){let _=rpc(n);} unsafe{LEASE_LEN=0}}
pub fn invalidate(){if !active(){return} let tag=unsafe{&VERSION[..VERSION_LEN]};let mut n=0;if prefix(b"invalidate",&mut n)&&write_bytes(req(),&mut n,br#","versionTag":""#)&&append_json_escaped(req(),&mut n,tag)&&write_bytes(req(),&mut n,b"\"}"){let _=rpc(n);}unsafe{ACTIVE=false}}
fn rsa(input:&[u8],out:&mut [u8])->Result<usize,FetchError>{let mut n=0;if !prefix(b"rsa_pkcs1",&mut n)||!write_bytes(req(),&mut n,br#","publicKey":""#)||!write_bytes(req(),&mut n,PUBKEY)||!write_bytes(req(),&mut n,br#"","plaintext":""#)||!append_json_escaped(req(),&mut n,input)||!write_bytes(req(),&mut n,b"\"}"){return Err(FetchError::Network)}let r=rpc(n)?;let v=extract_json_string(r,b"value").ok_or(FetchError::Network)?;if v.len()!=344||!v.iter().all(|x|x.is_ascii_alphanumeric()||matches!(x,b'+'|b'/'|b'=')){return Err(FetchError::Network)}save(out,v)}
fn random_hex(out:&[u8],dest:&mut [u8;96])->Result<(),FetchError>{let v=extract_json_string(out,b"randomHex").ok_or(FetchError::Network)?;if v.len()!=96||!v.iter().all(u8::is_ascii_hexdigit){return Err(FetchError::Network)}dest.copy_from_slice(v);Ok(())}
fn create(out:&[u8])->Result<(),FetchError>{
 set_clock(out)?;let now=extract_json_string(out,b"nowMs").ok_or(FetchError::Network)?;if now.len()<4||now.len()>24||!now.iter().all(u8::is_ascii_digit){return Err(FetchError::Network)}let mut first=[0u8;24];first[..now.len()].copy_from_slice(now);first[now.len()-3..now.len()].copy_from_slice(b"000");unsafe{FIRST_LEN=save(&mut *core::ptr::addr_of_mut!(FIRST),&first[..now.len()])?}
 let mut random=[0u8;96];random_hex(out,&mut random)?;let raw=extract_json_string(out,b"lease").ok_or(FetchError::Network)?;let n=decode(raw,value())?.len();unsafe{LEASE_LEN=save(&mut *core::ptr::addr_of_mut!(LEASE),&VALUE[..n])?}
 let mut sum=0u32;for i in 0..14 {let x=random[i];let d=if x.is_ascii_digit(){x-b'0'}else{x.to_ascii_lowercase()-b'a'+10};let d=d%10;unsafe{IMEI[i]=b'0'+d}let v=if i%2==0{d}else{let x=d*2;if x>=10{x-9}else{x}};sum+=v as u32;}unsafe{IMEI[14]=b'0'+((10-sum%10)%10) as u8}
 let mut android=[0u8;16];for i in 0..16{android[i]=random[14+i].to_ascii_uppercase()}let mut android_start=0usize;while android_start<15 && android[android_start]==b'0'{android_start+=1}let mut uuid=[0u8;36];let mut j=0;for (i,dst) in uuid.iter_mut().enumerate(){if [8,13,18,23].contains(&i){*dst=b'-'}else{*dst=random[30+j].to_ascii_lowercase();j+=1}}uuid[14]=b'4';let v=uuid[19];let v=if v.is_ascii_digit(){v-b'0'}else{v-b'a'+10};uuid[19]=b"89ab"[(v&3) as usize];
 let mut encoded=[[0u8;344];3];let texts:[&[u8];3]=[imei(),&android[android_start..],&uuid];for i in 0..3{rsa(texts[i],&mut encoded[i])?;}
 let mut n=0; if !write_bytes(body(),&mut n,br#"{"keys":["#){return Err(FetchError::Network)}for i in 0..3{if i>0&&!write_bytes(body(),&mut n,b","){return Err(FetchError::Network)}if !write_bytes(body(),&mut n,br#"{"key":""#)||!write_bytes(body(),&mut n,&encoded[i])||!write_bytes(body(),&mut n,br#"","keyType":""#)||!write_bytes(body(),&mut n,[b"0".as_slice(),b"2",b"-1"][i])||!write_bytes(body(),&mut n,b"\"}"){return Err(FetchError::Network)}}if !write_bytes(body(),&mut n,b"]}"){return Err(FetchError::Network)}
 let b=unsafe{core::slice::from_raw_parts(core::ptr::addr_of!(BODY) as *const u8,n)};unsafe{ACTIVE=true} clock()?; fetch_anony_user_with_body(b)?;
 let mut v=0;if !write_bytes(value(),&mut v,br#"{"imei":""#)||!write_bytes(value(),&mut v,imei())||!write_bytes(value(),&mut v,br#"","firstUsedTime":""#)||!write_bytes(value(),&mut v,self::first())||!write_bytes(value(),&mut v,br#"","userId":""#)||!append_json_escaped(value(),&mut v,auth_user_id_slice())||!write_bytes(value(),&mut v,br#"","token":""#)||!append_json_escaped(value(),&mut v,auth_token_slice())||!write_bytes(value(),&mut v,b"\"}"){return Err(FetchError::Network)}
 let session=unsafe{core::slice::from_raw_parts(core::ptr::addr_of!(VALUE) as *const u8,v)};let mut n=0;if !prefix(b"commit",&mut n)||!write_bytes(req(),&mut n,br#","lease":""#)||!append_json_escaped(req(),&mut n,lease())||!write_bytes(req(),&mut n,br#"","value":""#)||!append_json_escaped(req(),&mut n,session)||!write_bytes(req(),&mut n,b"\"}"){return Err(FetchError::Network)} let out=rpc(n)?;set_version(out)?;unsafe{LEASE_LEN=0;ACTIVE=true}Ok(())
}
pub fn begin()->Result<(),FetchError>{let mut n=0;if !prefix(b"begin",&mut n)||!write_bytes(req(),&mut n,b"}"){return Err(FetchError::Network)}let out=rpc(n)?;match extract_json_string(out,b"state"){Some(b"ready")=>load_ready(out),Some(b"owner")=>{let result=create(out);if result.is_err(){abort();unsafe{ACTIVE=false}}result},_=>Err(FetchError::Network)}}
