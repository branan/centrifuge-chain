// Copyright 2021 Centrifuge GmbH (centrifuge.io).
// This file is part of Centrifuge chain project.

// Centrifuge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).

// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// Ensure we're `no_std` when compiling for WebAssembly.
#![cfg_attr(not(feature = "std"), no_std)]

//! # A common trait for centrifuge
//!
//! This crate provides some common traits used by centrifuge.
//! # Reward trait
//! The trait does assume, that any call of reward has been
//! checked for validity. I.e. there are not validation checks
//! provided by the trait.

// Ensure we're `no_std` when compiling for WebAssembly.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::dispatch::{Codec, DispatchResult, DispatchResultWithPostInfo};
use frame_support::Parameter;
use sp_runtime::traits::{
	AtLeast32BitUnsigned, Bounded, MaybeDisplay, MaybeMallocSizeOf, MaybeSerialize,
	MaybeSerializeDeserialize, Member, Zero,
};
use sp_runtime::DispatchError;
use sp_std::fmt::Debug;
use sp_std::hash::Hash;
use sp_std::str::FromStr;

/// A trait used for loosely coupling the claim pallet with a reward mechanism.
///
/// ## Overview
/// The crowdloan reward mechanism is separated from the crowdloan claiming process, the latter
/// being generic, acting as a kind of proxy to the rewarding mechanism, that is specific to
/// to each crowdloan campaign. The aim of this pallet is to ensure that a claim for a reward
/// payout is well-formed, checking for replay attacks, spams or invalid claim (e.g. unknown
/// contributor, exceeding reward amount, ...).
/// See the [`crowdloan-reward`] pallet, that implements a reward mechanism with vesting, for
/// instance.
pub trait Reward {
	/// The account from the parachain, that the claimer provided in her/his transaction.
	type ParachainAccountId: Debug
		+ Default
		+ MaybeSerialize
		+ MaybeSerializeDeserialize
		+ Member
		+ Ord
		+ Parameter;

	/// The contribution amount in relay chain tokens.
	type ContributionAmount: AtLeast32BitUnsigned
		+ Codec
		+ Copy
		+ Debug
		+ Default
		+ MaybeSerializeDeserialize
		+ Member
		+ Parameter
		+ Zero;

	/// Block number type used by the runtime
	type BlockNumber: AtLeast32BitUnsigned
		+ Bounded
		+ Copy
		+ Debug
		+ Default
		+ FromStr
		+ Hash
		+ MaybeDisplay
		+ MaybeMallocSizeOf
		+ MaybeSerializeDeserialize
		+ Member
		+ Parameter;

	/// Rewarding function that is invoked from the claim pallet.
	///
	/// If this function returns successfully, any subsequent claim of the same claimer will be
	/// rejected by the claim module.
	fn reward(
		who: Self::ParachainAccountId,
		contribution: Self::ContributionAmount,
	) -> DispatchResultWithPostInfo;
}

/// A trait used to convert a type to BigEndian format
pub trait BigEndian<T> {
	fn to_big_endian(&self) -> T;
}

/// A trait that can be used to fetch the nav and update nav for a given pool
pub trait PoolNAV<PoolId, Amount> {
	// nav returns the nav and the last time it was calculated
	fn nav(pool_id: PoolId) -> Option<(Amount, u64)>;
	fn update_nav(pool_id: PoolId) -> Result<Amount, DispatchError>;
}

/// A trait that support pool reserve operations such as borrow and repay
pub trait PoolReserve<Origin, AccountId> {
	type PoolId: Parameter + Member + Debug + Copy;
	type Balance;

	/// transfers the amount to the account from reserve
	/// caller must be whitelisted.
	fn transfer_to(
		pool_id: Self::PoolId,
		caller: Origin,
		to: AccountId,
		amount: Self::Balance,
	) -> DispatchResult;

	/// transfers the amount from the account to reserve
	/// caller must be whitelisted.
	fn transfer_from(
		pool_id: Self::PoolId,
		caller: Origin,
		from: AccountId,
		amount: Self::Balance,
	) -> DispatchResult;
}
