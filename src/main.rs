use std::{
    env,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    num::ParseIntError,
    thread::sleep,
    time::Duration,
};

macro_rules! impl_coercion {
    ($from:ty, $to:ty, $var:expr) => {
        impl From<$from> for $to {
            fn from(e: $from) -> Self {
                $var(e)
            }
        }
    };
}

#[allow(dead_code)]
#[derive(Debug)]
enum MyError {
    IO(io::Error),
    Parse(ParseIntError),
}

impl_coercion!(io::Error, MyError, MyError::IO);
impl_coercion!(ParseIntError, MyError, MyError::Parse);

// linearly interpolates A's position between B and C to D and E
#[inline(always)]
fn lerp(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
    let x = (a - b) * (e - d) / (c - b) + d;
    if x > e {
        return e;
    } else if x < d {
        return d;
    } else {
        x
    }
}

#[inline(always)]
fn get_num(mut fd: &File) -> Result<u32, MyError> {
    let mut s = String::new();
    fd.read_to_string(&mut s)?;
    fd.seek(SeekFrom::Start(0))?;
    s = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !s.is_empty() {
        Ok(s.parse::<u32>()?)
    } else {
        Ok(1)
    }
}

#[inline(always)]
fn write_num(mut fd: &File, n: u32) -> Result<(), MyError> {
    let s = format!("{}", n);
    let mut buf = s.as_bytes();
    fd.write_all(&mut buf)?;
    fd.seek(SeekFrom::Start(0))?;
    Ok(())
}

struct Conf {
    max_lum: u32,
}

fn get_conf() -> Conf {
    let mut result = Conf { max_lum: 140 };
    let args: Vec<String> = env::args().collect();

    if let Some(s) = args.get(1) {
        if let Ok(n) = s.parse::<u32>() {
            result.max_lum = n;
        } else {
        }
    };
    result
}

/// ret meaning: (brightness fd, sensor fd, max brightness u32)
fn get_files() -> Result<(File, File, u32), MyError> {
    let mut br: Option<File> = None;
    let mut max_br: Option<u32> = None;
    let mut sen: Option<File> = None;

    if File::open("/sys/class/backlight/intel_backlight").is_ok() {
        let max_fd = File::options()
            .read(true)
            .open("/sys/class/backlight/intel_backlight/max_brightness")
            .unwrap();
        max_br = Some(get_num(&max_fd)?);

        br = Some(
            File::options()
                .read(true)
                .write(true)
                .open("/sys/class/backlight/intel_backlight/brightness")
                .unwrap(),
        )
    };
    if File::open("/sys/class/backlight/amdgpu_bl0").is_ok() {
        let max_fd = File::options()
            .read(true)
            .open("/sys/class/backlight/amdgpu_bl0/max_brightness")
            .unwrap();
        max_br = Some(get_num(&max_fd)?);

        br = Some(
            File::options()
                .read(true)
                .write(true)
                .open("/sys/class/backlight/amdgpu_bl0/brightness")
                .unwrap(),
        )
    };
    if File::open("/sys/bus/iio/devices/iio:device0").is_ok() {
        sen = Some(
            File::options()
                .read(true)
                .write(true)
                .open("/sys/bus/iio/devices/iio:device0/in_illuminance_raw")
                .unwrap(),
        )
    };
    Ok((br.unwrap(), sen.unwrap(), max_br.unwrap()))
}

fn compute_br(csen: u32, _conf: &Conf, max_br: u32) -> u32 {
    // let new = lerp(csen, 0, _conf.max_lum, 0, max_br);
    let new = lerp(csen.ilog(2), 0, 10, 0, max_br);
    // for x in [2, 3, 4, 5, 6] {
    //     println!("{}", (csen as f32).log(x as f32));
    // }
    new
}

fn main() -> Result<(), MyError> {
    let conf = get_conf();

    let (br, sen, max_br) = get_files()?;

    let mut prev = 0;

    loop {
        const DELAY: Duration = Duration::from_millis(980);
        sleep(DELAY);

        let Ok(_cbr) = get_num(&br) else { continue };
        let Ok(csen) = get_num(&sen) else { continue };

        // dbg!(csen, 0, conf.max_lum, 0, max_br);
        let new = compute_br(csen, &conf, max_br);

        if new != prev {
            write_num(&br, new)?;
            prev = new;
        }

        print!("  br:  {:4}", _cbr);
        print!("  sen: {:4}", csen);
        print!("  new: {:4}", new);
        print!("\n");
    }
}
