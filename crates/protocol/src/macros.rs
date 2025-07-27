macro_rules! __endian {
    ($a:expr, $b:expr) => {
        if cfg!(target_endian = "little") {
            $a
        } else {
            $b
        }
    };
}

pub(crate) use __endian as endian;
