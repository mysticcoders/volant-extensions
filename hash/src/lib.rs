//! Hash for Volant: shows the SHA-256, SHA-1, MD5 and SHA-512 digests of the text you type, or of the
//! clipboard's text when the field is empty, as lowercase hex. It works offline: the manifest lists no
//! hosts, so Volant sends one `render` request and shows the rows. Each row's title is the digest, so
//! Return copies it, and its badge names the algorithm.
//!
//! The digests are of the text's UTF-8 bytes exactly as given, spaces and line breaks included; each
//! row says how many bytes that is. The algorithms are implemented here, from FIPS 180-4 and RFC 1321,
//! and checked against their published test vectors, so the module needs no other crates. A digest is
//! at most 128 characters, well within the 300 Volant shows and copies from a row.
use serde::{Deserialize, Serialize};
use volant_extension_kit::volant_extension_output;

const RENDER: &str = "render";
const PROMPT: &str = "Type or copy some text to hash it.";
const PROMPT_DETAIL: &str = "With the field empty, Hash reads the clipboard's text when it runs";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const FROM_CLIPBOARD: &str = "From clipboard";
const FROM_TYPED: &str = "From typed text";
const SEPARATOR: &str = " · ";
const SPACE: &str = " ";
const THOUSANDS: &str = ",";
const BYTE: &str = "byte of UTF-8";
const BYTES: &str = "bytes of UTF-8";
const SHA256_NAME: &str = "SHA-256";
const SHA1_NAME: &str = "SHA-1";
const MD5_NAME: &str = "MD5";
const SHA512_NAME: &str = "SHA-512";
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
const PADDING_START: u8 = 0x80;

const SHA256_INITIAL: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
const SHA256_ROUNDS: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];
const SHA512_INITIAL: [u64; 8] = [
    0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
    0x510e527fade682d1, 0x9b05688c2b3e6c1f, 0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
];
const SHA512_ROUNDS: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc, 0x3956c25bf348b538, 0x59f111f1b605d019,
    0x923f82a4af194f9b, 0xab1c5ed5da6d8118, 0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694, 0xe49b69c19ef14ad2, 0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65, 0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4, 0xc6e00bf33da88fc2, 0xd5a79147930aa725,
    0x06ca6351e003826f, 0x142929670a0e6e70, 0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b, 0xa2bfe8a14cf10364, 0xa81a664bbc423001,
    0xc24b8b70d0f89791, 0xc76c51a30654be30, 0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8, 0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3, 0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b, 0xca273eceea26619c, 0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178, 0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c, 0x4cc5d4becb3e42b6, 0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];
const SHA1_INITIAL: [u32; 5] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];
const SHA1_ROUNDS: [u32; 4] = [0x5a827999, 0x6ed9eba1, 0x8f1bbcdc, 0xca62c1d6];
const MD5_INITIAL: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
const MD5_SHIFTS: [[u32; 4]; 4] = [[7, 12, 17, 22], [5, 9, 14, 20], [4, 11, 16, 23], [6, 10, 15, 21]];
const MD5_ROUNDS: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    clipboard: Option<String>,
}

#[derive(Serialize, Default, Debug, PartialEq)]
struct Row {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    badges: Vec<Badge>,
}

/// A row badge with only text. Volant 0.3.8 and later show it in place of the extension's name.
#[derive(Serialize, Debug, PartialEq)]
struct Badge { text: String }

/// Where the hashed text came from, named in every row's subtitle.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source { Typed, Clipboard }

impl Source {
    fn label(self) -> &'static str {
        match self { Source::Typed => FROM_TYPED, Source::Clipboard => FROM_CLIPBOARD }
    }
}

/// The text to hash: what was typed when it has more than whitespace, otherwise the clipboard's text
/// when it does, otherwise nothing. The text is hashed exactly as given.
fn input(request: &Request) -> Option<(&str, Source)> {
    let has_text = |text: &str| !text.trim().is_empty();
    if has_text(&request.query) { return Some((&request.query, Source::Typed)); }
    request.clipboard.as_deref().filter(|text| has_text(text)).map(|text| (text, Source::Clipboard))
}

/// The Merkle–Damgård padding all four algorithms share: `0x80`, zeros, then the message length in bits
/// in the last `length_bytes` of a `block`-byte block, big-endian or (for MD5) little-endian.
fn padded(message: &[u8], block: usize, length_bytes: usize, big_endian: bool) -> Vec<u8> {
    let mut data = message.to_vec();
    data.push(PADDING_START);
    while data.len() % block != block - length_bytes { data.push(0); }
    let bits = (message.len() as u128) * 8;
    let length = if big_endian { bits.to_be_bytes() } else { bits.to_le_bytes() };
    if big_endian { data.extend_from_slice(&length[16 - length_bytes..]); } else { data.extend_from_slice(&length[..length_bytes]); }
    data
}

fn hex(bytes: impl IntoIterator<Item = u8>) -> String {
    bytes.into_iter().flat_map(|byte| [HEX_DIGITS[usize::from(byte >> 4)], HEX_DIGITS[usize::from(byte & 0x0F)]]).map(char::from).collect()
}

/// SHA-256 (FIPS 180-4) of `message`, as lowercase hex.
pub fn sha256(message: &[u8]) -> String {
    let mut state = SHA256_INITIAL;
    for block in padded(message, 64, 8, true).chunks(64) {
        let mut w = [0u32; 64];
        for (index, word) in block.chunks(4).enumerate() { w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]); }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7) ^ w[index - 15].rotate_right(18) ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17) ^ w[index - 2].rotate_right(19) ^ (w[index - 2] >> 10);
            w[index] = w[index - 16].wrapping_add(s0).wrapping_add(w[index - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ (!e & g);
            let t1 = h.wrapping_add(s1).wrapping_add(choice).wrapping_add(SHA256_ROUNDS[index]).wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            (h, g, f, e, d, c, b, a) = (g, f, e, d.wrapping_add(t1), c, b, a, t1.wrapping_add(t2));
        }
        for (value, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) { *value = value.wrapping_add(add); }
    }
    hex(state.iter().flat_map(|word| word.to_be_bytes()))
}

/// SHA-512 (FIPS 180-4) of `message`, as lowercase hex.
pub fn sha512(message: &[u8]) -> String {
    let mut state = SHA512_INITIAL;
    for block in padded(message, 128, 16, true).chunks(128) {
        let mut w = [0u64; 80];
        for (index, word) in block.chunks(8).enumerate() {
            w[index] = u64::from_be_bytes([word[0], word[1], word[2], word[3], word[4], word[5], word[6], word[7]]);
        }
        for index in 16..80 {
            let s0 = w[index - 15].rotate_right(1) ^ w[index - 15].rotate_right(8) ^ (w[index - 15] >> 7);
            let s1 = w[index - 2].rotate_right(19) ^ w[index - 2].rotate_right(61) ^ (w[index - 2] >> 6);
            w[index] = w[index - 16].wrapping_add(s0).wrapping_add(w[index - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let choice = (e & f) ^ (!e & g);
            let t1 = h.wrapping_add(s1).wrapping_add(choice).wrapping_add(SHA512_ROUNDS[index]).wrapping_add(w[index]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            (h, g, f, e, d, c, b, a) = (g, f, e, d.wrapping_add(t1), c, b, a, t1.wrapping_add(t2));
        }
        for (value, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) { *value = value.wrapping_add(add); }
    }
    hex(state.iter().flat_map(|word| word.to_be_bytes()))
}

/// SHA-1 (FIPS 180-4) of `message`, as lowercase hex.
pub fn sha1(message: &[u8]) -> String {
    let mut state = SHA1_INITIAL;
    for block in padded(message, 64, 8, true).chunks(64) {
        let mut w = [0u32; 80];
        for (index, word) in block.chunks(4).enumerate() { w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]); }
        for index in 16..80 { w[index] = (w[index - 3] ^ w[index - 8] ^ w[index - 14] ^ w[index - 16]).rotate_left(1); }
        let [mut a, mut b, mut c, mut d, mut e] = state;
        for (index, &word) in w.iter().enumerate() {
            let mix = match index / 20 {
                0 => (b & c) | (!b & d),
                2 => (b & c) | (b & d) | (c & d),
                _ => b ^ c ^ d,
            };
            let next = a.rotate_left(5).wrapping_add(mix).wrapping_add(e).wrapping_add(SHA1_ROUNDS[index / 20]).wrapping_add(word);
            (e, d, c, b, a) = (d, c, b.rotate_left(30), a, next);
        }
        for (value, add) in state.iter_mut().zip([a, b, c, d, e]) { *value = value.wrapping_add(add); }
    }
    hex(state.iter().flat_map(|word| word.to_be_bytes()))
}

/// MD5 (RFC 1321) of `message`, as lowercase hex.
pub fn md5(message: &[u8]) -> String {
    let mut state = MD5_INITIAL;
    for block in padded(message, 64, 8, false).chunks(64) {
        let mut m = [0u32; 16];
        for (index, word) in block.chunks(4).enumerate() { m[index] = u32::from_le_bytes([word[0], word[1], word[2], word[3]]); }
        let [mut a, mut b, mut c, mut d] = state;
        for index in 0..64 {
            let round = index / 16;
            let (mix, word) = match round {
                0 => ((b & c) | (!b & d), index),
                1 => ((d & b) | (!d & c), (5 * index + 1) % 16),
                2 => (b ^ c ^ d, (3 * index + 5) % 16),
                _ => (c ^ (b | !d), (7 * index) % 16),
            };
            let sum = a.wrapping_add(mix).wrapping_add(MD5_ROUNDS[index]).wrapping_add(m[word]);
            (a, d, c) = (d, c, b);
            b = b.wrapping_add(sum.rotate_left(MD5_SHIFTS[round][index % 4]));
        }
        for (value, add) in state.iter_mut().zip([a, b, c, d]) { *value = value.wrapping_add(add); }
    }
    hex(state.iter().flat_map(|word| word.to_le_bytes()))
}

/// 1840 as "1,840".
fn thousands(value: usize) -> String {
    let digits = value.to_string();
    digits.as_bytes().rchunks(3).rev().map(|chunk| std::str::from_utf8(chunk).unwrap_or_default()).collect::<Vec<_>>().join(THOUSANDS)
}

/// One row per algorithm, titled with its digest so Return copies it.
fn rows(text: &str, source: Source) -> Vec<Row> {
    let bytes = text.as_bytes();
    let size = [thousands(bytes.len()), (if bytes.len() == 1 { BYTE } else { BYTES }).to_string()].join(SPACE);
    let subtitle = [source.label(), &size].join(SEPARATOR);
    let digests: [(&str, fn(&[u8]) -> String); 4] = [(SHA256_NAME, sha256), (SHA1_NAME, sha1), (MD5_NAME, md5), (SHA512_NAME, sha512)];
    digests.iter().map(|(name, digest)| Row {
        title: digest(bytes),
        subtitle: Some(subtitle.clone()),
        badges: vec![Badge { text: name.to_string() }],
    }).collect()
}

fn message(title: &str, subtitle: Option<&str>) -> Vec<Row> {
    vec![Row { title: title.to_string(), subtitle: subtitle.map(str::to_string), ..Default::default() }]
}

/// Handles one request and returns the reply Volant reads. The clipboard's text is only hashed, never
/// echoed, logged or stored.
pub fn respond(input_bytes: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input_bytes) else { return items(message(NOT_UNDERSTOOD, None)) };
    if request.phase != RENDER { return items(message(ONLY_LISTS, None)); }
    match input(&request) {
        Some((text, source)) => items(rows(text, source)),
        None => items(message(PROMPT, Some(PROMPT_DETAIL))),
    }
}

fn items(rows: Vec<Row>) -> String {
    serde_json::json!({ "items": rows }).to_string()
}

volant_extension_output!(respond);

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = include_str!("../manifest.json");
    const TWO_BLOCKS: &str = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    const SHA512_TWO_BLOCKS: &str = "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";
    const FOX: &str = "The quick brown fox jumps over the lazy dog";

    fn reply(request: serde_json::Value) -> Vec<serde_json::Value> {
        let reply: serde_json::Value = serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap();
        reply["items"].as_array().unwrap().clone()
    }

    fn all(text: &str) -> [String; 4] {
        let bytes = text.as_bytes();
        [sha256(bytes), sha1(bytes), md5(bytes), sha512(bytes)]
    }

    #[test]
    fn empty_message_vectors() {
        assert_eq!(all(""), [
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "da39a3ee5e6b4b0d3255bfef95601890afd80709",
            "d41d8cd98f00b204e9800998ecf8427e",
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
        ]);
    }

    #[test]
    fn abc_vectors() {
        assert_eq!(all("abc"), [
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "a9993e364706816aba3e25717850c26c9cd0d89d",
            "900150983cd24fb0d6963f7d28e17f72",
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
        ]);
    }

    #[test]
    fn multi_block_vectors() {
        assert_eq!(all(TWO_BLOCKS), [
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
            "8215ef0796a20bcaaae116d3876c664a",
            "204a8fc6dda82f0a0ced7beb8e08a41657c16ef468b228a8279be331a703c33596fd15c13b1b07f9aa1d3bea57789ca031ad85c7a71dd70354ec631238ca3445",
        ]);
        assert_eq!(sha512(SHA512_TWO_BLOCKS.as_bytes()),
            "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909");
        assert_eq!(all(FOX)[2], "9e107d9d372bb6826bd81d3542a419d6");
        assert_eq!(all(FOX)[1], "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
        assert_eq!(sha256("a".repeat(1_000_000).as_bytes()), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
    }

    #[test]
    fn padding_boundaries() {
        assert_eq!(all(&"y".repeat(55))[0], "fb66d40c3bfff05b0d5af8612d0abfbfacc6f5f26c330bc7ad634f1f44bc20ad");
        assert_eq!(all(&"z".repeat(56))[2], "fa2cae340242b29bd7ded21c7bc11da1");
        assert_eq!(all(&"x".repeat(64))[1], "bb2fa3ee7afb9f54c6dfb5d021f14b1ffe40c163");
        assert_eq!(all(&"a".repeat(1000))[3],
            "67ba5535a46e3f86dbfbed8cbbaf0125c76ed549ff8b0b9e03e0c88cf90fa634fa7b12b47d77b694de488ace8d9a65967dc96df599727d3292a8d9d447709c97");
    }

    #[test]
    fn emoji_are_hashed_as_utf8() {
        assert_eq!(all("😀 héllo"), [
            "84a076098522beb6e5f28d50d73b19b6fd38b15a556f81e2ceecd419d2616e10",
            "b5a39820d955d8a65522f990a1c78438e81ddd44",
            "7823a98708e6b58c1d7f5941a9e6e63f",
            "b2793eb7eda7b6d1582757af8ba35fc23c70fc675da220a5d86fd6549d1d8e5fc5d1081dd6648f754ed369cecb736020e08e2d0cdfc337fd0509735e5dc76ce1",
        ]);
        let rows = reply(serde_json::json!({"phase":"render","query":"😀 héllo","now":0}));
        assert_eq!(rows[0]["subtitle"], "From typed text · 11 bytes of UTF-8");
    }

    #[test]
    fn rows_name_their_algorithm() {
        let rows = reply(serde_json::json!({"phase":"render","query":"abc","now":0}));
        let names: Vec<&str> = rows.iter().map(|row| row["badges"][0]["text"].as_str().unwrap()).collect();
        assert_eq!(names, [SHA256_NAME, SHA1_NAME, MD5_NAME, SHA512_NAME]);
        assert_eq!(rows[2]["title"], "900150983cd24fb0d6963f7d28e17f72");
        assert!(rows.iter().all(|row| row.get("url").is_none()), "Without a link, Return copies the title");
        let one = reply(serde_json::json!({"phase":"render","query":"a","now":0}));
        assert_eq!(one[0]["subtitle"], "From typed text · 1 byte of UTF-8");
    }

    #[test]
    fn the_clipboard_is_used_only_when_nothing_is_typed() {
        let clipboard = reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":"abc"}));
        assert_eq!(clipboard[0]["title"], "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(clipboard[0]["subtitle"], "From clipboard · 3 bytes of UTF-8");
        let typed = reply(serde_json::json!({"phase":"render","query":"abc ","now":0,"clipboard":"abc"}));
        assert_eq!(typed[0]["title"], sha256(b"abc "), "Typed text wins and is hashed exactly, trailing space included");
        assert_eq!(reply(serde_json::json!({"phase":"render","query":"  ","now":0,"clipboard":"\n"}))[0]["title"], PROMPT);
        assert_eq!(reply(serde_json::json!({"phase":"render","query":"","now":0}))[0]["title"], PROMPT);
    }

    #[test]
    fn long_input_still_gives_whole_digests() {
        let text = "long text ".repeat(6_000);
        let rows = reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":text}));
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[3]["title"].as_str().unwrap().len(), 128);
        assert!(rows.iter().all(|row| row["title"].as_str().unwrap().len() <= 300));
        assert_eq!(rows[0]["subtitle"], "From clipboard · 60,000 bytes of UTF-8");
    }

    #[test]
    fn other_phases_and_bad_requests() {
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":"x","now":0}))[0]["title"], ONLY_LISTS);
        assert!(respond(b"").contains(NOT_UNDERSTOOD));
    }

    #[test]
    fn manifest_asks_only_for_the_clipboard() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["permissions"], serde_json::json!(["clipboard"]));
        assert!(manifest.get("hosts").is_none() && manifest.get("storage").is_none());
    }
}
