use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use crate::{address::H160, crypto::hash::{Hashable, H256}};

/// Account-based transaction
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RawTransaction {
    pub from_addr: H160,
    pub to_addr: H160,
    pub value: u64,
    pub nonce: u32,
}
impl Hashable for RawTransaction {
    fn hash(&self) -> H256 {
        let bytes = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
}

/// A signed transaction
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    // to avoid name confusion, we recommend renaming `Transaction` to `RawTransaction`:
    pub raw: RawTransaction,  
    pub pub_key: Vec<u8>,
    pub signature: Vec<u8>,
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let bytes = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
}

impl SignedTransaction {
    /// Create a new transaction from a raw transaction and a key pair
    pub fn from_raw(raw: RawTransaction, key: &Ed25519KeyPair) -> SignedTransaction {
        let pub_key = key.public_key().as_ref().to_vec();
        let signature = sign(&raw, key).as_ref().to_vec();
        SignedTransaction { raw, pub_key, signature }
    }

    /// Verify the signature of this transaction
    pub fn verify_signature(&self) -> bool {
        let serialized_raw = bincode::serialize(&self.raw).unwrap();
        let public_key = ring::signature::UnparsedPublicKey::new(
            &ring::signature::ED25519, &self.pub_key[..]);
        public_key.verify(&serialized_raw, self.signature.as_ref()).is_ok()
    }
}

/// Create digital signature of a transaction
pub fn sign(t: &RawTransaction, key: &Ed25519KeyPair) -> Signature {
    key.sign(bincode::serialize(&t).unwrap().as_ref())
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &RawTransaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key.as_ref())
        .verify(bincode::serialize(&t).unwrap().as_ref(), signature.as_ref())
        .is_ok()
}

// #[cfg(any(test, test_utilities))]
// mod tests {
//     use super::*;
//     use crate::crypto::key_pair;

//     pub fn generate_random_transaction() -> RawTransaction {
//         RawTransaction {
//             foo: rand::random(),
//         }
//     }

//     #[test]
//     fn sign_verify() {
//         let t = generate_random_transaction();
//         let key = key_pair::random();
//         let signature = sign(&t, &key);
//         assert!(verify(&t, &(key.public_key()), &signature));
//     }
// }
