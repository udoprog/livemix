use crate::Error;

use super::RecvBuf;

#[test]
fn test_as_bytes_mut() -> Result<(), Error> {
    let expected = u32::from_ne_bytes([1, 2, 3, 4]);

    let mut buf = RecvBuf::<u32>::new();
    assert!(buf.as_bytes_mut()?.len() > 0);

    buf.as_bytes_mut()?[..3].copy_from_slice(&[1, 2, 3]);

    unsafe {
        buf.advance_written_bytes(3);
    }

    assert_eq!(buf.remaining_bytes(), 3);
    assert_eq!(buf.as_slice(), &[]);

    buf.as_bytes_mut()?[..1].copy_from_slice(&[4]);

    unsafe {
        buf.advance_written_bytes(1);
    }

    assert_eq!(buf.remaining_bytes(), 4);
    assert_eq!(buf.as_slice(), &[expected]);
    Ok(())
}
