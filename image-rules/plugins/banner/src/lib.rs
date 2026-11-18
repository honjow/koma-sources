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
fn matches(s:&Sig,x:i32,y:i32,iw:i32,ih:i32,error_per_row:i64)->Result<bool,()> {
    if x<0 || y<0 || x+s.w>iw || y+s.h>ih {return Ok(false);}
    let mut row=vec![0;s.w as usize*4]; let start=(s.w as f64*0.2).floor() as i32; let end=(s.w as f64*0.8).floor() as i32; let step=((end-start) as f64/16.0).max(1.0); let threshold=s.h as i64*error_per_row; let mut diff=0_i64;
    for pass in 0..2 { for r in 0..s.h { if (r%4==0)!=(pass==0) {continue;} if host::cancelled(){return Err(());} host::region(x,y+r,s.w,1,&mut row)?;
        let mut sums=[0_u32;3]; for i in 0..16 { let col=(start+(i as f64*step).floor() as i32).min(s.w-1) as usize; for c in 0..3 {sums[c]+=row[col*4+c] as u32;} }
        for c in 0..3 {diff+=((sums[c]/16) as i64-s.rgb[r as usize*3+c] as i64).abs();} if diff > if pass==0 {threshold/2} else {threshold} {return Ok(false);}
    }} Ok(true)
}
// The narrow signature ends with a long white gutter followed by a template border.
// Use its printed area to establish identity, then require a separate white gutter
// and a clear first content row. If the boundary is ambiguous, leave the page intact.
fn short_narrow_top(original: &Sig, narrow: &Sig, y: i32, height: i32) -> Result<i32, ()> {
    if narrow.w == original.w || y + narrow.h >= height {
        return Ok(0);
    }

    let Some(gutter_start) = original.rgb.chunks_exact(24)
        .enumerate()
        .skip(original.h as usize / 16)
        .find(|(_, block)| block.iter().all(|&channel| channel >= 253))
        .map(|(index, _)| index * 8)
    else {
        return Ok(0);
    };
    let mut printed = narrow.clone();
    printed.h = ((gutter_start as f64 * narrow.h as f64) / original.h as f64).ceil() as i32;
    printed.rgb.truncate(printed.h as usize * 3);
    // Allow a small extra decoding/compression difference in the printed
    // area only. The independent gutter and first-ink requirements still
    // govern the crop, and the legacy full signatures keep their tolerance.
    if !matches(&printed, 0, y, narrow.w, height, 19)? {
        return Ok(0);
    }

    let mut row = vec![0; narrow.w as usize * 4];
    let mut white_rows = 0;
    for offset in printed.h..narrow.h {
        if host::cancelled() {
            return Err(());
        }
        host::region(0, y + offset, narrow.w, 1, &mut row)?;
        let ink = row.chunks_exact(4).filter(|pixel| {
            pixel[3] == 255 && pixel[..3].iter().any(|&channel| channel < 224)
        }).count();
        let white = row.chunks_exact(4).all(|pixel| {
            pixel[3] == 255 && pixel[..3].iter().all(|&channel| channel >= 248)
        });
        if white {
            white_rows += 1;
        } else if white_rows >= (narrow.w / 96) {
            // Stop at the first nonwhite row. A weak transition could be JPEG
            // noise or faint artwork, so it is never skipped to find stronger ink.
            return Ok(if ink >= narrow.w as usize / 16 { offset } else { 0 });
        } else if white_rows > 0 || ink > 0 {
            return Ok(0);
        }
    }
    Ok(0)
}

// Some wide pages contain a proportionally reduced, centered version of the
// current banner. Estimate its scale from the yellow logo, then verify the
// complete existing signature; never use the color alone as a match. Limit the
// crop to the confirmed white gutter so a page starting immediately after the
// banner cannot lose its first artwork row.
fn scaled_centered_top(original: &Sig, iw: i32, ih: i32, y: i32) -> Result<i32, ()> {
    if iw <= 800 || iw > 4096 || y != 0 || ih < iw / 5 { return Ok(0); }
    let scan_h = (iw / 9).min(ih).min(360);
    let mut row = vec![0; iw as usize * 4];
    let (mut left, mut right, mut count) = (iw, -1, 0_i32);
    for py in 0..scan_h {
        if host::cancelled() { return Err(()); }
        host::region(0, py, iw, 1, &mut row)?;
        for x in iw / 4..iw / 2 {
            let p = &row[(x as usize)*4..(x as usize)*4+4];
            if p[3] == 255 && p[0] > 180 && p[1] > 140 && p[2] < 80 && p[0] as u16 > (p[2] as u16)*2 && p[1] as u16 > (p[2] as u16)*2 {
                left = left.min(x); right = right.max(x); count += 1;
            }
        }
    }
    if count < 100 || right < left { return Ok(0); }
    // The original 800 px signature's yellow logo is 69 px wide. A small
    // search window accounts for JPEG edges and resampling, not new templates.
    let estimate = ((right - left + 1) * original.w + 34) / 69;
    let from = (estimate * 96 / 100).max(400);
    let to = (estimate * 104 / 100).min(iw);
    for width in from..=to {
        let h = ((original.h as f64 * width as f64 / original.w as f64).round() as i32).max(1);
        let sig = rescale_exact(original, width, h);
        if !matches(&sig, (iw-width)/2, 0, iw, ih, 22)? { continue; }
        let gutter = original.rgb.chunks_exact(24).enumerate().skip(original.h as usize * 3 / 32)
            .find(|(_, block)| block.iter().all(|&channel| channel >= 253))
            .map(|(index, _)| index * 8);
        let Some(gutter) = gutter else { return Ok(0); };
        let start = ((gutter as f64 * h as f64) / original.h as f64).ceil() as i32;
        let mut white = 0;
        for py in start..h {
            if host::cancelled() { return Err(()); }
            host::region(0, py, iw, 1, &mut row)?;
            let mut faint = 0;
            let mut dark = false;
            for p in row.chunks_exact(4) {
                if p[3] != 255 || p[..3].iter().any(|&c| c < 248) { faint += 1; }
                if p[3] != 255 || p[..3].iter().any(|&c| c < 232) { dark = true; }
            }
            if faint == 0 { white += 1; continue; }
            // Immediately after the printed letters, JPEG may add a few
            // nearly white pixels. Never skip a dark or substantial mark;
            // once a confirmed gutter begins, preserve its first uncertain row.
            if white >= width / 96 { return Ok(py); }
            if !dark && faint <= iw / 20 { white = 0; continue; }
            return Ok(0);
        }
        if white >= width / 96 { return Ok(h); }
    }
    Ok(0)
}
fn rescale_exact(s: &Sig, w: i32, h: i32) -> Sig {
    let mut rgb = Vec::with_capacity(h as usize * 3);
    for row in 0..h {
        let y = (((row as f64 + 0.5) * s.h as f64 / h as f64) - 0.5).clamp(0.0, (s.h-1) as f64);
        let a = y.floor() as usize; let b = (a+1).min(s.h as usize-1); let f = y-a as f64;
        for c in 0..3 { let v=s.rgb[a*3+c] as f64; rgb.push((v+(s.rgb[b*3+c] as f64-v)*f).round() as u8); }
    }
    Sig { w, h, rgb }
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
    let (iw,ih)=unsafe{(host::width(),host::height())}; if iw<=0 || ih<=0 {return Err(());} let nw=if iw>640 && iw<800 {iw} else {640}; let narrow=if nw==640 {original.clone()} else {scaled(&original,nw)};
    let bh=((160.0_f32)*(nw as f32/640.0_f32)).round().max(1.0) as i32; let mut nb=narrow.clone();nb.h=bh.min(nb.h);nb.rgb.truncate(nb.h as usize*3);
    let tops=[&current,&old,&narrow];let bottoms=[&old,&current,&nb];let mut top=0;let mut bottom=0;
    while top<ih {let mut used=0;for s in tops {if matches(s,(iw-s.w).div_euclid(2),top,iw,ih,18)? {used=s.h;break;}} if used==0 && narrow.w==iw {used=short_narrow_top(&original,&narrow,top,ih)?;} if used==0 {used=scaled_centered_top(&current,iw,ih,top)?;} if used==0 {break;}top+=used;if single {break;}}
    if !(single && top>0) {while top+bottom<ih {let mut used=0;for s in bottoms {let y=ih-s.h-bottom;if y>=top && matches(s,(iw-s.w).div_euclid(2),y,iw,ih,18)? {used=s.h;break;}} if used==0 {break;}bottom+=used;if single {break;}}}
    if top==0 && bottom==0 {return Ok(0);} let oh=ih-top-bottom;if oh<=0 {return Ok(0);} host::create(iw,oh)?;
    let mut row=vec![0;(iw as usize).checked_mul(4).ok_or(())?];for y in 0..oh {if host::cancelled(){return Err(());}host::region(0,top+y,iw,1,&mut row)?;host::write(y,iw,1,&row)?;} Ok(1)
}
