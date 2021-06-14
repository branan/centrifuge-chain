use crate::{hashing::sort_hash_of, Hasher, Proof};
use sp_core::{blake2_256, keccak_256, H256};
use sp_std::vec::Vec;

pub struct BundleHasher;

impl Hasher for BundleHasher {
	type Hash = H256;

	fn hash(data: &[u8]) -> [u8; 32] {
		keccak_256(data)
	}
}

pub struct ProofVerifier;

impl Hasher for ProofVerifier {
	type Hash = H256;

	fn hash(data: &[u8]) -> [u8; 32] {
		blake2_256(data)
	}
}

impl crate::Verifier for ProofVerifier {
	fn hash_of(a: Self::Hash, b: Self::Hash) -> Self::Hash {
		sort_hash_of::<Self>(a, b)
	}

	fn initial_matches(&self, doc_root: Self::Hash) -> Option<Vec<Self::Hash>> {
		Some(vec![doc_root])
	}
}

pub fn get_valid_proof() -> (Proof<H256>, H256) {
	let proof = Proof {
		leaf_hash: [
			1, 93, 41, 93, 124, 185, 25, 20, 141, 93, 101, 68, 16, 11, 142, 219, 3, 124, 155, 37,
			85, 23, 189, 209, 48, 97, 34, 3, 169, 157, 88, 159,
		]
		.into(),
		sorted_hashes: vec![
			[
				113, 229, 58, 223, 178, 220, 200, 69, 191, 246, 171, 254, 8, 183, 211, 75, 54, 223,
				224, 197, 170, 112, 248, 56, 10, 176, 17, 205, 86, 130, 233, 16,
			]
			.into(),
			[
				133, 11, 212, 75, 212, 65, 247, 178, 200, 157, 5, 39, 57, 135, 63, 126, 166, 92,
				232, 170, 46, 155, 223, 237, 50, 237, 43, 101, 180, 104, 126, 84,
			]
			.into(),
			[
				197, 248, 165, 165, 247, 119, 114, 231, 95, 114, 94, 16, 66, 142, 230, 184, 78,
				203, 73, 104, 24, 82, 134, 154, 180, 129, 71, 223, 72, 31, 230, 15,
			]
			.into(),
			[
				50, 5, 28, 219, 118, 141, 222, 221, 133, 174, 178, 212, 71, 94, 64, 44, 80, 218,
				29, 92, 77, 40, 241, 16, 126, 48, 119, 31, 6, 147, 224, 5,
			]
			.into(),
		],
	};

	let doc_root: H256 = [
		25, 102, 189, 46, 86, 242, 48, 217, 254, 16, 20, 211, 98, 206, 125, 92, 167, 175, 70, 161,
		35, 135, 33, 80, 225, 247, 4, 240, 138, 86, 167, 142,
	]
	.into();

	(proof, doc_root)
}

pub fn get_invalid_proof() -> (Proof<H256>, H256) {
	let proof = Proof {
		leaf_hash: [
			1, 93, 41, 93, 124, 185, 25, 20, 141, 93, 101, 68, 16, 11, 142, 219, 3, 124, 155, 37,
			85, 23, 189, 20, 48, 97, 34, 3, 169, 157, 88, 159,
		]
		.into(),
		sorted_hashes: vec![
			[
				113, 229, 58, 22, 178, 220, 200, 69, 191, 246, 171, 254, 8, 183, 211, 75, 54, 223,
				224, 197, 170, 112, 248, 56, 10, 176, 17, 205, 86, 130, 233, 16,
			]
			.into(),
			[
				133, 11, 212, 75, 212, 65, 247, 178, 200, 157, 5, 39, 57, 135, 63, 126, 166, 92,
				23, 170, 4, 155, 223, 237, 50, 237, 43, 101, 180, 104, 126, 84,
			]
			.into(),
		],
	};

	let doc_root: H256 = [
		25, 102, 189, 46, 86, 242, 48, 217, 254, 16, 20, 211, 98, 206, 125, 92, 167, 175, 70, 161,
		35, 135, 33, 80, 225, 247, 4, 240, 138, 86, 167, 142,
	]
	.into();

	(proof, doc_root)
}
