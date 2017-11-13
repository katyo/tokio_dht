use serde_bytes;
use serde::de::{Deserialize, Deserializer};

use std::net::SocketAddr;
use std::ops::BitXor;
use std::str::from_utf8;
//use std::mem::transmute;

use rand::{Rng, OsRng};

use crypto_hashes::digest::Digest;
use crypto_hashes::sha1::Sha1;

use super::krpc::{KMessage};
use super::serde::{serde_socket_addr, serde_option_bool};

use super::super::route::node::{NodeId};

#[derive(Serialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum BtDhtQuery {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "find_node")]
    FindNode,
    #[serde(rename = "get_peers")]
    GetPeers,
    #[serde(rename = "announce_peer")]
    AnnouncePeer,
}

impl<'de> Deserialize<'de> for BtDhtQuery {
    fn deserialize<D>(deserializer: D) -> Result<BtDhtQuery, D::Error>
        where D: Deserializer<'de>
    {
        use serde::de::Error;
        let buf: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        let name = from_utf8(&buf).map_err(|_| Error::custom("Invalid method"))?;
        match name {
            "ping" => Ok(BtDhtQuery::Ping),
            "find_node" => Ok(BtDhtQuery::FindNode),
            "get_peers" => Ok(BtDhtQuery::GetPeers),
            "announce_peer" => Ok(BtDhtQuery::AnnouncePeer),
            _ => Err(Error::custom("Unsupported method")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct BtDhtHash(
    #[serde(with = "serde_hash")]
    [u8; 20]
);

impl BtDhtHash {
    pub fn new() -> Self {
        let mut hasher = Sha1::default();
        let mut generator = OsRng::new().unwrap();
        let mut bytes = [0u8; 20];
        generator.fill_bytes(&mut bytes);
        hasher.input(&bytes);
        bytes.clone_from_slice(&hasher.result());
        BtDhtHash(bytes)
    }
}
/*
impl AsRef<[u8]> for BtDhtHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
*/
impl<'a> From<&'a str> for BtDhtHash {
    fn from(v: &'a str) -> Self {
        let mut node_id = [0u8; 20];
        node_id.clone_from_slice(v.as_bytes());
        BtDhtHash(node_id)
    }
}

impl<'a> From<&'a [u8; 20]> for BtDhtHash {
    fn from(v: &'a [u8; 20]) -> Self {
        let mut node_id = [0u8; 20];
        node_id.clone_from_slice(v);
        BtDhtHash(node_id)
    }
}

impl From<[u8; 20]> for BtDhtHash {
    fn from(node_id: [u8; 20]) -> Self {
        BtDhtHash(node_id)
    }
}

/*
impl<'a> From<&'a Vec<u8>> for BtDhtHash {
    fn from(v: &'a Vec<u8>) -> Self {
        let mut node_id = [0u8; 20];
        node_id.clone_from_slice(v);
        BtDhtHash(node_id)
    }
}

impl From<Vec<u8>> for BtDhtHash {
    fn from(v: Vec<u8>) -> Self {
        let mut node_id = [0u8; 20];
        node_id.clone_from_slice(&v);
        BtDhtHash(node_id)
    }
}
 */

impl Default for BtDhtHash {
    fn default() -> Self {
        BtDhtHash([0u8; 20])
    }
}

impl BitXor<BtDhtHash> for BtDhtHash {
    type Output = BtDhtHash;

    fn bitxor(self, other: Self) -> Self {
        let mut out = [0u8; 20];
        /*
        // for i in 0 .. 20 { hash[i] = self.0[i] ^ other.0[i]; }
        // optimization for 32-bit computation
        // up to 4 times faster than 8-bit variant
        let a = unsafe { transmute::<&[u8; 20], &[u32; 5]>(&self.0) };
        let b = unsafe { transmute::<&[u8; 20], &[u32; 5]>(&other.0) };
        let r = unsafe { transmute::<&mut [u8; 20], &mut [u32; 5]>(&mut out) };
        for i in 0 .. 5 {
            r[i] = a[i] ^ b[i];
        }
         */
        for i in 0 .. 20 {
            out[i] = self.0[i] ^ other.0[i];
        }
        BtDhtHash(out)
    }
}

impl NodeId for BtDhtHash {
    fn equal_bits(&self, other: &Self) -> usize {
        /*
        let s = unsafe { transmute::<&[u8; 20], &[u32; 5]>(&self.0) };
        let o = unsafe { transmute::<&[u8; 20], &[u32; 5]>(&other.0) };
        if let Some(i) = s.iter().zip(o.iter()).position(|(a, b)| a != b) {
            32 * i + {
                let x = s[i] ^ o[i];
                let x = if cfg!(target_endian = "little") {
                    x.swap_bytes()
                } else {
                    x
                };
                x.leading_zeros()
            } as usize
        } else {
            32 * 5
        }
         */
        let a = &self.0;
        let b = &other.0;
        if let Some(i) = a.iter().zip(b.iter()).position(|(a, b)| a != b) {
            i * 8 + (a[i] ^ b[i]).leading_zeros() as usize
        } else {
            20 * 8
        }
        /*
        for i in 0 .. 20 {
            if self.0[i] != other.0[i] {
                return (self.0[i] ^ other.0[i]).leading_zeros() as usize
            }
        }
        20 * 8
         */
    }
}

pub type BtDhtToken = Vec<u8>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum BtDhtArg {
    AnnouncePeer {
        id: BtDhtHash,
        #[serde(with = "serde_option_bool")]
        implied_port: bool,
        info_hash: BtDhtHash,
        port: u16,
        #[serde(with = "serde_bytes")]
        token: BtDhtToken,
    },
    GetPeers {
        id: BtDhtHash,
        info_hash: BtDhtHash,
    },
    FindNode {
        id: BtDhtHash,
        target: BtDhtHash,
    },
    Ping {
        id: BtDhtHash,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum BtDhtRes {
    GetPeersNodes {
        id: BtDhtHash,
        #[serde(with = "serde_bytes")]
        token: BtDhtToken,
        #[serde(with = "serde_nodes_info")]
        nodes: Vec<BtDhtNodeInfo>,
    },
    GetPeersValues {
        id: BtDhtHash,
        #[serde(with = "serde_bytes")]
        token: BtDhtToken,
        values: Vec<BtDhtPeerInfo>,
    },
    FindNode {
        id: BtDhtHash,
        #[serde(with = "serde_nodes_info")]
        nodes: Vec<BtDhtNodeInfo>,
    },
    Pong {
        id: BtDhtHash,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtDhtNodeInfo {
    pub id: BtDhtHash,
    pub addr: SocketAddr,
}

mod serde_nodes_info {
    use super::{BtDhtHash, BtDhtNodeInfo};
    use super::super::serde::socket_addr;
    use serde_bytes;
    use serde::ser::Serializer;
    use serde::de::{Deserializer, Error};
    use super::serde_hash;
    
    pub fn serialize<S>(nodes_info: &Vec<BtDhtNodeInfo>, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut buf = Vec::new();
        for node_info in nodes_info {
            serde_hash::to_bytes(&mut buf, &node_info.id.0);
            socket_addr::to_bytes(&mut buf, &node_info.addr);
        }
        serializer.serialize_bytes(&buf)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<BtDhtNodeInfo>, D::Error>
        where D: Deserializer<'de>
    {
        let buf: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        let len = buf.len();
        if len % 26 == 0 {
            let mut nodes_info = Vec::new();
            for buf in buf.chunks(26) {
                let id = serde_hash::from_bytes(&buf[..20]).unwrap();
                let addr = socket_addr::from_bytes(&buf[20..]).unwrap();
                nodes_info.push(BtDhtNodeInfo {id: BtDhtHash(id), addr});
            }
            Ok(nodes_info)
        } else {
            Err(Error::custom("Malformed compact node info"))
        }
    }
}

mod serde_hash {
    use serde_bytes;
    use serde::ser::Serializer;
    use serde::de::{Deserializer, Error};
    
    pub fn to_bytes(buf: &mut Vec<u8>, hash: &[u8; 20]) {
        buf.extend(hash);
    }

    pub fn from_bytes(buf: &[u8]) -> Result<[u8; 20], ()> {
        let len = buf.len();
        if len == 20 {
            let mut hash = [0u8; 20];
            hash.clone_from_slice(&buf);
            Ok(hash)
        } else {
            Err(())
        }
    }
    
    pub fn serialize<S>(hash: &[u8; 20], serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_bytes(hash)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 20], D::Error>
        where D: Deserializer<'de>
    {
        let buf: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        from_bytes(&buf).map_err(|_| Error::custom("Malformed compact node info"))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BtDhtPeerInfo {
    #[serde(with = "serde_socket_addr")]
    pub addr: SocketAddr,
}

pub type BtDhtMessage = KMessage<BtDhtQuery, BtDhtArg, BtDhtRes>;

#[cfg(test)]
mod tests {
    use test::{black_box, Bencher};
    use serde_bencode::ser::{to_bytes};
    use serde_bencode::de::{from_bytes};
    use hexdump::hexdump;
    use super::super::krpc::{KAddress, KMessage, KError};
    use super::{NodeId};
    use super::{BtDhtQuery, BtDhtMessage, BtDhtArg, BtDhtRes, BtDhtHash};

    #[test]
    pub fn test_hash_xor() {
        assert_eq!(BtDhtHash::from([0x00u8; 20]),
                   BtDhtHash::from([0x00u8; 20]) ^
                   BtDhtHash::from([0x00u8; 20]));

        assert_eq!(BtDhtHash::from([0xFFu8; 20]),
                   BtDhtHash::from([0xFFu8; 20]) ^
                   BtDhtHash::from([0x00u8; 20]));

        assert_eq!(BtDhtHash::from([0xFFu8; 20]),
                   BtDhtHash::from([0x00u8; 20]) ^
                   BtDhtHash::from([0xFFu8; 20]));

        assert_eq!(BtDhtHash::from([0x00u8; 20]),
                   BtDhtHash::from([0xFFu8; 20]) ^
                   BtDhtHash::from([0xFFu8; 20]));

        assert_eq!(BtDhtHash::from([0x55u8; 20]),
                   BtDhtHash::from([0x00u8; 20]) ^
                   BtDhtHash::from([0x55u8; 20]));

        assert_eq!(BtDhtHash::from([0xAAu8; 20]),
                   BtDhtHash::from([0xFFu8; 20]) ^
                   BtDhtHash::from([0x55u8; 20]));
        
        assert_eq!(BtDhtHash::from([0xFFu8; 20]),
                   BtDhtHash::from([0xAAu8; 20]) ^
                   BtDhtHash::from([0x55u8; 20]));
    }

    #[bench]
    pub fn bench_hash_xor(b: &mut Bencher) {
        let x = BtDhtHash::from([0xAAu8; 20]);
        let y = BtDhtHash::from([0x55u8; 20]);

        b.iter(|| {
            (0..black_box(1000)).fold(x, |a, _| { a ^ y; a })
        });
    }

    #[test]
    pub fn test_hash_beq() {
        assert_eq!(0,
                   BtDhtHash::from([0xFFu8; 20])
                   .equal_bits(&BtDhtHash::from([0x00u8; 20])));

        assert_eq!(0,
                   BtDhtHash::from([0x00u8; 20])
                   .equal_bits(&BtDhtHash::from([0xFFu8; 20])));
        
        assert_eq!(0,
                   BtDhtHash::from([0xAAu8; 20])
                   .equal_bits(&BtDhtHash::from([0x55u8; 20])));

        assert_eq!(1,
                   BtDhtHash::from([0x00u8; 20])
                   .equal_bits(&BtDhtHash::from([0x55u8; 20])));

        assert_eq!(1,
                   BtDhtHash::from([0xFFu8; 20])
                   .equal_bits(&BtDhtHash::from([0xAAu8; 20])));
        
        assert_eq!(160,
                   BtDhtHash::from([0x00u8; 20])
                   .equal_bits(&BtDhtHash::from([0x00u8; 20])));

        assert_eq!(160,
                   BtDhtHash::from([0xFFu8; 20])
                   .equal_bits(&BtDhtHash::from([0xFFu8; 20])));

        assert_eq!(160,
                   BtDhtHash::from([0x55u8; 20])
                   .equal_bits(&BtDhtHash::from([0x55u8; 20])));

        assert_eq!(160,
                   BtDhtHash::from([0xAAu8; 20])
                   .equal_bits(&BtDhtHash::from([0xAAu8; 20])));

        assert_eq!(21,
                   BtDhtHash::from([0x01, 0x23, 0x45, 0x67, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                   .equal_bits(&BtDhtHash::from([0x01, 0x23, 0x41, 0x67, 0x78, 0x90, 0xab, 0xef, 0xcd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])));

        assert_eq!(75,
                   BtDhtHash::from([0x01, 0x23, 0x45, 0x67, 0x78, 0x90, 0xab, 0xcd, 0xef, 0xa5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                   .equal_bits(&BtDhtHash::from([0x01, 0x23, 0x45, 0x67, 0x78, 0x90, 0xab, 0xcd, 0xef, 0xb5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])));
    }

    #[bench]
    pub fn bench_hash_beq(b: &mut Bencher) {
        let x = BtDhtHash::from([0xAAu8; 20]);
        let y = BtDhtHash::from([0x55u8; 20]);
        let mut t = 0;

        b.iter(|| {
            (0..black_box(1000)).fold(x, |a, _| { t += a.equal_bits(&y); a })
        });
    }
    
    #[test]
    pub fn test_serde_ping_query() {
        let ping_query: BtDhtMessage = KMessage::Query {
            tid: "aa".into(),
            query: BtDhtQuery::Ping,
            arg: BtDhtArg::Ping {
                id: "0123456789abcdefghij".into(),
            },
        };

        let ping_query_enc = to_bytes(&ping_query).unwrap();

        println!("ping_query enc:");
        hexdump(&ping_query_enc);

        assert_eq!(r#"d1:ad2:id20:0123456789abcdefghije1:q4:ping1:t2:aa1:y1:qe"#.as_bytes().to_vec(), ping_query_enc);

        let ping_query_dec: BtDhtMessage = from_bytes(&ping_query_enc).unwrap();

        println!("ping_query dec: {:?}", ping_query_dec);
        assert_eq!(ping_query_dec, ping_query);

        //assert!(false);
    }
    
    #[test]
    pub fn test_serde_ping_response() {
        let ping_response: BtDhtMessage = KMessage::Response {
            ip: Some(KAddress("1.2.3.4:56789".parse().unwrap())),
            //ip: None,
            tid: "aa".into(),
            res: BtDhtRes::Pong {
                id: "0123456789abcdefghij".into(),
            },
        };

        let ping_response_enc = to_bytes(&ping_response).unwrap();

        println!("ping_response enc:");
        hexdump(&ping_response_enc);

        assert_eq!(vec![100, 50, 58, 105, 112, 54, 58, 1, 2, 3, 4, 221, 213, 49, 58, 114, 100, 50, 58, 105, 100, 50, 48, 58, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 101, 49, 58, 116, 50, 58, 97, 97, 49, 58, 121, 49, 58, 114, 101], ping_response_enc);

        let ping_response_dec: BtDhtMessage = from_bytes(&ping_response_enc).unwrap();

        println!("ping_response dec: {:?}", ping_response_dec);
        assert_eq!(ping_response_dec, ping_response);
    }

    #[test]
    pub fn test_serde_method_error() {
        let method_error: BtDhtMessage = KMessage::Error {
            ip: None,
            tid: "55".into(),
            error: (KError::Method, "Unsupported method".into()),
        };

        let method_error_enc = to_bytes(&method_error).unwrap();

        println!("method_error enc:");
        hexdump(&method_error_enc);

        assert_eq!(r#"d1:eli204e18:Unsupported methode1:t2:551:y1:ee"#.as_bytes().to_vec(), method_error_enc);

        let method_error_dec: BtDhtMessage = from_bytes(&method_error_enc).unwrap();

        println!("method_error dec: {:?}", method_error_dec);
        assert_eq!(method_error_dec, method_error);
    }
}
