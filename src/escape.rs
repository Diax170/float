use std::io::Write;

/// Scan `chunk` for common terminal query escape sequences and write
/// the expected responses back to the PTY so the child shell does not block.
pub(crate) fn respond_to_queries(
    writer: &mut Box<dyn Write + Send>,
    chunk: &[u8],
    screen: &vt100::Screen,
) {
    let mut i = 0;
    while i < chunk.len() {
        if chunk[i] != 0x1b {
            i += 1;
            continue;
        }
        i += 1;
        if i >= chunk.len() {
            break;
        }
        match chunk[i] {
            b'[' => {
                i += 1;
                let start = i;
                while i < chunk.len() && !(0x40..=0x7E).contains(&chunk[i]) {
                    i += 1;
                }
                if i >= chunk.len() {
                    break;
                }
                let params = &chunk[start..i];
                let final_byte = chunk[i];
                i += 1;

                match final_byte {
                    b'n' if params == b"6" => {
                        let (row, col) = screen.cursor_position();
                        let _ = write!(writer, "\x1b[{};{}R", row + 1, col + 1);
                    }
                    b'n' if params == b"5" => {
                        let _ = writer.write_all(b"\x1b[0n");
                    }
                    b'n' => {
                        let _ = writer.write_all(b"\x1b[0n");
                    }
                    b'c' if params.is_empty() || params == b"0" => {
                        let _ = writer.write_all(b"\x1b[?1;0c");
                    }
                    b'c' if params == b">" => {
                        let _ = writer.write_all(b"\x1b[>0;0;0c");
                    }
                    b'c' => {
                        let _ = writer.write_all(b"\x1b[?1;0c");
                    }
                    b'q' if params.starts_with(b">") => {
                        let _ = writer.write_all(b"\x1b[>0;0;0q");
                    }
                    b'q' => {
                        let _ = writer.write_all(b"\x1b[0;0q");
                    }
                    b't' if params == b"18" => {
                        let (rows, cols) = screen.size();
                        let _ = write!(writer, "\x1b[8;{};{}t", rows, cols);
                    }
                    b't' if params == b"19" => {
                        let (rows, cols) = screen.size();
                        let _ = write!(writer, "\x1b[9;{};{}t", rows, cols);
                    }
                    _ => {}
                }
            }
            _ => {
                i += 1;
            }
        }
    }
}

/// Remap CSI escape sequences that the vt100 parser does not
/// handle natively but are commonly emitted by modern TUIs.
///
/// Only transforms standard (non-private) sequences. Sequences with `?`, `>`,
/// or other private marker prefix are passed through unchanged.
pub(crate) fn fix_escape_sequences(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != 0x1b || i + 1 >= bytes.len() || bytes[i + 1] != b'[' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }

        let csi_start = i;
        i += 2; // skip past ESC [

        let private = i < bytes.len() && matches!(bytes[i], b'?' | b'>' | b'<' | b'=');

        let mut param_bytes = 0usize;
        let mut has_intermediate = false;
        let mut found_final = false;

        while i < bytes.len() {
            let b = bytes[i];
            if (0x30..=0x3F).contains(&b) {
                param_bytes += 1;
                i += 1;
            } else if (0x20..=0x2F).contains(&b) {
                has_intermediate = true;
                i += 1;
            } else if (0x40..=0x7E).contains(&b) {
                found_final = true;
                if !has_intermediate && !private {
                    match b {
                        b'f' => {
                            out.extend_from_slice(&bytes[csi_start..i]);
                            out.push(b'H');
                        }
                        b's' if param_bytes == 0 => {
                            out.push(0x1b);
                            out.push(b'7');
                        }
                        b'u' if param_bytes == 0 => {
                            out.push(0x1b);
                            out.push(b'8');
                        }
                        _ => {
                            out.extend_from_slice(&bytes[csi_start..=i]);
                        }
                    }
                } else {
                    out.extend_from_slice(&bytes[csi_start..=i]);
                }
                i += 1;
                break;
            } else {
                out.extend_from_slice(&bytes[csi_start..i]);
                break;
            }
        }

        if !found_final {
            out.extend_from_slice(&bytes[csi_start..]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hvp_to_cup() {
        assert_eq!(fix_escape_sequences(b"\x1b[1;1f"), b"\x1b[1;1H");
        assert_eq!(fix_escape_sequences(b"\x1b[10;20f"), b"\x1b[10;20H");
        assert_eq!(fix_escape_sequences(b"\x1b[f"), b"\x1b[H");
    }

    #[test]
    fn sco_save_restore_cursor() {
        assert_eq!(fix_escape_sequences(b"\x1b[s"), b"\x1b\x37");
        assert_eq!(fix_escape_sequences(b"\x1b[u"), b"\x1b\x38");
    }

    #[test]
    fn csi_s_with_params_not_converted() {
        assert_eq!(fix_escape_sequences(b"\x1b[1s"), b"\x1b[1s");
    }

    #[test]
    fn intermediate_sequences_not_converted() {
        assert_eq!(fix_escape_sequences(b"\x1b[?f"), b"\x1b[?f");
        assert_eq!(fix_escape_sequences(b"\x1b[?1;1f"), b"\x1b[?1;1f");
    }

    #[test]
    fn non_csi_unchanged() {
        let input = b"hello world\x1b7\x1b8\x1b[M";
        assert_eq!(fix_escape_sequences(input), input);
    }

    #[test]
    fn mixed_content() {
        let input = b"\x1b7text\x1b[5;10fmore\x1b8";
        let expected = b"\x1b7text\x1b[5;10Hmore\x1b8";
        assert_eq!(fix_escape_sequences(input), expected);
    }

    #[test]
    fn partial_csi_at_end() {
        assert_eq!(fix_escape_sequences(b"abc\x1b[1;2"), b"abc\x1b[1;2");
    }
}
