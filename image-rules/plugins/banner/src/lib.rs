//! External edge-signature processor; parameters are supplied by the subscription pack.
use koma_image_sdk as host;
use serde_json::Value;
#[derive(Clone)] struct Sig { w:i32, h:i32, rgb:Vec<u8> }
fn num(v:&Value,k:&str)->Result<i32,()> { i32::try_from(v.get(k).and_then(Value::as_i64).ok_or(())?).map_err(|_|()) }
fn parse_sig(v:&Value)->Result<Sig,()> {
    let (w,h)=(num(v,"width")?,num(v,"height")?); if w<16 || h<=0 || w>16384 || h>16384 {return Err(());}
    let a=v.get("averages").and_then(Value::as_array).ok_or(())?; if a.len() != (h as usize)*3 {return Err(());}
    let rgb=a.iter().map(|x|x.as_u64().and_then(|n|u8::try_from(n).ok()).ok_or(())).collect::<Result<Vec<_>,_>>()?; Ok(Sig{w,h,rgb})
}
fn scaled(s:&Sig,w:i32)->Sig {
    let h=((s.h as f32)*(w as f32/640.0_f32)).round().max(1.0) as i32; let mut rgb=Vec::with_capacity(h as usize*3);
    for row in 0..h { let y=(((row as f64+0.5)*s.h as f64/h as f64)-0.5).clamp(0.0,(s.h-1) as f64); let a=y.floor() as usize; let b=(a+1).min(s.h as usize-1); let f=y-a as f64;
        for c in 0..3 { let x=s.rgb[a*3+c] as f64; rgb.push((x+(s.rgb[b*3+c] as f64-x)*f).round() as u8); }
    } Sig{w,h,rgb}
}
fn matches(s:&Sig,x:i32,y:i32,iw:i32,ih:i32)->Result<bool,()> {
    if x<0 || y<0 || x+s.w>iw || y+s.h>ih {return Ok(false);}
    let mut row=vec![0;s.w as usize*4]; let start=(s.w as f64*0.2).floor() as i32; let end=(s.w as f64*0.8).floor() as i32; let step=((end-start) as f64/16.0).max(1.0); let threshold=s.h as i64*18; let mut diff=0_i64;
    for pass in 0..2 { for r in 0..s.h { if (r%4==0)!=(pass==0) {continue;} if host::cancelled(){return Err(());} host::region(x,y+r,s.w,1,&mut row)?;
        let mut sums=[0_u32;3]; for i in 0..16 { let col=(start+(i as f64*step).floor() as i32).min(s.w-1) as usize; for c in 0..3 {sums[c]+=row[col*4+c] as u32;} }
        for c in 0..3 {diff+=((sums[c]/16) as i64-s.rgb[r as usize*3+c] as i64).abs();} if diff > if pass==0 {threshold/2} else {threshold} {return Ok(false);}
    }} Ok(true)
}
#[no_mangle] pub extern "C" fn koma_image_processor_abi()->i32 {1}
#[no_mangle] pub extern "C" fn koma_image_process()->i32 {run().unwrap_or(-1)}
fn run()->Result<i32,()> {
    let v:Value=serde_json::from_slice(&host::config()?).map_err(|_|())?;
    let context=v.get("context").ok_or(())?; let app=context.get("application").and_then(Value::as_str).ok_or(())?;
    let single=match app {"auto"=>false,"source"=>match context.pointer("/sourceHint/mode").and_then(Value::as_str) {Some("1")=>true,Some("2")=>false,_=>return Ok(0)},_=>return Err(())};
    let params=v.get("parameters").ok_or(())?; let sigs=params.get("signatures").and_then(Value::as_array).ok_or(())?;
    let find=|id:&str|->Result<Sig,()> {parse_sig(sigs.iter().find(|x|x.get("id").and_then(Value::as_str)==Some(id)).ok_or(())?)};
    let current=find("current")?; let old=find("old")?; let original=find("narrow")?;
    let (iw,ih)=unsafe{(host::width(),host::height())}; if iw<=0 || ih<=0 {return Err(());} let nw=if iw>640 && iw<800 {iw} else {640}; let narrow=if nw==640 {original} else {scaled(&original,nw)};
    let bh=((160.0_f32)*(nw as f32/640.0_f32)).round().max(1.0) as i32; let mut nb=narrow.clone();nb.h=bh.min(nb.h);nb.rgb.truncate(nb.h as usize*3);
    let tops=[&current,&old,&narrow];let bottoms=[&old,&current,&nb];let mut top=0;let mut bottom=0;
    while top<ih {let mut used=0;for s in tops {if matches(s,(iw-s.w).div_euclid(2),top,iw,ih)? {used=s.h;break;}} if used==0 {break;}top+=used;if single {break;}}
    if !(single && top>0) {while top+bottom<ih {let mut used=0;for s in bottoms {let y=ih-s.h-bottom;if y>=top && matches(s,(iw-s.w).div_euclid(2),y,iw,ih)? {used=s.h;break;}} if used==0 {break;}bottom+=used;if single {break;}}}
    if top==0 && bottom==0 {return Ok(0);} let oh=ih-top-bottom;if oh<=0 {return Ok(0);} host::create(iw,oh)?;
    let mut row=vec![0;(iw as usize).checked_mul(4).ok_or(())?];for y in 0..oh {if host::cancelled(){return Err(());}host::region(0,top+y,iw,1,&mut row)?;host::write(y,iw,1,&row)?;} Ok(1)
}
