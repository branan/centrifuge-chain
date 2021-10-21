//! Some configurable implementations as associated type for the substrate runtime.

use super::*;
use codec::{Decode, Encode};
use core::marker::PhantomData;
use frame_support::traits::{Currency, OnUnbalanced};
use frame_support::weights::{
	WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
};
use frame_system::pallet::Config as SystemConfig;
use pallet_authorship::{Config as AuthorshipConfig, Pallet as Authorship};
use pallet_balances::{Config as BalancesConfig, Pallet as Balances};
use pallet_tinlake_investor_pool::Config;
use primitives_tokens::CurrencyId;
use smallvec::smallvec;
use sp_arithmetic::Perbill;

primitives_tokens::impl_tranche_token!();

pub struct DealWithFees<Config>(PhantomData<Config>);
pub type NegativeImbalance<Config> =
	<Balances<Config> as Currency<<Config as SystemConfig>::AccountId>>::NegativeImbalance;
impl<Config> OnUnbalanced<NegativeImbalance<Config>> for DealWithFees<Config>
where
	Config: AuthorshipConfig + BalancesConfig + SystemConfig,
{
	fn on_nonzero_unbalanced(amount: NegativeImbalance<Config>) {
		Balances::<Config>::resolve_creating(&Authorship::<Config>::author(), amount);
	}
}

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - [0, frame_system::MaximumBlockWeight]
///   - [Balance::min, Balance::max]
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
///
/// Sample weight to Fee Calculation for 1 Rad Balance transfer:
/// ```rust
/// 	use node_primitives::Balance;
/// 	let extrinsic_bytes: Balance = 92;
/// 	let weight: Balance = 195000000;
/// 	let weight_coefficient: Balance = 315000;
/// 	let transaction_byte_fee: Balance = 10000000000; // 0.01 Micro RAD
///		let maximum_block_weight: Balance = 2000000000000; // 2 * WEIGHT_PER_SECOND
/// 	let extrinsic_base_weight: Balance = 125000000; // 125 * WEIGHT_PER_MICROS
///
/// 	// Calculation:
/// 	let base_fee: Balance = extrinsic_base_weight * weight_coefficient; // 39375000000000
/// 	let length_fee: Balance = extrinsic_bytes * transaction_byte_fee; // 920000000000
/// 	let weight_fee: Balance = weight * weight_coefficient; // 61425000000000
/// 	let fee: Balance = base_fee + length_fee + weight_fee;
/// 	assert_eq!(fee, 10172 * (centrifuge_chain_runtime::constants::currency::MICRO_AIR / 100));
/// ```
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;

	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		smallvec!(WeightToFeeCoefficient {
			coeff_integer: 315000,
			coeff_frac: Perbill::zero(),
			negative: false,
			degree: 1,
		})
	}
}

/// A global identifier for an nft/asset on-chain. Composed of a registry and token id.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, Debug)]
pub struct AssetId(pub RegistryId, pub TokenId);

/// Holds references to its component parts.
pub struct AssetIdRef<'a>(pub &'a RegistryId, pub &'a TokenId);

impl AssetId {
	pub fn destruct(self) -> (RegistryId, TokenId) {
		(self.0, self.1)
	}
}

impl<'a> From<&'a AssetId> for AssetIdRef<'a> {
	fn from(id: &'a AssetId) -> Self {
		AssetIdRef(&id.0, &id.1)
	}
}

impl<'a> AssetIdRef<'a> {
	pub fn destruct(self) -> (&'a RegistryId, &'a TokenId) {
		(self.0, self.1)
	}
}

/// All data for an instance of an NFT.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, Debug)]
pub struct AssetInfo {
	pub metadata: Bytes,
}
