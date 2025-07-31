use crate::Error;

use super::RecvBuf;

#[test]
fn test_as_bytes_mut() -> Result<(), Error> {
    let expected = [1, 2, 3, 4, 5, 6, 7, 8];

    let mut buf = RecvBuf::new();
    assert!(buf.as_bytes_mut()?.len() > 0);

    buf.as_bytes_mut()?[..3].copy_from_slice(&[1, 2, 3]);

    unsafe {
        buf.advance_written_bytes(3);
    }

    assert_eq!(buf.remaining_bytes(), 3);
    assert_eq!(buf.as_bytes(), &[1, 2, 3]);

    buf.as_bytes_mut()?[..5].copy_from_slice(&[4, 5, 6, 7, 8]);

    unsafe {
        buf.advance_written_bytes(5);
    }

    assert_eq!(buf.remaining_bytes(), 8);
    assert_eq!(buf.as_bytes(), &expected[..]);
    Ok(())
}
