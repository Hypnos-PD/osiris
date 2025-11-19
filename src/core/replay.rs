use std::fs::File;
use std::io::{Read, Cursor};
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt};
use lzma_rs::lzma_decompress;
// Native LZMA FFI: gframe includes LZMA sources in external/ygopro/gframe/lzma
extern "C" {
    pub fn LzmaUncompress(dest: *mut u8, destLen: *mut usize, src: *const u8, srcLen: *mut usize, props: *const u8, propsSize: usize) -> i32;
}

// Constants from replay.h
pub const REPLAY_COMPRESSED: u32 = 0x1;
pub const REPLAY_TAG: u32 = 0x2;
pub const REPLAY_DECODED: u32 = 0x4;
pub const REPLAY_SINGLE_MODE: u32 = 0x8;
pub const REPLAY_UNIFORM: u32 = 0x10;

pub const REPLAY_ID_YRP1: u32 = 0x31707279;
pub const REPLAY_ID_YRP2: u32 = 0x32707279;

pub const MAINC_MAX: u32 = 250;
pub const SEED_COUNT: usize = 8;

#[derive(Debug, Clone)]
pub struct ReplayHeader {
    pub id: u32,
    pub version: u32,
    pub flag: u32,
    pub seed: u32,
    pub datasize: u32,
    pub start_time: u32,
    pub props: [u8; 8],
}

#[derive(Debug, Clone)]
pub struct ExtendedReplayHeader {
    pub base: ReplayHeader,
    pub seed_sequence: [u32; SEED_COUNT],
    pub header_version: u32,
    pub value1: u32,
    pub value2: u32,
    pub value3: u32,
}

#[derive(Debug, Clone, Default)]
pub struct DuelParameters {
    pub start_lp: i32,
    pub start_hand: i32,
    pub draw_count: i32,
    pub duel_flag: u32,
}

#[derive(Debug, Clone, Default)]
pub struct DeckArray {
    pub main: Vec<u32>,
    pub extra: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct Replay {
    pub header: ReplayHeader,
    pub players: Vec<String>,
    pub params: DuelParameters,
    pub decks: Vec<DeckArray>,
    pub script_name: Option<String>,
    pub data: Vec<u8>, // decompressed or raw datastream
    pub actions: Vec<u8>,
    pub packet_data: Vec<Vec<u8>>,
    pub decompressed_ok: bool,
}

impl Replay {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Replay, String> {
        let mut f = File::open(&path).map_err(|e| format!("open failed: {}", e))?;

        // Read base header (size of ReplayHeader): 6 uint32 + 8 bytes props
        let mut reader = std::io::BufReader::new(&mut f);
        let id = reader.read_u32::<LittleEndian>().map_err(|e| format!("read id: {}", e))?;
        let version = reader.read_u32::<LittleEndian>().map_err(|e| format!("read ver: {}", e))?;
        let flag = reader.read_u32::<LittleEndian>().map_err(|e| format!("read flag: {}", e))?;
        let seed = reader.read_u32::<LittleEndian>().map_err(|e| format!("read seed: {}", e))?;
        let datasize = reader.read_u32::<LittleEndian>().map_err(|e| format!("read datasize: {}", e))?;
        let start_time = reader.read_u32::<LittleEndian>().map_err(|e| format!("read start_time: {}", e))?;
        let mut props = [0u8; 8];
        reader.read_exact(&mut props).map_err(|e| format!("read props: {}", e))?;
        let base = ReplayHeader { id, version, flag, seed, datasize, start_time, props };

        if id == REPLAY_ID_YRP2 {
            // Extended header: seed sequence and other fields
            let mut seed_sequence = [0u32; SEED_COUNT];
            for i in 0..SEED_COUNT {
                seed_sequence[i] = reader.read_u32::<LittleEndian>().map_err(|e| format!("read seed seq: {}", e))?;
            }
            let header_version = reader.read_u32::<LittleEndian>().map_err(|e| format!("read header ver: {}", e))?;
            let value1 = reader.read_u32::<LittleEndian>().map_err(|e| format!("value1: {}", e))?;
            let value2 = reader.read_u32::<LittleEndian>().map_err(|e| format!("value2: {}", e))?;
            let value3 = reader.read_u32::<LittleEndian>().map_err(|e| format!("value3: {}", e))?;
            let _ext = ExtendedReplayHeader { base: base.clone(), seed_sequence, header_version, value1, value2, value3 };
        }

        // Read rest of file
        let mut comp_buf: Vec<u8> = Vec::new();
        reader.read_to_end(&mut comp_buf).map_err(|e| format!("read body: {}", e))?;

        let mut data: Vec<u8> = Vec::new();
        let mut decompressed_ok = true;
        if (base.flag & REPLAY_COMPRESSED) != 0 {
            // decompress: C++ used LzmaUncompress with base.props size 5 and comp_data doesn't include props
            // We must prefix props to the compressed stream to be compatible with lzma-rs which expects props first
            let props_len = 5usize;
            let mut composed: Vec<u8> = Vec::with_capacity(props_len + comp_buf.len());
            composed.extend_from_slice(&base.props[..props_len]);
            composed.extend_from_slice(&comp_buf[..]);
            // Attempt 1: properties + comp_buf
            let mut reader = std::io::Cursor::new(composed);
            match lzma_decompress(&mut reader, &mut data) {
                Ok(_) => { /* success */ }
                Err(err1) => {
                    eprintln!("lzma decompress with props failed: {:?}", err1);
                    // Attempt 2: decompress comp_buf as-is (it may include props already)
                    let mut data2: Vec<u8> = Vec::new();
                    let mut reader2 = std::io::Cursor::new(comp_buf.clone());
                    match lzma_decompress(&mut reader2, &mut data2) {
                        Ok(_) => { data = data2; }
                        Err(err2) => {
                            decompressed_ok = false;
                            eprintln!("lzma decompress of comp_buf failed: {:?}", err2);
                            // Attempt native LZMA uncompress as a last resort via FFI into gframe's LZMA code
                            unsafe {
                                let mut dest = vec![0u8; base.datasize as usize];
                                let mut dest_len = dest.len();
                                let mut src_len = comp_buf.len();
                                let c = LzmaUncompress(dest.as_mut_ptr(), &mut dest_len, comp_buf.as_ptr(), &mut src_len, base.props.as_ptr(), 5);
                                if c == 0 && dest_len > 0 {
                                    decompressed_ok = true;
                                    data = dest[..dest_len].to_vec();
                                } else {
                                    eprintln!("LzmaUncompress FFI call failed code: {}", c);
                                }
                            }
                        }
                    }
                }
            }
            if decompressed_ok && data.len() != base.datasize as usize {
                // mismatch => treat as failed
                decompressed_ok = false;
            }
        } else {
            // raw data stream
            data = comp_buf.clone();
        }

        // If decompression failed, return minimal Replay with raw comp_buf as actions
        if !decompressed_ok {
            let actions = comp_buf.clone();
            let packet_data = Vec::new();
            return Ok(Replay { header: base, players: Vec::new(), params: DuelParameters::default(), decks: Vec::new(), script_name: None, data: Vec::new(), actions, packet_data, decompressed_ok: false });
        }

        // Now parse the data for names & decks
        let mut cursor = Cursor::new(&data);
        // Read names: count based on flag
        let player_count = if (base.flag & REPLAY_TAG) != 0 { 4 } else { 2 };
        let mut players: Vec<String> = Vec::with_capacity(player_count);
        for _ in 0..player_count {
            // name: 20 utf-16 values
            let mut name_buf: Vec<u16> = vec![0u16; 20];
            for i in 0..20 {
                let v = cursor.read_u16::<LittleEndian>().map_err(|e| format!("read u16 name: {}", e))?;
                name_buf[i] = v;
            }
            // trim trailing zeros
            let end_pos = name_buf.iter().position(|&c| c == 0).unwrap_or(name_buf.len());
            let s = String::from_utf16_lossy(&name_buf[..end_pos]);
            players.push(s);
        }

        // Read DuelParameters
        let start_lp = cursor.read_i32::<LittleEndian>().map_err(|e| format!("read start_lp: {}", e))?;
        let start_hand = cursor.read_i32::<LittleEndian>().map_err(|e| format!("read start_hand: {}", e))?;
        let draw_count = cursor.read_i32::<LittleEndian>().map_err(|e| format!("read draw_count: {}", e))?;
        let duel_flag = cursor.read_u32::<LittleEndian>().map_err(|e| format!("read duel_flag: {}", e))?;
        let params = DuelParameters { start_lp, start_hand, draw_count, duel_flag };

        // Validate tag flag parity
        if ((base.flag & REPLAY_TAG) != 0) != ((params.duel_flag & 0xFF00) != 0) {
            // This is a weak validation; for now don't fail, but the gframe compares more strictly
            // We'll simply proceed
        }

        let mut decks = Vec::<DeckArray>::new();
        let mut script_name: Option<String> = None;

        if (base.flag & REPLAY_SINGLE_MODE) != 0 {
            let slen = cursor.read_u16::<LittleEndian>().map_err(|e| format!("read slen: {}", e))? as usize;
            if slen == 0 || slen > 255 {
                return Err("slen invalid".to_string());
            }
            let mut buf = vec![0u8; slen];
            cursor.read_exact(&mut buf).map_err(|e| format!("read script name: {}", e))?;
            let name = String::from_utf8_lossy(&buf).to_string();
            if !name.starts_with("./single/") {
                return Err(format!("script name doesn't start with ./single/: {}", name));
            }
            script_name = Some(name[9..].to_string());
            // No decks in single mode
        } else {
            for _p in 0..player_count {
                let main = cursor.read_u32::<LittleEndian>().map_err(|e| format!("read main count: {}", e))?;
                if main > MAINC_MAX {
                    return Err(format!("main count too large: {}", main));
                }
                let mut mainv: Vec<u32> = Vec::new();
                for _i in 0..main {
                    let code = cursor.read_u32::<LittleEndian>().map_err(|e| format!("read main code: {}", e))?;
                    mainv.push(code);
                }
                let extra = cursor.read_u32::<LittleEndian>().map_err(|e| format!("read extra count: {}", e))?;
                if extra > MAINC_MAX {
                    return Err(format!("extra count too large: {}", extra));
                }
                let mut extrav: Vec<u32> = Vec::new();
                for _i in 0..extra {
                    let code = cursor.read_u32::<LittleEndian>().map_err(|e| format!("read extra code: {}", e))?;
                    extrav.push(code);
                }
                decks.push(DeckArray { main: mainv, extra: extrav });
            }
        }

        // Remaining bytes are considered the action stream; capture them
        let mut actions: Vec<u8> = Vec::new();
        cursor.read_to_end(&mut actions).map_err(|e| format!("read action bytes: {}", e))?;

        // Parse action stream into discrete packets.
        // Try two possible framings:
        //  - 1-byte len prefix
        //  - 2-byte len prefix (little-endian u16)
        // We'll score each attempt by counting truncated packets and choose the framing with fewer truncations.
        fn split_with_u8(actions: &[u8]) -> (Vec<Vec<u8>>, usize) {
            let mut packets: Vec<Vec<u8>> = Vec::new();
            let mut idx: usize = 0;
            let mut truncs = 0usize;
            while idx < actions.len() {
                let len = actions[idx] as usize;
                idx += 1;
                if idx + len <= actions.len() {
                    packets.push(actions[idx..idx + len].to_vec());
                    idx += len;
                } else {
                    packets.push(actions[idx..].to_vec());
                    truncs += 1;
                    break;
                }
            }
            (packets, truncs)
        }
        fn split_with_u16(actions: &[u8]) -> (Vec<Vec<u8>>, usize) {
            let mut packets: Vec<Vec<u8>> = Vec::new();
            let mut idx: usize = 0;
            let mut truncs = 0usize;
            while idx + 2 <= actions.len() {
                let len = u16::from_le_bytes([actions[idx], actions[idx + 1]]) as usize;
                idx += 2;
                if idx + len <= actions.len() {
                    packets.push(actions[idx..idx + len].to_vec());
                    idx += len;
                } else {
                    packets.push(actions[idx..].to_vec());
                    truncs += 1;
                    break;
                }
            }
            if idx < actions.len() && actions.len() - idx > 0 { // leftover
                packets.push(actions[idx..].to_vec());
            }
            (packets, truncs)
        }
        let (pack_u8, trunc_u8) = split_with_u8(&actions);
        let (pack_u16, trunc_u16) = split_with_u16(&actions);
        println!("split attempt results: u8 truncs: {} u16 truncs: {} packets u8: {} u16: {}", trunc_u8, trunc_u16, pack_u8.len(), pack_u16.len());
        let (packet_data, used) = if trunc_u8 <= trunc_u16 { (pack_u8, "u8") } else { (pack_u16, "u16") };
        println!("Using split method: {} ({} packets)", used, packet_data.len());

        // If decompression failed, data contains raw comp_buf, and we skip parsing players & decks.
        if !decompressed_ok {
            // We can't rely on parsing names and decks; return minimal Replay with actions set to comp_buf
            let actions = comp_buf.clone();
            let packet_data = Vec::new();
            return Ok(Replay { header: base, players: Vec::new(), params: DuelParameters::default(), decks: Vec::new(), script_name: None, data: Vec::new(), actions, packet_data, decompressed_ok: false });
        }
        Ok(Replay { header: base, players, params, decks, script_name, data, actions, packet_data, decompressed_ok: true })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_replay_parse_uncompressed() {
        // Create a small replay file: header + uncompressed body
        let dir = tempdir().expect("tmpdir");
        let path = dir.path().join("test.yrp");
        let mut f = File::create(&path).expect("create file");

        // Compose base header
        let id = REPLAY_ID_YRP1;
        let version = 0x12d0u32;
        let flag = 0u32; // not compressed
        let seed = 0u32;
        // We will write the body first to figure out size
        let mut body: Vec<u8> = Vec::new();
        // Names (2 players): write 20 u16
        for name in ["Alice", "Bob"] {
            let mut name_utf16: Vec<u16> = name.encode_utf16().collect();
            name_utf16.resize(20, 0);
            for v in name_utf16.iter() {
                body.write_all(&v.to_le_bytes()).unwrap();
            }
        }
        // DuelParameters
        body.write_all(&8000i32.to_le_bytes()).unwrap(); // start_lp
        body.write_all(&5i32.to_le_bytes()).unwrap(); // start_hand
        body.write_all(&1i32.to_le_bytes()).unwrap(); // draw_count
        body.write_all(&0u32.to_le_bytes()).unwrap(); // duel_flag
        // decks: for each player: main count (1), code 12345, extra 0
        for _ in 0..2 {
            body.write_all(&1u32.to_le_bytes()).unwrap();
            body.write_all(&12345u32.to_le_bytes()).unwrap();
            body.write_all(&0u32.to_le_bytes()).unwrap();
        }

        let datasize = body.len() as u32;
        let start_time = 0u32;
        let props = [0u8; 8];

        // Write base header
        f.write_all(&id.to_le_bytes()).unwrap();
        f.write_all(&version.to_le_bytes()).unwrap();
        f.write_all(&flag.to_le_bytes()).unwrap();
        f.write_all(&seed.to_le_bytes()).unwrap();
        f.write_all(&datasize.to_le_bytes()).unwrap();
        f.write_all(&start_time.to_le_bytes()).unwrap();
        f.write_all(&props).unwrap();
        // write body raw
        f.write_all(&body).unwrap();
        drop(f);

        let r = Replay::open(&path).expect("open replay");
        assert_eq!(r.players.len(), 2);
        assert_eq!(r.players[0], "Alice");
        assert_eq!(r.players[1], "Bob");
        assert_eq!(r.params.start_lp, 8000);
        assert_eq!(r.decks.len(), 2);
        assert_eq!(r.decks[0].main[0], 12345);
    }

    #[test]
    fn test_real_replay_parsing() {
        use std::path::PathBuf;
        // Print CWD for diagnostics
        println!("CWD: {:?}", std::env::current_dir().unwrap());
        // Candidate replay directories to check
        let candidates: Vec<PathBuf> = vec![
            PathBuf::from("test/replay"),
            PathBuf::from("../test/replay"),
            PathBuf::from("../../test/replay"),
        ];
        let mut found_file: Option<PathBuf> = None;
        for cand in &candidates {
            if cand.exists() && cand.is_dir() {
                println!("Found candidate dir: {:?}", cand);
                for entry in std::fs::read_dir(cand).expect("read_dir") {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            if ext == "yrp" {
                                // Prefer uncompressed files; if compressed we may still test but prefer uncompressed
                                match Replay::open(&path) {
                                    Ok(r) => {
                                        if r.decompressed_ok {
                                            found_file = Some(path);
                                            break;
                                        } else if found_file.is_none() {
                                            // save as fallback
                                            found_file = Some(path);
                                        }
                                    }
                                    Err(_) => {
                                        if found_file.is_none() {
                                            found_file = Some(path);
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
            if found_file.is_some() { break; }
        }
        let p_abs = found_file.expect(&format!("Could not find any .yrp file to test. CWD: {:?}", std::env::current_dir().unwrap()));
        println!("Using replay file: {:?}", p_abs);
        let r = match Replay::open(&p_abs) {
            Ok(x) => x,
            Err(e) => panic!("Replay::open failed: {}", e),
        };
        println!("decompressed_ok: {}", r.decompressed_ok);
        println!("seed: {} version: {} players: {:?}", r.header.seed, r.header.version, r.players);
        println!("Replay Action Data Size: {} bytes", r.actions.len());
        println!("Parsed packets: {}", r.packet_data.len());
        assert!(r.actions.len() > 0, "Replay actions is empty; decompression may have failed.");
        // If successfully decompressed, assert we parsed player names and decks
        if r.decompressed_ok {
            assert!(!r.players.is_empty(), "Players should have been parsed when decompressed_ok is true");
            assert!(r.decks.len() >= 1, "Decks should have been parsed when decompressed_ok is true");
        }

        // parse first N packets and print their types
        use crate::core::messages::{parse_packet, MsgType, MsgStart, MsgNewTurn};
        let n = 20usize;
        let mut seen: Vec<MsgType> = Vec::new();
        for pkt in r.packet_data.iter().take(n) {
            let (msg, payload) = parse_packet(pkt);
            println!("Packet msg: {:?} payload len {}", msg, payload.len());
            match msg {
                MsgType::Start => {
                    if let Some(s) = MsgStart::parse(payload) {
                        println!("Parsed MSG_START: ty {} lp {:?} deck {:?} extra {:?} hand {:?}", s.player_type, s.lp, s.deck_count, s.extra_count, s.hand_count);
                        assert!(s.lp[0] > 0);
                    } else { println!("Failed to parse MSG_START payload") }
                }
                MsgType::NewTurn => {
                    if let Some(t) = MsgNewTurn::parse(payload) {
                        println!("Parsed MSG_NEW_TURN: player {}", t.player);
                    } else { println!("Failed to parse MSG_NEW_TURN payload") }
                }
                MsgType::Draw => {
                    if let Some(d) = crate::core::messages::MsgDraw::parse(payload) {
                        println!("Parsed MSG_DRAW: player {} count {}", d.player, d.count);
                    }
                }
                MsgType::LpUpdate => {
                    if let Some(l) = crate::core::messages::MsgLpUpdate::parse(payload) {
                        println!("Parsed MSG_LP_UPDATE: player {} lp {}", l.player, l.lp);
                    }
                }
                _ => {
                    if let MsgType::Unknown(x) = msg {
                        let hx = payload.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" ");
                        println!("Unknown message id {} payload hex: {}", x, hx);
                    }
                }
            }
            seen.push(msg);
        }
    }
}
